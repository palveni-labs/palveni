//! Library items (main store).

use palveni_core::{Item, ItemId, ItemKind, LibraryId, LibraryKind, clock};
use sea_orm::sea_query::{Expr, OnConflict};
use sea_orm::{
    ActiveValue, ColumnTrait, Condition, EntityTrait, InsertMany, JoinType, QueryFilter,
    QueryOrder, QuerySelect, TransactionTrait,
};
use serde::{Deserialize, Serialize};

use crate::Db;
use crate::convert::{opt_ts, parse_id, parse_opt_id, sha256_hex, truncate_chars, ts};
use crate::entities::{items, libraries, library_paths};
use crate::error::DbError;
use crate::music::TrackLocation;
use crate::revision;

/// Folders sort before everything else.
pub const SORT_GROUP_FOLDER: i32 = 0;
pub const SORT_GROUP_LEAF: i32 = 1;
/// Length of the `sort_title` column, in characters.
pub const SORT_TITLE_MAX_CHARS: usize = 255;

pub struct Items<'a> {
    pub(crate) db: &'a Db,
}

/// Keyset position in the fixed `(sort_group, sort_title, id)` ordering.
///
/// Encoded for clients as lowercase hex of a compact JSON object: opaque,
/// URL-safe without escaping, and trivially versionable.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemCursor {
    #[serde(rename = "g")]
    pub sort_group: i32,
    #[serde(rename = "t")]
    pub sort_title: String,
    #[serde(rename = "i")]
    pub id: String,
}

impl ItemCursor {
    pub fn encode(&self) -> String {
        hex::encode(serde_json::to_vec(self).unwrap_or_default())
    }

    /// `None` for anything that is not a cursor this server produced.
    pub fn decode(text: &str) -> Option<ItemCursor> {
        if text.len() > 4096 {
            return None;
        }
        let bytes = hex::decode(text).ok()?;
        serde_json::from_slice(&bytes).ok()
    }
}

#[derive(Clone, Debug)]
pub struct ItemQuery {
    pub library_id: LibraryId,
    /// `None` lists the library's top level.
    pub parent_id: Option<ItemId>,
    pub kind: Option<ItemKind>,
    pub limit: u32,
    pub cursor: Option<ItemCursor>,
}

#[derive(Clone, Debug)]
pub struct ItemPage {
    pub items: Vec<Item>,
    pub next_cursor: Option<ItemCursor>,
}

/// What the scanner needs to know about an existing item.
#[derive(Clone, Debug)]
pub struct IndexedItem {
    pub id: ItemId,
    pub path: String,
    pub parent_id: Option<ItemId>,
    pub kind: ItemKind,
    pub size_bytes: Option<i64>,
    pub modified_at: Option<i64>,
    pub deleted: bool,
    pub music_indexed: bool,
    pub cue_stamp: Option<String>,
}

/// An item to insert or update during a scan, keyed by `(library, path)`.
#[derive(Clone, Debug)]
pub struct ItemUpsert {
    pub music: Option<palveni_core::music::TrackTags>,
    pub id: ItemId,
    pub parent_id: Option<ItemId>,
    pub kind: ItemKind,
    pub title: String,
    pub sort_title: String,
    pub search_text: String,
    pub path: String,
    pub size_bytes: Option<i64>,
    /// Unix milliseconds.
    pub modified_at: Option<i64>,
}

const SORT_GROUP_CUE: i32 = 2;

pub fn sort_group_for(kind: ItemKind) -> i32 {
    if kind.is_folder() {
        SORT_GROUP_FOLDER
    } else {
        SORT_GROUP_LEAF
    }
}

impl Items<'_> {
    pub async fn get(&self, id: ItemId) -> Result<Option<Item>, DbError> {
        items::Entity::find_by_id(id.to_string())
            .filter(items::Column::DeletedAt.is_null())
            .one(self.db.main())
            .await?
            .map(to_item)
            .transpose()
    }

    pub async fn original_location(
        &self,
        id: ItemId,
        kind: LibraryKind,
    ) -> Result<Option<TrackLocation>, DbError> {
        let rows = items::Entity::find_by_id(id.to_string())
            .join(
                JoinType::InnerJoin,
                items::Entity::belongs_to(libraries::Entity)
                    .from(items::Column::LibraryId)
                    .to(libraries::Column::Id)
                    .into(),
            )
            .join_rev(
                JoinType::InnerJoin,
                library_paths::Entity::belongs_to(libraries::Entity)
                    .from(library_paths::Column::LibraryId)
                    .to(libraries::Column::Id)
                    .into(),
            )
            .filter(items::Column::Kind.eq(kind.leaf_item_kind().as_str()))
            .filter(items::Column::SortGroup.eq(SORT_GROUP_LEAF))
            .filter(items::Column::Path.is_not_null())
            .filter(items::Column::DeletedAt.is_null())
            .filter(libraries::Column::Kind.eq(kind.as_str()))
            .filter(libraries::Column::DeletedAt.is_null())
            .select_only()
            .column_as(items::Column::Path, "file_path")
            .column_as(library_paths::Column::Path, "root_path")
            .into_tuple::<(String, String)>()
            .all(self.db.main())
            .await?;
        Ok(rows.into_iter().find_map(|(file, root)| {
            let relative_path = std::path::Path::new(&file)
                .strip_prefix(&root)
                .ok()?
                .to_owned();
            Some(TrackLocation {
                cue_frames: None,
                root: root.into(),
                relative_path,
            })
        }))
    }

    /// One page of live items; folders first, then by title.
    pub async fn list(&self, query: &ItemQuery) -> Result<ItemPage, DbError> {
        self.list_in(query, true).await
    }

    /// All book files in the library, including those nested inside folders.
    pub async fn book_catalog(
        &self,
        library_id: LibraryId,
        limit: u32,
        cursor: Option<ItemCursor>,
    ) -> Result<ItemPage, DbError> {
        self.list_in(
            &ItemQuery {
                library_id,
                kind: Some(ItemKind::Book),
                parent_id: None,
                limit,
                cursor,
            },
            false,
        )
        .await
    }

    async fn list_in(&self, query: &ItemQuery, one_folder: bool) -> Result<ItemPage, DbError> {
        let limit = query.limit.clamp(1, 1000);
        let mut condition = Condition::all()
            .add(items::Column::LibraryId.eq(query.library_id.to_string()))
            .add(items::Column::SortGroup.lte(SORT_GROUP_LEAF))
            .add(items::Column::DeletedAt.is_null());
        if one_folder {
            condition = match query.parent_id {
                Some(parent) => condition.add(items::Column::ParentId.eq(parent.to_string())),
                None => condition.add(items::Column::ParentId.is_null()),
            };
        }
        if let Some(kind) = query.kind {
            condition = condition.add(items::Column::Kind.eq(kind.as_str()));
        }
        if let Some(cursor) = &query.cursor {
            let after = Condition::any()
                .add(items::Column::SortGroup.gt(cursor.sort_group))
                .add(
                    Condition::all()
                        .add(items::Column::SortGroup.eq(cursor.sort_group))
                        .add(items::Column::SortTitle.gt(cursor.sort_title.clone())),
                )
                .add(
                    Condition::all()
                        .add(items::Column::SortGroup.eq(cursor.sort_group))
                        .add(items::Column::SortTitle.eq(cursor.sort_title.clone()))
                        .add(items::Column::Id.gt(cursor.id.clone())),
                );
            condition = condition.add(after);
        }

        let mut rows = items::Entity::find()
            .filter(condition)
            .order_by_asc(items::Column::SortGroup)
            .order_by_asc(items::Column::SortTitle)
            .order_by_asc(items::Column::Id)
            .limit(u64::from(limit) + 1)
            .all(self.db.main())
            .await?;

        let page_len = usize::try_from(limit).unwrap_or(usize::MAX);
        let has_more = rows.len() > page_len;
        rows.truncate(page_len);
        let next_cursor = if has_more {
            rows.last().map(|last| ItemCursor {
                sort_group: last.sort_group,
                sort_title: last.sort_title.clone(),
                id: last.id.clone(),
            })
        } else {
            None
        };
        let items = rows
            .into_iter()
            .map(to_item)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(ItemPage { items, next_cursor })
    }

    /// Every item of a library that has a path, including tombstoned ones
    /// (so a reappearing file keeps its ID).
    pub async fn scan_index(&self, library_id: LibraryId) -> Result<Vec<IndexedItem>, DbError> {
        let rows = items::Entity::find()
            .filter(items::Column::LibraryId.eq(library_id.to_string()))
            .filter(items::Column::Path.is_not_null())
            .all(self.db.main())
            .await?;
        let indexed = self.db.music().indexed_item_ids(library_id).await?;
        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let Some(path) = row.path else { continue };
            let track = indexed.get(&row.id);
            out.push(IndexedItem {
                id: parse_id(&row.id, "item id")?,
                parent_id: parse_opt_id(row.parent_id.as_deref(), "item parent id")?,
                kind: parse_kind(&row.kind)?,
                path,
                size_bytes: row.size_bytes,
                modified_at: row.modified_at,
                deleted: row.deleted_at.is_some(),
                music_indexed: track.is_some_and(|t| t.current),
                cue_stamp: track.and_then(|t| t.cue_stamp.clone()),
            });
        }
        Ok(out)
    }

    /// Applies one batch of scan results in a single short transaction.
    ///
    /// Fails with `NotFound("library")` if the library was deleted while
    /// the scan was running, so a scan cannot resurrect its items.
    pub async fn apply_scan_batch(
        &self,
        library_id: LibraryId,
        upserts: &[ItemUpsert],
        tombstones: &[ItemId],
    ) -> Result<(), DbError> {
        if upserts.is_empty() && tombstones.is_empty() {
            return Ok(());
        }
        let now = clock::to_millis(clock::now());
        let txn = self.db.main().begin().await?;
        let rev = revision::next_revision(&txn).await?;

        let live = libraries::Entity::find_by_id(library_id.to_string())
            .filter(libraries::Column::DeletedAt.is_null())
            .one(&txn)
            .await?;
        if live.is_none() {
            txn.rollback().await?;
            return Err(DbError::NotFound("library"));
        }

        write_scan_batch(&txn, library_id, upserts, tombstones, now, rev).await?;
        txn.commit().await?;
        Ok(())
    }
}

pub(crate) async fn write_scan_batch(
    txn: &sea_orm::DatabaseTransaction,
    library_id: LibraryId,
    upserts: &[ItemUpsert],
    tombstones: &[ItemId],
    now: i64,
    rev: i64,
) -> Result<(), DbError> {
    if !upserts.is_empty() {
        upsert_statement(library_id, upserts, now, rev)
            .exec_without_returning(txn)
            .await?;
    }

    if !tombstones.is_empty() {
        let ids: Vec<String> = tombstones.iter().map(ToString::to_string).collect();
        items::Entity::update_many()
            .col_expr(items::Column::DeletedAt, Expr::val(now))
            .col_expr(items::Column::UpdatedAt, Expr::val(now))
            .col_expr(items::Column::Revision, Expr::val(rev))
            .filter(items::Column::LibraryId.eq(library_id.to_string()))
            .filter(items::Column::Id.is_in(ids))
            .filter(items::Column::DeletedAt.is_null())
            .exec(txn)
            .await?;
    }

    crate::music::update(txn, library_id, upserts, tombstones, rev).await
}

/// Multi-row upsert keyed by `(library_id, path_hash)`: `ON CONFLICT … DO
/// UPDATE` on SQLite/PostgreSQL, `ON DUPLICATE KEY UPDATE` on MySQL
/// (sea-query picks the form). `created_at` is never overwritten.
fn upsert_statement(
    library_id: LibraryId,
    upserts: &[ItemUpsert],
    now: i64,
    rev: i64,
) -> InsertMany<items::ActiveModel> {
    let rows = upserts.iter().map(|u| items::ActiveModel {
        id: ActiveValue::Set(u.id.to_string()),
        library_id: ActiveValue::Set(library_id.to_string()),
        parent_id: ActiveValue::Set(u.parent_id.map(|p| p.to_string())),
        kind: ActiveValue::Set(u.kind.as_str().to_owned()),
        title: ActiveValue::Set(u.title.clone()),
        sort_title: ActiveValue::Set(truncate_chars(&u.sort_title, SORT_TITLE_MAX_CHARS)),
        search_text: ActiveValue::Set(u.search_text.clone()),
        sort_group: ActiveValue::Set(if u.music.as_ref().is_some_and(|m| m.cue.is_some()) {
            SORT_GROUP_CUE
        } else {
            sort_group_for(u.kind)
        }),
        path: ActiveValue::Set(Some(u.path.clone())),
        path_hash: ActiveValue::Set(Some(sha256_hex(&u.path))),
        size_bytes: ActiveValue::Set(u.size_bytes),
        modified_at: ActiveValue::Set(u.modified_at),
        created_at: ActiveValue::Set(now),
        updated_at: ActiveValue::Set(now),
        revision: ActiveValue::Set(rev),
        deleted_at: ActiveValue::Set(None),
    });
    let on_conflict = OnConflict::columns([items::Column::LibraryId, items::Column::PathHash])
        .update_columns([
            items::Column::ParentId,
            items::Column::Kind,
            items::Column::Title,
            items::Column::SortTitle,
            items::Column::SearchText,
            items::Column::SortGroup,
            items::Column::Path,
            items::Column::SizeBytes,
            items::Column::ModifiedAt,
            items::Column::UpdatedAt,
            items::Column::Revision,
            items::Column::DeletedAt,
        ])
        .to_owned();
    items::Entity::insert_many(rows).on_conflict(on_conflict)
}

fn parse_kind(kind: &str) -> Result<ItemKind, DbError> {
    kind.parse::<ItemKind>()
        .map_err(|_| DbError::Corrupt(format!("unknown item kind `{kind}`")))
}

fn to_item(row: items::Model) -> Result<Item, DbError> {
    Ok(Item {
        id: parse_id(&row.id, "item id")?,
        library_id: parse_id(&row.library_id, "library id")?,
        parent_id: parse_opt_id(row.parent_id.as_deref(), "item parent id")?,
        kind: parse_kind(&row.kind)?,
        title: row.title,
        path: if row.sort_group == SORT_GROUP_CUE {
            None
        } else {
            row.path
        },
        size_bytes: row.size_bytes,
        modified_at: opt_ts(row.modified_at)?,
        created_at: ts(row.created_at)?,
        updated_at: ts(row.updated_at)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_round_trip() {
        let cursor = ItemCursor {
            sort_group: 1,
            sort_title: "第一章 \"quoted\"".into(),
            id: "0192f3a4-5b6c-7d8e-9f01-23456789abcd".into(),
        };
        let encoded = cursor.encode();
        assert!(encoded.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(ItemCursor::decode(&encoded), Some(cursor));
    }

    #[test]
    fn cursor_rejects_garbage() {
        assert_eq!(ItemCursor::decode("zz"), None);
        assert_eq!(ItemCursor::decode(&hex::encode(b"{}")), None);
        assert_eq!(ItemCursor::decode(&"a".repeat(5000)), None);
    }

    #[test]
    fn upsert_sql_per_backend() {
        use sea_orm::{DbBackend, QueryTrait};

        let rows = [ItemUpsert {
            music: None,
            id: ItemId::generate(),
            parent_id: None,
            kind: ItemKind::Track,
            title: "a".into(),
            sort_title: "a".into(),
            search_text: "a".into(),
            path: "/m/a.flac".into(),
            size_bytes: Some(1),
            modified_at: Some(2),
        }];
        let stmt = upsert_statement(LibraryId::generate(), &rows, 3, 4);
        let sqlite = stmt.build(DbBackend::Sqlite).to_string();
        let postgres = stmt.build(DbBackend::Postgres).to_string();
        let mysql = stmt.build(DbBackend::MySql).to_string();
        for sql in [&sqlite, &postgres] {
            assert!(
                sql.contains(r#"ON CONFLICT ("library_id", "path_hash") DO UPDATE SET"#),
                "{sql}"
            );
            assert!(
                sql.contains(r#""deleted_at" = "excluded"."deleted_at""#),
                "{sql}"
            );
            assert!(!sql.contains(r#""created_at" = "excluded"#), "{sql}");
        }
        assert!(mysql.contains("ON DUPLICATE KEY UPDATE"), "{mysql}");
        assert!(
            mysql.contains("`deleted_at` = VALUES(`deleted_at`)"),
            "{mysql}"
        );
        assert!(!mysql.contains("`created_at` = VALUES"), "{mysql}");
    }

    #[test]
    fn folders_sort_first() {
        assert!(sort_group_for(ItemKind::Folder) < sort_group_for(ItemKind::Track));
    }
}

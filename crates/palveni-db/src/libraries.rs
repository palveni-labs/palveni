//! Libraries and their storage locations (main store).

use chrono::{DateTime, Utc};
use palveni_core::{Library, LibraryId, LibraryKind, LibraryPathId, clock};
use sea_orm::sea_query::Expr;
use sea_orm::{
    ActiveValue, ColumnTrait, ConnectionTrait, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, TransactionTrait,
};

use crate::Db;
use crate::convert::{count_to_i64, opt_ts, parse_id, truncate_chars, ts};
use crate::entities::{items, libraries, library_paths};
use crate::error::DbError;
use crate::items::SORT_GROUP_LEAF;
use crate::revision;

const NAME_MAX_CHARS: usize = 100;

pub struct Libraries<'a> {
    pub(crate) db: &'a Db,
}

#[derive(Clone, Debug)]
pub struct NewLibrary {
    pub name: String,
    pub kind: LibraryKind,
    /// Validated, canonical, de-duplicated absolute paths.
    pub paths: Vec<String>,
}

fn name_key(name: &str) -> String {
    name.trim().to_lowercase()
}

impl Libraries<'_> {
    /// Live libraries ordered by name.
    pub async fn list(&self) -> Result<Vec<Library>, DbError> {
        let rows = libraries::Entity::find()
            .filter(libraries::Column::DeletedAt.is_null())
            .order_by_asc(libraries::Column::NameKey)
            .order_by_asc(libraries::Column::Id)
            .all(self.db.main())
            .await?;
        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            out.push(assemble(self.db.main(), row).await?);
        }
        Ok(out)
    }

    pub async fn get(&self, id: LibraryId) -> Result<Option<Library>, DbError> {
        let row = libraries::Entity::find_by_id(id.to_string())
            .filter(libraries::Column::DeletedAt.is_null())
            .one(self.db.main())
            .await?;
        match row {
            Some(row) => Ok(Some(assemble(self.db.main(), row).await?)),
            None => Ok(None),
        }
    }

    /// Creates a library. Fails with `Conflict` if a live library already
    /// has the same name (case-insensitive).
    pub async fn create(&self, new: NewLibrary) -> Result<Library, DbError> {
        let name = truncate_chars(new.name.trim(), NAME_MAX_CHARS);
        let key = name_key(&name);
        let now = clock::now();
        let now_ms = clock::to_millis(now);
        let id = LibraryId::generate();

        let txn = self.db.main().begin().await?;
        let rev = revision::next_revision(&txn).await?;
        let clashes = libraries::Entity::find()
            .filter(libraries::Column::NameKey.eq(key.clone()))
            .filter(libraries::Column::DeletedAt.is_null())
            .count(&txn)
            .await?;
        if clashes > 0 {
            txn.rollback().await?;
            return Err(DbError::Conflict(format!(
                "A library named \"{name}\" already exists"
            )));
        }
        libraries::Entity::insert(libraries::ActiveModel {
            id: ActiveValue::Set(id.to_string()),
            name: ActiveValue::Set(name.clone()),
            name_key: ActiveValue::Set(key),
            kind: ActiveValue::Set(new.kind.as_str().to_owned()),
            last_scan_at: ActiveValue::Set(None),
            created_at: ActiveValue::Set(now_ms),
            updated_at: ActiveValue::Set(now_ms),
            revision: ActiveValue::Set(rev),
            deleted_at: ActiveValue::Set(None),
        })
        .exec_without_returning(&txn)
        .await?;
        if !new.paths.is_empty() {
            let path_rows =
                new.paths
                    .iter()
                    .enumerate()
                    .map(|(index, path)| library_paths::ActiveModel {
                        id: ActiveValue::Set(LibraryPathId::generate().to_string()),
                        library_id: ActiveValue::Set(id.to_string()),
                        path: ActiveValue::Set(path.clone()),
                        sort_order: ActiveValue::Set(i32::try_from(index).unwrap_or(i32::MAX)),
                        created_at: ActiveValue::Set(now_ms),
                    });
            library_paths::Entity::insert_many(path_rows)
                .exec_without_returning(&txn)
                .await?;
        }
        txn.commit().await?;

        Ok(Library {
            id,
            name,
            kind: new.kind,
            paths: new.paths,
            item_count: 0,
            last_scan_at: None,
            created_at: now,
            updated_at: now,
        })
    }

    /// Tombstones the library and removes its index. Files on disk are
    /// never touched. Returns `false` if it did not exist.
    ///
    /// Items are hard-deleted: clients drop a deleted library's items when
    /// they see the library tombstone, so per-item tombstones would only
    /// waste space.
    pub async fn delete(&self, id: LibraryId) -> Result<bool, DbError> {
        let now = clock::to_millis(clock::now());
        let txn = self.db.main().begin().await?;
        let rev = revision::next_revision(&txn).await?;
        let result = libraries::Entity::update_many()
            .col_expr(libraries::Column::DeletedAt, Expr::val(now))
            .col_expr(libraries::Column::UpdatedAt, Expr::val(now))
            .col_expr(libraries::Column::Revision, Expr::val(rev))
            .filter(libraries::Column::Id.eq(id.to_string()))
            .filter(libraries::Column::DeletedAt.is_null())
            .exec(&txn)
            .await?;
        if result.rows_affected == 0 {
            txn.rollback().await?;
            return Ok(false);
        }
        items::Entity::delete_many()
            .filter(items::Column::LibraryId.eq(id.to_string()))
            .exec(&txn)
            .await?;
        library_paths::Entity::delete_many()
            .filter(library_paths::Column::LibraryId.eq(id.to_string()))
            .exec(&txn)
            .await?;
        txn.commit().await?;
        Ok(true)
    }

    /// Records the end of a scan. Returns `false` if the library is gone.
    pub async fn mark_scanned(&self, id: LibraryId, at: DateTime<Utc>) -> Result<bool, DbError> {
        let at_ms = clock::to_millis(at);
        let txn = self.db.main().begin().await?;
        let rev = revision::next_revision(&txn).await?;
        let result = libraries::Entity::update_many()
            .col_expr(libraries::Column::LastScanAt, Expr::val(at_ms))
            .col_expr(libraries::Column::UpdatedAt, Expr::val(at_ms))
            .col_expr(libraries::Column::Revision, Expr::val(rev))
            .filter(libraries::Column::Id.eq(id.to_string()))
            .filter(libraries::Column::DeletedAt.is_null())
            .exec(&txn)
            .await?;
        txn.commit().await?;
        Ok(result.rows_affected > 0)
    }
}

async fn assemble<C: ConnectionTrait>(conn: &C, row: libraries::Model) -> Result<Library, DbError> {
    let paths = library_paths::Entity::find()
        .filter(library_paths::Column::LibraryId.eq(row.id.clone()))
        .order_by_asc(library_paths::Column::SortOrder)
        .all(conn)
        .await?
        .into_iter()
        .map(|p| p.path)
        .collect();
    let item_count = items::Entity::find()
        .filter(items::Column::LibraryId.eq(row.id.clone()))
        .filter(items::Column::SortGroup.eq(SORT_GROUP_LEAF))
        .filter(items::Column::DeletedAt.is_null())
        .count(conn)
        .await?;
    Ok(Library {
        id: parse_id(&row.id, "library id")?,
        kind: row
            .kind
            .parse::<LibraryKind>()
            .map_err(|_| DbError::Corrupt(format!("unknown library kind `{}`", row.kind)))?,
        name: row.name,
        paths,
        item_count: count_to_i64(item_count),
        last_scan_at: opt_ts(row.last_scan_at)?,
        created_at: ts(row.created_at)?,
        updated_at: ts(row.updated_at)?,
    })
}

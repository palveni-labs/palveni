//! Books and comics metadata, shelves, reading progress, bookmarks and OPDS catalog.
use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use chrono::{DateTime, Utc};
use palveni_core::reading::{
    CreateBookmarkRequest, ReadingBookmark, ReadingChapterSummary, ReadingEntry, ReadingHome,
    ReadingKind, ReadingManifest, ReadingPage, ReadingProgressUpdate, ReadingReaderMode,
    ReadingSeries, ReadingSort, ReadingSortDir, ReadingUserState,
};
use palveni_core::{BookmarkId, ItemId, LibraryId, LibraryKind, UserId};
use sea_orm::sea_query::{Condition, OnConflict};
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, ConnectionTrait, EntityTrait, JoinType,
    QueryFilter, QueryOrder, QuerySelect, TransactionTrait,
};
use serde::{Deserialize, Serialize};

use crate::convert::{now_ms, opt_ts, parse_id, sha256_hex, truncate_chars, ts};
use crate::entities::{
    items, libraries, library_paths,
    reading_bookmarks::{self, Column as B},
    reading_metadata::{self, Column as M},
    user_item_state::{self as state, Column as S},
};
use crate::items::{ItemCursor, SORT_GROUP_LEAF, SORT_TITLE_MAX_CHARS};
use crate::music::TrackLocation;
use crate::revision::next_revision;
use crate::{Db, DbError};

const INTERNAL_PROGRESS_PREFIX: &str = "p:";
const INTERNAL_PROGRESS_LABEL: &str = "__progress__";
const SERIES_MAX_CHARS: usize = 255;
const PUBLISHER_MAX_CHARS: usize = 255;
const LANGUAGE_MAX_CHARS: usize = 32;
const FORMAT_MAX_CHARS: usize = 16;
const LOCATOR_MAX_CHARS: usize = 512;
const LABEL_MAX_CHARS: usize = 200;
const NOTE_MAX_CHARS: usize = 1000;

pub struct Reading<'a> {
    pub(crate) db: &'a Db,
}

#[derive(Clone, Debug)]
pub struct ReadingMetadataUpsert {
    pub item_id: ItemId,
    pub library_id: LibraryId,
    pub stamp: String,
    pub kind: ReadingKind,
    pub format: String,
    pub title: String,
    pub sort_title: String,
    pub authors: Vec<String>,
    pub series_name: Option<String>,
    pub series_key: Option<String>,
    pub series_index: Option<f64>,
    pub summary: Option<String>,
    pub publisher: Option<String>,
    pub language: Option<String>,
    pub page_count: Option<i32>,
    pub has_cover: bool,
    pub tags: Vec<String>,
}

pub struct ReadingItemLocation {
    pub item_id: ItemId,
    pub library_id: LibraryId,
    pub kind: ReadingKind,
    pub title: String,
    pub file_path: String,
    pub size_bytes: Option<i64>,
    pub modified_at: Option<i64>,
    pub existing_stamp: Option<String>,
    pub location: TrackLocation,
}

#[derive(Clone, Debug, Default)]
pub struct ReadingFilters<'a> {
    pub kind: Option<ReadingKind>,
    pub series: Option<&'a str>,
    pub author: Option<&'a str>,
    pub format: Option<&'a str>,
    pub favorite: Option<bool>,
    pub completed: Option<bool>,
    pub q: Option<&'a str>,
    pub sort: ReadingSort,
    pub dir: Option<ReadingSortDir>,
    pub cursor: Option<ReadingCursor>,
    pub limit: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReadingCursor {
    #[serde(rename = "o")]
    pub offset: u64,
}

impl ReadingCursor {
    pub fn encode(self) -> String {
        hex::encode(serde_json::to_vec(&self).unwrap_or_default())
    }

    pub fn decode(raw: &str) -> Option<Self> {
        if raw.len() > 256 {
            return None;
        }
        let bytes = hex::decode(raw.trim()).ok()?;
        serde_json::from_slice(&bytes).ok()
    }
}

#[derive(Clone, Debug)]
pub struct OpdsShelf {
    pub id: LibraryId,
    pub name: String,
    pub kind: LibraryKind,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug)]
pub struct OpdsEntry {
    pub id: ItemId,
    pub kind: ReadingKind,
    pub format: String,
    pub title: String,
    pub path: Option<String>,
    pub updated_at: DateTime<Utc>,
    pub authors: Vec<String>,
    pub summary: Option<String>,
    pub series_name: Option<String>,
    pub series_index: Option<f64>,
    pub page_count: Option<i32>,
    pub has_cover: bool,
}

#[derive(Clone, Debug)]
pub struct OpdsCatalogPage {
    pub items: Vec<OpdsEntry>,
    pub next_cursor: Option<ItemCursor>,
}

#[derive(sea_orm::FromQueryResult)]
struct JoinedReadingRow {
    item_id: String,
    library_id: String,
    item_kind: String,
    item_title: String,
    item_sort_title: String,
    item_path: Option<String>,
    size_bytes: Option<i64>,
    created_at: i64,
    updated_at: i64,
    meta_kind: Option<String>,
    meta_format: Option<String>,
    meta_title: Option<String>,
    meta_sort_title: Option<String>,
    authors_json: Option<String>,
    series_name: Option<String>,
    series_key: Option<String>,
    series_index: Option<f64>,
    summary: Option<String>,
    publisher: Option<String>,
    language: Option<String>,
    page_count: Option<i32>,
    has_cover: Option<bool>,
    tags_json: Option<String>,
}

struct AssembledRow {
    sort_title: String,
    created_at: i64,
    entry: ReadingEntry,
}

fn progress_locator_id(user: UserId, item: ItemId) -> String {
    let digest = sha256_hex(&format!("{user}:{item}"));
    format!("{INTERNAL_PROGRESS_PREFIX}{}", &digest[..34])
}

fn extension_lower(path: &str) -> String {
    match path.rsplit_once('.') {
        Some((stem, ext)) if !stem.is_empty() && !ext.is_empty() => ext.to_ascii_lowercase(),
        _ => String::new(),
    }
}

fn parse_json_strings(raw: Option<&str>) -> Vec<String> {
    raw.and_then(|s| serde_json::from_str::<Vec<String>>(s).ok())
        .unwrap_or_default()
}

pub fn series_key_for(name: &str) -> Option<String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(truncate_chars(&trimmed.to_lowercase(), SERIES_MAX_CHARS))
    }
}

fn assemble_row(
    row: JoinedReadingRow,
    state_row: Option<&state::Model>,
    locator_row: Option<&reading_bookmarks::Model>,
) -> Result<AssembledRow, DbError> {
    let item_id: ItemId = parse_id(&row.item_id, "item id")?;
    let library_id: LibraryId = parse_id(&row.library_id, "library id")?;
    let kind = row
        .meta_kind
        .as_deref()
        .and_then(ReadingKind::parse)
        .or_else(|| ReadingKind::parse(&row.item_kind))
        .ok_or_else(|| DbError::Corrupt(format!("invalid reading kind `{}`", row.item_kind)))?;
    let format = row
        .meta_format
        .filter(|f| !f.is_empty())
        .unwrap_or_else(|| extension_lower(row.item_path.as_deref().unwrap_or_default()));
    let title = row
        .meta_title
        .filter(|t| !t.is_empty())
        .unwrap_or(row.item_title);
    let sort_title = row
        .meta_sort_title
        .filter(|t| !t.is_empty())
        .unwrap_or(row.item_sort_title);
    let authors = parse_json_strings(row.authors_json.as_deref());
    let tags = parse_json_strings(row.tags_json.as_deref());
    let page_count = row.page_count;
    let has_cover = row.has_cover.unwrap_or(false);
    let added_at = ts(row.created_at)?;

    let user_state = build_user_state(state_row, locator_row, page_count)?;

    Ok(AssembledRow {
        sort_title,
        created_at: row.created_at,
        entry: ReadingEntry {
            item_id,
            library_id,
            kind,
            format,
            title,
            authors,
            series_name: row.series_name,
            series_key: row.series_key,
            series_index: row.series_index,
            summary: row.summary,
            publisher: row.publisher,
            language: row.language,
            page_count,
            has_cover,
            tags,
            size_bytes: row.size_bytes,
            added_at,
            state: user_state,
        },
    })
}

fn build_user_state(
    state_row: Option<&state::Model>,
    locator_row: Option<&reading_bookmarks::Model>,
    meta_page_count: Option<i32>,
) -> Result<ReadingUserState, DbError> {
    let page = state_row
        .map(|s| i32::try_from(s.position_ms.max(0)).unwrap_or(i32::MAX))
        .or_else(|| locator_row.map(|l| l.page.max(0)))
        .unwrap_or(0);
    let completed = state_row.is_some_and(|s| s.completed);
    let favorite = state_row.is_some_and(|s| s.favorite_order.is_some());
    let last_read_at = match state_row {
        Some(s) => opt_ts(s.last_played_at)?,
        None => None,
    };
    let locator = locator_row
        .and_then(|l| l.locator.clone())
        .filter(|s| !s.is_empty());
    let reported_total = locator_row
        .and_then(|l| l.note.as_deref())
        .and_then(|n| n.parse::<i32>().ok())
        .filter(|&n| n > 0);
    let total_pages = meta_page_count.or(reported_total);

    Ok(ReadingUserState {
        page,
        total_pages,
        locator,
        completed,
        favorite,
        last_read_at,
    })
}

fn base_joined_select() -> sea_orm::Select<items::Entity> {
    items::Entity::find()
        .join(
            JoinType::InnerJoin,
            items::Entity::belongs_to(libraries::Entity)
                .from(items::Column::LibraryId)
                .to(libraries::Column::Id)
                .into(),
        )
        .join(
            JoinType::LeftJoin,
            items::Entity::has_one(reading_metadata::Entity)
                .from(items::Column::Id)
                .to(M::ItemId)
                .into(),
        )
        .filter(items::Column::DeletedAt.is_null())
        .filter(libraries::Column::DeletedAt.is_null())
        .filter(items::Column::SortGroup.eq(SORT_GROUP_LEAF))
        .filter(items::Column::Kind.is_in(["book", "comic"]))
        .filter(libraries::Column::Kind.is_in(["book", "comic"]))
        .select_only()
        .column_as(items::Column::Id, "item_id")
        .column_as(items::Column::LibraryId, "library_id")
        .column_as(items::Column::Kind, "item_kind")
        .column_as(items::Column::Title, "item_title")
        .column_as(items::Column::SortTitle, "item_sort_title")
        .column_as(items::Column::Path, "item_path")
        .column_as(items::Column::SizeBytes, "size_bytes")
        .column_as(items::Column::CreatedAt, "created_at")
        .column_as(items::Column::UpdatedAt, "updated_at")
        .column_as(M::Kind, "meta_kind")
        .column_as(M::Format, "meta_format")
        .column_as(M::Title, "meta_title")
        .column_as(M::SortTitle, "meta_sort_title")
        .column_as(M::AuthorsJson, "authors_json")
        .column_as(M::SeriesName, "series_name")
        .column_as(M::SeriesKey, "series_key")
        .column_as(M::SeriesIndex, "series_index")
        .column_as(M::Summary, "summary")
        .column_as(M::Publisher, "publisher")
        .column_as(M::Language, "language")
        .column_as(M::PageCount, "page_count")
        .column_as(M::HasCover, "has_cover")
        .column_as(M::TagsJson, "tags_json")
}

async fn load_user_maps(
    conn: &impl ConnectionTrait,
    user: UserId,
    item_ids: &[String],
) -> Result<
    (
        HashMap<String, state::Model>,
        HashMap<String, reading_bookmarks::Model>,
    ),
    DbError,
> {
    let mut states = HashMap::new();
    let mut locators = HashMap::new();
    if item_ids.is_empty() {
        return Ok((states, locators));
    }
    let user_str = user.to_string();
    for chunk in item_ids.chunks(200) {
        let s_rows = state::Entity::find()
            .filter(S::UserId.eq(&user_str))
            .filter(S::ItemId.is_in(chunk.iter().cloned()))
            .all(conn)
            .await?;
        for r in s_rows {
            states.insert(r.item_id.clone(), r);
        }
        let locator_ids: Vec<String> = chunk
            .iter()
            .filter_map(|id| parse_id::<ItemId>(id, "item id").ok())
            .map(|item_id| progress_locator_id(user, item_id))
            .collect();
        if !locator_ids.is_empty() {
            let l_rows = reading_bookmarks::Entity::find()
                .filter(B::Id.is_in(locator_ids))
                .all(conn)
                .await?;
            for r in l_rows {
                locators.insert(r.item_id.clone(), r);
            }
        }
    }
    Ok((states, locators))
}

impl Reading<'_> {
    pub async fn has_reading_library(&self, library_id: LibraryId) -> Result<bool, DbError> {
        Ok(libraries::Entity::find_by_id(library_id.to_string())
            .filter(libraries::Column::DeletedAt.is_null())
            .filter(libraries::Column::Kind.is_in(["book", "comic"]))
            .one(self.db.main())
            .await?
            .is_some())
    }

    pub async fn stamps(&self, library_id: LibraryId) -> Result<HashMap<ItemId, String>, DbError> {
        let rows = reading_metadata::Entity::find()
            .filter(M::LibraryId.eq(library_id.to_string()))
            .select_only()
            .column(M::ItemId)
            .column(M::Stamp)
            .into_tuple::<(String, String)>()
            .all(self.db.main())
            .await?;
        let mut out = HashMap::with_capacity(rows.len());
        for (id, stamp) in rows {
            out.insert(parse_id(&id, "item id")?, stamp);
        }
        Ok(out)
    }

    /// Live book/comic items in a library, resolved to their canonical library root.
    pub async fn library_candidates(
        &self,
        library_id: LibraryId,
    ) -> Result<Vec<ReadingItemLocation>, DbError> {
        let roots = library_paths::Entity::find()
            .filter(library_paths::Column::LibraryId.eq(library_id.to_string()))
            .order_by_asc(library_paths::Column::SortOrder)
            .select_only()
            .column(library_paths::Column::Path)
            .into_tuple::<String>()
            .all(self.db.main())
            .await?;
        if roots.is_empty() {
            return Ok(Vec::new());
        }
        let rows = items::Entity::find()
            .join(
                JoinType::LeftJoin,
                items::Entity::has_one(reading_metadata::Entity)
                    .from(items::Column::Id)
                    .to(M::ItemId)
                    .into(),
            )
            .filter(items::Column::LibraryId.eq(library_id.to_string()))
            .filter(items::Column::DeletedAt.is_null())
            .filter(items::Column::SortGroup.eq(SORT_GROUP_LEAF))
            .filter(items::Column::Kind.is_in(["book", "comic"]))
            .filter(items::Column::Path.is_not_null())
            .select_only()
            .column(items::Column::Id)
            .column(items::Column::Kind)
            .column(items::Column::Title)
            .column(items::Column::Path)
            .column(items::Column::SizeBytes)
            .column(items::Column::ModifiedAt)
            .column(M::Stamp)
            .into_tuple::<(
                String,
                String,
                String,
                Option<String>,
                Option<i64>,
                Option<i64>,
                Option<String>,
            )>()
            .all(self.db.main())
            .await?;

        let mut out = Vec::with_capacity(rows.len());
        for (id_str, kind_str, title, path_opt, size_bytes, modified_at, existing_stamp) in rows {
            let Some(file_path) = path_opt else { continue };
            let Some(kind) = ReadingKind::parse(&kind_str) else {
                continue;
            };
            let item_id: ItemId = parse_id(&id_str, "item id")?;
            let Some(location) = resolve_against_roots(&file_path, &roots) else {
                continue;
            };
            out.push(ReadingItemLocation {
                item_id,
                library_id,
                kind,
                title,
                file_path,
                size_bytes,
                modified_at,
                existing_stamp,
                location,
            });
        }
        Ok(out)
    }

    /// Resolves a single live book or comic item to its registered root and stamp.
    pub async fn item_location(
        &self,
        item_id: ItemId,
    ) -> Result<Option<ReadingItemLocation>, DbError> {
        let rows = items::Entity::find_by_id(item_id.to_string())
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
            .join(
                JoinType::LeftJoin,
                items::Entity::has_one(reading_metadata::Entity)
                    .from(items::Column::Id)
                    .to(M::ItemId)
                    .into(),
            )
            .filter(items::Column::DeletedAt.is_null())
            .filter(items::Column::SortGroup.eq(SORT_GROUP_LEAF))
            .filter(items::Column::Kind.is_in(["book", "comic"]))
            .filter(items::Column::Path.is_not_null())
            .filter(libraries::Column::DeletedAt.is_null())
            .filter(libraries::Column::Kind.is_in(["book", "comic"]))
            .select_only()
            .column(items::Column::LibraryId)
            .column(items::Column::Kind)
            .column(items::Column::Title)
            .column(items::Column::Path)
            .column(items::Column::SizeBytes)
            .column(items::Column::ModifiedAt)
            .column(M::Stamp)
            .column(library_paths::Column::Path)
            .into_tuple::<(
                String,
                String,
                String,
                Option<String>,
                Option<i64>,
                Option<i64>,
                Option<String>,
                String,
            )>()
            .all(self.db.main())
            .await?;

        for (lib_str, kind_str, title, path_opt, size_bytes, modified_at, existing_stamp, root) in
            rows
        {
            let Some(file_path) = path_opt else { continue };
            let Some(kind) = ReadingKind::parse(&kind_str) else {
                continue;
            };
            let Ok(rel) = Path::new(&file_path).strip_prefix(&root) else {
                continue;
            };
            let library_id: LibraryId = parse_id(&lib_str, "library id")?;
            return Ok(Some(ReadingItemLocation {
                item_id,
                library_id,
                kind,
                title,
                file_path: file_path.clone(),
                size_bytes,
                modified_at,
                existing_stamp,
                location: TrackLocation {
                    cue_frames: None,
                    root: root.into(),
                    relative_path: rel.to_owned(),
                },
            }));
        }
        Ok(None)
    }

    pub async fn upsert_batch(&self, rows: &[ReadingMetadataUpsert]) -> Result<(), DbError> {
        if rows.is_empty() {
            return Ok(());
        }
        for chunk in rows.chunks(200) {
            let models: Vec<reading_metadata::ActiveModel> = chunk
                .iter()
                .map(|u| {
                    let series_name = u
                        .series_name
                        .as_deref()
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .map(|s| truncate_chars(s, SERIES_MAX_CHARS));
                    let series_key = u
                        .series_key
                        .as_deref()
                        .and_then(series_key_for)
                        .or_else(|| series_name.as_deref().and_then(series_key_for));
                    reading_metadata::ActiveModel {
                        item_id: ActiveValue::Set(u.item_id.to_string()),
                        library_id: ActiveValue::Set(u.library_id.to_string()),
                        stamp: ActiveValue::Set(truncate_chars(&u.stamp, 128)),
                        kind: ActiveValue::Set(u.kind.as_str().to_owned()),
                        format: ActiveValue::Set(truncate_chars(
                            &u.format.to_ascii_lowercase(),
                            FORMAT_MAX_CHARS,
                        )),
                        title: ActiveValue::Set(u.title.clone()),
                        sort_title: ActiveValue::Set(truncate_chars(
                            &u.sort_title,
                            SORT_TITLE_MAX_CHARS,
                        )),
                        authors_json: ActiveValue::Set(
                            serde_json::to_string(&u.authors).unwrap_or_else(|_| "[]".to_owned()),
                        ),
                        series_name: ActiveValue::Set(series_name),
                        series_key: ActiveValue::Set(series_key),
                        series_index: ActiveValue::Set(u.series_index),
                        summary: ActiveValue::Set(
                            u.summary
                                .as_deref()
                                .map(str::trim)
                                .filter(|s| !s.is_empty())
                                .map(ToOwned::to_owned),
                        ),
                        publisher: ActiveValue::Set(
                            u.publisher
                                .as_deref()
                                .map(str::trim)
                                .filter(|s| !s.is_empty())
                                .map(|s| truncate_chars(s, PUBLISHER_MAX_CHARS)),
                        ),
                        language: ActiveValue::Set(
                            u.language
                                .as_deref()
                                .map(str::trim)
                                .filter(|s| !s.is_empty())
                                .map(|s| truncate_chars(s, LANGUAGE_MAX_CHARS)),
                        ),
                        page_count: ActiveValue::Set(u.page_count),
                        has_cover: ActiveValue::Set(u.has_cover),
                        tags_json: ActiveValue::Set(
                            serde_json::to_string(&u.tags).unwrap_or_else(|_| "[]".to_owned()),
                        ),
                    }
                })
                .collect();

            let on_conflict = OnConflict::column(M::ItemId)
                .update_columns([
                    M::LibraryId,
                    M::Stamp,
                    M::Kind,
                    M::Format,
                    M::Title,
                    M::SortTitle,
                    M::AuthorsJson,
                    M::SeriesName,
                    M::SeriesKey,
                    M::SeriesIndex,
                    M::Summary,
                    M::Publisher,
                    M::Language,
                    M::PageCount,
                    M::HasCover,
                    M::TagsJson,
                ])
                .to_owned();
            reading_metadata::Entity::insert_many(models)
                .on_conflict(on_conflict)
                .exec_without_returning(self.db.main())
                .await?;
        }
        Ok(())
    }

    pub async fn prune_tombstoned(&self, library_id: LibraryId) -> Result<(), DbError> {
        let deleted_ids = items::Entity::find()
            .filter(items::Column::LibraryId.eq(library_id.to_string()))
            .filter(items::Column::DeletedAt.is_not_null())
            .select_only()
            .column(items::Column::Id)
            .into_tuple::<String>()
            .all(self.db.main())
            .await?;
        for chunk in deleted_ids.chunks(200) {
            reading_metadata::Entity::delete_many()
                .filter(M::ItemId.is_in(chunk.iter().cloned()))
                .exec(self.db.main())
                .await?;
        }
        Ok(())
    }

    pub async fn list(
        &self,
        user: UserId,
        library_id: LibraryId,
        filters: &ReadingFilters<'_>,
    ) -> Result<Option<ReadingPage>, DbError> {
        if !self.has_reading_library(library_id).await? {
            return Ok(None);
        }
        let mut select =
            base_joined_select().filter(items::Column::LibraryId.eq(library_id.to_string()));
        if let Some(kind) = filters.kind {
            select = select.filter(items::Column::Kind.eq(kind.as_str()));
        }
        let rows = select
            .into_model::<JoinedReadingRow>()
            .all(self.db.main())
            .await?;
        let item_ids: Vec<String> = rows.iter().map(|r| r.item_id.clone()).collect();
        let (states, locators) = load_user_maps(self.db.main(), user, &item_ids).await?;

        let series_filter = filters
            .series
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_lowercase);
        let author_filter = filters
            .author
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_lowercase);
        let format_filter = filters
            .format
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_ascii_lowercase);
        let q_filter = filters
            .q
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_lowercase);

        let mut assembled = Vec::with_capacity(rows.len());
        for row in rows {
            let state_row = states.get(&row.item_id);
            let loc_row = locators.get(&row.item_id);
            let item = assemble_row(row, state_row, loc_row)?;

            if let Some(ref s) = series_filter {
                let matched = item
                    .entry
                    .series_key
                    .as_deref()
                    .is_some_and(|k| k == s.as_str())
                    || item
                        .entry
                        .series_name
                        .as_deref()
                        .is_some_and(|n| n.to_lowercase() == *s);
                if !matched {
                    continue;
                }
            }
            if let Some(ref a) = author_filter
                && !item
                    .entry
                    .authors
                    .iter()
                    .any(|author| author.to_lowercase() == *a || author.to_lowercase().contains(a))
            {
                continue;
            }
            if let Some(ref f) = format_filter
                && item.entry.format != *f
            {
                continue;
            }
            if let Some(fav) = filters.favorite
                && item.entry.state.favorite != fav
            {
                continue;
            }
            if let Some(comp) = filters.completed
                && item.entry.state.completed != comp
            {
                continue;
            }
            if let Some(ref q) = q_filter {
                let matches_title = item.entry.title.to_lowercase().contains(q);
                let matches_author = item
                    .entry
                    .authors
                    .iter()
                    .any(|a| a.to_lowercase().contains(q));
                let matches_series = item
                    .entry
                    .series_name
                    .as_deref()
                    .is_some_and(|s| s.to_lowercase().contains(q));
                let matches_tag = item.entry.tags.iter().any(|t| t.to_lowercase().contains(q));
                let matches_summary = item
                    .entry
                    .summary
                    .as_deref()
                    .is_some_and(|s| s.to_lowercase().contains(q));
                if !(matches_title
                    || matches_author
                    || matches_series
                    || matches_tag
                    || matches_summary)
                {
                    continue;
                }
            }
            assembled.push(item);
        }

        let dir = filters.dir.unwrap_or_else(|| filters.sort.default_dir());
        sort_assembled(&mut assembled, filters.sort, dir);

        let total = u64::try_from(assembled.len()).unwrap_or(u64::MAX);
        let limit = filters.limit.clamp(1, 200);
        let offset = filters.cursor.map(|c| c.offset).unwrap_or(0);
        let start = usize::try_from(offset).unwrap_or(usize::MAX);
        let end = start.saturating_add(usize::try_from(limit).unwrap_or(50));

        let entries: Vec<ReadingEntry> = if start >= assembled.len() {
            Vec::new()
        } else {
            let capped = end.min(assembled.len());
            assembled
                .into_iter()
                .skip(start)
                .take(capped - start)
                .map(|a| a.entry)
                .collect()
        };

        let next_offset = offset.saturating_add(u64::from(limit));
        let next_cursor = (next_offset < total).then(|| {
            ReadingCursor {
                offset: next_offset,
            }
            .encode()
        });

        Ok(Some(ReadingPage {
            entries,
            next_cursor,
            total,
        }))
    }

    pub async fn home(
        &self,
        user: UserId,
        library_id: Option<LibraryId>,
    ) -> Result<Option<ReadingHome>, DbError> {
        let mut select = base_joined_select();
        if let Some(lib_id) = library_id {
            if !self.has_reading_library(lib_id).await? {
                return Ok(None);
            }
            select = select.filter(items::Column::LibraryId.eq(lib_id.to_string()));
        }
        let rows = select
            .into_model::<JoinedReadingRow>()
            .all(self.db.main())
            .await?;
        let item_ids: Vec<String> = rows.iter().map(|r| r.item_id.clone()).collect();
        let (states, locators) = load_user_maps(self.db.main(), user, &item_ids).await?;

        let mut all = Vec::with_capacity(rows.len());
        for row in rows {
            let state_row = states.get(&row.item_id);
            let loc_row = locators.get(&row.item_id);
            all.push(assemble_row(row, state_row, loc_row)?);
        }

        // Continue reading: started & not completed, ordered by last_read_at desc.
        let mut continuing: Vec<&AssembledRow> = all
            .iter()
            .filter(|a| {
                !a.entry.state.completed
                    && a.entry.state.last_read_at.is_some()
                    && (a.entry.state.page > 0 || a.entry.state.locator.is_some())
            })
            .collect();
        continuing.sort_by(|a, b| {
            b.entry
                .state
                .last_read_at
                .cmp(&a.entry.state.last_read_at)
                .then_with(|| b.created_at.cmp(&a.created_at))
                .then_with(|| a.entry.item_id.cmp(&b.entry.item_id))
        });
        let continue_reading: Vec<ReadingEntry> = continuing
            .into_iter()
            .take(16)
            .map(|a| a.entry.clone())
            .collect();

        // Recent books & recent comics
        let mut recent_sorted: Vec<&AssembledRow> = all.iter().collect();
        recent_sorted.sort_by(|a, b| {
            b.created_at
                .cmp(&a.created_at)
                .then_with(|| b.entry.item_id.cmp(&a.entry.item_id))
        });
        let recent_books: Vec<ReadingEntry> = recent_sorted
            .iter()
            .filter(|a| a.entry.kind == ReadingKind::Book)
            .take(16)
            .map(|a| a.entry.clone())
            .collect();
        let recent_comics: Vec<ReadingEntry> = recent_sorted
            .iter()
            .filter(|a| a.entry.kind == ReadingKind::Comic)
            .take(16)
            .map(|a| a.entry.clone())
            .collect();

        // Series shelves
        let mut series_groups: BTreeMap<(LibraryId, String, String), Vec<&AssembledRow>> =
            BTreeMap::new();
        for item in &all {
            if let (Some(key), Some(_name)) = (&item.entry.series_key, &item.entry.series_name) {
                series_groups
                    .entry((
                        item.entry.library_id,
                        item.entry.kind.as_str().to_owned(),
                        key.clone(),
                    ))
                    .or_default()
                    .push(item);
            }
        }
        let mut series = Vec::with_capacity(series_groups.len());
        for ((lib_id, _, key), mut members) in series_groups {
            members.sort_by(|a, b| {
                cmp_opt_f64(a.entry.series_index, b.entry.series_index)
                    .then_with(|| a.sort_title.cmp(&b.sort_title))
                    .then_with(|| a.entry.item_id.cmp(&b.entry.item_id))
            });
            let Some(first) = members.first() else {
                continue;
            };
            let cover_member = members.iter().find(|m| m.entry.has_cover).unwrap_or(first);
            let completed_count =
                u32::try_from(members.iter().filter(|m| m.entry.state.completed).count())
                    .unwrap_or(u32::MAX);
            let item_count = u32::try_from(members.len()).unwrap_or(u32::MAX);
            series.push(ReadingSeries {
                series_key: key,
                series_name: first
                    .entry
                    .series_name
                    .clone()
                    .unwrap_or_else(|| first.entry.title.clone()),
                library_id: lib_id,
                kind: first.entry.kind,
                item_count,
                completed_count,
                cover_item_id: cover_member.entry.item_id,
                has_cover: cover_member.entry.has_cover,
            });
        }
        series.sort_by(|a, b| {
            a.series_name
                .to_lowercase()
                .cmp(&b.series_name.to_lowercase())
                .then_with(|| a.library_id.cmp(&b.library_id))
        });
        series.truncate(24);

        Ok(Some(ReadingHome {
            continue_reading,
            recent_books,
            recent_comics,
            series,
        }))
    }

    pub async fn entry(
        &self,
        user: UserId,
        item_id: ItemId,
    ) -> Result<Option<ReadingEntry>, DbError> {
        let Some(row) = base_joined_select()
            .filter(items::Column::Id.eq(item_id.to_string()))
            .into_model::<JoinedReadingRow>()
            .one(self.db.main())
            .await?
        else {
            return Ok(None);
        };
        let ids = [row.item_id.clone()];
        let (states, locators) = load_user_maps(self.db.main(), user, &ids).await?;
        let assembled = assemble_row(row, states.get(&ids[0]), locators.get(&ids[0]))?;
        Ok(Some(assembled.entry))
    }

    pub async fn manifest(
        &self,
        user: UserId,
        item_id: ItemId,
        chapters: Vec<ReadingChapterSummary>,
    ) -> Result<Option<ReadingManifest>, DbError> {
        let Some(entry) = self.entry(user, item_id).await? else {
            return Ok(None);
        };
        let bookmarks = self.bookmarks(user, item_id).await?;
        let reader_mode = ReadingReaderMode::from_kind_and_format(entry.kind, &entry.format);
        Ok(Some(ReadingManifest {
            entry,
            reader_mode,
            chapters,
            bookmarks,
        }))
    }

    pub async fn bookmarks(
        &self,
        user: UserId,
        item_id: ItemId,
    ) -> Result<Vec<ReadingBookmark>, DbError> {
        let rows = reading_bookmarks::Entity::find()
            .filter(B::UserId.eq(user.to_string()))
            .filter(B::ItemId.eq(item_id.to_string()))
            .filter(B::Id.not_like(format!("{INTERNAL_PROGRESS_PREFIX}%")))
            .order_by_asc(B::Page)
            .order_by_asc(B::CreatedAt)
            .order_by_asc(B::Id)
            .all(self.db.main())
            .await?;
        rows.into_iter().map(to_bookmark).collect()
    }

    pub async fn save_progress(
        &self,
        user: UserId,
        item_id: ItemId,
        update: ReadingProgressUpdate,
    ) -> Result<ReadingUserState, DbError> {
        if update.page < 0 {
            return Err(DbError::Invalid("page must be nonnegative"));
        }
        if update.total_pages.is_some_and(|t| t < 1) {
            return Err(DbError::Invalid("totalPages must be at least 1"));
        }
        let locator = update
            .locator
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| truncate_chars(s, LOCATOR_MAX_CHARS));

        let txn = self.db.main().begin().await?;
        let live_meta = items::Entity::find_by_id(item_id.to_string())
            .join(
                JoinType::InnerJoin,
                items::Entity::belongs_to(libraries::Entity)
                    .from(items::Column::LibraryId)
                    .to(libraries::Column::Id)
                    .into(),
            )
            .join(
                JoinType::LeftJoin,
                items::Entity::has_one(reading_metadata::Entity)
                    .from(items::Column::Id)
                    .to(M::ItemId)
                    .into(),
            )
            .filter(items::Column::DeletedAt.is_null())
            .filter(items::Column::SortGroup.eq(SORT_GROUP_LEAF))
            .filter(items::Column::Kind.is_in(["book", "comic"]))
            .filter(libraries::Column::DeletedAt.is_null())
            .select_only()
            .column(M::PageCount)
            .into_tuple::<(Option<i32>,)>()
            .one(&txn)
            .await?;
        let Some((meta_page_count,)) = live_meta else {
            txn.rollback().await?;
            return Err(DbError::NotFound("Reading item"));
        };

        let rev = next_revision(&txn).await?;
        let now = now_ms();
        let key = (user.to_string(), item_id.to_string());
        let existing_state = state::Entity::find_by_id(key.clone()).one(&txn).await?;
        let was_completed = existing_state.as_ref().is_some_and(|s| s.completed);

        let state_model = match existing_state {
            Some(mut row) => {
                row.previous_version = Some(row.playback_version);
                row.playback_version = rev;
                row.position_ms = i64::from(update.page);
                row.completed = update.completed;
                row.last_played_at = Some(now);
                if update.completed && !was_completed {
                    row.play_count = row.play_count.saturating_add(1);
                } else if update.completed && row.play_count == 0 {
                    row.play_count = 1;
                }
                let updated = row.clone();
                let active: state::ActiveModel = row.into();
                active.reset_all().update(&txn).await?;
                updated
            }
            None => {
                let row = state::Model {
                    user_id: key.0,
                    item_id: key.1,
                    favorite_order: None,
                    stars: 0,
                    position_ms: i64::from(update.page),
                    completed: update.completed,
                    play_count: i64::from(update.completed),
                    last_played_at: Some(now),
                    playback_version: rev,
                    previous_version: Some(0),
                };
                state::Entity::insert(state::ActiveModel::from(row.clone()))
                    .exec_without_returning(&txn)
                    .await?;
                row
            }
        };

        let loc_id = progress_locator_id(user, item_id);
        let total_str = update.total_pages.map(|t| t.to_string());
        let loc_model = reading_bookmarks::Model {
            id: loc_id.clone(),
            user_id: user.to_string(),
            item_id: item_id.to_string(),
            page: update.page,
            locator,
            label: INTERNAL_PROGRESS_LABEL.to_owned(),
            note: total_str,
            created_at: now,
        };
        let on_conflict = OnConflict::column(B::Id)
            .update_columns([B::Page, B::Locator, B::Note, B::CreatedAt])
            .to_owned();
        reading_bookmarks::Entity::insert(reading_bookmarks::ActiveModel::from(loc_model.clone()))
            .on_conflict(on_conflict)
            .exec_without_returning(&txn)
            .await?;

        txn.commit().await?;
        build_user_state(Some(&state_model), Some(&loc_model), meta_page_count)
    }

    pub async fn create_bookmark(
        &self,
        user: UserId,
        item_id: ItemId,
        req: CreateBookmarkRequest,
    ) -> Result<ReadingBookmark, DbError> {
        if req.page < 0 {
            return Err(DbError::Invalid("page must be nonnegative"));
        }
        let label = truncate_chars(req.label.trim(), LABEL_MAX_CHARS);
        if label.is_empty() {
            return Err(DbError::Invalid("label cannot be empty"));
        }
        let locator = req
            .locator
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| truncate_chars(s, LOCATOR_MAX_CHARS));
        let note = req
            .note
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| truncate_chars(s, NOTE_MAX_CHARS));

        let exists = items::Entity::find_by_id(item_id.to_string())
            .join(
                JoinType::InnerJoin,
                items::Entity::belongs_to(libraries::Entity)
                    .from(items::Column::LibraryId)
                    .to(libraries::Column::Id)
                    .into(),
            )
            .filter(items::Column::DeletedAt.is_null())
            .filter(items::Column::SortGroup.eq(SORT_GROUP_LEAF))
            .filter(items::Column::Kind.is_in(["book", "comic"]))
            .filter(libraries::Column::DeletedAt.is_null())
            .select_only()
            .column(items::Column::Id)
            .into_tuple::<String>()
            .one(self.db.main())
            .await?
            .is_some();
        if !exists {
            return Err(DbError::NotFound("Reading item"));
        }

        let id = BookmarkId::generate();
        let now = now_ms();
        let model = reading_bookmarks::Model {
            id: id.to_string(),
            user_id: user.to_string(),
            item_id: item_id.to_string(),
            page: req.page,
            locator,
            label,
            note,
            created_at: now,
        };
        reading_bookmarks::Entity::insert(reading_bookmarks::ActiveModel::from(model.clone()))
            .exec_without_returning(self.db.main())
            .await?;
        to_bookmark(model)
    }

    pub async fn delete_bookmark(
        &self,
        user: UserId,
        bookmark_id: BookmarkId,
    ) -> Result<bool, DbError> {
        let res = reading_bookmarks::Entity::delete_many()
            .filter(B::Id.eq(bookmark_id.to_string()))
            .filter(B::UserId.eq(user.to_string()))
            .exec(self.db.main())
            .await?;
        Ok(res.rows_affected > 0)
    }

    /// OPDS 1.2 shelves: both `Book` and `Comic` libraries.
    pub async fn opds_shelves(&self, id: Option<LibraryId>) -> Result<Vec<OpdsShelf>, DbError> {
        let mut query = libraries::Entity::find()
            .filter(libraries::Column::DeletedAt.is_null())
            .filter(libraries::Column::Kind.is_in(["book", "comic"]));
        if let Some(id) = id {
            query = query.filter(libraries::Column::Id.eq(id.to_string()));
        }
        query
            .order_by_asc(libraries::Column::NameKey)
            .order_by_asc(libraries::Column::Id)
            .select_only()
            .column(libraries::Column::Id)
            .column(libraries::Column::Name)
            .column(libraries::Column::Kind)
            .column(libraries::Column::UpdatedAt)
            .into_tuple::<(String, String, String, i64)>()
            .all(self.db.main())
            .await?
            .into_iter()
            .map(|(id, name, kind_str, updated)| {
                let kind = kind_str
                    .parse::<LibraryKind>()
                    .map_err(|_| DbError::Corrupt(format!("invalid library kind `{kind_str}`")))?;
                Ok(OpdsShelf {
                    id: parse_id(&id, "library id")?,
                    name,
                    kind,
                    updated_at: ts(updated)?,
                })
            })
            .collect()
    }

    /// OPDS 1.2 acquisition feed page for a Book or Comic library, preserving keyset cursor.
    pub async fn opds_catalog(
        &self,
        library_id: LibraryId,
        limit: u32,
        cursor: Option<ItemCursor>,
    ) -> Result<OpdsCatalogPage, DbError> {
        let limit = limit.clamp(1, 200);
        let mut condition = Condition::all()
            .add(items::Column::LibraryId.eq(library_id.to_string()))
            .add(items::Column::SortGroup.eq(SORT_GROUP_LEAF))
            .add(items::Column::Kind.is_in(["book", "comic"]))
            .add(items::Column::DeletedAt.is_null());

        if let Some(ref c) = cursor {
            let after = Condition::any()
                .add(items::Column::SortGroup.gt(c.sort_group))
                .add(
                    Condition::all()
                        .add(items::Column::SortGroup.eq(c.sort_group))
                        .add(items::Column::SortTitle.gt(c.sort_title.clone())),
                )
                .add(
                    Condition::all()
                        .add(items::Column::SortGroup.eq(c.sort_group))
                        .add(items::Column::SortTitle.eq(c.sort_title.clone()))
                        .add(items::Column::Id.gt(c.id.clone())),
                );
            condition = condition.add(after);
        }

        let mut rows = base_joined_select()
            .filter(condition)
            .order_by_asc(items::Column::SortGroup)
            .order_by_asc(items::Column::SortTitle)
            .order_by_asc(items::Column::Id)
            .limit(u64::from(limit) + 1)
            .into_model::<JoinedReadingRow>()
            .all(self.db.main())
            .await?;

        let page_len = usize::try_from(limit).unwrap_or(usize::MAX);
        let has_more = rows.len() > page_len;
        rows.truncate(page_len);
        let next_cursor = if has_more {
            rows.last().map(|last| ItemCursor {
                sort_group: SORT_GROUP_LEAF,
                sort_title: last.item_sort_title.clone(),
                id: last.item_id.clone(),
            })
        } else {
            None
        };

        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let id: ItemId = parse_id(&row.item_id, "item id")?;
            let kind = row
                .meta_kind
                .as_deref()
                .and_then(ReadingKind::parse)
                .or_else(|| ReadingKind::parse(&row.item_kind))
                .unwrap_or(ReadingKind::Book);
            let format = row
                .meta_format
                .filter(|f| !f.is_empty())
                .unwrap_or_else(|| extension_lower(row.item_path.as_deref().unwrap_or_default()));
            let title = row
                .meta_title
                .filter(|t| !t.is_empty())
                .unwrap_or(row.item_title);
            out.push(OpdsEntry {
                id,
                kind,
                format,
                title,
                path: row.item_path,
                updated_at: ts(row.updated_at)?,
                authors: parse_json_strings(row.authors_json.as_deref()),
                summary: row.summary,
                series_name: row.series_name,
                series_index: row.series_index,
                page_count: row.page_count,
                has_cover: row.has_cover.unwrap_or(false),
            });
        }
        Ok(OpdsCatalogPage {
            items: out,
            next_cursor,
        })
    }
}

fn resolve_against_roots(file_path: &str, roots: &[String]) -> Option<TrackLocation> {
    let p = Path::new(file_path);
    for root in roots {
        if let Ok(rel) = p.strip_prefix(root) {
            return Some(TrackLocation {
                cue_frames: None,
                root: root.into(),
                relative_path: rel.to_owned(),
            });
        }
    }
    None
}

fn to_bookmark(row: reading_bookmarks::Model) -> Result<ReadingBookmark, DbError> {
    Ok(ReadingBookmark {
        id: parse_id(&row.id, "bookmark id")?,
        item_id: parse_id(&row.item_id, "item id")?,
        page: row.page,
        locator: row.locator,
        label: row.label,
        note: row.note,
        created_at: ts(row.created_at)?,
    })
}

fn cmp_opt_f64(a: Option<f64>, b: Option<f64>) -> std::cmp::Ordering {
    match (a, b) {
        (Some(x), Some(y)) => x.total_cmp(&y),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    }
}

fn sort_assembled(rows: &mut [AssembledRow], sort: ReadingSort, dir: ReadingSortDir) {
    rows.sort_by(|a, b| {
        let ord = match sort {
            ReadingSort::Added => a
                .created_at
                .cmp(&b.created_at)
                .then_with(|| a.entry.item_id.cmp(&b.entry.item_id)),
            ReadingSort::Title => a
                .sort_title
                .cmp(&b.sort_title)
                .then_with(|| a.entry.item_id.cmp(&b.entry.item_id)),
            ReadingSort::Series => match (&a.entry.series_key, &b.entry.series_key) {
                (Some(ka), Some(kb)) => {
                    let base = ka
                        .cmp(kb)
                        .then_with(|| cmp_opt_f64(a.entry.series_index, b.entry.series_index))
                        .then_with(|| a.sort_title.cmp(&b.sort_title))
                        .then_with(|| a.entry.item_id.cmp(&b.entry.item_id));
                    return match dir {
                        ReadingSortDir::Asc => base,
                        ReadingSortDir::Desc => base.reverse(),
                    };
                }
                (Some(_), None) => return std::cmp::Ordering::Less,
                (None, Some(_)) => return std::cmp::Ordering::Greater,
                (None, None) => a
                    .sort_title
                    .cmp(&b.sort_title)
                    .then_with(|| a.entry.item_id.cmp(&b.entry.item_id)),
            },
        };
        match dir {
            ReadingSortDir::Asc => ord,
            ReadingSortDir::Desc => ord.reverse(),
        }
    });
}

//! Client API keys (Jellyfin/WebDAV/general), video intro/credits segments,
//! and Jellyfin-compatible catalog & playback persistence helpers.

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};
use palveni_core::jellyfin::{
    ClientApiKeyKind, ClientApiKeySummary, CreatedClientApiKey, MAX_API_KEY_NAME_CHARS,
    MAX_SEGMENTS_PER_ITEM, VideoSegment, VideoSegmentInput, VideoSegmentKind, VideoSegmentSource,
};
use palveni_core::personal::MediaState;
use palveni_core::{ItemId, ItemKind, LibraryId, User, UserId, clock};
use sea_orm::sea_query::Expr;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter, QueryOrder,
    TransactionTrait,
};

use crate::Db;
use crate::convert::{now_ms, opt_ts, parse_id, sha256_hex, truncate_chars, ts};
use crate::entities::{
    client_api_key, items, track_metadata, user_item_state, users, video_favorites, video_genres,
    video_metadata, video_segment,
};
use crate::error::DbError;
use crate::users::to_user;

const API_KEY_PREFIX: &str = "pvk_";
const API_KEY_BYTES: usize = 24;
const API_KEY_TOUCH_INTERVAL_MS: i64 = 60_000;

pub struct Jellyfin<'a> {
    pub(crate) db: &'a Db,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChapterMarker {
    pub start_ms: u64,
    pub title: Option<String>,
}

#[derive(Clone, Debug)]
pub struct JellyfinVideoRecord {
    pub item_id: ItemId,
    pub library_id: LibraryId,
    pub title: String,
    pub path: Option<String>,
    pub size_bytes: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub work_key: String,
    pub work_title: String,
    pub work_original_title: Option<String>,
    pub work_year: Option<i32>,
    pub work_overview: Option<String>,
    pub work_rating: Option<f64>,
    pub work_official_rating: Option<String>,
    pub work_premiered: Option<String>,
    pub work_studios: Vec<String>,
    pub work_poster: bool,
    pub work_backdrop: bool,
    pub season: Option<i32>,
    pub season_poster: bool,
    pub episode: Option<i32>,
    pub episode_poster: bool,
    pub overview: Option<String>,
    pub runtime_ms: Option<i64>,
    pub rating: Option<f64>,
    pub premiered: Option<String>,
    pub thumb: bool,
    pub genres: Vec<String>,
    pub state: MediaState,
    pub work_favorite: bool,
}

#[derive(Clone, Debug)]
pub struct JellyfinAudioRecord {
    pub item_id: ItemId,
    pub library_id: LibraryId,
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub album_artist: Option<String>,
    pub track_number: Option<i64>,
    pub disc_number: Option<i64>,
    pub year: Option<i64>,
    pub duration_ms: Option<i64>,
    pub codec: Option<String>,
    pub bitrate_kbps: Option<i64>,
    pub path: Option<String>,
    pub size_bytes: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub state: MediaState,
}

#[derive(Clone, Debug)]
pub struct JellyfinItemLookup {
    pub item_id: ItemId,
    pub library_id: LibraryId,
    pub kind: ItemKind,
    pub title: String,
    pub path: Option<String>,
    pub size_bytes: Option<i64>,
    pub work_key: Option<String>,
}

fn generate_api_key() -> Result<String, DbError> {
    let mut bytes = [0u8; API_KEY_BYTES];
    getrandom::fill(&mut bytes)
        .map_err(|e| DbError::Internal(format!("system random source failed: {e}")))?;
    Ok(format!("{API_KEY_PREFIX}{}", hex::encode(bytes)))
}

fn api_key_summary(row: client_api_key::Model) -> Result<ClientApiKeySummary, DbError> {
    Ok(ClientApiKeySummary {
        id: row.id,
        name: row.name,
        client_kind: ClientApiKeyKind::parse(&row.client_kind).unwrap_or(ClientApiKeyKind::General),
        key_prefix: row.key_prefix,
        last_used_at: opt_ts(row.last_used_at)?,
        revoked_at: opt_ts(row.revoked_at)?,
        created_at: ts(row.created_at)?,
    })
}

fn segment_from_row(row: video_segment::Model) -> Result<VideoSegment, DbError> {
    let item_id: ItemId = parse_id(&row.item_id, "item id")?;
    let kind = VideoSegmentKind::parse(&row.kind)
        .ok_or_else(|| DbError::Corrupt(format!("invalid video segment kind `{}`", row.kind)))?;
    let source = VideoSegmentSource::parse(&row.source).unwrap_or(VideoSegmentSource::Manual);
    Ok(VideoSegment {
        id: row.id,
        item_id,
        kind,
        start_ms: row.start_ms,
        end_ms: row.end_ms,
        source,
        updated_at: ts(row.updated_at)?,
    })
}

fn classify_chapter_title(title: &str) -> Option<VideoSegmentKind> {
    let trimmed = title.trim();
    if trimmed.is_empty() {
        return None;
    }
    let lower = trimmed.to_ascii_lowercase();
    if matches!(
        lower.as_str(),
        "op" | "opening"
            | "intro"
            | "opening credits"
            | "opening theme"
            | "title sequence"
            | "op1"
            | "op2"
    ) || matches!(trimmed, "片头" | "片头曲" | "主题曲" | "片頭" | "片頭曲")
        || lower.starts_with("op ")
        || lower.starts_with("op:")
        || lower.starts_with("op-")
        || lower.starts_with("opening ")
        || lower.starts_with("opening:")
        || lower.starts_with("opening-")
        || lower.starts_with("intro ")
        || lower.starts_with("intro:")
        || lower.starts_with("intro-")
        || lower.ends_with(" op")
        || lower.ends_with(" opening")
        || lower.ends_with(" intro")
        || trimmed.starts_with("片头")
        || trimmed.starts_with("片頭")
    {
        return Some(VideoSegmentKind::Intro);
    }
    if matches!(
        lower.as_str(),
        "ed" | "ending"
            | "credits"
            | "outro"
            | "end credits"
            | "closing"
            | "closing credits"
            | "ending theme"
            | "ed1"
            | "ed2"
    ) || matches!(
        trimmed,
        "片尾" | "片尾曲" | "演职员表" | "片尾字幕" | "片尾鳴謝"
    ) || lower.starts_with("ed ")
        || lower.starts_with("ed:")
        || lower.starts_with("ed-")
        || lower.starts_with("ending ")
        || lower.starts_with("ending:")
        || lower.starts_with("ending-")
        || lower.starts_with("credits ")
        || lower.starts_with("credits:")
        || lower.starts_with("outro ")
        || lower.starts_with("outro:")
        || lower.ends_with(" ed")
        || lower.ends_with(" ending")
        || lower.ends_with(" credits")
        || lower.ends_with(" outro")
        || trimmed.starts_with("片尾")
        || trimmed.contains("演职员表")
    {
        return Some(VideoSegmentKind::Credits);
    }
    if matches!(
        lower.as_str(),
        "recap" | "previously" | "previously on" | "前情提要" | "上集回顾"
    ) || lower.starts_with("recap ")
        || lower.starts_with("previously on")
        || trimmed.starts_with("前情提要")
        || trimmed.starts_with("上集回顾")
    {
        return Some(VideoSegmentKind::Recap);
    }
    None
}

/// Auto-infers intro, credits, and recap markers from embedded chapter titles.
pub fn infer_segments_from_chapters(
    item_id: ItemId,
    chapters: &[ChapterMarker],
    duration_ms: Option<u64>,
) -> Vec<VideoSegment> {
    if chapters.is_empty() {
        return Vec::new();
    }
    let mut sorted: Vec<&ChapterMarker> = chapters.iter().collect();
    sorted.sort_by_key(|c| c.start_ms);

    let now = clock::now();
    let mut out = Vec::new();
    for (idx, chapter) in sorted.iter().enumerate() {
        let Some(title) = chapter.title.as_deref() else {
            continue;
        };
        let Some(kind) = classify_chapter_title(title) else {
            continue;
        };
        let start = chapter.start_ms;
        let next_start = sorted.get(idx + 1).map(|c| c.start_ms);
        let end = match next_start {
            Some(next) if next > start => next,
            _ => match duration_ms {
                Some(dur) if dur > start => dur,
                _ => continue,
            },
        };
        let Ok(start_ms) = i64::try_from(start) else {
            continue;
        };
        let Ok(end_ms) = i64::try_from(end) else {
            continue;
        };
        if end_ms <= start_ms {
            continue;
        }
        out.push(VideoSegment {
            id: format!("chapter-{}-{start_ms}-{end_ms}", kind.as_str()),
            item_id,
            kind,
            start_ms,
            end_ms,
            source: VideoSegmentSource::Chapter,
            updated_at: now,
        });
    }
    out
}

fn rating_from_tenths(tenths: Option<i32>) -> Option<f64> {
    tenths.map(|r| f64::from(r) / 10.0)
}

impl Jellyfin<'_> {
    // -----------------------------------------------------------------------
    // Client API Keys
    // -----------------------------------------------------------------------

    pub async fn create_api_key(
        &self,
        user_id: UserId,
        name: &str,
        client_kind: ClientApiKeyKind,
    ) -> Result<CreatedClientApiKey, DbError> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(DbError::Invalid("API key name must not be empty"));
        }
        let clean_name = truncate_chars(trimmed, MAX_API_KEY_NAME_CHARS);
        let raw_token = generate_api_key()?;
        let key_prefix = truncate_chars(&raw_token, 12);
        let key_hash = sha256_hex(&raw_token);
        let now = now_ms();
        let id = palveni_core::AuditId::generate().to_string();

        let txn = self.db.main().begin().await?;
        crate::revision::next_revision(&txn).await?;

        let user_exists = users::Entity::find_by_id(user_id.to_string())
            .filter(users::Column::DisabledAt.is_null())
            .one(&txn)
            .await?
            .is_some();
        if !user_exists {
            txn.rollback().await?;
            return Err(DbError::NotFound("user"));
        }

        let model = client_api_key::ActiveModel {
            id: Set(id.clone()),
            user_id: Set(user_id.to_string()),
            name: Set(clean_name.clone()),
            client_kind: Set(client_kind.as_str().to_owned()),
            key_prefix: Set(key_prefix.clone()),
            key_hash: Set(key_hash),
            last_used_at: Set(None),
            revoked_at: Set(None),
            created_at: Set(now),
        };
        client_api_key::Entity::insert(model)
            .exec_without_returning(&txn)
            .await?;
        txn.commit().await?;

        Ok(CreatedClientApiKey {
            key: ClientApiKeySummary {
                id,
                name: clean_name,
                client_kind,
                key_prefix,
                last_used_at: None,
                revoked_at: None,
                created_at: ts(now)?,
            },
            token: raw_token,
        })
    }

    pub async fn list_api_keys(
        &self,
        user_id: UserId,
    ) -> Result<Vec<ClientApiKeySummary>, DbError> {
        let rows = client_api_key::Entity::find()
            .filter(client_api_key::Column::UserId.eq(user_id.to_string()))
            .filter(client_api_key::Column::RevokedAt.is_null())
            .order_by_desc(client_api_key::Column::CreatedAt)
            .order_by_desc(client_api_key::Column::Id)
            .all(self.db.main())
            .await?;
        rows.into_iter().map(api_key_summary).collect()
    }

    pub async fn revoke_api_key(
        &self,
        key_id: &str,
        actor: UserId,
        is_admin: bool,
    ) -> Result<bool, DbError> {
        let txn = self.db.main().begin().await?;
        crate::revision::next_revision(&txn).await?;

        let existing = client_api_key::Entity::find_by_id(key_id.to_owned())
            .filter(client_api_key::Column::RevokedAt.is_null())
            .one(&txn)
            .await?;
        let Some(row) = existing else {
            txn.rollback().await?;
            return Ok(false);
        };
        if !is_admin && row.user_id != actor.to_string() {
            txn.rollback().await?;
            return Ok(false);
        }
        client_api_key::Entity::update_many()
            .col_expr(client_api_key::Column::RevokedAt, Expr::val(now_ms()))
            .filter(client_api_key::Column::Id.eq(row.id))
            .filter(client_api_key::Column::RevokedAt.is_null())
            .exec(&txn)
            .await?;
        txn.commit().await?;
        Ok(true)
    }

    /// Resolves a raw API key to its active user, updating `last_used_at` cleanly.
    pub async fn authenticate_api_key(
        &self,
        raw_key: &str,
    ) -> Result<Option<(User, String)>, DbError> {
        let trimmed = raw_key.trim();
        if trimmed.is_empty() {
            return Ok(None);
        }
        let hash = sha256_hex(trimmed);
        let key_row = client_api_key::Entity::find()
            .filter(client_api_key::Column::KeyHash.eq(hash))
            .filter(client_api_key::Column::RevokedAt.is_null())
            .one(self.db.main())
            .await?;
        let Some(key_row) = key_row else {
            return Ok(None);
        };
        let user_row = users::Entity::find_by_id(key_row.user_id.clone())
            .one(self.db.main())
            .await?;
        let Some(user_row) = user_row else {
            return Ok(None);
        };
        if user_row.disabled_at.is_some() {
            return Ok(None);
        }
        let now = now_ms();
        let should_touch = key_row
            .last_used_at
            .is_none_or(|prev| now.saturating_sub(prev) >= API_KEY_TOUCH_INTERVAL_MS);
        if should_touch {
            let _ = client_api_key::Entity::update_many()
                .col_expr(client_api_key::Column::LastUsedAt, Expr::val(now))
                .filter(client_api_key::Column::Id.eq(key_row.id.clone()))
                .exec(self.db.main())
                .await;
        }
        Ok(Some((to_user(user_row)?, key_row.id)))
    }

    // -----------------------------------------------------------------------
    // Video Segments (Skip Intro / Credits / Recap)
    // -----------------------------------------------------------------------

    pub async fn list_video_segments(&self, item_id: ItemId) -> Result<Vec<VideoSegment>, DbError> {
        let item_exists = items::Entity::find_by_id(item_id.to_string())
            .filter(items::Column::DeletedAt.is_null())
            .one(self.db.main())
            .await?
            .is_some();
        if !item_exists {
            return Err(DbError::NotFound("video"));
        }
        let rows = video_segment::Entity::find()
            .filter(video_segment::Column::ItemId.eq(item_id.to_string()))
            .order_by_asc(video_segment::Column::StartMs)
            .order_by_asc(video_segment::Column::Id)
            .all(self.db.main())
            .await?;
        rows.into_iter().map(segment_from_row).collect()
    }

    pub async fn replace_video_segments(
        &self,
        item_id: ItemId,
        inputs: &[VideoSegmentInput],
    ) -> Result<Vec<VideoSegment>, DbError> {
        if inputs.len() > MAX_SEGMENTS_PER_ITEM {
            return Err(DbError::Invalid("Too many segments for a single item"));
        }
        let mut normalized: Vec<(VideoSegmentKind, i64, i64, VideoSegmentSource)> =
            Vec::with_capacity(inputs.len());
        for seg in inputs {
            if seg.start_ms < 0 || seg.end_ms <= seg.start_ms {
                return Err(DbError::Invalid(
                    "Segment startMs must be nonnegative and endMs must be greater than startMs",
                ));
            }
            normalized.push((
                seg.kind,
                seg.start_ms,
                seg.end_ms,
                seg.source.unwrap_or(VideoSegmentSource::Manual),
            ));
        }
        normalized.sort_by_key(|(_, start, end, _)| (*start, *end));

        let txn = self.db.main().begin().await?;
        crate::revision::next_revision(&txn).await?;

        let item_exists = items::Entity::find_by_id(item_id.to_string())
            .filter(items::Column::DeletedAt.is_null())
            .one(&txn)
            .await?
            .is_some();
        if !item_exists {
            txn.rollback().await?;
            return Err(DbError::NotFound("video"));
        }

        video_segment::Entity::delete_many()
            .filter(video_segment::Column::ItemId.eq(item_id.to_string()))
            .exec(&txn)
            .await?;

        let now = now_ms();
        let now_dt = ts(now)?;
        let mut saved = Vec::with_capacity(normalized.len());
        for (kind, start_ms, end_ms, source) in normalized {
            let id = palveni_core::AuditId::generate().to_string();
            video_segment::Entity::insert(video_segment::ActiveModel {
                id: Set(id.clone()),
                item_id: Set(item_id.to_string()),
                kind: Set(kind.as_str().to_owned()),
                start_ms: Set(start_ms),
                end_ms: Set(end_ms),
                source: Set(source.as_str().to_owned()),
                updated_at: Set(now),
            })
            .exec_without_returning(&txn)
            .await?;
            saved.push(VideoSegment {
                id,
                item_id,
                kind,
                start_ms,
                end_ms,
                source,
                updated_at: now_dt,
            });
        }
        txn.commit().await?;
        Ok(saved)
    }

    // -----------------------------------------------------------------------
    // Jellyfin Protocol Catalog & Playback State Queries
    // -----------------------------------------------------------------------

    pub async fn get_item_meta(
        &self,
        item_id: ItemId,
    ) -> Result<Option<JellyfinItemLookup>, DbError> {
        let Some(item) = items::Entity::find_by_id(item_id.to_string())
            .filter(items::Column::DeletedAt.is_null())
            .one(self.db.main())
            .await?
        else {
            return Ok(None);
        };
        let work_key = video_metadata::Entity::find_by_id(item.id.clone())
            .one(self.db.main())
            .await?
            .map(|m| m.work_key);
        Ok(Some(JellyfinItemLookup {
            item_id: parse_id(&item.id, "item id")?,
            library_id: parse_id(&item.library_id, "library id")?,
            kind: parse_id(&item.kind, "item kind")?,
            title: item.title,
            path: item.path,
            size_bytes: item.size_bytes,
            work_key,
        }))
    }

    pub async fn list_video_records(
        &self,
        user: UserId,
        library_ids: &[LibraryId],
    ) -> Result<Vec<JellyfinVideoRecord>, DbError> {
        if library_ids.is_empty() {
            return Ok(Vec::new());
        }
        let lib_strs: Vec<String> = library_ids.iter().map(ToString::to_string).collect();
        let item_rows = items::Entity::find()
            .filter(items::Column::LibraryId.is_in(lib_strs.clone()))
            .filter(items::Column::Kind.eq("video"))
            .filter(items::Column::DeletedAt.is_null())
            .order_by_asc(items::Column::SortTitle)
            .order_by_asc(items::Column::Id)
            .all(self.db.main())
            .await?;
        if item_rows.is_empty() {
            return Ok(Vec::new());
        }
        self.hydrate_video_records(user, item_rows).await
    }

    pub async fn list_video_records_for_work(
        &self,
        user: UserId,
        work_key: &str,
    ) -> Result<Vec<JellyfinVideoRecord>, DbError> {
        let meta_rows = video_metadata::Entity::find()
            .filter(video_metadata::Column::WorkKey.eq(work_key.to_owned()))
            .all(self.db.main())
            .await?;
        if meta_rows.is_empty() {
            return Ok(Vec::new());
        }
        let ids: Vec<String> = meta_rows.iter().map(|m| m.item_id.clone()).collect();
        let item_rows = items::Entity::find()
            .filter(items::Column::Id.is_in(ids))
            .filter(items::Column::DeletedAt.is_null())
            .all(self.db.main())
            .await?;
        self.hydrate_video_records(user, item_rows).await
    }

    pub async fn get_video_record(
        &self,
        user: UserId,
        item_id: ItemId,
    ) -> Result<Option<JellyfinVideoRecord>, DbError> {
        let Some(item) = items::Entity::find_by_id(item_id.to_string())
            .filter(items::Column::Kind.eq("video"))
            .filter(items::Column::DeletedAt.is_null())
            .one(self.db.main())
            .await?
        else {
            return Ok(None);
        };
        let mut records = self.hydrate_video_records(user, vec![item]).await?;
        Ok(records.pop())
    }

    async fn hydrate_video_records(
        &self,
        user: UserId,
        item_rows: Vec<items::Model>,
    ) -> Result<Vec<JellyfinVideoRecord>, DbError> {
        if item_rows.is_empty() {
            return Ok(Vec::new());
        }
        let item_ids: Vec<String> = item_rows.iter().map(|i| i.id.clone()).collect();
        let mut meta_map: HashMap<String, video_metadata::Model> = HashMap::new();
        let mut genres_map: HashMap<String, Vec<String>> = HashMap::new();
        for chunk in item_ids.chunks(200) {
            let chunk_vec: Vec<String> = chunk.to_vec();
            for row in video_metadata::Entity::find()
                .filter(video_metadata::Column::ItemId.is_in(chunk_vec.clone()))
                .all(self.db.main())
                .await?
            {
                meta_map.insert(row.item_id.clone(), row);
            }
            for row in video_genres::Entity::find()
                .filter(video_genres::Column::ItemId.is_in(chunk_vec))
                .all(self.db.main())
                .await?
            {
                genres_map.entry(row.item_id).or_default().push(row.name);
            }
        }
        let states_map = self.db.personal().states_for(user, &item_ids).await?;

        let fav_rows = video_favorites::Entity::find()
            .filter(video_favorites::Column::UserId.eq(user.to_string()))
            .all(self.db.main())
            .await?;
        let fav_works: HashSet<(String, String)> = fav_rows
            .into_iter()
            .map(|r| (r.library_id, r.work_key))
            .collect();

        let mut records = Vec::with_capacity(item_rows.len());
        for item in item_rows {
            let item_id: ItemId = parse_id(&item.id, "item id")?;
            let library_id: LibraryId = parse_id(&item.library_id, "library id")?;
            let state = states_map.get(&item.id).cloned().unwrap_or(MediaState {
                favorite: false,
                stars: 0,
                position_ms: 0,
                completed: false,
                play_count: 0,
                last_played_at: None,
                playback_version: 0,
            });
            let genres = genres_map.remove(&item.id).unwrap_or_default();
            if let Some(meta) = meta_map.remove(&item.id) {
                let work_studios: Vec<String> = meta
                    .work_studios
                    .as_deref()
                    .and_then(|raw| serde_json::from_str(raw).ok())
                    .unwrap_or_default();
                let work_favorite =
                    fav_works.contains(&(item.library_id.clone(), meta.work_key.clone()));
                records.push(JellyfinVideoRecord {
                    item_id,
                    library_id,
                    title: meta.title,
                    path: item.path,
                    size_bytes: item.size_bytes,
                    created_at: ts(item.created_at)?,
                    work_key: meta.work_key,
                    work_title: meta.work_title,
                    work_original_title: meta.work_original_title,
                    work_year: meta.work_year,
                    work_overview: meta.work_overview,
                    work_rating: rating_from_tenths(meta.work_rating),
                    work_official_rating: meta.work_official_rating,
                    work_premiered: meta.work_premiered,
                    work_studios,
                    work_poster: meta.work_poster.is_some(),
                    work_backdrop: meta.work_backdrop.is_some(),
                    season: meta.season,
                    season_poster: meta.season_poster.is_some(),
                    episode: meta.episode,
                    episode_poster: meta.episode_poster.is_some(),
                    overview: meta.overview,
                    runtime_ms: meta.runtime_ms,
                    rating: rating_from_tenths(meta.rating),
                    premiered: meta.premiered,
                    thumb: meta.thumb.is_some(),
                    genres,
                    state,
                    work_favorite,
                });
            } else {
                let fallback_work_key = sha256_hex(&format!("item:{}", item.id));
                let work_favorite =
                    fav_works.contains(&(item.library_id.clone(), fallback_work_key.clone()));
                records.push(JellyfinVideoRecord {
                    item_id,
                    library_id,
                    title: item.title.clone(),
                    path: item.path,
                    size_bytes: item.size_bytes,
                    created_at: ts(item.created_at)?,
                    work_key: fallback_work_key,
                    work_title: item.title,
                    work_original_title: None,
                    work_year: None,
                    work_overview: None,
                    work_rating: None,
                    work_official_rating: None,
                    work_premiered: None,
                    work_studios: Vec::new(),
                    work_poster: false,
                    work_backdrop: false,
                    season: None,
                    season_poster: false,
                    episode: None,
                    episode_poster: false,
                    overview: None,
                    runtime_ms: None,
                    rating: None,
                    premiered: None,
                    thumb: false,
                    genres,
                    state,
                    work_favorite,
                });
            }
        }
        Ok(records)
    }

    pub async fn list_audio_records(
        &self,
        user: UserId,
        library_ids: &[LibraryId],
    ) -> Result<Vec<JellyfinAudioRecord>, DbError> {
        if library_ids.is_empty() {
            return Ok(Vec::new());
        }
        let lib_strs: Vec<String> = library_ids.iter().map(ToString::to_string).collect();
        let item_rows = items::Entity::find()
            .filter(items::Column::LibraryId.is_in(lib_strs))
            .filter(items::Column::Kind.is_in(["track", "chapter", "episode"]))
            .filter(items::Column::DeletedAt.is_null())
            .order_by_asc(items::Column::SortTitle)
            .order_by_asc(items::Column::Id)
            .all(self.db.main())
            .await?;
        self.hydrate_audio_records(user, item_rows).await
    }

    pub async fn get_audio_record(
        &self,
        user: UserId,
        item_id: ItemId,
    ) -> Result<Option<JellyfinAudioRecord>, DbError> {
        let Some(item) = items::Entity::find_by_id(item_id.to_string())
            .filter(items::Column::Kind.is_in(["track", "chapter", "episode"]))
            .filter(items::Column::DeletedAt.is_null())
            .one(self.db.main())
            .await?
        else {
            return Ok(None);
        };
        let mut rows = self.hydrate_audio_records(user, vec![item]).await?;
        Ok(rows.pop())
    }

    async fn hydrate_audio_records(
        &self,
        user: UserId,
        item_rows: Vec<items::Model>,
    ) -> Result<Vec<JellyfinAudioRecord>, DbError> {
        if item_rows.is_empty() {
            return Ok(Vec::new());
        }
        let item_ids: Vec<String> = item_rows.iter().map(|i| i.id.clone()).collect();
        let mut meta_map: HashMap<String, track_metadata::Model> = HashMap::new();
        for chunk in item_ids.chunks(200) {
            for row in track_metadata::Entity::find()
                .filter(track_metadata::Column::ItemId.is_in(chunk.to_vec()))
                .all(self.db.main())
                .await?
            {
                meta_map.insert(row.item_id.clone(), row);
            }
        }
        let states_map = self.db.personal().states_for(user, &item_ids).await?;
        let mut out = Vec::with_capacity(item_rows.len());
        for item in item_rows {
            let item_id: ItemId = parse_id(&item.id, "item id")?;
            let library_id: LibraryId = parse_id(&item.library_id, "library id")?;
            let state = states_map.get(&item.id).cloned().unwrap_or(MediaState {
                favorite: false,
                stars: 0,
                position_ms: 0,
                completed: false,
                play_count: 0,
                last_played_at: None,
                playback_version: 0,
            });
            let meta = meta_map.remove(&item.id);
            out.push(JellyfinAudioRecord {
                item_id,
                library_id,
                title: meta.as_ref().map_or(item.title, |m| m.title.clone()),
                artist: meta.as_ref().and_then(|m| m.artist.clone()),
                album: meta.as_ref().and_then(|m| m.album.clone()),
                album_artist: meta.as_ref().and_then(|m| m.album_artist.clone()),
                track_number: meta.as_ref().and_then(|m| m.track_number),
                disc_number: meta.as_ref().and_then(|m| m.disc_number),
                year: meta.as_ref().and_then(|m| m.year),
                duration_ms: meta.as_ref().and_then(|m| m.duration_ms),
                codec: meta.as_ref().and_then(|m| m.codec.clone()),
                bitrate_kbps: meta.as_ref().and_then(|m| m.bitrate_kbps),
                path: item.path,
                size_bytes: item.size_bytes,
                created_at: ts(item.created_at)?,
                state,
            });
        }
        Ok(out)
    }

    /// Updates playback state from Jellyfin `/Sessions/Playing*` reports without
    /// requiring a client-tracked optimistic lock version.
    pub async fn report_playback(
        &self,
        user: UserId,
        item_id: ItemId,
        position_ms: i64,
        completed: bool,
        increment_play_count: bool,
    ) -> Result<MediaState, DbError> {
        let clamped_pos = position_ms.max(0);
        let txn = self.db.main().begin().await?;
        let revision = crate::revision::next_revision(&txn).await?;

        let live = items::Entity::find_by_id(item_id.to_string())
            .filter(items::Column::DeletedAt.is_null())
            .one(&txn)
            .await?
            .is_some();
        if !live {
            txn.rollback().await?;
            return Err(DbError::NotFound("media item"));
        }

        let key = (user.to_string(), item_id.to_string());
        let existing = user_item_state::Entity::find_by_id(key.clone())
            .one(&txn)
            .await?;
        let now = now_ms();
        let (mut row, exists) = match existing {
            Some(row) => (row, true),
            None => (
                user_item_state::Model {
                    user_id: key.0,
                    item_id: key.1,
                    favorite_order: None,
                    stars: 0,
                    position_ms: 0,
                    completed: false,
                    play_count: 0,
                    last_played_at: None,
                    playback_version: 0,
                    previous_version: None,
                },
                false,
            ),
        };

        row.previous_version = Some(row.playback_version);
        row.playback_version = revision;
        row.last_played_at = Some(now);
        row.position_ms = if completed { 0 } else { clamped_pos };
        row.completed = completed;
        if increment_play_count || (completed && row.play_count == 0) {
            row.play_count = row.play_count.saturating_add(1);
        }

        let result = MediaState {
            favorite: row.favorite_order.is_some(),
            stars: row.stars,
            position_ms: if row.completed { 0 } else { row.position_ms },
            completed: row.completed,
            play_count: row.play_count,
            last_played_at: opt_ts(row.last_played_at)?,
            playback_version: row.playback_version,
        };
        let active: user_item_state::ActiveModel = row.into();
        if exists {
            active.reset_all().update(&txn).await?;
        } else {
            user_item_state::Entity::insert(active)
                .exec_without_returning(&txn)
                .await?;
        }
        txn.commit().await?;
        Ok(result)
    }
}

//! Incremental sync delta queries & offline push mutations, multi-device
//! playback sessions, and MQTT 3.1.1 ambient lighting bridges.

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};
use palveni_core::sync_cast::{
    MqttBridgeSummary, PlaybackClientKind, PlaybackMediaKind, PlaybackSession,
    PlaybackSessionState, SyncChangesResponse, SyncConflict, SyncItemDelta, SyncLibraryDelta,
    SyncMutation, SyncPlaylistDelta, SyncPushResponse, SyncReadingBookmarkDelta,
    SyncUserStateDelta,
};
use palveni_core::{BookmarkId, ItemId, LibraryId, UserId, clock};
use sea_orm::sea_query::{Condition, Expr};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, JoinType, QueryFilter, QueryOrder, QuerySelect,
    RelationTrait, TransactionTrait,
};
use serde::{Deserialize, Serialize};

use crate::Db;
use crate::convert::{now_ms, opt_ts, parse_id, parse_opt_id, truncate_chars, ts};
use crate::entities::{
    items, libraries, mqtt_bridge, playback_session, playlist_entries, playlists,
    reading_bookmarks, user_item_state,
};
use crate::error::DbError;
use crate::revision;

/// Default active window for cross-device playback sessions (120 seconds).
pub const ACTIVE_SESSION_WINDOW_MS: i64 = 120_000;
const INTERNAL_PROGRESS_PREFIX: &str = "p:";
const MAX_SYNC_LIMIT: u32 = 500;
const LABEL_MAX_CHARS: usize = 200;
const LOCATOR_MAX_CHARS: usize = 512;
const NOTE_MAX_CHARS: usize = 1000;

pub struct SyncCast<'a> {
    pub(crate) db: &'a Db,
}

/// Opaque keyset cursor for paginating items within the same sync revision.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncCursor {
    #[serde(rename = "r")]
    pub revision: i64,
    #[serde(rename = "i")]
    pub item_id: String,
}

impl SyncCursor {
    pub fn encode(&self) -> String {
        hex::encode(serde_json::to_vec(self).unwrap_or_default())
    }

    pub fn decode(raw: &str) -> Option<Self> {
        if raw.len() > 2048 {
            return None;
        }
        let bytes = hex::decode(raw).ok()?;
        serde_json::from_slice(&bytes).ok()
    }
}

#[derive(Clone, Debug)]
pub struct UpsertPlaybackSession {
    pub session_id: String,
    pub device_name: String,
    pub client_kind: PlaybackClientKind,
    pub media_kind: PlaybackMediaKind,
    pub item_id: Option<String>,
    pub library_id: Option<String>,
    pub title: Option<String>,
    pub subtitle: Option<String>,
    pub artwork_id: Option<String>,
    pub state: PlaybackSessionState,
    pub position_ms: i64,
    pub duration_ms: i64,
    pub volume: i32,
    pub ambient_hex: Option<String>,
}

#[derive(Clone, Debug)]
pub enum UpsertSessionOutcome {
    Saved(Box<PlaybackSession>),
    ForbiddenOwner,
}

#[derive(Clone, Debug)]
pub struct MqttBridgeRecord {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: i32,
    pub username: Option<String>,
    pub password_cipher: Option<String>,
    pub topic_prefix: String,
    pub ambient_light_device_id: Option<String>,
    pub publish_playback: bool,
    pub enabled: bool,
    pub last_published_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl MqttBridgeRecord {
    pub fn to_summary(&self) -> MqttBridgeSummary {
        MqttBridgeSummary {
            id: self.id.clone(),
            name: self.name.clone(),
            host: self.host.clone(),
            port: self.port,
            username: self.username.clone(),
            password_configured: self
                .password_cipher
                .as_deref()
                .is_some_and(|cipher| !cipher.is_empty()),
            topic_prefix: self.topic_prefix.clone(),
            ambient_light_device_id: self.ambient_light_device_id.clone(),
            publish_playback: self.publish_playback,
            enabled: self.enabled,
            last_published_at: self.last_published_at,
            created_at: self.created_at,
        }
    }
}

#[derive(Clone, Debug)]
pub struct SaveMqttBridgeInput {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: i32,
    pub username: Option<String>,
    pub password_cipher: Option<String>,
    pub keep_existing_password: bool,
    pub topic_prefix: String,
    pub ambient_light_device_id: Option<String>,
    pub publish_playback: bool,
    pub enabled: bool,
}

const fn flag_to_i32(value: bool) -> i32 {
    if value { 1 } else { 0 }
}

fn to_playback_session(row: playback_session::Model) -> Result<PlaybackSession, DbError> {
    let client_kind = PlaybackClientKind::parse_str(&row.client_kind).ok_or_else(|| {
        DbError::Corrupt(format!(
            "invalid playback client_kind `{}`",
            row.client_kind
        ))
    })?;
    let media_kind = PlaybackMediaKind::parse_str(&row.media_kind).ok_or_else(|| {
        DbError::Corrupt(format!("invalid playback media_kind `{}`", row.media_kind))
    })?;
    let state = PlaybackSessionState::parse_str(&row.state)
        .ok_or_else(|| DbError::Corrupt(format!("invalid playback state `{}`", row.state)))?;
    Ok(PlaybackSession {
        id: row.id,
        user_id: parse_id(&row.user_id, "user ID")?,
        device_name: row.device_name,
        client_kind,
        media_kind,
        item_id: row.item_id,
        library_id: row.library_id,
        title: row.title,
        subtitle: row.subtitle,
        artwork_id: row.artwork_id,
        state,
        position_ms: row.position_ms,
        duration_ms: row.duration_ms,
        volume: row.volume,
        ambient_hex: row.ambient_hex,
        updated_at: ts(row.updated_at)?,
    })
}

fn to_mqtt_record(row: mqtt_bridge::Model) -> Result<MqttBridgeRecord, DbError> {
    Ok(MqttBridgeRecord {
        id: row.id,
        name: row.name,
        host: row.host,
        port: row.port,
        username: row.username,
        password_cipher: row.password_cipher,
        topic_prefix: row.topic_prefix,
        ambient_light_device_id: row.ambient_light_device_id,
        publish_playback: row.publish_playback != 0,
        enabled: row.enabled != 0,
        last_published_at: opt_ts(row.last_published_at)?,
        created_at: ts(row.created_at)?,
    })
}

impl SyncCast<'_> {
    // -----------------------------------------------------------------------
    // Native Incremental Sync
    // -----------------------------------------------------------------------

    pub async fn changes(
        &self,
        user_id: UserId,
        accessible_library_ids: &[LibraryId],
        all_libraries_allowed: bool,
        since_revision: i64,
        limit: u32,
        cursor: Option<&SyncCursor>,
    ) -> Result<SyncChangesResponse, DbError> {
        let limit = limit.clamp(1, MAX_SYNC_LIMIT);
        let since = since_revision.max(0);
        let fetch_limit = u64::from(limit) + 1;
        let conn = self.db.main();
        let latest_revision = revision::current_revision(conn).await?;

        let accessible_strs: Vec<String> = accessible_library_ids
            .iter()
            .map(ToString::to_string)
            .collect();
        let user_str = user_id.to_string();

        // 1. Libraries delta (filtered by ACL)
        let (libraries_out, libs_more) = if all_libraries_allowed || !accessible_strs.is_empty() {
            let mut query = libraries::Entity::find()
                .filter(libraries::Column::Revision.gt(since))
                .order_by_asc(libraries::Column::Revision)
                .order_by_asc(libraries::Column::Id)
                .limit(fetch_limit);
            if !all_libraries_allowed {
                query = query.filter(libraries::Column::Id.is_in(accessible_strs.clone()));
            }
            let mut rows = query.all(conn).await?;
            let more = rows.len() > limit as usize;
            rows.truncate(limit as usize);
            let out = rows
                .into_iter()
                .map(|row| {
                    Ok(SyncLibraryDelta {
                        id: parse_id(&row.id, "library ID")?,
                        name: row.name,
                        kind: parse_id(&row.kind, "library kind")?,
                        last_scan_at: opt_ts(row.last_scan_at)?,
                        updated_at: ts(row.updated_at)?,
                        revision: row.revision,
                        deleted: row.deleted_at.is_some(),
                    })
                })
                .collect::<Result<Vec<_>, DbError>>()?;
            (out, more)
        } else {
            (Vec::new(), false)
        };

        // 2. Items delta (filtered by ACL, includes tombstones)
        let (items_out, items_more, next_cursor) = if accessible_strs.is_empty() {
            (Vec::new(), false, None)
        } else {
            let mut query = items::Entity::find()
                .filter(items::Column::LibraryId.is_in(accessible_strs.clone()))
                .order_by_asc(items::Column::Revision)
                .order_by_asc(items::Column::Id)
                .limit(fetch_limit);
            if let Some(c) = cursor {
                query = query.filter(
                    Condition::any()
                        .add(items::Column::Revision.gt(c.revision))
                        .add(
                            Condition::all()
                                .add(items::Column::Revision.eq(c.revision))
                                .add(items::Column::Id.gt(c.item_id.clone())),
                        ),
                );
            } else {
                query = query.filter(items::Column::Revision.gt(since));
            }
            let mut rows = query.all(conn).await?;
            let more = rows.len() > limit as usize;
            rows.truncate(limit as usize);
            let cursor_out = if more {
                rows.last().map(|r| {
                    SyncCursor {
                        revision: r.revision,
                        item_id: r.id.clone(),
                    }
                    .encode()
                })
            } else {
                None
            };
            let out = rows
                .into_iter()
                .map(|row| {
                    Ok(SyncItemDelta {
                        id: parse_id(&row.id, "item ID")?,
                        library_id: parse_id(&row.library_id, "library ID")?,
                        parent_id: parse_opt_id(row.parent_id.as_deref(), "parent item ID")?,
                        kind: parse_id(&row.kind, "item kind")?,
                        title: row.title,
                        sort_title: row.sort_title,
                        size_bytes: row.size_bytes,
                        modified_at: opt_ts(row.modified_at)?,
                        updated_at: ts(row.updated_at)?,
                        revision: row.revision,
                        deleted: row.deleted_at.is_some(),
                    })
                })
                .collect::<Result<Vec<_>, DbError>>()?;
            (out, more, cursor_out)
        };

        // 3. Per-user item states updated after `since_revision` (ACL-filtered via items)
        let (states_out, states_more) = if accessible_strs.is_empty() {
            (Vec::new(), false)
        } else {
            let mut query = user_item_state::Entity::find()
                .join(JoinType::InnerJoin, user_item_state::Relation::Item.def())
                .filter(user_item_state::Column::UserId.eq(&user_str))
                .filter(items::Column::LibraryId.is_in(accessible_strs.clone()))
                .order_by_asc(user_item_state::Column::PlaybackVersion)
                .order_by_asc(user_item_state::Column::ItemId)
                .limit(fetch_limit);
            if since > 0 {
                query = query.filter(
                    Condition::any()
                        .add(user_item_state::Column::PlaybackVersion.gt(since))
                        .add(user_item_state::Column::FavoriteOrder.gt(since)),
                );
            }
            let mut rows = query.all(conn).await?;
            let more = rows.len() > limit as usize;
            rows.truncate(limit as usize);
            let out = rows
                .into_iter()
                .map(|row| {
                    Ok(SyncUserStateDelta {
                        item_id: parse_id(&row.item_id, "item ID")?,
                        favorite: row.favorite_order.is_some(),
                        favorite_order: row.favorite_order,
                        stars: row.stars,
                        position_ms: row.position_ms,
                        completed: row.completed,
                        play_count: row.play_count,
                        last_played_at: opt_ts(row.last_played_at)?,
                        playback_version: row.playback_version,
                    })
                })
                .collect::<Result<Vec<_>, DbError>>()?;
            (out, more)
        };

        // 4. Playlists owned by `user_id` updated after `since_revision`
        let mut pl_rows = playlists::Entity::find()
            .filter(playlists::Column::UserId.eq(&user_str))
            .filter(playlists::Column::Version.gt(since))
            .order_by_asc(playlists::Column::Version)
            .order_by_asc(playlists::Column::Id)
            .limit(fetch_limit)
            .all(conn)
            .await?;
        let playlists_more = pl_rows.len() > limit as usize;
        pl_rows.truncate(limit as usize);

        let live_pl_ids: Vec<String> = pl_rows
            .iter()
            .filter(|r| !r.deleted)
            .map(|r| r.id.clone())
            .collect();
        let mut tracks_by_playlist: HashMap<String, Vec<ItemId>> = HashMap::new();
        if !live_pl_ids.is_empty() && !accessible_strs.is_empty() {
            for chunk in live_pl_ids.chunks(200) {
                let entry_rows = playlist_entries::Entity::find()
                    .join(JoinType::InnerJoin, playlist_entries::Relation::Item.def())
                    .filter(playlist_entries::Column::PlaylistId.is_in(chunk.iter().cloned()))
                    .filter(items::Column::DeletedAt.is_null())
                    .filter(items::Column::LibraryId.is_in(accessible_strs.clone()))
                    .order_by_asc(playlist_entries::Column::PlaylistId)
                    .order_by_asc(playlist_entries::Column::Position)
                    .all(conn)
                    .await?;
                for entry in entry_rows {
                    let track_id: ItemId = parse_id(&entry.item_id, "track ID")?;
                    tracks_by_playlist
                        .entry(entry.playlist_id)
                        .or_default()
                        .push(track_id);
                }
            }
        }
        let playlists_out = pl_rows
            .into_iter()
            .map(|row| {
                let track_ids = tracks_by_playlist.remove(&row.id).unwrap_or_default();
                Ok(SyncPlaylistDelta {
                    id: parse_id(&row.id, "playlist ID")?,
                    name: row.name,
                    comment: row.comment,
                    version: row.version,
                    changed_at: opt_ts(row.changed_at)?,
                    deleted: row.deleted,
                    track_ids,
                })
            })
            .collect::<Result<Vec<_>, DbError>>()?;

        // 5. Reading bookmarks owned by `user_id` (excluding internal progress markers)
        let (bookmarks_out, bookmarks_more) = if accessible_strs.is_empty() {
            (Vec::new(), false)
        } else {
            let mut bm_rows = reading_bookmarks::Entity::find()
                .join(JoinType::InnerJoin, reading_bookmarks::Relation::Item.def())
                .filter(reading_bookmarks::Column::UserId.eq(&user_str))
                .filter(
                    reading_bookmarks::Column::Id.not_like(format!("{INTERNAL_PROGRESS_PREFIX}%")),
                )
                .filter(items::Column::DeletedAt.is_null())
                .filter(items::Column::LibraryId.is_in(accessible_strs.clone()))
                .order_by_asc(reading_bookmarks::Column::CreatedAt)
                .order_by_asc(reading_bookmarks::Column::Id)
                .limit(fetch_limit)
                .all(conn)
                .await?;
            let more = bm_rows.len() > limit as usize;
            bm_rows.truncate(limit as usize);
            let out = bm_rows
                .into_iter()
                .map(|row| {
                    Ok(SyncReadingBookmarkDelta {
                        id: parse_id(&row.id, "bookmark ID")?,
                        item_id: parse_id(&row.item_id, "item ID")?,
                        page: row.page,
                        locator: row.locator,
                        label: row.label,
                        note: row.note,
                        created_at: ts(row.created_at)?,
                    })
                })
                .collect::<Result<Vec<_>, DbError>>()?;
            (out, more)
        };

        let has_more = libs_more || items_more || states_more || playlists_more || bookmarks_more;

        Ok(SyncChangesResponse {
            latest_revision,
            since,
            has_more,
            next_cursor,
            libraries: libraries_out,
            items: items_out,
            user_states: states_out,
            playlists: playlists_out,
            reading_bookmarks: bookmarks_out,
        })
    }

    pub async fn push_mutations(
        &self,
        user_id: UserId,
        accessible_library_ids: &[LibraryId],
        mutations: &[SyncMutation],
    ) -> Result<SyncPushResponse, DbError> {
        if mutations.is_empty() {
            let latest_revision = revision::current_revision(self.db.main()).await?;
            return Ok(SyncPushResponse {
                applied: 0,
                conflicts: Vec::new(),
                latest_revision,
            });
        }

        let txn = self.db.main().begin().await?;
        let rev = revision::next_revision(&txn).await?;
        let now = now_ms();
        let user_str = user_id.to_string();
        let accessible: HashSet<String> = accessible_library_ids
            .iter()
            .map(ToString::to_string)
            .collect();

        let mut applied = 0usize;
        let mut conflicts = Vec::new();

        for (index, mutation) in mutations.iter().enumerate() {
            match mutation {
                SyncMutation::SetFavorite {
                    item_id,
                    favorite,
                    base_revision,
                    ..
                } => {
                    let item_str = item_id.to_string();
                    let Some(item_row) = items::Entity::find_by_id(&item_str)
                        .filter(items::Column::DeletedAt.is_null())
                        .one(&txn)
                        .await?
                    else {
                        conflicts.push(SyncConflict {
                            index,
                            kind: "set_favorite".into(),
                            target_id: item_str,
                            reason: "item_not_found".into(),
                            server_revision: rev,
                        });
                        continue;
                    };
                    if !accessible.contains(&item_row.library_id) {
                        conflicts.push(SyncConflict {
                            index,
                            kind: "set_favorite".into(),
                            target_id: item_str,
                            reason: "forbidden".into(),
                            server_revision: rev,
                        });
                        continue;
                    }
                    let existing =
                        user_item_state::Entity::find_by_id((user_str.clone(), item_str.clone()))
                            .one(&txn)
                            .await?;
                    match existing {
                        Some(mut row) => {
                            if row.favorite_order.is_some() == *favorite {
                                applied += 1;
                                continue;
                            }
                            if let Some(base_rev) = base_revision
                                && let Some(server_fav_rev) = row.favorite_order
                                && server_fav_rev > *base_rev
                            {
                                conflicts.push(SyncConflict {
                                    index,
                                    kind: "set_favorite".into(),
                                    target_id: item_str,
                                    reason: "server_newer".into(),
                                    server_revision: server_fav_rev,
                                });
                                continue;
                            }
                            row.favorite_order = favorite.then_some(rev);
                            let active: user_item_state::ActiveModel = row.into();
                            active.reset_all().update(&txn).await?;
                            applied += 1;
                        }
                        None => {
                            if !*favorite {
                                applied += 1;
                                continue;
                            }
                            let row = user_item_state::Model {
                                user_id: user_str.clone(),
                                item_id: item_str,
                                favorite_order: Some(rev),
                                stars: 0,
                                position_ms: 0,
                                completed: false,
                                play_count: 0,
                                last_played_at: None,
                                playback_version: 0,
                                previous_version: None,
                            };
                            user_item_state::Entity::insert(user_item_state::ActiveModel::from(
                                row,
                            ))
                            .exec_without_returning(&txn)
                            .await?;
                            applied += 1;
                        }
                    }
                }
                SyncMutation::SetProgress {
                    item_id,
                    position_ms,
                    completed,
                    client_updated_at,
                    base_revision,
                } => {
                    let item_str = item_id.to_string();
                    let Some(item_row) = items::Entity::find_by_id(&item_str)
                        .filter(items::Column::DeletedAt.is_null())
                        .one(&txn)
                        .await?
                    else {
                        conflicts.push(SyncConflict {
                            index,
                            kind: "set_progress".into(),
                            target_id: item_str,
                            reason: "item_not_found".into(),
                            server_revision: rev,
                        });
                        continue;
                    };
                    if !accessible.contains(&item_row.library_id) {
                        conflicts.push(SyncConflict {
                            index,
                            kind: "set_progress".into(),
                            target_id: item_str,
                            reason: "forbidden".into(),
                            server_revision: rev,
                        });
                        continue;
                    }
                    let client_ms = client_updated_at.map(clock::to_millis);
                    let existing =
                        user_item_state::Entity::find_by_id((user_str.clone(), item_str.clone()))
                            .one(&txn)
                            .await?;
                    match existing {
                        Some(mut row) => {
                            if row.position_ms == *position_ms && row.completed == *completed {
                                applied += 1;
                                continue;
                            }
                            let server_newer_by_rev = base_revision
                                .is_some_and(|base| row.playback_version > base)
                                && !client_ms.is_some_and(|c| c > row.last_played_at.unwrap_or(0));
                            let server_newer_by_ts = base_revision.is_none()
                                && client_ms
                                    .is_some_and(|c| row.last_played_at.is_some_and(|s| c < s));
                            if server_newer_by_rev || server_newer_by_ts {
                                conflicts.push(SyncConflict {
                                    index,
                                    kind: "set_progress".into(),
                                    target_id: item_str,
                                    reason: "server_newer".into(),
                                    server_revision: row.playback_version,
                                });
                                continue;
                            }
                            let was_completed = row.completed;
                            row.previous_version = Some(row.playback_version);
                            row.playback_version = rev;
                            row.position_ms = *position_ms;
                            row.completed = *completed;
                            row.last_played_at = Some(client_ms.unwrap_or(now));
                            if *completed && !was_completed {
                                row.play_count = row.play_count.saturating_add(1);
                            }
                            let active: user_item_state::ActiveModel = row.into();
                            active.reset_all().update(&txn).await?;
                            applied += 1;
                        }
                        None => {
                            let row = user_item_state::Model {
                                user_id: user_str.clone(),
                                item_id: item_str,
                                favorite_order: None,
                                stars: 0,
                                position_ms: *position_ms,
                                completed: *completed,
                                play_count: i64::from(*completed),
                                last_played_at: Some(client_ms.unwrap_or(now)),
                                playback_version: rev,
                                previous_version: Some(0),
                            };
                            user_item_state::Entity::insert(user_item_state::ActiveModel::from(
                                row,
                            ))
                            .exec_without_returning(&txn)
                            .await?;
                            applied += 1;
                        }
                    }
                }
                SyncMutation::AddReadingBookmark {
                    item_id,
                    bookmark_id,
                    page,
                    locator,
                    label,
                    note,
                    client_updated_at,
                    ..
                } => {
                    let item_str = item_id.to_string();
                    let Some(item_row) = items::Entity::find_by_id(&item_str)
                        .filter(items::Column::DeletedAt.is_null())
                        .filter(items::Column::Kind.is_in(["book", "comic"]))
                        .one(&txn)
                        .await?
                    else {
                        conflicts.push(SyncConflict {
                            index,
                            kind: "add_reading_bookmark".into(),
                            target_id: item_str,
                            reason: "item_not_found".into(),
                            server_revision: rev,
                        });
                        continue;
                    };
                    if !accessible.contains(&item_row.library_id) {
                        conflicts.push(SyncConflict {
                            index,
                            kind: "add_reading_bookmark".into(),
                            target_id: item_str,
                            reason: "forbidden".into(),
                            server_revision: rev,
                        });
                        continue;
                    }
                    let clean_label = truncate_chars(label.trim(), LABEL_MAX_CHARS);
                    let clean_locator = locator
                        .as_deref()
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .map(|s| truncate_chars(s, LOCATOR_MAX_CHARS));
                    let clean_note = note
                        .as_deref()
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .map(|s| truncate_chars(s, NOTE_MAX_CHARS));

                    if let Some(bid) = bookmark_id {
                        if let Some(existing) =
                            reading_bookmarks::Entity::find_by_id(bid.to_string())
                                .one(&txn)
                                .await?
                        {
                            if existing.user_id == user_str {
                                applied += 1;
                            } else {
                                conflicts.push(SyncConflict {
                                    index,
                                    kind: "add_reading_bookmark".into(),
                                    target_id: bid.to_string(),
                                    reason: "conflict".into(),
                                    server_revision: rev,
                                });
                            }
                            continue;
                        }
                    } else {
                        let dup = reading_bookmarks::Entity::find()
                            .filter(reading_bookmarks::Column::UserId.eq(&user_str))
                            .filter(reading_bookmarks::Column::ItemId.eq(&item_str))
                            .filter(reading_bookmarks::Column::Page.eq(*page))
                            .filter(reading_bookmarks::Column::Label.eq(&clean_label))
                            .filter(
                                reading_bookmarks::Column::Id
                                    .not_like(format!("{INTERNAL_PROGRESS_PREFIX}%")),
                            )
                            .one(&txn)
                            .await?;
                        if dup.is_some() {
                            applied += 1;
                            continue;
                        }
                    }

                    let id = bookmark_id.unwrap_or_else(BookmarkId::generate);
                    let model = reading_bookmarks::Model {
                        id: id.to_string(),
                        user_id: user_str.clone(),
                        item_id: item_str,
                        page: *page,
                        locator: clean_locator,
                        label: clean_label,
                        note: clean_note,
                        created_at: client_updated_at.map(clock::to_millis).unwrap_or(now),
                    };
                    reading_bookmarks::Entity::insert(reading_bookmarks::ActiveModel::from(model))
                        .exec_without_returning(&txn)
                        .await?;
                    applied += 1;
                }
                SyncMutation::RemoveReadingBookmark { bookmark_id, .. } => {
                    let bid_str = bookmark_id.to_string();
                    reading_bookmarks::Entity::delete_many()
                        .filter(reading_bookmarks::Column::Id.eq(&bid_str))
                        .filter(reading_bookmarks::Column::UserId.eq(&user_str))
                        .filter(
                            reading_bookmarks::Column::Id
                                .not_like(format!("{INTERNAL_PROGRESS_PREFIX}%")),
                        )
                        .exec(&txn)
                        .await?;
                    applied += 1;
                }
            }
        }

        txn.commit().await?;
        Ok(SyncPushResponse {
            applied,
            conflicts,
            latest_revision: rev,
        })
    }

    // -----------------------------------------------------------------------
    // Playback Sessions CRUD
    // -----------------------------------------------------------------------

    pub async fn upsert_playback_session(
        &self,
        user_id: UserId,
        input: UpsertPlaybackSession,
    ) -> Result<UpsertSessionOutcome, DbError> {
        let txn = self.db.main().begin().await?;
        revision::next_revision(&txn).await?;

        let user_str = user_id.to_string();
        let now = now_ms();
        let existing = playback_session::Entity::find_by_id(&input.session_id)
            .one(&txn)
            .await?;

        if let Some(ref row) = existing
            && row.user_id != user_str
        {
            txn.rollback().await?;
            return Ok(UpsertSessionOutcome::ForbiddenOwner);
        }

        let model = playback_session::Model {
            id: truncate_chars(input.session_id.trim(), 64),
            user_id: user_str,
            device_name: truncate_chars(input.device_name.trim(), 128),
            client_kind: input.client_kind.as_str().to_owned(),
            media_kind: input.media_kind.as_str().to_owned(),
            item_id: input.item_id.map(|s| truncate_chars(s.trim(), 36)),
            library_id: input.library_id.map(|s| truncate_chars(s.trim(), 36)),
            title: input.title.map(|s| truncate_chars(s.trim(), 255)),
            subtitle: input.subtitle.map(|s| truncate_chars(s.trim(), 255)),
            artwork_id: input.artwork_id.map(|s| truncate_chars(s.trim(), 80)),
            state: input.state.as_str().to_owned(),
            position_ms: input.position_ms.max(0),
            duration_ms: input.duration_ms.max(0),
            volume: input.volume.clamp(0, 100),
            ambient_hex: input.ambient_hex.map(|s| truncate_chars(s.trim(), 16)),
            updated_at: now,
        };

        if existing.is_some() {
            let active: playback_session::ActiveModel = model.clone().into();
            active.reset_all().update(&txn).await?;
        } else {
            playback_session::Entity::insert(playback_session::ActiveModel::from(model.clone()))
                .exec_without_returning(&txn)
                .await?;
        }

        txn.commit().await?;
        Ok(UpsertSessionOutcome::Saved(Box::new(to_playback_session(
            model,
        )?)))
    }

    pub async fn list_active_playback_sessions(
        &self,
        scope_user: Option<UserId>,
        max_age_ms: i64,
    ) -> Result<Vec<PlaybackSession>, DbError> {
        let cutoff = now_ms().saturating_sub(max_age_ms.max(0));
        let mut query = playback_session::Entity::find()
            .filter(playback_session::Column::UpdatedAt.gte(cutoff))
            .order_by_desc(playback_session::Column::UpdatedAt)
            .order_by_asc(playback_session::Column::Id);
        if let Some(uid) = scope_user {
            query = query.filter(playback_session::Column::UserId.eq(uid.to_string()));
        }
        let rows = query.all(self.db.main()).await?;
        rows.into_iter().map(to_playback_session).collect()
    }

    pub async fn get_playback_session(
        &self,
        session_id: &str,
    ) -> Result<Option<PlaybackSession>, DbError> {
        playback_session::Entity::find_by_id(session_id.to_owned())
            .one(self.db.main())
            .await?
            .map(to_playback_session)
            .transpose()
    }

    pub async fn delete_playback_session(&self, session_id: &str) -> Result<bool, DbError> {
        let res = playback_session::Entity::delete_by_id(session_id.to_owned())
            .exec(self.db.main())
            .await?;
        Ok(res.rows_affected > 0)
    }

    // -----------------------------------------------------------------------
    // MQTT Bridges CRUD
    // -----------------------------------------------------------------------

    pub async fn save_mqtt_bridge(
        &self,
        input: SaveMqttBridgeInput,
    ) -> Result<MqttBridgeRecord, DbError> {
        let txn = self.db.main().begin().await?;
        revision::next_revision(&txn).await?;

        let now = now_ms();
        let existing = mqtt_bridge::Entity::find_by_id(&input.id).one(&txn).await?;

        let password_cipher = if input.password_cipher.is_some() {
            input.password_cipher
        } else if input.keep_existing_password {
            existing.as_ref().and_then(|r| r.password_cipher.clone())
        } else {
            None
        };

        let model = mqtt_bridge::Model {
            id: truncate_chars(input.id.trim(), 36),
            name: truncate_chars(input.name.trim(), 128),
            host: truncate_chars(input.host.trim(), 255),
            port: input.port,
            username: input
                .username
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(|s| truncate_chars(s, 128)),
            password_cipher,
            topic_prefix: truncate_chars(input.topic_prefix.trim(), 128),
            ambient_light_device_id: input
                .ambient_light_device_id
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(|s| truncate_chars(s, 64)),
            publish_playback: flag_to_i32(input.publish_playback),
            enabled: flag_to_i32(input.enabled),
            last_published_at: existing.as_ref().and_then(|r| r.last_published_at),
            created_at: existing.as_ref().map_or(now, |r| r.created_at),
        };

        if existing.is_some() {
            let active: mqtt_bridge::ActiveModel = model.clone().into();
            active.reset_all().update(&txn).await?;
        } else {
            mqtt_bridge::Entity::insert(mqtt_bridge::ActiveModel::from(model.clone()))
                .exec_without_returning(&txn)
                .await?;
        }

        txn.commit().await?;
        to_mqtt_record(model)
    }

    pub async fn list_mqtt_bridges(&self) -> Result<Vec<MqttBridgeRecord>, DbError> {
        let rows = mqtt_bridge::Entity::find()
            .order_by_asc(mqtt_bridge::Column::CreatedAt)
            .order_by_asc(mqtt_bridge::Column::Id)
            .all(self.db.main())
            .await?;
        rows.into_iter().map(to_mqtt_record).collect()
    }

    pub async fn list_enabled_mqtt_bridges(&self) -> Result<Vec<MqttBridgeRecord>, DbError> {
        let rows = mqtt_bridge::Entity::find()
            .filter(mqtt_bridge::Column::Enabled.ne(0))
            .order_by_asc(mqtt_bridge::Column::CreatedAt)
            .order_by_asc(mqtt_bridge::Column::Id)
            .all(self.db.main())
            .await?;
        rows.into_iter().map(to_mqtt_record).collect()
    }

    pub async fn get_mqtt_bridge(&self, id: &str) -> Result<Option<MqttBridgeRecord>, DbError> {
        mqtt_bridge::Entity::find_by_id(id.to_owned())
            .one(self.db.main())
            .await?
            .map(to_mqtt_record)
            .transpose()
    }

    pub async fn delete_mqtt_bridge(&self, id: &str) -> Result<bool, DbError> {
        let res = mqtt_bridge::Entity::delete_by_id(id.to_owned())
            .exec(self.db.main())
            .await?;
        Ok(res.rows_affected > 0)
    }

    pub async fn touch_mqtt_bridge_published(&self, id: &str, at_ms: i64) -> Result<(), DbError> {
        mqtt_bridge::Entity::update_many()
            .col_expr(mqtt_bridge::Column::LastPublishedAt, Expr::val(Some(at_ms)))
            .filter(mqtt_bridge::Column::Id.eq(id.to_owned()))
            .exec(self.db.main())
            .await?;
        Ok(())
    }
}

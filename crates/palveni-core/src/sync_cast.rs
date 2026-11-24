//! Native incremental sync, multi-device playback sessions, remote cast commands,
//! and pure-Rust MQTT 3.1.1 ambient lighting bridge wire types.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::ids::{BookmarkId, ItemId, LibraryId, PlaylistId, UserId};
use crate::model::{ItemKind, LibraryKind};

// ---------------------------------------------------------------------------
// Incremental Sync (`/api/v1/sync/*`)
// ---------------------------------------------------------------------------

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncLibraryDelta {
    pub id: LibraryId,
    pub name: String,
    pub kind: LibraryKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_scan_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
    pub revision: i64,
    pub deleted: bool,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncItemDelta {
    pub id: ItemId,
    pub library_id: LibraryId,
    #[schema(required = true)]
    pub parent_id: Option<ItemId>,
    pub kind: ItemKind,
    pub title: String,
    pub sort_title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modified_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
    pub revision: i64,
    pub deleted: bool,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncUserStateDelta {
    pub item_id: ItemId,
    pub favorite: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub favorite_order: Option<i64>,
    pub stars: i32,
    pub position_ms: i64,
    pub completed: bool,
    pub play_count: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_played_at: Option<DateTime<Utc>>,
    pub playback_version: i64,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncPlaylistDelta {
    pub id: PlaylistId,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    pub version: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub changed_at: Option<DateTime<Utc>>,
    pub deleted: bool,
    pub track_ids: Vec<ItemId>,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncReadingBookmarkDelta {
    pub id: BookmarkId,
    pub item_id: ItemId,
    pub page: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub locator: Option<String>,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncChangesResponse {
    pub latest_revision: i64,
    pub since: i64,
    pub has_more: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
    pub libraries: Vec<SyncLibraryDelta>,
    pub items: Vec<SyncItemDelta>,
    pub user_states: Vec<SyncUserStateDelta>,
    pub playlists: Vec<SyncPlaylistDelta>,
    pub reading_bookmarks: Vec<SyncReadingBookmarkDelta>,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SyncMutation {
    #[serde(rename_all = "camelCase")]
    SetFavorite {
        item_id: ItemId,
        favorite: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        client_updated_at: Option<DateTime<Utc>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        base_revision: Option<i64>,
    },
    #[serde(rename_all = "camelCase")]
    SetProgress {
        item_id: ItemId,
        position_ms: i64,
        #[serde(default)]
        completed: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        client_updated_at: Option<DateTime<Utc>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        base_revision: Option<i64>,
    },
    #[serde(rename_all = "camelCase")]
    AddReadingBookmark {
        item_id: ItemId,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        bookmark_id: Option<BookmarkId>,
        page: i32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        locator: Option<String>,
        label: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        note: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        client_updated_at: Option<DateTime<Utc>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        base_revision: Option<i64>,
    },
    #[serde(rename_all = "camelCase")]
    RemoveReadingBookmark {
        bookmark_id: BookmarkId,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        client_updated_at: Option<DateTime<Utc>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        base_revision: Option<i64>,
    },
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncPushRequest {
    pub client_id: String,
    #[serde(default)]
    pub mutations: Vec<SyncMutation>,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncConflict {
    pub index: usize,
    pub kind: String,
    pub target_id: String,
    pub reason: String,
    pub server_revision: i64,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncPushResponse {
    pub applied: usize,
    pub conflicts: Vec<SyncConflict>,
    pub latest_revision: i64,
}

// ---------------------------------------------------------------------------
// Multi-Device Playback & Remote Cast (`/api/v1/sessions/playback*`)
// ---------------------------------------------------------------------------

#[derive(utoipa::ToSchema, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlaybackClientKind {
    Web,
    Ios,
    Tvos,
    Macos,
    Android,
    Desktop,
}

impl PlaybackClientKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Web => "web",
            Self::Ios => "ios",
            Self::Tvos => "tvos",
            Self::Macos => "macos",
            Self::Android => "android",
            Self::Desktop => "desktop",
        }
    }

    pub fn parse_str(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "web" => Some(Self::Web),
            "ios" => Some(Self::Ios),
            "tvos" => Some(Self::Tvos),
            "macos" => Some(Self::Macos),
            "android" => Some(Self::Android),
            "desktop" => Some(Self::Desktop),
            _ => None,
        }
    }
}

#[derive(utoipa::ToSchema, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlaybackMediaKind {
    Audio,
    Video,
    Audiobook,
    None,
}

impl PlaybackMediaKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Audio => "audio",
            Self::Video => "video",
            Self::Audiobook => "audiobook",
            Self::None => "none",
        }
    }

    pub fn parse_str(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "audio" => Some(Self::Audio),
            "video" => Some(Self::Video),
            "audiobook" => Some(Self::Audiobook),
            "none" => Some(Self::None),
            _ => None,
        }
    }
}

#[derive(utoipa::ToSchema, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlaybackSessionState {
    Playing,
    Paused,
    Stopped,
    Idle,
}

impl PlaybackSessionState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Playing => "playing",
            Self::Paused => "paused",
            Self::Stopped => "stopped",
            Self::Idle => "idle",
        }
    }

    pub fn parse_str(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "playing" => Some(Self::Playing),
            "paused" => Some(Self::Paused),
            "stopped" => Some(Self::Stopped),
            "idle" => Some(Self::Idle),
            _ => None,
        }
    }
}

#[derive(utoipa::ToSchema, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlaybackCommandKind {
    Play,
    Pause,
    Toggle,
    Seek,
    Next,
    Prev,
    Volume,
    PlayItem,
}

impl PlaybackCommandKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Play => "play",
            Self::Pause => "pause",
            Self::Toggle => "toggle",
            Self::Seek => "seek",
            Self::Next => "next",
            Self::Prev => "prev",
            Self::Volume => "volume",
            Self::PlayItem => "play_item",
        }
    }
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackHeartbeatRequest {
    pub session_id: String,
    pub device_name: String,
    pub client_kind: PlaybackClientKind,
    pub media_kind: PlaybackMediaKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub library_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subtitle: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artwork_id: Option<String>,
    pub state: PlaybackSessionState,
    #[serde(default)]
    pub position_ms: i64,
    #[serde(default)]
    pub duration_ms: i64,
    #[serde(default = "default_volume")]
    pub volume: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ambient_hex: Option<String>,
}

const fn default_volume() -> i32 {
    80
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackSession {
    pub id: String,
    pub user_id: UserId,
    pub device_name: String,
    pub client_kind: PlaybackClientKind,
    pub media_kind: PlaybackMediaKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub library_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subtitle: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artwork_id: Option<String>,
    pub state: PlaybackSessionState,
    pub position_ms: i64,
    pub duration_ms: i64,
    pub volume: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ambient_hex: Option<String>,
    pub updated_at: DateTime<Utc>,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackSessionList {
    pub sessions: Vec<PlaybackSession>,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackCommandRequest {
    pub command: PlaybackCommandKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position_ms: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub volume: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item_id: Option<String>,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackCommandAck {
    pub accepted: bool,
    pub target_session_id: String,
    pub command: PlaybackCommandKind,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackSessionUpdatedEvent {
    pub session_id: String,
    pub device_name: String,
    pub state: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub position_ms: i64,
    pub volume: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ambient_hex: Option<String>,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackCommandEvent {
    pub target_session_id: String,
    pub command: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position_ms: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub volume: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item_id: Option<String>,
    pub issued_by: String,
}

// ---------------------------------------------------------------------------
// MQTT 3.1.1 Bridge & Ambient Light (`/api/v1/home/mqtt*`)
// ---------------------------------------------------------------------------

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MqttBridgeSummary {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    pub password_configured: bool,
    pub topic_prefix: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ambient_light_device_id: Option<String>,
    pub publish_playback: bool,
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_published_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MqttBridgeList {
    pub bridges: Vec<MqttBridgeSummary>,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveMqttBridgeRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub name: String,
    pub host: String,
    #[serde(default = "default_mqtt_port")]
    pub port: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    #[serde(default = "default_topic_prefix")]
    pub topic_prefix: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ambient_light_device_id: Option<String>,
    #[serde(default = "default_true")]
    pub publish_playback: bool,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

const fn default_mqtt_port() -> i32 {
    1883
}

fn default_topic_prefix() -> String {
    "palveni".to_owned()
}

const fn default_true() -> bool {
    true
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MqttTestResponse {
    pub ok: bool,
    pub topic: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

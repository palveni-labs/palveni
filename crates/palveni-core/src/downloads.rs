//! External downloader bridge (`qBittorrent` / `Transmission` / `aria2`), RSS media
//! subscriptions, and reversible template media organizer types.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::assistant::Block;
use crate::ids::{ItemId, LibraryId};

fn default_true() -> bool {
    true
}

// ---------------------------------------------------------------------------
// External Downloader Clients & Transfer Queue (`module: downloads`)
// ---------------------------------------------------------------------------

#[derive(utoipa::ToSchema, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DownloaderKind {
    Qbittorrent,
    Transmission,
    Aria2,
}

impl DownloaderKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Qbittorrent => "qbittorrent",
            Self::Transmission => "transmission",
            Self::Aria2 => "aria2",
        }
    }

    pub fn from_db(raw: &str) -> Option<Self> {
        match raw {
            "qbittorrent" => Some(Self::Qbittorrent),
            "transmission" => Some(Self::Transmission),
            "aria2" => Some(Self::Aria2),
            _ => None,
        }
    }
}

#[derive(utoipa::ToSchema, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DownloaderClientStatus {
    Online,
    Offline,
    Disabled,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloaderClientSummary {
    pub id: String,
    pub name: String,
    pub kind: DownloaderKind,
    pub base_url: String,
    #[schema(required = true)]
    pub save_path: Option<String>,
    pub enabled: bool,
    pub credential_configured: bool,
    pub status: DownloaderClientStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status_message: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(utoipa::ToSchema, Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateDownloaderClientRequest {
    #[serde(default)]
    pub id: Option<String>,
    pub name: String,
    pub kind: DownloaderKind,
    pub base_url: String,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub password: Option<String>,
    #[serde(default)]
    pub save_path: Option<String>,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

#[derive(utoipa::ToSchema, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransferState {
    Downloading,
    Seeding,
    Paused,
    Queued,
    Completed,
    Error,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadTransfer {
    pub id: String,
    pub client_id: String,
    pub client_name: String,
    pub client_kind: DownloaderKind,
    pub name: String,
    pub state: TransferState,
    pub progress: f64,
    pub size_bytes: i64,
    pub downloaded_bytes: i64,
    pub download_speed_bps: i64,
    pub upload_speed_bps: i64,
    #[schema(required = true)]
    pub eta_seconds: Option<i64>,
    #[schema(required = true)]
    pub save_path: Option<String>,
}

#[derive(utoipa::ToSchema, Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateDownloadTaskRequest {
    #[serde(default)]
    pub client_id: Option<String>,
    pub uri: String,
    #[serde(default)]
    pub save_path: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateDownloadTaskResponse {
    pub accepted: bool,
    pub client_id: String,
    pub client_name: String,
    pub message: String,
}

// ---------------------------------------------------------------------------
// RSS / Atom Media Subscriptions (`module: downloads`)
// ---------------------------------------------------------------------------

#[derive(utoipa::ToSchema, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubscriptionMediaKind {
    Series,
    Movie,
    Music,
    Book,
}

impl SubscriptionMediaKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Series => "series",
            Self::Movie => "movie",
            Self::Music => "music",
            Self::Book => "book",
        }
    }

    pub fn from_db(raw: &str) -> Option<Self> {
        match raw {
            "series" => Some(Self::Series),
            "movie" => Some(Self::Movie),
            "music" => Some(Self::Music),
            "book" => Some(Self::Book),
            _ => None,
        }
    }
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaSubscription {
    pub id: String,
    pub owner_id: String,
    pub title: String,
    pub media_kind: SubscriptionMediaKind,
    #[schema(required = true)]
    pub quality_rule: Option<String>,
    #[schema(required = true)]
    pub feed_url: Option<String>,
    #[schema(required = true)]
    pub target_library_id: Option<String>,
    pub enabled: bool,
    #[schema(required = true)]
    pub last_checked_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(utoipa::ToSchema, Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateSubscriptionRequest {
    pub title: String,
    pub media_kind: SubscriptionMediaKind,
    #[serde(default)]
    pub quality_rule: Option<String>,
    #[serde(default)]
    pub feed_url: Option<String>,
    #[serde(default)]
    pub target_library_id: Option<String>,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeedMatchItem {
    pub title: String,
    #[schema(required = true)]
    pub link: Option<String>,
    #[schema(required = true)]
    pub pub_date: Option<String>,
    pub matched_quality: bool,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionCheckResult {
    pub subscription: MediaSubscription,
    pub total_feed_items: usize,
    pub matches: Vec<FeedMatchItem>,
    pub submitted_count: usize,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadsOverviewResponse {
    pub clients: Vec<DownloaderClientSummary>,
    pub transfers: Vec<DownloadTransfer>,
    pub subscriptions: Vec<MediaSubscription>,
}

// ---------------------------------------------------------------------------
// Media Organizer Preview, Apply & 1-Click Undo (`module: organizer`)
// ---------------------------------------------------------------------------

#[derive(utoipa::ToSchema, Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrganizeMode {
    #[default]
    Rename,
    Hardlink,
}

impl OrganizeMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Rename => "rename",
            Self::Hardlink => "hardlink",
        }
    }

    pub fn from_db(raw: &str) -> Option<Self> {
        match raw {
            "rename" => Some(Self::Rename),
            "hardlink" => Some(Self::Hardlink),
            _ => None,
        }
    }
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrganizePreset {
    pub id: String,
    pub label: String,
    pub media_kind: String,
    pub template: String,
    pub example: String,
}

pub fn builtin_organize_presets() -> Vec<OrganizePreset> {
    vec![
        OrganizePreset {
            id: "movie-year".into(),
            label: "电影标准（片名 + 年份）".into(),
            media_kind: "video".into(),
            template: "{title} ({year})".into(),
            example: "沙丘2 (2024).mkv".into(),
        },
        OrganizePreset {
            id: "series-season".into(),
            label: "剧集分季（剧名 / Season / SxxExx）".into(),
            media_kind: "video".into(),
            template: "{title}/Season {season}/{title} - S{season}E{episode}".into(),
            example: "三体/Season 01/三体 - S01E01.mkv".into(),
        },
        OrganizePreset {
            id: "music-artist-title".into(),
            label: "音乐曲目（艺人 - 标题）".into(),
            media_kind: "music".into(),
            template: "{artist} - {title}".into(),
            example: "周杰伦 - 晴天 Live.flac".into(),
        },
        OrganizePreset {
            id: "book-author-title".into(),
            label: "电子书标准（作者 - 书名）".into(),
            media_kind: "book".into(),
            template: "{author} - {title}".into(),
            example: "刘慈欣 - 三体.epub".into(),
        },
    ]
}

#[derive(utoipa::ToSchema, Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OrganizePreviewRequest {
    pub library_id: LibraryId,
    #[serde(default)]
    pub mode: OrganizeMode,
    pub template: String,
    #[serde(default)]
    pub target_subdir: Option<String>,
    #[serde(default)]
    pub limit: Option<u32>,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrganizeCandidate {
    pub row_id: String,
    pub item_id: ItemId,
    pub source_path: String,
    pub target_path: String,
    pub relative_before: String,
    pub relative_after: String,
    #[schema(required = true)]
    pub size_bytes: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conflict: Option<String>,
    pub selectable: bool,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrganizePreviewResponse {
    pub library_id: LibraryId,
    pub library_name: String,
    pub mode: OrganizeMode,
    pub template: String,
    pub read_only_mount: bool,
    pub collision_count: usize,
    pub ready_count: usize,
    pub candidates: Vec<OrganizeCandidate>,
    pub preview_block: Block,
    pub presets: Vec<OrganizePreset>,
}

#[derive(utoipa::ToSchema, Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OrganizeApplyRequest {
    pub library_id: LibraryId,
    #[serde(default)]
    pub mode: OrganizeMode,
    pub template: String,
    #[serde(default)]
    pub target_subdir: Option<String>,
    #[serde(default)]
    pub selected_item_ids: Vec<ItemId>,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrganizeOperationRecord {
    pub item_id: ItemId,
    pub from_path: String,
    pub to_path: String,
    #[serde(default)]
    pub created_dirs: Vec<String>,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrganizeBatchSummary {
    pub id: String,
    pub actor_id: String,
    pub library_id: LibraryId,
    #[schema(required = true)]
    pub library_name: Option<String>,
    pub mode: OrganizeMode,
    pub template: String,
    pub operation_count: usize,
    pub operations: Vec<OrganizeOperationRecord>,
    #[schema(required = true)]
    pub undone_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrganizeBatchList {
    pub batches: Vec<OrganizeBatchSummary>,
    pub presets: Vec<OrganizePreset>,
}

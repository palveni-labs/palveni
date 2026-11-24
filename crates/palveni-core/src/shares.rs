//! Public and password-protected share links (`/s/$token`, `/api/v1/shares`,
//! `/api/v1/public/shares/*`) with optional GPS EXIF privacy stripping.

use std::fmt;
use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{Error, ItemId, LibraryId, ShareId, UserId};

#[derive(utoipa::ToSchema, Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShareTargetKind {
    Item,
    VideoWork,
    MusicAlbum,
    Playlist,
    PhotoAlbum,
    Collection,
}

impl ShareTargetKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Item => "item",
            Self::VideoWork => "video_work",
            Self::MusicAlbum => "music_album",
            Self::Playlist => "playlist",
            Self::PhotoAlbum => "photo_album",
            Self::Collection => "collection",
        }
    }
}

impl fmt::Display for ShareTargetKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for ShareTargetKind {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim() {
            "item" => Ok(Self::Item),
            "video_work" => Ok(Self::VideoWork),
            "music_album" => Ok(Self::MusicAlbum),
            "playlist" => Ok(Self::Playlist),
            "photo_album" => Ok(Self::PhotoAlbum),
            "collection" => Ok(Self::Collection),
            other => Err(Error::invalid(format!(
                "`{other}` is not a valid share target kind"
            ))),
        }
    }
}

#[derive(utoipa::ToSchema, Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShareMediaKind {
    Video,
    Audio,
    Photo,
    Book,
    Comic,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShareLinkSummary {
    pub id: ShareId,
    pub token: String,
    pub share_path: String,
    pub creator_id: UserId,
    pub target_kind: ShareTargetKind,
    pub target_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub library_id: Option<LibraryId>,
    pub title: String,
    pub has_password: bool,
    pub allow_download: bool,
    pub strip_gps: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_views: Option<i32>,
    pub view_count: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revoked_at: Option<DateTime<Utc>>,
    pub expired: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShareLinkList {
    pub shares: Vec<ShareLinkSummary>,
}

#[derive(utoipa::ToSchema, Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateShareRequest {
    pub target_kind: ShareTargetKind,
    pub target_id: String,
    #[serde(default)]
    pub library_id: Option<LibraryId>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub password: Option<String>,
    #[serde(default)]
    pub allow_download: Option<bool>,
    #[serde(default)]
    pub strip_gps: Option<bool>,
    #[serde(default)]
    pub expires_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub expires_in_hours: Option<u32>,
    #[serde(default)]
    pub max_views: Option<i32>,
}

#[derive(utoipa::ToSchema, Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnlockShareRequest {
    pub password: String,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnlockShareResponse {
    pub proof: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicShareItem {
    pub item_id: ItemId,
    pub media_kind: ShareMediaKind,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtitle: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub taken_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub camera: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latitude: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub longitude: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<i64>,
    pub has_cover: bool,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicShareView {
    pub token: String,
    pub title: String,
    pub target_kind: ShareTargetKind,
    pub locked: bool,
    pub allow_download: bool,
    pub strip_gps: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proof: Option<String>,
    pub items: Vec<PublicShareItem>,
}

//! Domain types for client API keys (Jellyfin/WebDAV/general) and video intro/credits segments.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::ItemId;

pub const MAX_API_KEY_NAME_CHARS: usize = 128;
pub const MAX_SEGMENTS_PER_ITEM: usize = 32;

#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, utoipa::ToSchema,
)]
#[serde(rename_all = "lowercase")]
pub enum ClientApiKeyKind {
    #[default]
    Jellyfin,
    Webdav,
    General,
}

impl ClientApiKeyKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Jellyfin => "jellyfin",
            Self::Webdav => "webdav",
            Self::General => "general",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "jellyfin" => Some(Self::Jellyfin),
            "webdav" => Some(Self::Webdav),
            "general" => Some(Self::General),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ClientApiKeySummary {
    pub id: String,
    pub name: String,
    pub client_kind: ClientApiKeyKind,
    pub key_prefix: String,
    #[schema(required = true)]
    pub last_used_at: Option<DateTime<Utc>>,
    #[schema(required = true)]
    pub revoked_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ClientApiKeyList {
    pub keys: Vec<ClientApiKeySummary>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateClientApiKeyRequest {
    pub name: String,
    #[serde(default)]
    pub client_kind: Option<ClientApiKeyKind>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreatedClientApiKey {
    pub key: ClientApiKeySummary,
    pub token: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum VideoSegmentKind {
    Intro,
    Credits,
    Recap,
}

impl VideoSegmentKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Intro => "intro",
            Self::Credits => "credits",
            Self::Recap => "recap",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "intro" => Some(Self::Intro),
            "credits" => Some(Self::Credits),
            "recap" => Some(Self::Recap),
            _ => None,
        }
    }
}

#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, utoipa::ToSchema,
)]
#[serde(rename_all = "lowercase")]
pub enum VideoSegmentSource {
    #[default]
    Manual,
    Chapter,
    Auto,
}

impl VideoSegmentSource {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Manual => "manual",
            Self::Chapter => "chapter",
            Self::Auto => "auto",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "manual" => Some(Self::Manual),
            "chapter" => Some(Self::Chapter),
            "auto" => Some(Self::Auto),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct VideoSegment {
    pub id: String,
    pub item_id: ItemId,
    pub kind: VideoSegmentKind,
    pub start_ms: i64,
    pub end_ms: i64,
    pub source: VideoSegmentSource,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct VideoSegmentInput {
    pub kind: VideoSegmentKind,
    pub start_ms: i64,
    pub end_ms: i64,
    #[serde(default)]
    pub source: Option<VideoSegmentSource>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct VideoSegmentList {
    pub segments: Vec<VideoSegment>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PutVideoSegmentsRequest {
    pub segments: Vec<VideoSegmentInput>,
}

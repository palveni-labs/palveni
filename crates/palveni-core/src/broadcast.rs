use std::fmt;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum BroadcastKind {
    Radio,
    Iptv,
    Podcast,
}

impl fmt::Display for BroadcastKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Radio => "radio",
            Self::Iptv => "iptv",
            Self::Podcast => "podcast",
        })
    }
}

#[derive(Clone, Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BroadcastSource {
    pub id: String,
    pub kind: BroadcastKind,
    pub name: String,
    pub version: i64,
    pub entry_count: i32,
    pub feed_url: Option<String>,
    pub guide_url: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct BroadcastSourceList {
    pub sources: Vec<BroadcastSource>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BroadcastImport {
    pub kind: BroadcastKind,
    pub name: String,
    pub expected_version: i64,
    pub content: Option<String>,
    pub guide: Option<String>,
    pub feed_url: Option<String>,
    pub guide_url: Option<String>,
}

#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct BroadcastFavorite {
    pub favorite: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BroadcastProgramme {
    pub title: String,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BroadcastEntry {
    pub id: String,
    pub title: String,
    pub group: Option<String>,
    pub streams: Vec<String>,
    pub published_at: Option<DateTime<Utc>>,
    pub current: Option<BroadcastProgramme>,
    pub next: Option<BroadcastProgramme>,
    pub favorite: bool,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BroadcastEntryPage {
    pub entries: Vec<BroadcastEntry>,
    pub next_offset: Option<u32>,
}

/// Input identity survives changing enclosure and channel URLs during refresh.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ImportedEntry {
    pub key: String,
    pub title: String,
    pub group: Option<String>,
    pub streams: Vec<String>,
    pub published_at: Option<DateTime<Utc>>,
    pub programmes: Vec<BroadcastProgramme>,
}

pub const MAX_BROADCAST_BYTES: usize = 1024 * 1024;
pub const MAX_BROADCAST_ENTRIES: usize = 5000;

pub fn source_id(raw: &str) -> Option<String> {
    uuid::Uuid::parse_str(raw)
        .ok()
        .map(|id| id.hyphenated().to_string())
}

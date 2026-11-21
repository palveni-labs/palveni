use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ConnectionKind {
    Smb,
    Webdav,
    Jellyfin,
    Emby,
    Plex,
}

impl ConnectionKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Smb => "smb",
            Self::Webdav => "webdav",
            Self::Jellyfin => "jellyfin",
            Self::Emby => "emby",
            Self::Plex => "plex",
        }
    }
    pub fn parse(value: &str) -> Option<Self> {
        [
            Self::Smb,
            Self::Webdav,
            Self::Jellyfin,
            Self::Emby,
            Self::Plex,
        ]
        .into_iter()
        .find(|kind| kind.as_str() == value)
    }
}

#[derive(Serialize, ToSchema)]
pub struct ConnectionSummary {
    pub id: String,
    pub name: String,
    pub kind: ConnectionKind,
}

#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CreateConnection {
    pub name: String,
    pub kind: ConnectionKind,
    pub address: String,
    pub username: Option<String>,
    pub secret: Option<String>,
}

#[derive(Deserialize, Serialize, Default)]
pub struct Credentials {
    pub username: String,
    pub secret: String,
}

#[derive(Clone, Copy, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ConnectionEntryKind {
    Folder,
    Audio,
    Video,
    File,
}

#[derive(Serialize, ToSchema)]
pub struct ConnectionEntry {
    pub id: String,
    pub name: String,
    pub kind: ConnectionEntryKind,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionPage {
    pub entries: Vec<ConnectionEntry>,
    pub next_offset: Option<u32>,
}

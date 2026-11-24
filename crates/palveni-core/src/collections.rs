//! Cross-media collections (`电影 + 原声带 + 原著/漫画 + 相册`) and automatic
//! cross-media work discovery by normalized title stem.

use std::fmt;
use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{CollectionEntryId, CollectionId, Error, LibraryId, UserId};

#[derive(utoipa::ToSchema, Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaCollectionEntryKind {
    Video,
    MusicAlbum,
    Track,
    Book,
    Comic,
    Photo,
}

impl MediaCollectionEntryKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Video => "video",
            Self::MusicAlbum => "music_album",
            Self::Track => "track",
            Self::Book => "book",
            Self::Comic => "comic",
            Self::Photo => "photo",
        }
    }
}

impl fmt::Display for MediaCollectionEntryKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for MediaCollectionEntryKind {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim() {
            "video" => Ok(Self::Video),
            "music_album" => Ok(Self::MusicAlbum),
            "track" => Ok(Self::Track),
            "book" => Ok(Self::Book),
            "comic" => Ok(Self::Comic),
            "photo" => Ok(Self::Photo),
            other => Err(Error::invalid(format!(
                "`{other}` is not a valid collection entry kind"
            ))),
        }
    }
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaCollectionSummary {
    pub id: CollectionId,
    pub owner_id: UserId,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover_artwork_id: Option<String>,
    pub preview_artwork_ids: Vec<String>,
    pub item_count: u32,
    pub kinds: Vec<MediaCollectionEntryKind>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaCollectionList {
    pub collections: Vec<MediaCollectionSummary>,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaCollectionEntry {
    pub id: CollectionEntryId,
    pub collection_id: CollectionId,
    pub entry_kind: MediaCollectionEntryKind,
    pub target_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub library_id: Option<LibraryId>,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtitle: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artwork_id: Option<String>,
    pub position: i32,
    pub added_at: DateTime<Utc>,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RelatedWork {
    pub entry_kind: MediaCollectionEntryKind,
    pub target_id: String,
    pub library_id: LibraryId,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtitle: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artwork_id: Option<String>,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RelatedWorkList {
    pub works: Vec<RelatedWork>,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaCollectionDetail {
    pub collection: MediaCollectionSummary,
    pub items: Vec<MediaCollectionEntry>,
    pub auto_related: Vec<RelatedWork>,
}

#[derive(utoipa::ToSchema, Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateMediaCollectionRequest {
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub cover_artwork_id: Option<String>,
}

#[derive(utoipa::ToSchema, Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateMediaCollectionRequest {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub cover_artwork_id: Option<String>,
    #[serde(default)]
    pub item_order: Option<Vec<CollectionEntryId>>,
}

#[derive(utoipa::ToSchema, Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddMediaCollectionItemRequest {
    pub entry_kind: MediaCollectionEntryKind,
    pub target_id: String,
    #[serde(default)]
    pub library_id: Option<LibraryId>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub subtitle: Option<String>,
    #[serde(default)]
    pub artwork_id: Option<String>,
}

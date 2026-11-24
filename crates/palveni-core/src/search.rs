//! Unified cross-domain global search (`GET /api/v1/search`) powering the `⌘K`
//! Command Palette live omnibox across all media kinds, collections, and smart home.

use serde::{Deserialize, Serialize};

use crate::LibraryId;

#[derive(utoipa::ToSchema, Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GlobalSearchKind {
    VideoWork,
    MusicAlbum,
    Track,
    Book,
    Comic,
    Photo,
    Broadcast,
    Collection,
    HomeDevice,
    HomeScene,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GlobalSearchHit {
    pub kind: GlobalSearchKind,
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub library_id: Option<LibraryId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtitle: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artwork_id: Option<String>,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GlobalSearchResponse {
    pub query: String,
    pub hits: Vec<GlobalSearchHit>,
}

//! Per-account media state shared by native clients.
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{ItemId, ItemKind, LibraryId, ids::PlaylistId};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MediaState {
    pub favorite: bool,
    pub stars: i32,
    pub position_ms: i64,
    pub completed: bool,
    pub play_count: i64,
    #[schema(required = true)]
    pub last_played_at: Option<DateTime<Utc>>,
    pub playback_version: i64,
}

#[derive(Clone, Copy, Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlaybackUpdate {
    #[schema(minimum = 0)]
    pub expected_version: i64,
    #[schema(minimum = 0)]
    pub position_ms: i64,
    pub completed: bool,
}

#[derive(Clone, Copy, Debug, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum MediaView {
    Favorites,
    Recent,
    Continue,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PersonalItem {
    pub item_id: ItemId,
    pub library_id: LibraryId,
    pub title: String,
    pub kind: ItemKind,
    pub state: MediaState,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PersonalItemPage {
    pub items: Vec<PersonalItem>,
    #[schema(required = true)]
    pub next_before: Option<i64>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PlaylistSummary {
    pub id: PlaylistId,
    pub name: String,
    pub version: i64,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Playlist {
    #[serde(flatten)]
    pub summary: PlaylistSummary,
    pub track_ids: Vec<ItemId>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistPage {
    pub playlists: Vec<PlaylistSummary>,
    #[schema(required = true)]
    pub next_before: Option<i64>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistTrack {
    pub position: i32,
    pub item_id: ItemId,
    pub library_id: LibraryId,
    pub title: String,
    #[schema(required = true)]
    pub artist: Option<String>,
    #[schema(required = true)]
    pub album: Option<String>,
    #[schema(required = true)]
    pub duration_ms: Option<i64>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistTrackPage {
    pub tracks: Vec<PlaylistTrack>,
    #[schema(required = true)]
    pub next_after: Option<i32>,
}

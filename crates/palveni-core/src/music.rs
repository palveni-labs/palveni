//! Embedded audio tags and library-scoped browse results.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{ItemId, LibraryId};

pub const LOUDNESS_JOB_KIND: &str = "music.loudness";

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LoudnessValues {
    pub integrated_lufs: Option<f64>,
    pub range_lu: Option<f64>,
    pub true_peak_dbtp: Option<f64>,
}

#[derive(utoipa::ToSchema, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoudnessMeasurement {
    #[schema(required = true)]
    pub integrated_lufs: Option<f64>,
    #[schema(required = true)]
    pub range_lu: Option<f64>,
    #[schema(required = true)]
    pub true_peak_dbtp: Option<f64>,
    pub measured_at: chrono::DateTime<chrono::Utc>,
    pub current: bool,
}

#[derive(utoipa::ToSchema, Debug, Serialize)]
pub struct TrackLoudness {
    #[schema(required = true)]
    pub measurement: Option<LoudnessMeasurement>,
}

#[derive(Clone, Debug, Default)]
pub struct TrackTags {
    pub cue: Option<CueSegment>,
    pub cue_backing: bool,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album_artist: Option<String>,
    pub album: Option<String>,
    pub disc_number: Option<u32>,
    pub track_number: Option<u32>,
    pub duration_ms: Option<i64>,
    /// Local source directory or file that distinguishes identically named books and shows.
    pub spoken_location: Option<String>,
    pub year: Option<i32>,
    /// Distinct genre names in tag order; compared without regard to case.
    pub genres: Vec<String>,
    pub audio: AudioProperties,
}

/// Stream parameters read from the container, not from editable tags.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AudioProperties {
    pub codec: Option<String>,
    pub bitrate_kbps: Option<u32>,
    pub sample_rate_hz: Option<u32>,
    pub bit_depth: Option<u8>,
    pub channels: Option<u8>,
}

#[derive(Clone, Debug)]
pub struct CueSegment {
    pub source: String,
    pub start_frame: i64,
    pub end_frame: i64,
    pub stamp: String,
}

#[derive(utoipa::ToSchema, Debug, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum MusicEntry {
    Artist {
        id: String,
        #[schema(required = true)]
        name: Option<String>,
    },
    Album {
        id: String,
        #[serde(rename = "artworkItemId")]
        artwork_item_id: ItemId,
        #[schema(required = true)]
        name: Option<String>,
    },
    Track {
        id: ItemId,
        title: String,
        #[schema(required = true)]
        artist: Option<String>,
        #[schema(required = true)]
        #[serde(rename = "discNumber")]
        disc_number: Option<u32>,
        #[schema(required = true)]
        #[serde(rename = "trackNumber")]
        track_number: Option<u32>,
        #[schema(required = true)]
        #[serde(rename = "durationMs")]
        duration_ms: Option<i64>,
    },
}

#[derive(utoipa::ToSchema, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MusicPage {
    pub entries: Vec<MusicEntry>,
    #[schema(required = true)]
    pub next_cursor: Option<String>,
}

#[derive(utoipa::ToSchema, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LyricsSource {
    Sidecar,
    Embedded,
}

#[derive(utoipa::ToSchema, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LyricsFormat {
    Lrc,
    Plain,
}

#[derive(utoipa::ToSchema, Debug, Serialize)]
pub struct Lyrics {
    pub source: LyricsSource,
    pub format: LyricsFormat,
    pub text: String,
}

#[derive(utoipa::ToSchema, Debug, Serialize)]
pub struct TrackLyrics {
    #[schema(required = true)]
    pub lyrics: Option<Lyrics>,
}

#[derive(utoipa::ToSchema, Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum MusicSearchKind {
    Artist,
    Album,
    Track,
}

#[derive(utoipa::ToSchema, Clone, Debug, serde::Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MusicSearchHit {
    pub id: String,
    pub title: String,
    #[schema(required = true)]
    pub artist: Option<String>,
    #[schema(required = true)]
    pub album: Option<String>,
    #[schema(required = true)]
    pub artist_id: Option<String>,
}

#[derive(utoipa::ToSchema, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MusicSearchPage {
    pub entries: Vec<MusicSearchHit>,
    #[schema(required = true)]
    pub next_cursor: Option<String>,
}

#[derive(utoipa::ToSchema, Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GlobalMusicSearchHit {
    pub library_id: LibraryId,
    pub library_name: String,
    #[serde(flatten)]
    pub hit: MusicSearchHit,
}

#[derive(utoipa::ToSchema, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GlobalMusicSearchPage {
    pub entries: Vec<GlobalMusicSearchHit>,
    #[schema(required = true)]
    pub next_cursor: Option<String>,
}

#[derive(utoipa::ToSchema, Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum MusicSortOrder {
    Asc,
    Desc,
}

#[derive(utoipa::ToSchema, Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum MusicAlbumSort {
    #[default]
    Title,
    Artist,
    Year,
    Added,
    Played,
    Plays,
    Rating,
    Random,
}

impl MusicAlbumSort {
    pub const fn default_order(self) -> MusicSortOrder {
        match self {
            Self::Title | Self::Artist | Self::Random => MusicSortOrder::Asc,
            Self::Year | Self::Added | Self::Played | Self::Plays | Self::Rating => {
                MusicSortOrder::Desc
            }
        }
    }
}

#[derive(utoipa::ToSchema, Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum MusicArtistSort {
    #[default]
    Name,
    Albums,
    Tracks,
    Plays,
    Random,
}

impl MusicArtistSort {
    pub const fn default_order(self) -> MusicSortOrder {
        match self {
            Self::Name | Self::Random => MusicSortOrder::Asc,
            Self::Albums | Self::Tracks | Self::Plays => MusicSortOrder::Desc,
        }
    }
}

#[derive(utoipa::ToSchema, Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum MusicTrackSort {
    #[default]
    Title,
    Artist,
    Album,
    Year,
    Added,
    Duration,
    Plays,
    Played,
    Rating,
    Random,
}

impl MusicTrackSort {
    pub const fn default_order(self) -> MusicSortOrder {
        match self {
            Self::Title | Self::Artist | Self::Album | Self::Random => MusicSortOrder::Asc,
            Self::Year
            | Self::Added
            | Self::Duration
            | Self::Plays
            | Self::Played
            | Self::Rating => MusicSortOrder::Desc,
        }
    }
}

#[derive(utoipa::ToSchema, Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MusicTrackState {
    pub favorite: bool,
    pub stars: i32,
    pub play_count: i64,
    #[schema(required = true)]
    pub last_played_at: Option<DateTime<Utc>>,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MusicTrack {
    pub id: ItemId,
    pub library_id: LibraryId,
    pub title: String,
    #[schema(required = true)]
    pub artist: Option<String>,
    #[schema(required = true)]
    pub album_artist: Option<String>,
    #[schema(required = true)]
    pub album: Option<String>,
    #[schema(required = true)]
    pub album_id: Option<String>,
    #[schema(required = true)]
    pub artist_id: Option<String>,
    #[schema(required = true)]
    pub track_number: Option<u32>,
    #[schema(required = true)]
    pub disc_number: Option<u32>,
    #[schema(required = true)]
    pub year: Option<i32>,
    pub genres: Vec<String>,
    #[schema(required = true)]
    pub duration_ms: Option<i64>,
    #[schema(required = true)]
    pub artwork_item_id: Option<ItemId>,
    #[schema(required = true)]
    pub codec: Option<String>,
    #[schema(required = true)]
    pub bitrate_kbps: Option<u32>,
    #[schema(required = true)]
    pub sample_rate_hz: Option<u32>,
    #[schema(required = true)]
    pub bit_depth: Option<u8>,
    #[schema(required = true)]
    pub channels: Option<u8>,
    #[schema(required = true)]
    pub size_bytes: Option<i64>,
    pub added_at: DateTime<Utc>,
    pub state: MusicTrackState,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MusicAlbum {
    pub id: String,
    pub library_id: LibraryId,
    #[schema(required = true)]
    pub name: Option<String>,
    #[schema(required = true)]
    pub artist: Option<String>,
    pub artist_id: String,
    #[schema(required = true)]
    pub year: Option<i32>,
    pub genres: Vec<String>,
    pub track_count: i64,
    pub disc_count: i64,
    pub duration_ms: i64,
    #[schema(required = true)]
    pub artwork_item_id: Option<ItemId>,
    pub added_at: DateTime<Utc>,
    pub favorite: bool,
    pub stars: i32,
    pub play_count: i64,
    #[schema(required = true)]
    pub last_played_at: Option<DateTime<Utc>>,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MusicArtist {
    pub id: String,
    pub library_id: LibraryId,
    #[schema(required = true)]
    pub name: Option<String>,
    pub album_count: i64,
    pub track_count: i64,
    #[schema(required = true)]
    pub artwork_item_id: Option<ItemId>,
    pub favorite: bool,
    pub play_count: i64,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MusicGenre {
    pub name: String,
    pub album_count: i64,
    pub track_count: i64,
}

#[derive(utoipa::ToSchema, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MusicAlbumPage {
    pub albums: Vec<MusicAlbum>,
    #[schema(required = true)]
    pub next_cursor: Option<String>,
    pub total: i64,
}

#[derive(utoipa::ToSchema, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MusicAlbumDetail {
    pub album: MusicAlbum,
    pub tracks: Vec<MusicTrack>,
}

#[derive(utoipa::ToSchema, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MusicArtistPage {
    pub artists: Vec<MusicArtist>,
    #[schema(required = true)]
    pub next_cursor: Option<String>,
    pub total: i64,
}

#[derive(utoipa::ToSchema, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MusicArtistDetail {
    pub artist: MusicArtist,
    pub albums: Vec<MusicAlbum>,
    pub top_tracks: Vec<MusicTrack>,
}

#[derive(utoipa::ToSchema, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MusicGenreList {
    pub genres: Vec<MusicGenre>,
}

#[derive(utoipa::ToSchema, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MusicTrackPage {
    pub tracks: Vec<MusicTrack>,
    #[schema(required = true)]
    pub next_cursor: Option<String>,
    pub total: i64,
}

#[derive(utoipa::ToSchema, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MusicTrackLookupRequest {
    #[schema(max_items = 500)]
    pub ids: Vec<ItemId>,
}

#[derive(utoipa::ToSchema, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MusicTrackLookupResponse {
    pub tracks: Vec<MusicTrack>,
}

//! Embedded audio tags and library-scoped browse results.

use serde::Serialize;

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

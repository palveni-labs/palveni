//! Embedded audio tags and library-scoped browse results.

use serde::Serialize;

use crate::ItemId;

#[derive(Clone, Debug, Default)]
pub struct TrackTags {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album_artist: Option<String>,
    pub album: Option<String>,
    pub disc_number: Option<u32>,
    pub track_number: Option<u32>,
    pub duration_ms: Option<i64>,
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

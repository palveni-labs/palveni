use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{ItemId, LibraryId};

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum VideoOrder {
    #[default]
    Title,
    Poster,
    Year,
    Added,
    Rating,
    Runtime,
    Random,
}

impl VideoOrder {
    /// Orders that keep the original keyset cursors.
    pub fn is_keyset(self) -> bool {
        matches!(self, Self::Title | Self::Poster)
    }

    pub fn default_dir(self) -> SortDir {
        match self {
            Self::Year | Self::Added | Self::Rating => SortDir::Desc,
            _ => SortDir::Asc,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SortDir {
    Asc,
    Desc,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum WorkKind {
    Movie,
    Series,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum VideoKind {
    Movie,
    Series,
    Season,
    Episode,
}

/// Per-account state. For series and seasons the values summarize episodes.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct VideoUserState {
    pub played: bool,
    pub position_ms: i64,
    pub play_count: i64,
    pub favorite: bool,
    #[schema(required = true)]
    pub last_played_at: Option<DateTime<Utc>>,
    /// Series and seasons only.
    #[schema(required = true)]
    pub unplayed_count: Option<u64>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct VideoEntry {
    pub id: String,
    pub kind: VideoKind,
    pub library_id: LibraryId,
    pub title: String,
    pub file_count: u64,
    #[schema(required = true)]
    pub year: Option<i32>,
    #[schema(required = true)]
    pub overview: Option<String>,
    #[schema(required = true)]
    pub season_number: Option<i32>,
    #[schema(required = true)]
    pub episode_number: Option<i32>,
    #[schema(required = true)]
    pub poster_item_id: Option<ItemId>,
    #[schema(required = true)]
    pub item_id: Option<ItemId>,
    /// The series work for seasons and episodes.
    #[schema(required = true)]
    pub series_id: Option<String>,
    #[schema(required = true)]
    pub series_title: Option<String>,
    #[schema(required = true)]
    pub original_title: Option<String>,
    #[schema(required = true)]
    pub tagline: Option<String>,
    pub genres: Vec<String>,
    #[schema(required = true)]
    pub runtime_ms: Option<i64>,
    /// 0–10.
    #[schema(required = true)]
    pub community_rating: Option<f64>,
    #[schema(required = true)]
    pub official_rating: Option<String>,
    /// `YYYY-MM-DD`.
    #[schema(required = true)]
    pub premiered: Option<String>,
    #[schema(required = true)]
    pub backdrop_item_id: Option<ItemId>,
    #[schema(required = true)]
    pub thumb_item_id: Option<ItemId>,
    pub added_at: DateTime<Utc>,
    #[schema(required = true)]
    pub state: Option<VideoUserState>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct VideoPage {
    pub entries: Vec<VideoEntry>,
    #[schema(required = true)]
    pub next_cursor: Option<String>,
    /// Number of entries matching the filters.
    pub total: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum VideoPersonKind {
    Actor = 0,
    Director = 1,
    Writer = 2,
    Producer = 3,
}

impl VideoPersonKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Actor => "actor",
            Self::Director => "director",
            Self::Writer => "writer",
            Self::Producer => "producer",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "actor" => Self::Actor,
            "director" => Self::Director,
            "writer" => Self::Writer,
            "producer" => Self::Producer,
            _ => return None,
        })
    }
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct VideoPerson {
    pub name: String,
    #[schema(required = true)]
    pub role: Option<String>,
    pub kind: VideoPersonKind,
    /// Relative URL of a local portrait.
    #[schema(required = true)]
    pub image_url: Option<String>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct VideoFile {
    pub item_id: ItemId,
    pub title: String,
    #[schema(required = true)]
    pub size_bytes: Option<i64>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct VideoWorkDetail {
    pub work: VideoEntry,
    pub studios: Vec<String>,
    pub people: Vec<VideoPerson>,
    pub seasons: Vec<VideoEntry>,
    pub files: Vec<VideoFile>,
    #[schema(required = true)]
    pub next_up: Option<VideoEntry>,
    pub similar: Vec<VideoEntry>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct VideoGenre {
    pub name: String,
    pub count: u64,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct VideoGenreList {
    pub genres: Vec<VideoGenre>,
}

#[derive(Debug, Default, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct VideoHome {
    pub continue_watching: Vec<VideoEntry>,
    pub next_up: Vec<VideoEntry>,
    pub latest_movies: Vec<VideoEntry>,
    pub latest_episodes: Vec<VideoEntry>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum SubtitleFormat {
    Srt,
    Vtt,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SubtitleTrack {
    pub id: String,
    #[schema(required = true)]
    pub language: Option<String>,
    pub forced: bool,
    pub hearing_impaired: bool,
    pub format: SubtitleFormat,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct SubtitleList {
    pub tracks: Vec<SubtitleTrack>,
    pub truncated: bool,
}

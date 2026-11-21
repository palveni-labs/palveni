use serde::{Deserialize, Serialize};

use crate::ItemId;

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum VideoOrder {
    #[default]
    Title,
    Poster,
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

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct VideoEntry {
    pub id: String,
    pub kind: VideoKind,
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
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct VideoPage {
    pub entries: Vec<VideoEntry>,
    #[schema(required = true)]
    pub next_cursor: Option<String>,
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

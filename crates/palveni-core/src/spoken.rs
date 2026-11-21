use serde::Serialize;

use crate::ItemId;

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SpokenRecording {
    pub id: ItemId,
    pub title: String,
    #[schema(required = true)]
    pub artist: Option<String>,
    #[schema(required = true)]
    pub disc_number: Option<u32>,
    #[schema(required = true)]
    pub number: Option<u32>,
    #[schema(required = true)]
    pub duration_ms: Option<i64>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum SpokenEntry {
    Book { id: String, title: String },
    Show { id: String, title: String },
    Chapter(SpokenRecording),
    Episode(SpokenRecording),
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SpokenPage {
    pub entries: Vec<SpokenEntry>,
    #[schema(required = true)]
    pub next_cursor: Option<String>,
}

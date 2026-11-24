//! Online metadata scraping (TMDB / Bangumi / OpenLibrary) and manual item metadata overrides.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::ids::ItemId;

#[derive(utoipa::ToSchema, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MetadataProvider {
    Manual,
    Tmdb,
    Bangumi,
    Openlibrary,
}

impl MetadataProvider {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Manual => "manual",
            Self::Tmdb => "tmdb",
            Self::Bangumi => "bangumi",
            Self::Openlibrary => "openlibrary",
        }
    }

    pub fn from_db(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "manual" => Some(Self::Manual),
            "tmdb" => Some(Self::Tmdb),
            "bangumi" => Some(Self::Bangumi),
            "openlibrary" => Some(Self::Openlibrary),
            _ => None,
        }
    }
}

#[derive(utoipa::ToSchema, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MetadataSearchMediaType {
    Movie,
    Tv,
    Book,
    Anime,
}

impl MetadataSearchMediaType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Movie => "movie",
            Self::Tv => "tv",
            Self::Book => "book",
            Self::Anime => "anime",
        }
    }
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetadataConfigResponse {
    pub tmdb_configured: bool,
    pub bangumi_configured: bool,
    pub preferred_language: String,
    pub auto_fetch_posters: bool,
}

#[derive(utoipa::ToSchema, Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateMetadataConfigRequest {
    #[serde(default)]
    pub tmdb_api_key: Option<String>,
    #[serde(default)]
    pub clear_tmdb_key: Option<bool>,
    #[serde(default)]
    pub bangumi_token: Option<String>,
    #[serde(default)]
    pub clear_bangumi_token: Option<bool>,
    #[serde(default)]
    pub preferred_language: Option<String>,
    #[serde(default)]
    pub auto_fetch_posters: Option<bool>,
}

#[derive(utoipa::ToSchema, Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MetadataSearchRequest {
    pub provider: MetadataProvider,
    pub query: String,
    #[serde(default)]
    pub media_type: Option<MetadataSearchMediaType>,
    #[serde(default)]
    pub year: Option<i32>,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetadataSearchCandidate {
    pub provider: MetadataProvider,
    pub external_id: String,
    pub title: String,
    #[schema(required = true)]
    pub original_title: Option<String>,
    #[schema(required = true)]
    pub year: Option<i32>,
    #[schema(required = true)]
    pub overview: Option<String>,
    #[schema(required = true)]
    pub rating: Option<f64>,
    pub genres: Vec<String>,
    #[schema(required = true)]
    pub poster_url: Option<String>,
    #[schema(required = true)]
    pub backdrop_url: Option<String>,
    pub media_type: String,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetadataSearchResponse {
    pub provider: MetadataProvider,
    pub query: String,
    pub candidates: Vec<MetadataSearchCandidate>,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetadataOverride {
    pub item_id: ItemId,
    #[schema(required = true)]
    pub work_key: Option<String>,
    pub provider: MetadataProvider,
    #[schema(required = true)]
    pub external_id: Option<String>,
    #[schema(required = true)]
    pub title: Option<String>,
    #[schema(required = true)]
    pub original_title: Option<String>,
    #[schema(required = true)]
    pub year: Option<i32>,
    #[schema(required = true)]
    pub overview: Option<String>,
    #[schema(required = true)]
    pub rating: Option<f64>,
    pub genres: Vec<String>,
    #[schema(required = true)]
    pub poster_url: Option<String>,
    #[schema(required = true)]
    pub backdrop_url: Option<String>,
    pub updated_at: DateTime<Utc>,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetadataOverrideResponse {
    pub item_id: ItemId,
    #[serde(rename = "override")]
    #[schema(required = true)]
    pub override_data: Option<MetadataOverride>,
}

#[derive(utoipa::ToSchema, Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SaveMetadataOverrideRequest {
    #[serde(default)]
    pub work_key: Option<String>,
    #[serde(default)]
    pub provider: Option<MetadataProvider>,
    #[serde(default)]
    pub external_id: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub original_title: Option<String>,
    #[serde(default)]
    pub year: Option<i32>,
    #[serde(default)]
    pub overview: Option<String>,
    #[serde(default)]
    pub rating: Option<f64>,
    #[serde(default)]
    pub genres: Option<Vec<String>>,
    #[serde(default)]
    pub poster_url: Option<String>,
    #[serde(default)]
    pub backdrop_url: Option<String>,
}

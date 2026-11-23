//! Domain types and wire contracts for the Photos & Albums module.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{ItemId, LibraryId, PhotoAlbumId};

/// Format a clean camera display name from EXIF `Make` and `Model` without repeating the brand.
pub fn format_camera_name(make: Option<&str>, model: Option<&str>) -> Option<String> {
    let mk = make.map(str::trim).filter(|s| !s.is_empty());
    let md = model.map(str::trim).filter(|s| !s.is_empty());
    match (mk, md) {
        (Some(mk), Some(md)) => {
            if md
                .to_ascii_lowercase()
                .starts_with(&mk.to_ascii_lowercase())
            {
                Some(md.to_owned())
            } else {
                Some(format!("{mk} {md}"))
            }
        }
        (None, Some(md)) => Some(md.to_owned()),
        (Some(mk), None) => Some(mk.to_owned()),
        (None, None) => None,
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PhotoTimelineItem {
    pub id: ItemId,
    pub library_id: LibraryId,
    pub title: String,
    pub taken_at: DateTime<Utc>,
    pub day_bucket: String,
    pub month_bucket: String,
    #[schema(required = true)]
    pub width: Option<i32>,
    #[schema(required = true)]
    pub height: Option<i32>,
    pub orientation: i32,
    #[schema(required = true)]
    pub camera_make: Option<String>,
    #[schema(required = true)]
    pub camera_model: Option<String>,
    #[schema(required = true)]
    pub latitude: Option<f64>,
    #[schema(required = true)]
    pub longitude: Option<f64>,
    pub favorite: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PhotoMonthBucket {
    pub month: String,
    pub count: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PhotoTimelinePage {
    pub items: Vec<PhotoTimelineItem>,
    pub months: Vec<PhotoMonthBucket>,
    #[schema(required = true)]
    pub next_cursor: Option<String>,
    pub total: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PhotoPlaceCluster {
    pub key: String,
    pub label: String,
    pub latitude: f64,
    pub longitude: f64,
    pub count: u64,
    pub cover_item_id: ItemId,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PhotoCameraBucket {
    pub name: String,
    pub count: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PhotoMemories {
    pub on_this_day: Vec<PhotoTimelineItem>,
    pub recent_highlights: Vec<PhotoTimelineItem>,
    pub places: Vec<PhotoPlaceCluster>,
    pub cameras: Vec<PhotoCameraBucket>,
    pub total_photos: u64,
    pub geotagged_count: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PhotoAlbumRef {
    pub id: PhotoAlbumId,
    pub title: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PhotoInfo {
    pub id: ItemId,
    pub library_id: LibraryId,
    pub title: String,
    pub file_name: String,
    pub path: String,
    #[schema(required = true)]
    pub size_bytes: Option<i64>,
    pub taken_at: DateTime<Utc>,
    pub day_bucket: String,
    pub month_bucket: String,
    #[schema(required = true)]
    pub width: Option<i32>,
    #[schema(required = true)]
    pub height: Option<i32>,
    pub orientation: i32,
    #[schema(required = true)]
    pub camera_make: Option<String>,
    #[schema(required = true)]
    pub camera_model: Option<String>,
    #[schema(required = true)]
    pub lens_model: Option<String>,
    #[schema(required = true)]
    pub iso: Option<i32>,
    #[schema(required = true)]
    pub f_number: Option<f64>,
    #[schema(required = true)]
    pub exposure_time: Option<String>,
    #[schema(required = true)]
    pub focal_length: Option<f64>,
    #[schema(required = true)]
    pub latitude: Option<f64>,
    #[schema(required = true)]
    pub longitude: Option<f64>,
    #[schema(required = true)]
    pub content_hash: Option<String>,
    pub favorite: bool,
    pub albums: Vec<PhotoAlbumRef>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PhotoAlbumSummary {
    pub id: PhotoAlbumId,
    pub title: String,
    #[schema(required = true)]
    pub description: Option<String>,
    #[schema(required = true)]
    pub cover_item_id: Option<ItemId>,
    pub preview_item_ids: Vec<ItemId>,
    pub item_count: u64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PhotoAlbumList {
    pub albums: Vec<PhotoAlbumSummary>,
}

#[derive(Clone, Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreatePhotoAlbumRequest {
    #[schema(min_length = 1, max_length = 128)]
    pub title: String,
    #[schema(max_length = 1024)]
    pub description: Option<String>,
    pub cover_item_id: Option<ItemId>,
}

#[derive(Clone, Debug, Default, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdatePhotoAlbumRequest {
    #[schema(min_length = 1, max_length = 128)]
    pub title: Option<String>,
    #[schema(max_length = 1024)]
    pub description: Option<String>,
    pub cover_item_id: Option<ItemId>,
}

#[derive(Clone, Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PhotoAlbumItemsRequest {
    #[schema(min_items = 1, max_items = 500)]
    pub item_ids: Vec<ItemId>,
}

#[derive(Clone, Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PhotoUploadRequest {
    #[schema(min_length = 1, max_length = 255)]
    pub file_name: String,
    pub data_base64: String,
}

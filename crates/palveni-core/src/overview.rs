//! Overview storage report, live metrics, and catalog cover art types

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageVolume {
    pub id: String,
    pub label: String,
    pub path: String,
    pub total_bytes: u64,
    pub used_bytes: u64,
}

#[derive(utoipa::ToSchema, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MountState {
    Online,
    Offline,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageMount {
    pub id: String,
    pub label: String,
    pub state: MountState,
    pub affected_libraries: usize,
    pub since: DateTime<Utc>,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryBytes {
    pub library_id: String,
    pub name: String,
    pub bytes: u64,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageReport {
    pub volumes: Vec<StorageVolume>,
    pub by_library: Vec<LibraryBytes>,
    pub mounts: Vec<StorageMount>,
}

#[derive(utoipa::ToSchema, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CatalogWorkKind {
    Movie,
    Series,
    Album,
    Book,
    Photo,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogWork {
    pub id: String,
    pub title: String,
    pub kind: CatalogWorkKind,
    #[schema(required = true)]
    pub meta: Option<String>,
    #[schema(required = true)]
    pub artwork_id: Option<String>,
    #[schema(required = true)]
    pub library_id: Option<String>,
    pub added_at: DateTime<Utc>,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Catalog {
    pub recent: Vec<CatalogWork>,
    pub library_covers: BTreeMap<String, Vec<String>>,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemMetrics {
    pub cpu: f64,
    pub memory_used_bytes: u64,
    pub memory_total_bytes: u64,
    pub devices_online: usize,
}

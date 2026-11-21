use axum::Json;
use axum::extract::rejection::{PathRejection, QueryRejection};
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use palveni_core::{Library, LibraryId};
use palveni_db::audit::AuditEntry;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::auth::AdminUser;
use crate::error::ApiError;
use crate::routes::{audit, libraries::ScanAccepted, path_id};
use crate::state::AppState;

#[derive(utoipa::ToSchema, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanReview {
    scope: String,
    revision: String,
    missing_count: u64,
    sample_paths: Vec<String>,
}

#[derive(utoipa::ToSchema, Serialize)]
pub struct ScanReviewList {
    reviews: Vec<ScanReview>,
}

#[derive(utoipa::ToSchema, Serialize)]
pub struct ScanReviewPaths {
    paths: Vec<String>,
    total: u64,
}

#[derive(Deserialize)]
pub struct Page {
    offset: Option<u32>,
    limit: Option<u32>,
}

fn expired() -> ApiError {
    ApiError::conflict("This review is no longer current")
}

async fn library(state: &AppState, raw: &str) -> Result<Library, ApiError> {
    let id: LibraryId = path_id(raw, "Library")?;
    state
        .db
        .libraries()
        .get(id)
        .await?
        .ok_or_else(|| ApiError::not_found("Library not found"))
}

pub async fn list(
    _admin: AdminUser,
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
) -> Result<Json<ScanReviewList>, ApiError> {
    let library = library(&state, &path?.0).await?;
    let reviews = state
        .db
        .scan_reviews()
        .list(library.id)
        .await?
        .into_iter()
        .map(|r| ScanReview {
            scope: r.scope,
            revision: r.revision,
            missing_count: r.missing_count,
            sample_paths: r.sample_paths,
        })
        .collect();
    Ok(Json(ScanReviewList { reviews }))
}

pub async fn paths(
    _admin: AdminUser,
    State(state): State<AppState>,
    path: Result<Path<(String, String)>, PathRejection>,
    query: Result<Query<Page>, QueryRejection>,
) -> Result<Json<ScanReviewPaths>, ApiError> {
    let Path((id, revision)) = path?;
    let library = library(&state, &id).await?;
    let Query(page) = query?;
    let limit = page.limit.unwrap_or(50);
    if !(1..=100).contains(&limit) {
        return Err(ApiError::validation("limit must be between 1 and 100").field("limit"));
    }
    let (paths, total) = state
        .db
        .scan_reviews()
        .paths(
            library.id,
            &revision,
            u64::from(page.offset.unwrap_or(0)),
            limit,
        )
        .await?
        .ok_or_else(expired)?;
    Ok(Json(ScanReviewPaths { paths, total }))
}

pub async fn confirm(
    AdminUser(admin): AdminUser,
    State(state): State<AppState>,
    path: Result<Path<(String, String)>, PathRejection>,
) -> Result<(StatusCode, Json<ScanAccepted>), ApiError> {
    let Path((id, revision)) = path?;
    let library = library(&state, &id).await?;
    if !state
        .db
        .scan_reviews()
        .contains(library.id, &revision)
        .await?
    {
        return Err(expired());
    }
    let (job, created) = state
        .jobs
        .enqueue_scan_with_review(library.id, library.name, Some(revision.clone()))
        .await?;
    if !created {
        return Err(ApiError::conflict(
            "A scan is already active for this library",
        ));
    }
    audit(
        &state,
        AuditEntry {
            action: "library.scan_review_confirmed",
            actor: Some(admin.user.id),
            target: Some(library.id.to_string()),
            detail: Some(json!({"revision": revision, "jobId": job.id})),
        },
    )
    .await;
    Ok((StatusCode::ACCEPTED, Json(ScanAccepted { job_id: job.id })))
}

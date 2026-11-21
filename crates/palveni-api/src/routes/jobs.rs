use axum::Json;
use axum::extract::rejection::QueryRejection;
use axum::extract::{Query, State};
use palveni_core::Job;
use serde::{Deserialize, Serialize};

use crate::auth::AdminUser;
use crate::error::ApiError;
use crate::routes::{DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE, parse_limit};
use crate::state::AppState;

#[derive(Deserialize)]
pub struct JobsQuery {
    limit: Option<String>,
}

#[derive(utoipa::ToSchema, Serialize)]
pub struct JobList {
    jobs: Vec<Job>,
}

pub async fn cancel(
    _admin: AdminUser,
    State(state): State<AppState>,
    path: Result<axum::extract::Path<String>, axum::extract::rejection::PathRejection>,
) -> Result<axum::http::StatusCode, ApiError> {
    let id = super::path_id(&path?.0, "Job")?;
    state.jobs.request_cancel(id).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

/// Most recent jobs first, with live progress for running ones.
pub async fn list(
    _admin: AdminUser,
    State(state): State<AppState>,
    query: Result<Query<JobsQuery>, QueryRejection>,
) -> Result<Json<JobList>, ApiError> {
    let Query(query) = query?;
    let limit = parse_limit(query.limit.as_deref(), DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE)?;
    let jobs = state.jobs.list_recent(u64::from(limit)).await?;
    Ok(Json(JobList { jobs }))
}

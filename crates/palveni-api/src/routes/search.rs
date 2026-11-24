//! Global cross-domain search endpoint (`GET /api/v1/search`).

use axum::{
    Json,
    extract::{Query, State, rejection::QueryRejection},
};
use palveni_core::search::GlobalSearchResponse;
use serde::Deserialize;

use super::parse_limit;
use crate::{AppState, auth::CurrentUser, error::ApiError};

#[derive(Deserialize)]
pub struct SearchQuery {
    q: Option<String>,
    limit: Option<String>,
}

/// `GET /api/v1/search?q=...&limit=...`
pub async fn search_all(
    user: CurrentUser,
    State(state): State<AppState>,
    query: Result<Query<SearchQuery>, QueryRejection>,
) -> Result<Json<GlobalSearchResponse>, ApiError> {
    let Query(params) = query?;
    let q = params.q.as_deref().map(str::trim).unwrap_or("");
    if q.chars().count() > 128 || q.chars().any(char::is_control) {
        return Err(ApiError::validation("Invalid search query").field("q"));
    }
    let limit = parse_limit(params.limit.as_deref(), 20, 60)? as usize;
    let hits = if q.is_empty() {
        Vec::new()
    } else {
        state
            .db
            .global_search()
            .search_all(user.user.id, q, limit)
            .await?
    };
    Ok(Json(GlobalSearchResponse {
        query: q.to_owned(),
        hits,
    }))
}

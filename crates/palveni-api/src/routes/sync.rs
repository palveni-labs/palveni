//! Native client incremental sync (`/api/v1/sync/*`, module `sync`).
//!
//! Serves revision-based delta feeds (`GET /api/v1/sync/changes`) and
//! idempotent offline mutation batches (`POST /api/v1/sync/push`) for native
//! clients (Primuse, Viozi, Elumi) while enforcing per-user library ACLs.

use axum::Json;
use axum::extract::{Query, State};
use palveni_core::LibraryId;
use palveni_core::sync_cast::{
    SyncChangesResponse, SyncMutation, SyncPushRequest, SyncPushResponse,
};
use palveni_db::sync_cast::SyncCursor;
use serde::Deserialize;

use crate::auth::CurrentUser;
use crate::error::ApiError;
use crate::json::ApiJson;
use crate::routes::parse_limit;
use crate::state::AppState;

const DEFAULT_SYNC_LIMIT: u32 = 200;
const MAX_SYNC_LIMIT: u32 = 500;
const MAX_PUSH_MUTATIONS: usize = 500;

#[derive(Deserialize)]
pub struct SyncChangesQuery {
    since: Option<String>,
    limit: Option<String>,
    cursor: Option<String>,
}

fn parse_since(raw: Option<&str>) -> Result<i64, ApiError> {
    match raw.map(str::trim).filter(|s| !s.is_empty()) {
        None => Ok(0),
        Some(text) => {
            let value: i64 = text.parse().map_err(|_| {
                ApiError::validation("since must be a nonnegative integer").field("since")
            })?;
            if value < 0 {
                return Err(
                    ApiError::validation("since must be a nonnegative integer").field("since")
                );
            }
            Ok(value)
        }
    }
}

async fn resolve_accessible_libraries(
    state: &AppState,
    current: &CurrentUser,
) -> Result<(Vec<LibraryId>, bool), ApiError> {
    let all_live = state.db.libraries().list().await?;
    let allowed = state
        .db
        .accounts()
        .filter_libraries(current.user.id, current.user.role, all_live)
        .await?;
    let all_allowed = current.is_admin()
        || state
            .db
            .accounts()
            .get_policy(current.user.id)
            .await?
            .all_libraries;
    Ok((allowed.into_iter().map(|lib| lib.id).collect(), all_allowed))
}

/// `GET /api/v1/sync/changes?since=<i64>&limit=<u32>&cursor=<opaque>`
pub async fn changes(
    current: CurrentUser,
    State(state): State<AppState>,
    Query(query): Query<SyncChangesQuery>,
) -> Result<Json<SyncChangesResponse>, ApiError> {
    let since = parse_since(query.since.as_deref())?;
    let limit = parse_limit(query.limit.as_deref(), DEFAULT_SYNC_LIMIT, MAX_SYNC_LIMIT)?;
    let cursor = match query
        .cursor
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        None => None,
        Some(raw) => Some(
            SyncCursor::decode(raw)
                .ok_or_else(|| ApiError::validation("Invalid sync cursor").field("cursor"))?,
        ),
    };
    let (accessible_ids, all_allowed) = resolve_accessible_libraries(&state, &current).await?;
    let response = state
        .db
        .sync_cast()
        .changes(
            current.user.id,
            &accessible_ids,
            all_allowed,
            since,
            limit,
            cursor.as_ref(),
        )
        .await?;
    Ok(Json(response))
}

/// `POST /api/v1/sync/push`
pub async fn push(
    current: CurrentUser,
    State(state): State<AppState>,
    body: Result<ApiJson<SyncPushRequest>, ApiError>,
) -> Result<Json<SyncPushResponse>, ApiError> {
    let ApiJson(req) = body?;
    let client_id = req.client_id.trim();
    if client_id.is_empty()
        || client_id.chars().count() > 128
        || client_id.chars().any(char::is_control)
    {
        return Err(
            ApiError::validation("clientId must be 1 to 128 printable characters")
                .field("clientId"),
        );
    }
    if req.mutations.len() > MAX_PUSH_MUTATIONS {
        return Err(ApiError::validation(format!(
            "At most {MAX_PUSH_MUTATIONS} mutations may be pushed per request"
        ))
        .field("mutations"));
    }

    for (index, mutation) in req.mutations.iter().enumerate() {
        match mutation {
            SyncMutation::SetFavorite { base_revision, .. } => {
                if base_revision.is_some_and(|r| r < 0) {
                    return Err(ApiError::validation("baseRevision must be nonnegative")
                        .field_at("baseRevision", index));
                }
            }
            SyncMutation::SetProgress {
                position_ms,
                base_revision,
                ..
            } => {
                if *position_ms < 0 {
                    return Err(ApiError::validation("positionMs must be nonnegative")
                        .field_at("positionMs", index));
                }
                if base_revision.is_some_and(|r| r < 0) {
                    return Err(ApiError::validation("baseRevision must be nonnegative")
                        .field_at("baseRevision", index));
                }
            }
            SyncMutation::AddReadingBookmark {
                page,
                label,
                base_revision,
                ..
            } => {
                if *page < 0 {
                    return Err(
                        ApiError::validation("page must be nonnegative").field_at("page", index)
                    );
                }
                let trimmed = label.trim();
                if trimmed.is_empty()
                    || trimmed.chars().count() > 200
                    || trimmed.chars().any(char::is_control)
                {
                    return Err(ApiError::validation(
                        "label must be 1 to 200 printable characters",
                    )
                    .field_at("label", index));
                }
                if base_revision.is_some_and(|r| r < 0) {
                    return Err(ApiError::validation("baseRevision must be nonnegative")
                        .field_at("baseRevision", index));
                }
            }
            SyncMutation::RemoveReadingBookmark { base_revision, .. } => {
                if base_revision.is_some_and(|r| r < 0) {
                    return Err(ApiError::validation("baseRevision must be nonnegative")
                        .field_at("baseRevision", index));
                }
            }
        }
    }

    let (accessible_ids, _) = resolve_accessible_libraries(&state, &current).await?;
    let response = state
        .db
        .sync_cast()
        .push_mutations(current.user.id, &accessible_ids, &req.mutations)
        .await?;
    Ok(Json(response))
}

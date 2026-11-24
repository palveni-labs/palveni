//! Cross-media collections and related works discovery HTTP endpoints
//! (`/api/v1/collections*`, `/api/v1/related`).

use axum::{
    Json,
    extract::{
        Path, Query, State,
        rejection::{PathRejection, QueryRejection},
    },
    http::StatusCode,
    response::{IntoResponse, Response},
};
use palveni_core::collections::{
    AddMediaCollectionItemRequest, CreateMediaCollectionRequest, MediaCollectionDetail,
    MediaCollectionEntry, MediaCollectionEntryKind, MediaCollectionList, MediaCollectionSummary,
    RelatedWorkList, UpdateMediaCollectionRequest,
};
use palveni_core::{CollectionEntryId, CollectionId};
use palveni_db::collections::{AddCollectionItemInput, CollectionPatch};
use serde::Deserialize;

use super::{parse_limit, path_id};
use crate::{AppState, auth::CurrentUser, error::ApiError, json::ApiJson};

/// `GET /api/v1/collections`
pub async fn list(
    user: CurrentUser,
    State(state): State<AppState>,
) -> Result<Json<MediaCollectionList>, ApiError> {
    let collections = state.db.collections().list(user.user.id).await?;
    Ok(Json(MediaCollectionList { collections }))
}

/// `POST /api/v1/collections`
pub async fn create(
    user: CurrentUser,
    State(state): State<AppState>,
    ApiJson(body): ApiJson<CreateMediaCollectionRequest>,
) -> Result<Response, ApiError> {
    let created = state
        .db
        .collections()
        .create(
            user.user.id,
            &body.title,
            body.description.as_deref(),
            body.cover_artwork_id.as_deref(),
        )
        .await?;
    Ok((StatusCode::CREATED, Json::<MediaCollectionSummary>(created)).into_response())
}

/// `GET /api/v1/collections/{id}`
pub async fn get_one(
    user: CurrentUser,
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
) -> Result<Json<MediaCollectionDetail>, ApiError> {
    let id: CollectionId = path_id(&path?.0, "Collection")?;
    let detail = state
        .db
        .collections()
        .get_detail(user.user.id, id, user.is_admin())
        .await?
        .ok_or_else(|| ApiError::not_found("Collection not found"))?;
    Ok(Json(detail))
}

/// `PATCH /api/v1/collections/{id}`
pub async fn update(
    user: CurrentUser,
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
    ApiJson(body): ApiJson<UpdateMediaCollectionRequest>,
) -> Result<Json<MediaCollectionSummary>, ApiError> {
    let id: CollectionId = path_id(&path?.0, "Collection")?;
    let updated = state
        .db
        .collections()
        .update(
            user.user.id,
            id,
            user.is_admin(),
            CollectionPatch {
                title: body.title.as_deref(),
                description: body.description.as_deref(),
                cover_artwork_id: body.cover_artwork_id.as_deref(),
                item_order: body.item_order.as_deref(),
            },
        )
        .await?;
    Ok(Json(updated))
}

/// `DELETE /api/v1/collections/{id}`
pub async fn delete(
    user: CurrentUser,
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
) -> Result<StatusCode, ApiError> {
    let id: CollectionId = path_id(&path?.0, "Collection")?;
    if !state
        .db
        .collections()
        .delete(user.user.id, id, user.is_admin())
        .await?
    {
        return Err(ApiError::not_found("Collection not found"));
    }
    Ok(StatusCode::NO_CONTENT)
}

/// `POST /api/v1/collections/{id}/items`
pub async fn add_item(
    user: CurrentUser,
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
    ApiJson(body): ApiJson<AddMediaCollectionItemRequest>,
) -> Result<Response, ApiError> {
    let collection_id: CollectionId = path_id(&path?.0, "Collection")?;
    let entry = state
        .db
        .collections()
        .add_item(
            user.user.id,
            collection_id,
            user.is_admin(),
            AddCollectionItemInput {
                entry_kind: body.entry_kind,
                target_id: &body.target_id,
                library_id: body.library_id,
                title: body.title.as_deref(),
                subtitle: body.subtitle.as_deref(),
                artwork_id: body.artwork_id.as_deref(),
            },
        )
        .await?;
    Ok((StatusCode::CREATED, Json::<MediaCollectionEntry>(entry)).into_response())
}

/// `DELETE /api/v1/collections/{id}/items/{entryId}`
pub async fn remove_item(
    user: CurrentUser,
    State(state): State<AppState>,
    path: Result<Path<(String, String)>, PathRejection>,
) -> Result<StatusCode, ApiError> {
    let Path((raw_coll, raw_entry)) = path?;
    let collection_id: CollectionId = path_id(&raw_coll, "Collection")?;
    let entry_id: CollectionEntryId = path_id(&raw_entry, "Collection item")?;
    if !state
        .db
        .collections()
        .remove_item(user.user.id, collection_id, entry_id, user.is_admin())
        .await?
    {
        return Err(ApiError::not_found("Collection item not found"));
    }
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RelatedQuery {
    title: Option<String>,
    exclude_kind: Option<MediaCollectionEntryKind>,
    limit: Option<String>,
}

/// `GET /api/v1/related?title=...&excludeKind=...`
pub async fn related(
    _user: CurrentUser,
    State(state): State<AppState>,
    query: Result<Query<RelatedQuery>, QueryRejection>,
) -> Result<Json<RelatedWorkList>, ApiError> {
    let Query(q) = query?;
    let title = q
        .title
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| ApiError::validation("title is required").field("title"))?;
    if title.chars().count() > 255 || title.chars().any(char::is_control) {
        return Err(ApiError::validation("Invalid title query").field("title"));
    }
    let limit = parse_limit(q.limit.as_deref(), 16, 50)? as usize;
    let works = state
        .db
        .collections()
        .related_across_media(title, q.exclude_kind, limit)
        .await?;
    Ok(Json(RelatedWorkList { works }))
}

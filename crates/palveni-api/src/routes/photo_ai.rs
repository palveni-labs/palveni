//! Pure-local Photo AI analysis, bilingual natural-language search,
//! exact & perceptual duplicate grouping, and owner-scoped people tagging.

use std::collections::HashSet;

use axum::Json;
use axum::body::Bytes;
use axum::extract::rejection::{PathRejection, QueryRejection};
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use chrono::Datelike;
use palveni_core::photos::{
    CreatePhotoPersonRequest, PhotoAnalyzeResponse, PhotoDuplicatesResponse,
    PhotoItemPeopleResponse, PhotoPersonList, PhotoPersonSummary, PhotoSearchHit,
    PhotoSearchResponse, UpdatePhotoPersonRequest,
};
use palveni_core::{ItemId, LibraryId, LibraryKind, PhotoPersonId, clock};
use palveni_db::DbError;
use palveni_library::photos::ai::{
    analyze_unanalyzed_photo, build_duplicate_clusters, matches_photo_query, parse_photo_query,
};
use serde::Deserialize;

use super::{MAX_PAGE_SIZE, parse_limit, path_id, query_id};
use crate::auth::CurrentUser;
use crate::json::ApiJson;
use crate::{ApiError, AppState};

const ANALYZE_BATCH_LIMIT: usize = 200;
const DEFAULT_SEARCH_LIMIT: u32 = 60;
const DEFAULT_DUPLICATE_THRESHOLD: u32 = 6;
const MAX_DUPLICATE_THRESHOLD: u32 = 32;

type IdPath = Result<Path<String>, PathRejection>;
type ItemPersonPath = Result<Path<(String, String)>, PathRejection>;

/// Resolves the caller's accessible photo libraries (`None` means unrestricted).
async fn accessible_photo_libraries(
    state: &AppState,
    user: &CurrentUser,
) -> Result<Option<HashSet<LibraryId>>, ApiError> {
    if user.is_admin() {
        return Ok(None);
    }
    let policy = state.db.accounts().get_policy(user.user.id).await?;
    if policy.all_libraries {
        return Ok(None);
    }
    let libs = state.db.libraries().list().await?;
    let allowed = state
        .db
        .accounts()
        .filter_libraries(user.user.id, user.user.role, libs)
        .await?;
    let ids = allowed
        .into_iter()
        .filter(|lib| lib.kind == LibraryKind::Photo)
        .map(|lib| lib.id)
        .collect();
    Ok(Some(ids))
}

async fn ensure_photo_accessible(
    state: &AppState,
    user: &CurrentUser,
    item_id: ItemId,
) -> Result<(), ApiError> {
    let Some(library_id) = state
        .db
        .plugins_photo_ai()
        .photo_item_library(item_id)
        .await?
    else {
        return Err(ApiError::not_found("Photo not found"));
    };
    let can = state
        .db
        .accounts()
        .can_access_library(user.user.id, user.user.role, library_id)
        .await?;
    if !can {
        return Err(ApiError::not_found("Photo not found"));
    }
    Ok(())
}

/// `POST /api/v1/photos/analyze` (Authenticated).
pub async fn analyze(
    user: CurrentUser,
    State(state): State<AppState>,
) -> Result<Json<PhotoAnalyzeResponse>, ApiError> {
    let allowed = accessible_photo_libraries(&state, &user).await?;
    let unanalyzed = state
        .db
        .plugins_photo_ai()
        .list_unanalyzed_photos(allowed.as_ref(), ANALYZE_BATCH_LIMIT)
        .await?;

    let mut upserts = Vec::with_capacity(unanalyzed.len());
    for row in &unanalyzed {
        upserts.push(analyze_unanalyzed_photo(row).await);
    }
    let analyzed_count = u64::try_from(upserts.len()).unwrap_or(u64::MAX);
    if !upserts.is_empty() {
        state
            .db
            .plugins_photo_ai()
            .upsert_photo_ai_tags(&upserts)
            .await?;
    }

    let (total_tagged, remaining) = state
        .db
        .plugins_photo_ai()
        .count_photo_ai_progress(allowed.as_ref())
        .await?;

    Ok(Json(PhotoAnalyzeResponse {
        analyzed_count,
        total_tagged,
        remaining,
    }))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PhotoSearchParams {
    q: Option<String>,
    person_id: Option<String>,
    limit: Option<String>,
}

/// `GET /api/v1/photos/search?q=<text>&limit=<n>&personId=<uuid>` (Authenticated).
pub async fn search(
    user: CurrentUser,
    State(state): State<AppState>,
    params: Result<Query<PhotoSearchParams>, QueryRejection>,
) -> Result<Json<PhotoSearchResponse>, ApiError> {
    let Query(p) = params?;
    let q = p.q.as_deref().map(str::trim).unwrap_or("");
    if q.chars().count() > 128 || q.chars().any(char::is_control) {
        return Err(ApiError::validation("Invalid search query").field("q"));
    }
    let person_id: Option<PhotoPersonId> = query_id(p.person_id.as_deref(), "personId")?;
    let limit = parse_limit(p.limit.as_deref(), DEFAULT_SEARCH_LIMIT, MAX_PAGE_SIZE)? as usize;

    let allowed = accessible_photo_libraries(&state, &user).await?;
    let parsed_filters = parse_photo_query(q, clock::now().year());
    let candidates = state
        .db
        .plugins_photo_ai()
        .list_accessible_photos_for_ai(user.user.id, allowed.as_ref())
        .await?;

    let items: Vec<PhotoSearchHit> = candidates
        .into_iter()
        .filter(|row| matches_photo_query(row, &parsed_filters, person_id))
        .take(limit)
        .map(|row| PhotoSearchHit {
            id: row.id,
            library_id: row.library_id,
            title: row.title,
            taken_at: row.taken_at,
            day_bucket: row.day_bucket,
            month_bucket: row.month_bucket,
            width: row.width,
            height: row.height,
            orientation: row.orientation,
            camera_make: row.camera_make,
            camera_model: row.camera_model,
            latitude: row.latitude,
            longitude: row.longitude,
            favorite: row.favorite,
            dominant_color: row.dominant_color,
            dominant_hex: row.dominant_hex,
            time_of_day: row.time_of_day,
            season: row.season,
            scene_tags: row.scene_tags,
            people: row.people,
        })
        .collect();

    Ok(Json(PhotoSearchResponse {
        query: q.to_owned(),
        parsed_filters,
        items,
    }))
}

#[derive(Deserialize)]
pub struct DuplicatesParams {
    threshold: Option<String>,
}

/// `GET /api/v1/photos/duplicates?threshold=<u32>` (Authenticated).
pub async fn duplicates(
    user: CurrentUser,
    State(state): State<AppState>,
    params: Result<Query<DuplicatesParams>, QueryRejection>,
) -> Result<Json<PhotoDuplicatesResponse>, ApiError> {
    let Query(p) = params?;
    let threshold = match p
        .threshold
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        None => DEFAULT_DUPLICATE_THRESHOLD,
        Some(raw) => {
            let val: u32 = raw.parse().map_err(|_| {
                ApiError::validation("threshold must be an integer between 0 and 32")
                    .field("threshold")
            })?;
            if val > MAX_DUPLICATE_THRESHOLD {
                return Err(ApiError::validation(format!(
                    "threshold must be between 0 and {MAX_DUPLICATE_THRESHOLD}"
                ))
                .field("threshold"));
            }
            val
        }
    };

    let allowed = accessible_photo_libraries(&state, &user).await?;
    let candidates = state
        .db
        .plugins_photo_ai()
        .list_accessible_photos_for_ai(user.user.id, allowed.as_ref())
        .await?;

    Ok(Json(build_duplicate_clusters(&candidates, threshold)))
}

/// `GET /api/v1/me/photo-people` (Authenticated).
pub async fn list_people(
    user: CurrentUser,
    State(state): State<AppState>,
) -> Result<Json<PhotoPersonList>, ApiError> {
    let allowed = accessible_photo_libraries(&state, &user).await?;
    let people = state
        .db
        .plugins_photo_ai()
        .list_people(user.user.id, allowed.as_ref())
        .await?;
    Ok(Json(PhotoPersonList { people }))
}

/// `POST /api/v1/me/photo-people` (Authenticated).
pub async fn create_person(
    user: CurrentUser,
    State(state): State<AppState>,
    body: Result<ApiJson<CreatePhotoPersonRequest>, ApiError>,
) -> Result<(StatusCode, Json<PhotoPersonSummary>), ApiError> {
    let ApiJson(req) = body?;
    if let Some(cover_id) = req.cover_item_id {
        ensure_photo_accessible(&state, &user, cover_id).await?;
    }
    let created = state
        .db
        .plugins_photo_ai()
        .create_person(user.user.id, &req.name, req.cover_item_id)
        .await
        .map_err(|err| match err {
            DbError::Invalid(msg) => ApiError::validation(msg).field("name"),
            other => other.into(),
        })?;
    Ok((StatusCode::CREATED, Json(created)))
}

/// `PATCH /api/v1/me/photo-people/{id}` (Authenticated).
pub async fn update_person(
    user: CurrentUser,
    State(state): State<AppState>,
    path: IdPath,
    body: Result<ApiJson<UpdatePhotoPersonRequest>, ApiError>,
) -> Result<Json<PhotoPersonSummary>, ApiError> {
    let person_id: PhotoPersonId = path_id(&path?.0, "Photo person")?;
    let ApiJson(req) = body?;
    if req.name.is_none() && req.cover_item_id.is_none() {
        return Err(ApiError::validation(
            "Provide `name` or `coverItemId` to update",
        ));
    }
    if let Some(cover_id) = req.cover_item_id {
        ensure_photo_accessible(&state, &user, cover_id).await?;
    }
    let updated = state
        .db
        .plugins_photo_ai()
        .update_person(
            user.user.id,
            person_id,
            req.name.as_deref(),
            req.cover_item_id,
        )
        .await
        .map_err(|err| match err {
            DbError::Invalid(msg) => ApiError::validation(msg).field("name"),
            other => other.into(),
        })?;
    Ok(Json(updated))
}

/// `DELETE /api/v1/me/photo-people/{id}` (Authenticated).
pub async fn delete_person(
    user: CurrentUser,
    State(state): State<AppState>,
    path: IdPath,
) -> Result<StatusCode, ApiError> {
    let person_id: PhotoPersonId = path_id(&path?.0, "Photo person")?;
    let deleted = state
        .db
        .plugins_photo_ai()
        .delete_person(user.user.id, person_id)
        .await?;
    if !deleted {
        return Err(ApiError::not_found("Photo person not found"));
    }
    Ok(StatusCode::NO_CONTENT)
}

/// `GET /api/v1/items/{id}/photo-people` (Authenticated).
pub async fn list_item_people(
    user: CurrentUser,
    State(state): State<AppState>,
    path: IdPath,
) -> Result<Json<PhotoItemPeopleResponse>, ApiError> {
    let item_id: ItemId = path_id(&path?.0, "Photo")?;
    ensure_photo_accessible(&state, &user, item_id).await?;
    let people = state
        .db
        .plugins_photo_ai()
        .list_people_for_item(user.user.id, item_id)
        .await?;
    Ok(Json(PhotoItemPeopleResponse { people }))
}

#[derive(Deserialize)]
struct OptionalBboxBody {
    bbox: Option<serde_json::Value>,
}

/// `POST /api/v1/items/{id}/photo-people/{personId}` (Authenticated).
pub async fn tag_person(
    user: CurrentUser,
    State(state): State<AppState>,
    path: ItemPersonPath,
    raw_body: Bytes,
) -> Result<Json<PhotoPersonSummary>, ApiError> {
    let Path((raw_item, raw_person)) = path?;
    let item_id: ItemId = path_id(&raw_item, "Photo")?;
    let person_id: PhotoPersonId = path_id(&raw_person, "Photo person")?;
    ensure_photo_accessible(&state, &user, item_id).await?;

    let bbox_json = if raw_body.is_empty() {
        None
    } else {
        let parsed: OptionalBboxBody = serde_json::from_slice(&raw_body)
            .map_err(|_| ApiError::validation("Invalid JSON body").field("bbox"))?;
        parsed
            .bbox
            .map(|v| serde_json::to_string(&v))
            .transpose()
            .map_err(ApiError::internal)?
    };

    let updated = state
        .db
        .plugins_photo_ai()
        .tag_person_on_item(user.user.id, item_id, person_id, bbox_json)
        .await?;
    Ok(Json(updated))
}

/// `DELETE /api/v1/items/{id}/photo-people/{personId}` (Authenticated).
pub async fn untag_person(
    user: CurrentUser,
    State(state): State<AppState>,
    path: ItemPersonPath,
) -> Result<StatusCode, ApiError> {
    let Path((raw_item, raw_person)) = path?;
    let item_id: ItemId = path_id(&raw_item, "Photo")?;
    let person_id: PhotoPersonId = path_id(&raw_person, "Photo person")?;
    ensure_photo_accessible(&state, &user, item_id).await?;

    let removed = state
        .db
        .plugins_photo_ai()
        .untag_person_from_item(user.user.id, item_id, person_id)
        .await?;
    if !removed {
        return Err(ApiError::not_found("Photo person tag not found"));
    }
    Ok(StatusCode::NO_CONTENT)
}

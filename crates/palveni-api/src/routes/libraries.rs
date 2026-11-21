use std::path::Path as FsPath;

use axum::Json;
use axum::extract::rejection::PathRejection;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use palveni_core::{JobId, Library, LibraryId, LibraryKind, ServerEvent};
use palveni_db::DbError;
use palveni_db::audit::{AuditEntry, actions};
use palveni_db::libraries::NewLibrary;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::auth::{AdminUser, CurrentUser};
use crate::error::ApiError;
use crate::json::ApiJson;
use crate::routes::{audit, path_id};
use crate::state::AppState;

const NAME_MAX_CHARS: usize = 100;

#[derive(utoipa::ToSchema, Serialize)]
pub struct LibraryList {
    libraries: Vec<Library>,
}

pub async fn list(
    _user: CurrentUser,
    State(state): State<AppState>,
) -> Result<Json<LibraryList>, ApiError> {
    let libraries = state.db.libraries().list().await?;
    Ok(Json(LibraryList { libraries }))
}

#[derive(utoipa::ToSchema, Deserialize)]
pub struct CreateLibraryRequest {
    name: String,
    kind: LibraryKind,
    paths: Vec<String>,
}

pub async fn create(
    AdminUser(admin): AdminUser,
    State(state): State<AppState>,
    body: Result<ApiJson<CreateLibraryRequest>, ApiError>,
) -> Result<(StatusCode, Json<Library>), ApiError> {
    let ApiJson(request) = body?;
    let name = validate_name(&request.name)?;
    let paths = validate_library_paths(&request.paths).await?;
    let library = state
        .db
        .libraries()
        .create(NewLibrary {
            name: name.to_owned(),
            kind: request.kind,
            paths,
        })
        .await
        .map_err(|err| match err {
            DbError::Conflict(message) => ApiError::conflict(message).field("name"),
            other => other.into(),
        })?;
    state
        .bus
        .publish(ServerEvent::LibraryUpdated(library.clone()));
    audit(
        &state,
        AuditEntry {
            action: actions::LIBRARY_CREATED,
            actor: Some(admin.user.id),
            target: Some(library.id.to_string()),
            detail: Some(json!({ "name": library.name, "paths": library.paths })),
        },
    )
    .await;
    Ok((StatusCode::CREATED, Json(library)))
}

fn validate_name(value: &str) -> Result<&str, ApiError> {
    let name = value.trim();
    let name_chars = name.chars().count();
    if name_chars == 0 || name_chars > NAME_MAX_CHARS {
        return Err(
            ApiError::validation(format!("Name must be 1 to {NAME_MAX_CHARS} characters"))
                .field("name"),
        );
    }
    Ok(name)
}

#[derive(utoipa::ToSchema, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateLibraryRequest {
    name: Option<String>,
    #[serde(default)]
    add_paths: Vec<String>,
}

pub async fn update(
    AdminUser(admin): AdminUser,
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
    body: Result<ApiJson<UpdateLibraryRequest>, ApiError>,
) -> Result<Json<Library>, ApiError> {
    let id = path_id(&path?.0, "Library")?;
    let ApiJson(request) = body?;
    let name = request.name.as_deref().map(validate_name).transpose()?;
    if name.is_none() && request.add_paths.is_empty() {
        return Err(ApiError::validation("Provide a name or folders to add"));
    }
    let paths = if request.add_paths.is_empty() {
        Vec::new()
    } else {
        validate_library_paths(&request.add_paths).await?
    };
    let library = state
        .db
        .libraries()
        .update(id, name, &paths)
        .await
        .map_err(|error| match error {
            DbError::Conflict(message) => ApiError::conflict(message).field("name"),
            DbError::Invalid(message) => ApiError::validation(message).field("paths"),
            other => other.into(),
        })?;
    state
        .bus
        .publish(ServerEvent::LibraryUpdated(library.clone()));
    audit(
        &state,
        AuditEntry {
            action: actions::LIBRARY_UPDATED,
            actor: Some(admin.user.id),
            target: Some(id.to_string()),
            detail: Some(json!({ "name": library.name, "paths": library.paths })),
        },
    )
    .await;
    Ok(Json(library))
}

pub async fn get_one(
    _user: CurrentUser,
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
) -> Result<Json<Library>, ApiError> {
    let Path(raw) = path?;
    let id: LibraryId = path_id(&raw, "Library")?;
    let library = state
        .db
        .libraries()
        .get(id)
        .await?
        .ok_or_else(|| ApiError::not_found("Library not found"))?;
    Ok(Json(library))
}

/// Removes the library from the index; files on disk are never touched.
pub async fn delete(
    AdminUser(admin): AdminUser,
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
) -> Result<StatusCode, ApiError> {
    let Path(raw) = path?;
    let id: LibraryId = path_id(&raw, "Library")?;
    if !state.db.libraries().delete(id).await? {
        return Err(ApiError::not_found("Library not found"));
    }
    state.music_search.forget(id).await;
    state.bus.publish(ServerEvent::LibraryDeleted { id });
    audit(
        &state,
        AuditEntry {
            action: actions::LIBRARY_DELETED,
            actor: Some(admin.user.id),
            target: Some(id.to_string()),
            detail: None,
        },
    )
    .await;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(utoipa::ToSchema, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanAccepted {
    job_id: JobId,
}

/// Queues a scan. While a scan of the same library is queued or running,
/// that job's ID is returned instead of queueing another.
pub async fn scan(
    _admin: AdminUser,
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
) -> Result<(StatusCode, Json<ScanAccepted>), ApiError> {
    let Path(raw) = path?;
    let id: LibraryId = path_id(&raw, "Library")?;
    let library = state
        .db
        .libraries()
        .get(id)
        .await?
        .ok_or_else(|| ApiError::not_found("Library not found"))?;
    let (job, _) = state.jobs.enqueue_library_scan(id, library.name).await?;
    Ok((StatusCode::ACCEPTED, Json(ScanAccepted { job_id: job.id })))
}

/// Paths must be absolute, existing directories. They are stored in
/// canonical form, de-duplicated, and may not contain one another. Errors
/// carry the index of the offending entry in the request's `paths`.
async fn validate_library_paths(raw: &[String]) -> Result<Vec<String>, ApiError> {
    if raw.is_empty() {
        return Err(ApiError::validation("At least one path is required").field("paths"));
    }
    // (index in the request, canonical path)
    let mut canonical_paths: Vec<(usize, String)> = Vec::with_capacity(raw.len());
    for (index, entry) in raw.iter().enumerate() {
        let trimmed = entry.trim();
        if trimmed.is_empty() {
            return Err(ApiError::validation("Paths must not be empty").field_at("paths", index));
        }
        let path = FsPath::new(trimmed);
        if !path.is_absolute() {
            return Err(
                ApiError::validation(format!("Path must be absolute: {trimmed}"))
                    .field_at("paths", index),
            );
        }
        let missing =
            || ApiError::path_not_found(format!("Path does not exist: {trimmed}"), index, trimmed);
        let canonical = tokio::fs::canonicalize(path).await.map_err(|_| missing())?;
        let meta = tokio::fs::metadata(&canonical)
            .await
            .map_err(|_| missing())?;
        if !meta.is_dir() {
            return Err(ApiError::path_not_found(
                format!("Path is not a directory: {trimmed}"),
                index,
                trimmed,
            ));
        }
        let text = canonical
            .to_str()
            .ok_or_else(|| {
                ApiError::validation(format!("Path is not valid UTF-8: {trimmed}"))
                    .field_at("paths", index)
            })?
            .to_owned();
        if !canonical_paths
            .iter()
            .any(|(_, existing)| *existing == text)
        {
            canonical_paths.push((index, text));
        }
    }
    for (index, inner) in &canonical_paths {
        for (_, outer) in &canonical_paths {
            if inner != outer && FsPath::new(inner).starts_with(FsPath::new(outer)) {
                return Err(
                    ApiError::validation(format!("Path {inner} is inside {outer}"))
                        .field_at("paths", *index),
                );
            }
        }
    }
    Ok(canonical_paths.into_iter().map(|(_, path)| path).collect())
}

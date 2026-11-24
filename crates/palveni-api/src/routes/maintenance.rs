//! System maintenance endpoints: security audit log (`/api/v1/system/audit`),
//! portable snapshot backups (`/api/v1/system/backups*`), and external
//! history/playlist migration importer (`/api/v1/system/import`).

use axum::Json;
use axum::body::Body;
use axum::extract::rejection::{PathRejection, QueryRejection};
use axum::extract::{Path, Query, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::Response;
use palveni_core::accounts::{
    AuditLogPage, BackupList, BackupRestoreResult, BackupSummary, CreateBackupRequest,
    ExternalImportRequest, ExternalImportResult, MAX_BACKUP_NAME_CHARS, MAX_IMPORT_BYTES,
};
use palveni_core::settings::RuntimeSettings;
use palveni_core::{BackupId, ServerEvent};
use palveni_db::audit::{AuditEntry, actions};
use palveni_db::settings::KEY_RUNTIME;
use serde::Deserialize;
use serde_json::json;

use crate::auth::{AdminUser, CurrentUser};
use crate::error::ApiError;
use crate::json::ApiJson;
use crate::routes::{DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE, audit, parse_limit, path_id};
use crate::state::AppState;

type IdPath = Result<Path<String>, PathRejection>;

#[derive(Deserialize, Default)]
pub struct AuditQuery {
    pub action: Option<String>,
    pub cursor: Option<String>,
    pub limit: Option<String>,
}

/// `GET /system/audit` (Admin).
pub async fn list_audit(
    _admin: AdminUser,
    State(state): State<AppState>,
    query: Result<Query<AuditQuery>, QueryRejection>,
) -> Result<Json<AuditLogPage>, ApiError> {
    let Query(params) = query?;
    let limit = parse_limit(params.limit.as_deref(), DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE)?;
    if let Some(prefix) = params.action.as_deref()
        && prefix.chars().count() > 64
    {
        return Err(
            ApiError::validation("action filter must be at most 64 characters").field("action"),
        );
    }
    let page = state
        .db
        .accounts()
        .list_audit_logs(params.action.as_deref(), params.cursor.as_deref(), limit)
        .await?;
    Ok(Json(page))
}

/// `GET /system/backups` (Admin).
pub async fn list_backups(
    _admin: AdminUser,
    State(state): State<AppState>,
) -> Result<Json<BackupList>, ApiError> {
    let backups = state.db.accounts().list_backups().await?;
    Ok(Json(BackupList { backups }))
}

/// `POST /system/backups` (Admin).
pub async fn create_backup(
    AdminUser(admin): AdminUser,
    State(state): State<AppState>,
    body: Result<ApiJson<CreateBackupRequest>, ApiError>,
) -> Result<(StatusCode, Json<BackupSummary>), ApiError> {
    let ApiJson(request) = body?;
    if let Some(raw_name) = request.name.as_deref() {
        let trimmed = raw_name.trim();
        if trimmed.chars().count() > MAX_BACKUP_NAME_CHARS || trimmed.chars().any(char::is_control)
        {
            return Err(ApiError::validation(format!(
                "Backup name must be at most {MAX_BACKUP_NAME_CHARS} printable characters"
            ))
            .field("name"));
        }
    }

    let summary = state
        .db
        .accounts()
        .create_backup(Some(admin.user.id), request.name.as_deref())
        .await?;

    audit(
        &state,
        AuditEntry {
            action: actions::BACKUP_CREATED,
            actor: Some(admin.user.id),
            target: Some(summary.id.to_string()),
            detail: Some(json!({
                "name": summary.name,
                "sizeBytes": summary.size_bytes,
                "sha256": summary.sha256,
            })),
        },
    )
    .await;

    Ok((StatusCode::CREATED, Json(summary)))
}

/// `GET /system/backups/{id}/download` (Admin).
pub async fn download_backup(
    _admin: AdminUser,
    State(state): State<AppState>,
    path: IdPath,
) -> Result<Response, ApiError> {
    let id: BackupId = path_id(&path?.0, "Backup")?;
    let (summary, payload_json) = state
        .db
        .accounts()
        .get_backup_payload(id)
        .await?
        .ok_or_else(|| ApiError::not_found("Backup not found"))?;

    let filename = format!("palveni-backup-{}.json", summary.id);
    let disposition = format!("attachment; filename=\"{filename}\"");

    Response::builder()
        .status(StatusCode::OK)
        .header(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/json; charset=utf-8"),
        )
        .header(
            header::CONTENT_DISPOSITION,
            HeaderValue::from_str(&disposition).map_err(ApiError::internal)?,
        )
        .header(
            header::CACHE_CONTROL,
            HeaderValue::from_static("private, no-store"),
        )
        .header(header::CONTENT_LENGTH, payload_json.len())
        .body(Body::from(payload_json))
        .map_err(ApiError::internal)
}

/// `POST /system/backups/{id}/restore` (Admin).
pub async fn restore_backup(
    AdminUser(admin): AdminUser,
    State(state): State<AppState>,
    path: IdPath,
) -> Result<Json<BackupRestoreResult>, ApiError> {
    let id: BackupId = path_id(&path?.0, "Backup")?;
    let result = state.db.accounts().restore_backup(id).await?;

    // Synchronize runtime settings cache if restored.
    if let Ok(Some(raw_runtime)) = state.db.settings().get(KEY_RUNTIME).await
        && let Ok(restored_runtime) = serde_json::from_str::<RuntimeSettings>(&raw_runtime)
    {
        let mut guard = state.settings.write().await;
        if *guard != restored_runtime {
            *guard = restored_runtime.clone();
            state.bus.publish(ServerEvent::SettingsUpdated {
                server_name: restored_runtime.server_name,
                perf_tier: restored_runtime.perf_tier,
            });
        }
    }

    if let Ok(libraries) = state.db.libraries().list().await {
        for library in libraries {
            state.bus.publish(ServerEvent::LibraryUpdated(library));
        }
    }

    audit(
        &state,
        AuditEntry {
            action: actions::BACKUP_RESTORED,
            actor: Some(admin.user.id),
            target: Some(id.to_string()),
            detail: Some(json!({
                "librariesRestored": result.libraries_restored,
                "playlistsRestored": result.playlists_restored,
                "itemStatesRestored": result.item_states_restored,
            })),
        },
    )
    .await;

    Ok(Json(result))
}

/// `DELETE /system/backups/{id}` (Admin).
pub async fn delete_backup(
    AdminUser(admin): AdminUser,
    State(state): State<AppState>,
    path: IdPath,
) -> Result<StatusCode, ApiError> {
    let id: BackupId = path_id(&path?.0, "Backup")?;
    if !state.db.accounts().delete_backup(id).await? {
        return Err(ApiError::not_found("Backup not found"));
    }

    audit(
        &state,
        AuditEntry {
            action: actions::BACKUP_DELETED,
            actor: Some(admin.user.id),
            target: Some(id.to_string()),
            detail: None,
        },
    )
    .await;

    Ok(StatusCode::NO_CONTENT)
}

/// `POST /system/import` (Authenticated).
pub async fn import_external(
    current: CurrentUser,
    State(state): State<AppState>,
    body: Result<ApiJson<ExternalImportRequest>, ApiError>,
) -> Result<Json<ExternalImportResult>, ApiError> {
    let ApiJson(request) = body?;
    if request.content.len() > MAX_IMPORT_BYTES {
        return Err(ApiError::new(
            StatusCode::PAYLOAD_TOO_LARGE,
            "payload_too_large",
            "Import content exceeds 10 MiB",
        ));
    }

    let result = state
        .db
        .accounts()
        .import_external_state(current.user.id, &request)
        .await?;

    if !request.dry_run {
        audit(
            &state,
            AuditEntry {
                action: actions::SYSTEM_IMPORTED,
                actor: Some(current.user.id),
                target: Some(format!("{:?}", result.detected_source).to_lowercase()),
                detail: Some(json!({
                    "matchedItems": result.matched_items,
                    "favoritesImported": result.favorites_imported,
                    "historyImported": result.history_imported,
                    "playlistsImported": result.playlists_imported,
                })),
            },
        )
        .await;
    }

    Ok(Json(result))
}

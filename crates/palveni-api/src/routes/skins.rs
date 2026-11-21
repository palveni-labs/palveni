use std::sync::Arc;

use axum::Json;
use axum::body::{Body, to_bytes};
use axum::extract::{Path, Request, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use palveni_db::audit::AuditEntry;
use palveni_plugin_host::skin_store::{
    InstalledSkin, MAX_PACKAGE_BYTES, RegistrySkin, SkinAsset, SkinError, SkinStore,
};
use serde::Serialize;

use crate::auth::{AdminUser, CurrentUser};
use crate::{ApiError, AppState};

impl From<SkinError> for ApiError {
    fn from(err: SkinError) -> Self {
        match err {
            SkinError::NotFound => Self::not_found("Skin or resource not found"),
            SkinError::Conflict => Self::conflict("A skin with this identifier already exists"),
            SkinError::TooLarge => too_large(),
            SkinError::Invalid(reason) => Self::validation(reason),
            SkinError::Unavailable => Self::new(
                StatusCode::SERVICE_UNAVAILABLE,
                "unavailable",
                "Skin package is unavailable",
            ),
            SkinError::Io(error) => Self::internal(error),
        }
    }
}

fn too_large() -> ApiError {
    ApiError::new(
        StatusCode::PAYLOAD_TOO_LARGE,
        "payload_too_large",
        "Skin package exceeds its size limit",
    )
}

pub(super) async fn on_disk<T, F>(store: Arc<SkinStore>, work: F) -> Result<T, ApiError>
where
    T: Send + 'static,
    F: FnOnce(&SkinStore) -> Result<T, SkinError> + Send + 'static,
{
    tokio::task::spawn_blocking(move || work(&store))
        .await
        .map_err(ApiError::internal)?
        .map_err(Into::into)
}

#[derive(Serialize)]
pub struct SkinList<T> {
    skins: Vec<T>,
}

pub async fn installed(
    _: CurrentUser,
    State(state): State<AppState>,
) -> Result<Json<SkinList<InstalledSkin>>, ApiError> {
    let skins = on_disk(state.skins, SkinStore::installed).await?;
    Ok(Json(SkinList { skins }))
}

pub async fn registry(
    _: AdminUser,
    State(state): State<AppState>,
) -> Result<Json<SkinList<RegistrySkin>>, ApiError> {
    let skins = on_disk(state.skins, SkinStore::registry).await?;
    Ok(Json(SkinList { skins }))
}

async fn package_body(request: Request, optional: bool) -> Result<axum::body::Bytes, ApiError> {
    let archive = request
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .is_some_and(|mime| mime.trim().eq_ignore_ascii_case("application/x-tar"));
    if !archive && !optional {
        return Err(ApiError::validation("Expected application/x-tar"));
    }
    let limit = if archive { MAX_PACKAGE_BYTES } else { 0 };
    let bytes = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        to_bytes(request.into_body(), limit),
    )
    .await
    .map_err(|_| {
        ApiError::new(
            StatusCode::REQUEST_TIMEOUT,
            "timeout",
            "Skin upload timed out",
        )
    })?
    .map_err(|_| {
        if archive {
            too_large()
        } else {
            ApiError::validation("Expected application/x-tar")
        }
    })?;
    if archive && bytes.is_empty() {
        return Err(ApiError::validation("Empty skin archive"));
    }
    Ok(bytes)
}

async fn audit_skin(state: &AppState, current: CurrentUser, action: &'static str, id: String) {
    super::audit(
        state,
        AuditEntry {
            actor: Some(current.user.id),
            action,
            target: Some(id),
            detail: None,
        },
    )
    .await;
}

fn write_slot(state: &AppState) -> Result<tokio::sync::OwnedSemaphorePermit, ApiError> {
    Arc::clone(&state.skin_writes)
        .try_acquire_owned()
        .map_err(|_| {
            ApiError::new(
                StatusCode::SERVICE_UNAVAILABLE,
                "busy",
                "Another skin installation is in progress",
            )
        })
}

pub async fn install(
    AdminUser(current): AdminUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
    request: Request,
) -> Result<Json<InstalledSkin>, ApiError> {
    let permit = write_slot(&state)?;
    let bytes = package_body(request, true).await?;
    let skin = on_disk(Arc::clone(&state.skins), move |store| {
        let _permit = permit;
        if bytes.is_empty() {
            store.install_official(&id)
        } else {
            store.install_official_bytes(&id, &bytes)
        }
    })
    .await?;
    audit_skin(&state, current, "skin.installed", skin.skin.id.clone()).await;
    Ok(Json(skin))
}

pub async fn upload(
    AdminUser(current): AdminUser,
    State(state): State<AppState>,
    request: Request,
) -> Result<(StatusCode, Json<InstalledSkin>), ApiError> {
    let permit = write_slot(&state)?;
    let bytes = package_body(request, false).await?;
    let skin = on_disk(Arc::clone(&state.skins), move |store| {
        let _permit = permit;
        store.upload(&bytes)
    })
    .await?;
    audit_skin(&state, current, "skin.uploaded", skin.skin.id.clone()).await;
    Ok((StatusCode::CREATED, Json(skin)))
}

pub async fn remove(
    AdminUser(current): AdminUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let target = id.clone();
    on_disk(Arc::clone(&state.skins), move |store| store.remove(&id)).await?;
    audit_skin(&state, current, "skin.uninstalled", target).await;
    Ok(StatusCode::NO_CONTENT)
}

fn resource(asset: SkinAsset) -> Response {
    (
        [
            (header::CONTENT_TYPE, asset.mime),
            (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
            (header::CACHE_CONTROL, "no-cache"),
            (
                header::CONTENT_SECURITY_POLICY,
                "default-src 'none'; sandbox",
            ),
        ],
        Body::from(asset.bytes),
    )
        .into_response()
}

pub async fn manifest(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Response, ApiError> {
    let (manifest, source) = on_disk(state.skins, move |store| store.manifest(&id)).await?;
    Ok((
        [("x-palveni-skin-source", source.as_str())],
        resource(manifest),
    )
        .into_response())
}

pub async fn asset(
    State(state): State<AppState>,
    Path((id, path)): Path<(String, String)>,
) -> Result<Response, ApiError> {
    on_disk(state.skins, move |store| store.asset(&id, &path))
        .await
        .map(resource)
}

pub async fn preview(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Response, ApiError> {
    on_disk(state.skins, move |store| store.preview(&id))
        .await
        .map(resource)
}

pub async fn package(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Response, ApiError> {
    on_disk(state.skins, move |store| store.package(&id))
        .await
        .map(resource)
}

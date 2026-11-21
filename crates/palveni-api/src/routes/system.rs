use axum::Json;
use axum::extract::rejection::QueryRejection;
use axum::extract::{Query, State};
use palveni_core::ServerId;
use palveni_media::HardwareReport;
use serde::{Deserialize, Serialize};

use crate::auth::AdminUser;
use crate::error::ApiError;
use crate::state::AppState;

#[derive(utoipa::ToSchema, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemInfo {
    name: String,
    version: String,
    server_id: ServerId,
    setup_completed: bool,
    /// Capabilities registered at startup; clients hide missing modules.
    features: Vec<String>,
}

pub async fn info(State(state): State<AppState>) -> Result<Json<SystemInfo>, ApiError> {
    let setup_completed = state.db.users().count().await? > 0;
    Ok(Json(SystemInfo {
        name: state
            .settings
            .read()
            .await
            .server_name
            .clone()
            .unwrap_or_else(|| state.server.name.clone()),
        version: state.server.version.clone(),
        server_id: state.server.id,
        setup_completed,
        features: state.features.list(),
    }))
}

#[derive(utoipa::ToSchema, Serialize)]
pub struct Health {
    status: &'static str,
}

/// Liveness only; it does not touch the database.
pub async fn health() -> Json<Health> {
    Json(Health { status: "ok" })
}

#[derive(Deserialize)]
pub struct HardwareQuery {
    refresh: Option<String>,
}

pub async fn hardware(
    _admin: AdminUser,
    State(state): State<AppState>,
    query: Result<Query<HardwareQuery>, QueryRejection>,
) -> Result<Json<HardwareReport>, ApiError> {
    let Query(query) = query?;
    let refresh = matches!(query.refresh.as_deref(), Some("true" | "1"));
    let mut report = state.hardware.get(refresh).await;
    let settings = state.settings.read().await;
    if let Some(tier) = settings.perf_tier {
        report.perf_tier = tier;
    } else if refresh {
        state
            .jobs
            .set_concurrency(report.perf_tier.job_concurrency());
    }
    Ok(Json(report))
}

use axum::Json;
use axum::extract::State;
use palveni_core::{ServerEvent, settings::RuntimeSettings};
use palveni_db::{audit::AuditEntry, settings::KEY_RUNTIME};

use crate::auth::AdminUser;
use crate::json::ApiJson;
use crate::{ApiError, AppState};

pub async fn get(_: AdminUser, State(state): State<AppState>) -> Json<RuntimeSettings> {
    Json(state.settings.read().await.clone())
}

pub async fn put(
    AdminUser(admin): AdminUser,
    State(state): State<AppState>,
    ApiJson(mut next): ApiJson<RuntimeSettings>,
) -> Result<Json<RuntimeSettings>, ApiError> {
    if let Some(name) = &mut next.server_name {
        *name = name.trim().to_owned();
        if name.is_empty() || name.chars().count() > 64 || name.chars().any(char::is_control) {
            return Err(
                ApiError::validation("Server name must contain 1–64 printable characters")
                    .field("serverName"),
            );
        }
    }
    // Once persistence starts, finish applying the update even if the client disconnects.
    tokio::spawn(async move {
        let mut current = state.settings.write().await;
        let fallback = match next.perf_tier {
            Some(tier) => tier,
            None => state
                .hardware
                .cached()
                .await
                .map_or(state.boot_tier, |report| report.perf_tier),
        };
        if *current != next {
            let json = serde_json::to_string(&next).map_err(ApiError::internal)?;
            state.db.settings().set(KEY_RUNTIME, &json).await?;
            state.jobs.set_concurrency(fallback.job_concurrency());
            *current = next.clone();
            state.bus.publish(ServerEvent::SettingsUpdated {
                server_name: next.server_name.clone(),
                perf_tier: next.perf_tier,
            });
            super::audit(
                &state,
                AuditEntry {
                    action: "settings.updated",
                    actor: Some(admin.user.id),
                    target: None,
                    detail: Some(serde_json::to_value(&next).map_err(ApiError::internal)?),
                },
            )
            .await;
        }
        Ok(Json(next))
    })
    .await
    .map_err(ApiError::internal)?
}

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use chrono::{DateTime, Utc};
use palveni_core::User;
use palveni_db::audit::{AuditEntry, actions};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::auth::{
    CurrentUser, PASSWORD_MAX_CHARS, dummy_hash, hash_password, validate_password,
    validate_username, verify_password,
};
use crate::error::ApiError;
use crate::json::ApiJson;
use crate::routes::audit;
use crate::state::AppState;

const DEFAULT_DEVICE_NAME: &str = "Unknown device";

#[derive(utoipa::ToSchema, Deserialize)]
pub struct SetupRequest {
    username: String,
    password: String,
}

#[derive(utoipa::ToSchema, Serialize)]
pub struct SetupResponse {
    user: User,
}

/// `POST /setup`: creates the first administrator.
pub async fn setup(
    State(state): State<AppState>,
    body: Result<ApiJson<SetupRequest>, ApiError>,
) -> Result<(StatusCode, Json<SetupResponse>), ApiError> {
    // Checked before the body so a finished setup always answers 409.
    if state.db.users().count().await? > 0 {
        return Err(ApiError::setup_completed());
    }
    let ApiJson(request) = body?;
    validate_username(&request.username)?;
    validate_password(&request.password)?;
    let hash = hash_password(request.password).await?;
    let user = state
        .db
        .users()
        .create_first_admin(&request.username, &hash)
        .await?
        .ok_or_else(ApiError::setup_completed)?;
    audit(
        &state,
        AuditEntry {
            action: actions::SETUP_COMPLETED,
            actor: Some(user.id),
            target: Some(user.username.clone()),
            detail: None,
        },
    )
    .await;
    Ok((StatusCode::CREATED, Json(SetupResponse { user })))
}

#[derive(utoipa::ToSchema, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginRequest {
    username: String,
    password: String,
    #[serde(default)]
    device_name: Option<String>,
}

#[derive(utoipa::ToSchema, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginResponse {
    access_token: String,
    token_type: &'static str,
    #[schema(required = true)]
    expires_at: Option<DateTime<Utc>>,
    user: User,
}

/// `POST /auth/login`.
pub async fn login(
    axum::extract::ConnectInfo(peer): axum::extract::ConnectInfo<std::net::SocketAddr>,
    State(state): State<AppState>,
    body: Result<ApiJson<LoginRequest>, ApiError>,
) -> Result<Json<LoginResponse>, ApiError> {
    let ApiJson(request) = body?;
    let LoginRequest {
        username,
        password,
        device_name,
    } = request;
    let attempt = state.login_limit.begin(peer.ip(), &username).await?;
    let db = state.db.clone();
    let login_name = username.clone();
    let user = tokio::spawn(async move {
        let user = if password.chars().count() > PASSWORD_MAX_CHARS {
            None
        } else {
            match db.users().find_credentials(&login_name).await? {
                Some(found) if !found.disabled => verify_password(password, found.password_hash)
                    .await?
                    .then_some(found.user),
                _ => {
                    if let Some(dummy) = dummy_hash().await {
                        let _ignored = verify_password(password, dummy).await;
                    }
                    None
                }
            }
        };
        attempt.finish(user.is_some());
        Ok::<_, ApiError>(user)
    })
    .await
    .map_err(ApiError::internal)??;
    let Some(user) = user else {
        audit(
            &state,
            AuditEntry {
                action: actions::LOGIN_FAILED,
                actor: None,
                target: Some(username),
                detail: None,
            },
        )
        .await;
        return Err(ApiError::invalid_credentials());
    };

    let device_name = device_name
        .map(|name| name.trim().to_owned())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| DEFAULT_DEVICE_NAME.to_owned());
    // Session tokens are revoked by logout or device removal.
    let issued = state
        .db
        .sessions()
        .create(user.id, &device_name, None)
        .await?;
    audit(
        &state,
        AuditEntry {
            action: actions::LOGIN_SUCCEEDED,
            actor: Some(user.id),
            target: Some(issued.device_id.to_string()),
            detail: Some(json!({ "deviceName": device_name })),
        },
    )
    .await;
    Ok(Json(LoginResponse {
        access_token: issued.token,
        token_type: "Bearer",
        expires_at: issued.expires_at,
        user,
    }))
}

/// `POST /auth/logout`: revokes the token used for this request.
pub async fn logout(
    current: CurrentUser,
    State(state): State<AppState>,
) -> Result<StatusCode, ApiError> {
    state.db.sessions().revoke(current.session_id).await?;
    audit(
        &state,
        AuditEntry {
            action: actions::LOGOUT,
            actor: Some(current.user.id),
            target: current.device_id.map(|d| d.to_string()),
            detail: None,
        },
    )
    .await;
    Ok(StatusCode::NO_CONTENT)
}

/// `GET /auth/me`.
pub async fn me(current: CurrentUser) -> Json<User> {
    Json(current.user)
}

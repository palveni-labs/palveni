use crate::{ApiError, AppState, auth::CurrentUser, routes::audit};
use axum::{
    Json,
    extract::State,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use md5::{Digest, Md5};
use palveni_db::audit::AuditEntry;
use palveni_link::NodeIdentity;
use ring::{
    digest, hmac,
    rand::{SecureRandom, SystemRandom},
};
use serde::Serialize;

#[derive(Serialize, utoipa::ToSchema)]
pub struct SubsonicPassword {
    username: String,
    password: String,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct SubsonicPasswordStatus {
    enabled: bool,
}

pub async fn status(
    current: CurrentUser,
    State(state): State<AppState>,
) -> Result<Response, ApiError> {
    let enabled = state
        .db
        .users()
        .subsonic_identity(&current.user.username)
        .await?
        .is_some();
    Ok((
        [(header::CACHE_CONTROL, "private, no-store")],
        Json(SubsonicPasswordStatus { enabled }),
    )
        .into_response())
}

fn password(node: &NodeIdentity, user: &str, generation: &str) -> String {
    let signature =
        node.sign(format!("palveni:subsonic-password:v1\n{user}\n{generation}").as_bytes());
    hex::encode(digest::digest(&digest::SHA256, &signature))
}

pub async fn issue(
    current: CurrentUser,
    State(state): State<AppState>,
) -> Result<Response, ApiError> {
    let mut random = [0; 32];
    SystemRandom::new()
        .fill(&mut random)
        .map_err(|_| ApiError::internal("Cannot generate music client password"))?;
    let generation = hex::encode(random);
    let password = password(&state.identity, &current.user.id.to_string(), &generation);
    state
        .db
        .users()
        .set_subsonic_generation(current.user.id, Some(generation))
        .await?;
    audit(
        &state,
        AuditEntry {
            action: "music.client_password_issued",
            actor: Some(current.user.id),
            target: None,
            detail: None,
        },
    )
    .await;
    Ok((
        StatusCode::CREATED,
        [(header::CACHE_CONTROL, "private, no-store")],
        Json(SubsonicPassword {
            username: current.user.username,
            password,
        }),
    )
        .into_response())
}

pub async fn revoke(
    current: CurrentUser,
    State(state): State<AppState>,
) -> Result<StatusCode, ApiError> {
    state
        .db
        .users()
        .set_subsonic_generation(current.user.id, None)
        .await?;
    audit(
        &state,
        AuditEntry {
            action: "music.client_password_revoked",
            actor: Some(current.user.id),
            target: None,
            detail: None,
        },
    )
    .await;
    Ok(StatusCode::NO_CONTENT)
}

pub(super) async fn verify(
    state: &AppState,
    username: &str,
    salt: &str,
    token: &str,
) -> Result<bool, ApiError> {
    let mut supplied = [0; 16];
    if !(6..=256).contains(&salt.chars().count())
        || username.len() > 256
        || hex::decode_to_slice(token, &mut supplied).is_err()
    {
        return Ok(false);
    }
    let identity = state.db.users().subsonic_identity(username).await?;
    let (user, generation) = identity
        .as_ref()
        .map(|(u, g)| (u.as_str(), g.as_str()))
        .unwrap_or(("", ""));
    let secret = password(&state.identity, user, generation);
    let expected = Md5::new()
        .chain_update(secret.as_bytes())
        .chain_update(salt.as_bytes())
        .finalize();
    // Verify the two fixed-size challenges with ring's constant-time HMAC verifier.
    let key = hmac::Key::new(hmac::HMAC_SHA256, secret.as_bytes());
    let tag = hmac::sign(&key, &expected);
    let matches = hmac::verify(&key, &supplied, tag.as_ref()).is_ok();
    Ok(matches && identity.is_some())
}

use std::net::SocketAddr;

use axum::{
    Json,
    extract::{ConnectInfo, FromRequestParts, State},
    http::{HeaderMap, StatusCode, header, request::Parts},
    response::{IntoResponse, Response},
};
use palveni_core::SessionId;
use serde::Serialize;

use crate::{ApiError, AppState, auth::CurrentUser};

#[derive(Serialize, utoipa::ToSchema)]
pub struct OpdsAccess {
    username: String,
    password: String,
}

fn scope(session: SessionId) -> String {
    format!("palveni:opds-reader:v1\n{session}")
}

pub async fn access(current: CurrentUser, State(state): State<AppState>) -> Response {
    let session = current.session_id;
    let password = hex::encode(state.identity.sign(scope(session).as_bytes()));
    (
        [(header::CACHE_CONTROL, "private, no-store")],
        Json(OpdsAccess {
            username: session.to_string(),
            password,
        }),
    )
        .into_response()
}

// A UUID, colon and hex Ed25519 signature occupy exactly 165 bytes in Basic authentication.
// Each four Base64 digits encode three bytes, with no trailing padding for this wire format.
fn credentials(headers: &HeaderMap) -> Option<(SessionId, [u8; 64])> {
    let value = headers.get(header::AUTHORIZATION)?.to_str().ok()?;
    let (scheme, encoded) = value.trim().split_once(' ')?;
    let encoded = encoded.trim_start();
    if !scheme.eq_ignore_ascii_case("basic") || encoded.len() != 220 {
        return None;
    }
    fn digit(b: u8) -> Option<u8> {
        match b {
            b'A'..=b'Z' => Some(b - b'A'),
            b'a'..=b'z' => Some(b - b'a' + 26),
            b'0'..=b'9' => Some(b - b'0' + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    }
    let mut decoded = [0_u8; 165];
    for (src, dst) in encoded
        .as_bytes()
        .chunks_exact(4)
        .zip(decoded.chunks_exact_mut(3))
    {
        let (a, b, c, d) = (
            digit(src[0])?,
            digit(src[1])?,
            digit(src[2])?,
            digit(src[3])?,
        );
        dst.copy_from_slice(&[(a << 2) | (b >> 4), (b << 4) | (c >> 2), (c << 6) | d]);
    }
    let (session, secret) = std::str::from_utf8(&decoded).ok()?.split_once(':')?;
    let mut signature = [0; 64];
    hex::decode_to_slice(secret, &mut signature).ok()?;
    Some((session.parse().ok()?, signature))
}

pub(super) struct Reader;

fn challenge() -> Response {
    (
        StatusCode::UNAUTHORIZED,
        [
            (
                header::WWW_AUTHENTICATE,
                "Basic realm=\"Palveni OPDS\", charset=\"UTF-8\"",
            ),
            (header::CACHE_CONTROL, "private, no-store"),
        ],
    )
        .into_response()
}

impl FromRequestParts<AppState> for Reader {
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Response> {
        let (session, signature) = credentials(&parts.headers).ok_or_else(challenge)?;
        let ConnectInfo(peer) = ConnectInfo::<SocketAddr>::from_request_parts(parts, state)
            .await
            .map_err(IntoResponse::into_response)?;
        let attempt = state
            .login_limit
            .begin(peer.ip(), &format!("opds:{session}"))
            .await
            .map_err(IntoResponse::into_response)?;
        if !state
            .identity
            .node_id()
            .verify(scope(session).as_bytes(), &signature)
        {
            attempt.finish(false);
            return Err(challenge());
        }
        let active = state
            .db
            .sessions()
            .active_by_id(session)
            .await
            .map_err(|err| ApiError::from(err).into_response())?
            .is_some();
        attempt.finish(active);
        if active { Ok(Reader) } else { Err(challenge()) }
    }
}

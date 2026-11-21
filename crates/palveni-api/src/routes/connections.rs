use axum::extract::rejection::QueryRejection;
use axum::{
    Json,
    body::Body,
    extract::{Path, Query, State},
    http::StatusCode,
    http::{HeaderMap, header},
    response::{IntoResponse, Response},
};
use palveni_core::{
    connections::{
        ConnectionKind, ConnectionPage, ConnectionSummary, CreateConnection, Credentials,
    },
    ids::ConnectionId,
};
use palveni_db::connections::RemoteConnection;
use palveni_library::remote;
use palveni_link::NodeIdentity;
use ring::{
    aead,
    rand::{SecureRandom, SystemRandom},
};
use serde::{Deserialize, Serialize};

use crate::{
    ApiError, AppState,
    auth::{AdminUser, CurrentUser},
    json::ApiJson,
};

#[derive(Serialize, utoipa::ToSchema)]
pub struct ConnectionList {
    connections: Vec<ConnectionSummary>,
}

fn credential_key(identity: &NodeIdentity) -> Result<aead::LessSafeKey, ApiError> {
    let seed = identity.sign(b"palveni:media-source-credentials:v1");
    let key = aead::UnboundKey::new(&aead::AES_256_GCM, &seed[..32])
        .map_err(|_| ApiError::internal("Invalid media credential key"))?;
    Ok(aead::LessSafeKey::new(key))
}

fn encrypt(
    identity: &NodeIdentity,
    id: &str,
    credentials: &Credentials,
) -> Result<String, ApiError> {
    let mut nonce = [0u8; aead::NONCE_LEN];
    SystemRandom::new()
        .fill(&mut nonce)
        .map_err(|_| ApiError::internal("Secure randomness unavailable"))?;
    let mut data = serde_json::to_vec(credentials).map_err(ApiError::internal)?;
    credential_key(identity)?
        .seal_in_place_append_tag(
            aead::Nonce::assume_unique_for_key(nonce),
            aead::Aad::from(id),
            &mut data,
        )
        .map_err(|_| ApiError::internal("Cannot protect media credentials"))?;
    Ok(format!("{}.{}", hex::encode(nonce), hex::encode(data)))
}

fn decrypt(identity: &NodeIdentity, id: &str, encrypted: &str) -> Result<Credentials, ApiError> {
    let corrupt = || ApiError::internal("Unreadable media credentials");
    let (nonce, text) = encrypted.split_once('.').ok_or_else(corrupt)?;
    let mut data = hex::decode(text).map_err(|_| corrupt())?;
    let mut iv = [0u8; aead::NONCE_LEN];
    hex::decode_to_slice(nonce, &mut iv).map_err(|_| corrupt())?;
    let plain = credential_key(identity)?
        .open_in_place(
            aead::Nonce::assume_unique_for_key(iv),
            aead::Aad::from(id),
            &mut data,
        )
        .map_err(|_| corrupt())?;
    serde_json::from_slice(plain).map_err(|_| corrupt())
}

fn upstream_error(err: remote::RemoteError) -> ApiError {
    match err {
        remote::RemoteError::Range { .. } => ApiError::validation("Invalid media range"),
        remote::RemoteError::Invalid => {
            ApiError::validation("Invalid upstream address, item or credentials")
        }
        remote::RemoteError::Unavailable => ApiError::new(
            StatusCode::BAD_GATEWAY,
            "unavailable",
            "The upstream source could not be read",
        ),
    }
}

fn summary(conn: &RemoteConnection) -> Result<ConnectionSummary, ApiError> {
    Ok(ConnectionSummary {
        id: conn.id.clone(),
        name: conn.name.clone(),
        kind: ConnectionKind::parse(&conn.kind)
            .ok_or_else(|| ApiError::internal("Unknown media connection kind"))?,
    })
}

pub async fn list(
    _: CurrentUser,
    State(state): State<AppState>,
) -> Result<Json<ConnectionList>, ApiError> {
    let rows = state.db.media_connections().await?;
    Ok(Json(ConnectionList {
        connections: rows.iter().map(summary).collect::<Result<_, _>>()?,
    }))
}

pub async fn create(
    _: AdminUser,
    State(state): State<AppState>,
    body: Result<ApiJson<CreateConnection>, ApiError>,
) -> Result<(StatusCode, Json<ConnectionSummary>), ApiError> {
    let ApiJson(input) = body?;
    let name = input.name.trim();
    if name.is_empty() || name.chars().count() > 100 || name.chars().any(char::is_control) {
        return Err(ApiError::validation("Name must contain 1 to 100 characters").field("name"));
    }
    let address = remote::parse_address(input.kind, &input.address).map_err(upstream_error)?;
    let cred = Credentials {
        username: input.username.unwrap_or_default(),
        secret: input.secret.unwrap_or_default(),
    };
    if cred.username.len() > 256
        || cred.secret.len() > 4096
        || cred.username.chars().any(char::is_control)
        || cred.secret.chars().any(char::is_control)
    {
        return Err(ApiError::validation("Invalid source credentials"));
    }
    remote::entries(input.kind, &address, &cred, "", 0, 1)
        .await
        .map_err(upstream_error)?;
    let id = ConnectionId::generate().to_string();
    let conn = RemoteConnection {
        id: id.clone(),
        name: name.to_owned(),
        kind: input.kind.as_str().into(),
        address: address.to_string(),
        credentials: encrypt(&state.identity, &id, &cred)?,
    };
    let public = summary(&conn)?;
    state.db.add_media_connection(conn).await?;
    Ok((StatusCode::CREATED, Json(public)))
}

#[derive(Deserialize)]
pub struct Browse {
    parent: Option<String>,
    offset: Option<u32>,
    limit: Option<u32>,
}

pub async fn entries(
    _: CurrentUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
    query: Result<Query<Browse>, QueryRejection>,
) -> Result<Json<ConnectionPage>, ApiError> {
    let Query(query) = query?;
    let conn = state
        .db
        .media_connection(&id)
        .await?
        .ok_or_else(|| ApiError::not_found("Media source not found"))?;
    let kind = summary(&conn)?.kind;
    let url = remote::parse_address(kind, &conn.address).map_err(upstream_error)?;
    let credentials = decrypt(&state.identity, &id, &conn.credentials)?;
    let limit = query.limit.unwrap_or(50).min(200);
    if limit == 0 {
        return Err(ApiError::validation("Expected a positive page size").field("limit"));
    }
    Ok(Json(
        remote::entries(
            kind,
            &url,
            &credentials,
            query.parent.as_deref().unwrap_or_default(),
            query.offset.unwrap_or_default(),
            limit,
        )
        .await
        .map_err(upstream_error)?,
    ))
}

pub async fn remove(
    _: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    if state.db.remove_media_connection(&id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::not_found("Media source not found"))
    }
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ConnectionPlayback {
    entry: String,
}

#[derive(Deserialize)]
pub struct StreamQuery {
    entry: String,
    ticket: Option<String>,
}

fn scope(id: &str, entry: &str) -> String {
    format!("palveni:connection-stream:v1\n{id}\n{entry}")
}

pub async fn prepare(
    current: CurrentUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
    body: Result<ApiJson<ConnectionPlayback>, ApiError>,
) -> Result<Response, ApiError> {
    let ApiJson(input) = body?;
    remote::validate_entry(&input.entry).map_err(upstream_error)?;
    let id: ConnectionId = super::path_id(&id, "Media source")?;
    let id = id.to_string();
    state
        .db
        .media_connection(&id)
        .await?
        .ok_or_else(|| ApiError::not_found("Media source not found"))?;
    let (expiry, ticket) = super::playback::issue_ticket(
        &state.identity,
        &scope(&id, &input.entry),
        current.session_id,
        palveni_core::clock::now(),
    );
    let query = reqwest::Url::parse_with_params(
        "https://localhost/",
        [("entry", input.entry.as_str()), ("ticket", ticket.as_str())],
    )
    .map_err(ApiError::internal)?;
    let url = format!(
        "/api/v1/connections/{id}/stream?{}",
        query.query().unwrap_or_default()
    );
    Ok((
        [(header::CACHE_CONTROL, "private, no-store")],
        Json(super::playback::direct_playback(url, expiry)),
    )
        .into_response())
}

pub async fn stream(
    State(state): State<AppState>,
    Path(id): Path<String>,
    query: Result<Query<StreamQuery>, QueryRejection>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let Query(query) = query?;
    let id: ConnectionId = super::path_id(&id, "Media source")?;
    let id = id.to_string();
    remote::validate_entry(&query.entry).map_err(upstream_error)?;
    if headers.contains_key(header::AUTHORIZATION) {
        CurrentUser::from_headers(&headers, &state).await?;
    } else {
        let session = query
            .ticket
            .as_deref()
            .and_then(|s| {
                super::playback::verify_ticket(
                    &state.identity,
                    &scope(&id, &query.entry),
                    s,
                    palveni_core::clock::now(),
                )
            })
            .ok_or_else(ApiError::unauthorized)?;
        state
            .db
            .sessions()
            .active_by_id(session)
            .await?
            .ok_or_else(ApiError::unauthorized)?;
    }
    let conn = state
        .db
        .media_connection(&id)
        .await?
        .ok_or_else(|| ApiError::not_found("Media source not found"))?;
    let kind = summary(&conn)?.kind;
    let address = remote::parse_address(kind, &conn.address).map_err(upstream_error)?;
    let credentials = decrypt(&state.identity, &id, &conn.credentials)?;
    let range = headers
        .get(header::RANGE)
        .map(|s| s.to_str())
        .transpose()
        .map_err(|_| ApiError::validation("Invalid range header"))?;
    let (file, permit) = match remote::open(kind, &address, &credentials, &query.entry, range).await
    {
        Err(remote::RemoteError::Range { total }) => {
            return Response::builder()
                .status(StatusCode::RANGE_NOT_SATISFIABLE)
                .header(header::CONTENT_RANGE, format!("bytes */{total}"))
                .body(Body::empty())
                .map_err(ApiError::internal);
        }
        result => result.map_err(upstream_error)?,
    };
    let mime = file.mime.split(';').next().unwrap_or_default();
    let mime = if mime.starts_with("video/") || mime.starts_with("audio/") {
        mime
    } else {
        "application/octet-stream"
    };
    let mut response = Response::builder()
        .header(header::CONTENT_TYPE, mime)
        .header(header::X_CONTENT_TYPE_OPTIONS, "nosniff")
        .header(header::ACCEPT_RANGES, "bytes")
        .header(header::CACHE_CONTROL, "private, no-store")
        .header(header::CONTENT_LENGTH, file.len);
    if file.partial {
        let end = file
            .start
            .checked_add(file.len)
            .and_then(|n| n.checked_sub(1))
            .filter(|end| *end < file.total)
            .ok_or_else(|| upstream_error(remote::RemoteError::Unavailable))?;
        response = response.status(StatusCode::PARTIAL_CONTENT).header(
            header::CONTENT_RANGE,
            format!("bytes {}-{end}/{}", file.start, file.total),
        );
    }
    let reader = tokio::io::AsyncReadExt::take(file.reader, file.len);
    let stream = tokio_util::io::ReaderStream::with_capacity(reader, 64 * 1024);
    let body = futures_util::StreamExt::map(stream, move |chunk| {
        let _ = &permit;
        chunk
    });
    response
        .body(Body::from_stream(body))
        .map_err(ApiError::internal)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn credentials_are_bound_to_one_source_and_installation() {
        let node = NodeIdentity::generate().unwrap();
        let other = NodeIdentity::generate().unwrap();
        let data = Credentials {
            username: "listener".into(),
            secret: "example-only".into(),
        };
        let first = encrypt(&node, "a", &data).unwrap();
        assert_ne!(first, encrypt(&node, "a", &data).unwrap());
        assert!(!first.contains("example-only"));
        assert_eq!(decrypt(&node, "a", &first).unwrap().secret, data.secret);
        assert!(decrypt(&node, "b", &first).is_err());
        assert!(decrypt(&other, "a", &first).is_err());
    }
}

mod byte_range;
mod cue;
mod transcode;
pub use transcode::audio;

use axum::{
    Json,
    body::Body,
    extract::{
        Path, Query, State,
        rejection::{PathRejection, QueryRejection},
    },
    http::{HeaderMap, Method, StatusCode, header},
    response::{IntoResponse, Response},
};
use chrono::{DateTime, TimeDelta, Utc};
use palveni_core::{ItemId, SessionId, clock, storage::ByteRange};
use palveni_library::local::TrackFile;
use palveni_link::NodeIdentity;
use serde::{Deserialize, Serialize};
use tokio_util::io::ReaderStream;

use super::path_id;
use crate::{AppState, auth::CurrentUser, error::ApiError};
use byte_range::Selection;

const TICKET_LIFETIME: TimeDelta = TimeDelta::minutes(15);
const PRIVATE_CACHE: &str = "private, no-store";

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum PlaybackMode {
    Direct,
    Transcode,
}

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackInfo {
    mode: PlaybackMode,
    url: String,
    expires_at: DateTime<Utc>,
}

#[derive(Deserialize)]
pub struct Access {
    ticket: Option<String>,
}

fn ticket_scope(item: ItemId) -> String {
    format!("palveni:music-stream:v1\n{item}")
}

pub(super) fn issue_ticket(
    identity: &NodeIdentity,
    scope: &str,
    session: SessionId,
    now: DateTime<Utc>,
) -> (DateTime<Utc>, String) {
    let expires_at = now + TICKET_LIFETIME;
    let expiry = expires_at.timestamp_millis();
    let signature = identity.sign(format!("{scope}\n{session}\n{expiry}").as_bytes());
    (
        expires_at,
        format!("{session}.{expiry}.{}", hex::encode(signature)),
    )
}

pub(super) fn direct_playback(url: String, expires_at: DateTime<Utc>) -> PlaybackInfo {
    PlaybackInfo {
        mode: PlaybackMode::Direct,
        url,
        expires_at,
    }
}

fn issue(
    identity: &NodeIdentity,
    item: ItemId,
    session: SessionId,
    now: DateTime<Utc>,
) -> PlaybackInfo {
    let (expires_at, ticket) = issue_ticket(identity, &ticket_scope(item), session, now);
    direct_playback(
        format!("/api/v1/items/{item}/stream?ticket={ticket}"),
        expires_at,
    )
}

fn verify(
    identity: &NodeIdentity,
    item: ItemId,
    ticket: &str,
    now: DateTime<Utc>,
) -> Option<SessionId> {
    verify_ticket(identity, &ticket_scope(item), ticket, now)
}

pub(super) fn verify_ticket(
    identity: &NodeIdentity,
    scope: &str,
    ticket: &str,
    now: DateTime<Utc>,
) -> Option<SessionId> {
    if ticket.len() > 192 {
        return None;
    }
    let (session, rest) = ticket.split_once('.')?;
    let (deadline, signature) = rest.split_once('.')?;
    let session: SessionId = session.parse().ok()?;
    let expiry: i64 = deadline.parse().ok()?;
    if expiry <= now.timestamp_millis() {
        return None;
    }
    let mut bytes = [0; 64];
    hex::decode_to_slice(signature, &mut bytes).ok()?;
    identity
        .node_id()
        .verify(format!("{scope}\n{session}\n{expiry}").as_bytes(), &bytes)
        .then_some(session)
}

pub(super) enum Media {
    Original(TrackFile),
    Cue(cue::Audio),
}

pub(super) async fn open_audio(
    location: palveni_db::music::TrackLocation,
) -> palveni_core::Result<Media> {
    if let Some(frames) = location.cue_frames.clone() {
        let file = palveni_library::local::open_audio_input(location).await?;
        Ok(Media::Cue(cue::Audio::new(file, frames)?))
    } else {
        palveni_library::local::open_track(location)
            .await
            .map(Media::Original)
    }
}

async fn open_media(state: &AppState, id: ItemId) -> Result<Media, ApiError> {
    let file = if let Some(location) = state.db.music().audio_location(id).await? {
        open_audio(location).await
    } else if let Some(location) = state.db.video().location(id).await? {
        palveni_library::local::open_video(location)
            .await
            .map(Media::Original)
    } else {
        return Err(ApiError::not_found("Media item not found"));
    };
    file.map_err(|err| match err {
        palveni_core::Error::Internal(_) => ApiError::internal(err),
        _ => ApiError::not_found("Media file unavailable"),
    })
}

pub async fn prepare(
    current: CurrentUser,
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
    query: Result<Query<transcode::Parameters>, QueryRejection>,
) -> Result<Response, ApiError> {
    let id = path_id(&path?.0, "Media item")?;
    if let Some(profile) = query?.profile()? {
        return transcode::prepare(&state, id, current.session_id, profile).await;
    }
    if matches!(open_media(&state, id).await?, Media::Cue(_)) {
        state
            .hardware
            .audio
            .check_format(
                transcode::tier(&state).await,
                palveni_media::audio::AudioFormat::Pcm,
            )
            .await
            .map_err(transcode::problem)?;
    }
    let info = issue(&state.identity, id, current.session_id, clock::now());
    Ok(([(header::CACHE_CONTROL, PRIVATE_CACHE)], Json(info)).into_response())
}

pub(super) fn playback_key(
    id: ItemId,
    namespace: &[u8],
    credential: &[u8],
) -> palveni_media::audio::PlaybackKey {
    let mut hash = ring::digest::Context::new(&ring::digest::SHA256);
    hash.update(namespace);
    hash.update(id.as_uuid().as_bytes());
    hash.update(credential);
    let mut key = [0; 32];
    key.copy_from_slice(hash.finish().as_ref());
    key
}

pub async fn stream(
    State(state): State<AppState>,
    method: Method,
    path: Result<Path<String>, PathRejection>,
    query: Result<Query<Access>, QueryRejection>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let (id, key) = if let Some(bearer) = headers.get(header::AUTHORIZATION) {
        CurrentUser::from_headers(&headers, &state).await?;
        let id = path_id(&path?.0, "Media item")?;
        (id, playback_key(id, b"bearer", bearer.as_bytes()))
    } else {
        let ticket = query
            .map_err(|_| ApiError::unauthorized())?
            .0
            .ticket
            .ok_or_else(ApiError::unauthorized)?;
        let id = path_id(&path?.0, "Media item")?;
        let session = verify(&state.identity, id, &ticket, clock::now())
            .ok_or_else(ApiError::unauthorized)?;
        state
            .db
            .sessions()
            .active_by_id(session)
            .await?
            .ok_or_else(ApiError::unauthorized)?;
        (id, playback_key(id, b"ticket", ticket.as_bytes()))
    };
    let track = open_media(&state, id).await?;
    serve_track(&state, track, method, &headers, key).await
}

pub(super) async fn serve_track(
    state: &AppState,
    track: Media,
    method: Method,
    headers: &HeaderMap,
    key: palveni_media::audio::PlaybackKey,
) -> Result<Response, ApiError> {
    let (size, content_type) = match &track {
        Media::Original(track) => (track.source.size(), track.content_type),
        Media::Cue(track) => (track.len(), "audio/wav"),
    };
    let mut response = Response::builder()
        .header(header::ACCEPT_RANGES, "bytes")
        .header(header::CONTENT_TYPE, content_type)
        .header(header::CACHE_CONTROL, PRIVATE_CACHE);
    let selection = if method == Method::HEAD {
        Selection::Full
    } else {
        byte_range::select(headers, size)
    };
    let is_range = matches!(&selection, Selection::Partial(_));
    let range = match selection {
        Selection::Full => 0..size,
        Selection::Partial(range) => {
            response = response.status(StatusCode::PARTIAL_CONTENT).header(
                header::CONTENT_RANGE,
                format!("bytes {}-{}/{size}", range.start, range.end - 1),
            );
            range
        }
        Selection::Unsatisfiable => {
            return response
                .status(StatusCode::RANGE_NOT_SATISFIABLE)
                .header(header::CONTENT_RANGE, format!("bytes */{size}"))
                .header(header::CONTENT_LENGTH, 0)
                .body(Body::empty())
                .map_err(ApiError::internal);
        }
    };
    let length = range.end - range.start;
    let body = if method == Method::HEAD || length == 0 {
        Body::empty()
    } else {
        match track {
            Media::Cue(track) => {
                track
                    .body(
                        state,
                        range,
                        palveni_media::audio::CueRequest {
                            key,
                            range: is_range,
                        },
                    )
                    .await?
            }
            Media::Original(track) => {
                let reader = track
                    .source
                    .read_range(ByteRange {
                        start: range.start,
                        end: Some(range.end),
                    })
                    .await
                    .map_err(ApiError::internal)?;
                Body::from_stream(ReaderStream::with_capacity(reader, 64 * 1024))
            }
        }
    };
    response
        .header(header::CONTENT_LENGTH, length)
        .body(body)
        .map_err(ApiError::internal)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_range_handoff_is_scoped_to_the_proof_protocol_and_item() {
        let item = ItemId::generate();
        let a = playback_key(item, b"ticket", b"first");
        assert_eq!(a, playback_key(item, b"ticket", b"first"));
        assert_ne!(a, playback_key(item, b"ticket", b"second"));
        assert_ne!(a, playback_key(item, b"bearer", b"first"));
        assert_ne!(a, playback_key(ItemId::generate(), b"ticket", b"first"));
    }

    #[test]
    fn tickets_are_bound_to_the_item_issuer_session_and_expiry() {
        let key = NodeIdentity::generate().unwrap();
        let item = ItemId::generate();
        let session = SessionId::generate();
        let now = clock::now();
        let info = issue(&key, item, session, now);
        let (_, token) = info.url.split_once("?ticket=").unwrap();
        assert_eq!(verify(&key, item, token, now), Some(session));
        assert_eq!(verify(&key, ItemId::generate(), token, now), None);
        assert_eq!(verify(&key, item, token, info.expires_at), None);
        assert_eq!(
            verify(&NodeIdentity::generate().unwrap(), item, token, now),
            None
        );
        let changed_session =
            token.replace(&session.to_string(), &SessionId::generate().to_string());
        assert_eq!(verify(&key, item, &changed_session, now), None);
        let extended = token.replace(
            &info.expires_at.timestamp_millis().to_string(),
            &(info.expires_at.timestamp_millis() + 1000).to_string(),
        );
        assert_eq!(verify(&key, item, &extended, now), None);
    }
}

//! Video media info probing, playback mode decision, and ticketed HLS session serving.
use axum::{
    Json,
    body::Body,
    extract::{
        Path, Query, State,
        rejection::{PathRejection, QueryRejection},
    },
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
};
use chrono::{DateTime, TimeDelta, Utc};
use palveni_core::{ItemId, SessionId, clock};
use palveni_link::NodeIdentity;
use palveni_media::video::{
    ClientProfile, DecisionError, DecisionRequest, DeliveryMode, MediaInfo, ProbeFailure,
    RequestedMode, StartHlsRequest, VideoError, decide, is_valid_segment_name, rewrite_playlist,
    segment_content_type,
};
use serde::{Deserialize, Serialize};
use tokio_util::io::ReaderStream;
use uuid::Uuid;

use super::{
    Access, PRIVATE_CACHE, issue_ticket_with_lifetime, ticket_scope, transcode, verify_ticket,
};
use crate::{AppState, auth::CurrentUser, error::ApiError, json::ApiJson, routes::path_id};

/// Video sessions can span a full-length movie; logout or session deletion still revokes access immediately.
const VIDEO_TICKET_LIFETIME: TimeDelta = TimeDelta::hours(6);
const MAX_START_MS: u64 = 7 * 24 * 3600 * 1000;

#[derive(utoipa::ToSchema, Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoPlaybackRequest {
    #[serde(default)]
    pub start_ms: Option<u64>,
    #[serde(default)]
    pub audio_index: Option<u32>,
    #[serde(default)]
    pub max_height: Option<u32>,
    #[serde(default)]
    pub max_bitrate_kbps: Option<u32>,
    #[serde(default)]
    pub mode: Option<RequestedMode>,
    pub client: ClientProfile,
}

#[derive(utoipa::ToSchema, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PlaybackProtocol {
    File,
    Hls,
}

#[derive(utoipa::ToSchema, Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoPlaybackResponse {
    pub session_id: String,
    pub mode: DeliveryMode,
    pub protocol: PlaybackProtocol,
    pub url: String,
    pub expires_at: DateTime<Utc>,
    pub start_ms: u64,
    #[schema(required = true)]
    pub reason: Option<String>,
}

fn hls_scope(session_id: Uuid) -> String {
    format!("palveni:video-hls:v1\n{session_id}")
}

fn issue_hls_ticket(
    identity: &NodeIdentity,
    video_session: Uuid,
    auth_session: SessionId,
    now: DateTime<Utc>,
) -> (DateTime<Utc>, String) {
    issue_ticket_with_lifetime(
        identity,
        &hls_scope(video_session),
        auth_session,
        now,
        VIDEO_TICKET_LIFETIME,
    )
}

fn verify_hls_ticket(
    identity: &NodeIdentity,
    video_session: Uuid,
    ticket: &str,
    now: DateTime<Utc>,
) -> Option<SessionId> {
    verify_ticket(identity, &hls_scope(video_session), ticket, now)
}

fn probe_error(err: ProbeFailure) -> ApiError {
    match err {
        ProbeFailure::Unavailable => ApiError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "transcode_unavailable",
            err.to_string(),
        ),
        ProbeFailure::Busy | ProbeFailure::Timeout => ApiError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "transcode_busy",
            err.to_string(),
        ),
        ProbeFailure::Invalid => ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "video_unsupported",
            err.to_string(),
        ),
    }
}

fn video_error(err: VideoError) -> ApiError {
    match err {
        VideoError::Disabled(_) | VideoError::Unavailable => ApiError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "transcode_unavailable",
            err.to_string(),
        ),
        VideoError::Busy { .. } | VideoError::SegmentNotReady => ApiError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "transcode_busy",
            err.to_string(),
        ),
        VideoError::NotFound => ApiError::not_found(err.to_string()),
        VideoError::Invalid(_) => ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "video_unsupported",
            err.to_string(),
        ),
    }
}

fn decision_error(err: DecisionError) -> ApiError {
    match err {
        DecisionError::UnknownAudio(_) => ApiError::validation(err.to_string()).field("audioIndex"),
        DecisionError::NoHls => ApiError::validation(err.to_string()).field("mode"),
    }
}

async fn open_video_source(
    state: &AppState,
    id: ItemId,
) -> Result<(std::fs::File, Option<String>), ApiError> {
    let location = state
        .db
        .video()
        .location(id)
        .await?
        .ok_or_else(|| ApiError::not_found("Video not found"))?;
    let extension = location
        .relative_path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(str::to_owned);
    let file = palveni_library::local::open_audio_input(location)
        .await
        .map_err(|err| match err {
            palveni_core::Error::Internal(_) => ApiError::internal(err),
            _ => ApiError::not_found("Video file unavailable"),
        })?;
    Ok((file, extension))
}

async fn authorize_hls(
    state: &AppState,
    video_session: Uuid,
    headers: &HeaderMap,
    query: Result<Query<Access>, QueryRejection>,
) -> Result<Option<String>, ApiError> {
    if headers.contains_key(header::AUTHORIZATION) {
        CurrentUser::from_headers(headers, state).await?;
        let ticket = query.ok().and_then(|q| q.0.ticket).filter(|t| {
            verify_hls_ticket(&state.identity, video_session, t, clock::now()).is_some()
        });
        return Ok(ticket);
    }
    let ticket = query
        .map_err(|_| ApiError::unauthorized())?
        .0
        .ticket
        .ok_or_else(ApiError::unauthorized)?;
    let auth_session = verify_hls_ticket(&state.identity, video_session, &ticket, clock::now())
        .ok_or_else(ApiError::unauthorized)?;
    state
        .db
        .sessions()
        .active_by_id(auth_session)
        .await?
        .ok_or_else(ApiError::unauthorized)?;
    Ok(Some(ticket))
}

/// `GET /api/v1/items/{id}/media-info`
pub async fn media_info(
    _: CurrentUser,
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
) -> Result<Response, ApiError> {
    let id = path_id(&path?.0, "Video")?;
    let (file, extension) = open_video_source(&state, id).await?;
    let probed = state
        .hardware
        .video
        .probe_media(id, &file, extension.as_deref())
        .await
        .map_err(probe_error)?;
    let info: MediaInfo = probed.info;
    Ok(([(header::CACHE_CONTROL, PRIVATE_CACHE)], Json(info)).into_response())
}

/// `POST /api/v1/items/{id}/video-playback`
pub async fn start(
    current: CurrentUser,
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
    body: Result<ApiJson<VideoPlaybackRequest>, ApiError>,
) -> Result<Response, ApiError> {
    let id = path_id(&path?.0, "Video")?;
    let ApiJson(request) = body?;
    let start_ms = request.start_ms.unwrap_or(0);
    if start_ms > MAX_START_MS {
        return Err(ApiError::validation("Start time exceeds seven days").field("startMs"));
    }
    if let Some(max_height) = request.max_height
        && !(16..=8640).contains(&max_height)
    {
        return Err(
            ApiError::validation("Maximum height must be between 16 and 8640").field("maxHeight"),
        );
    }
    if let Some(max_bitrate) = request.max_bitrate_kbps
        && !(100..=200_000).contains(&max_bitrate)
    {
        return Err(
            ApiError::validation("Maximum bitrate must be between 100 and 200000 kbit/s")
                .field("maxBitrateKbps"),
        );
    }

    let (file, extension) = open_video_source(&state, id).await?;
    let probed = state
        .hardware
        .video
        .probe_media(id, &file, extension.as_deref())
        .await
        .map_err(probe_error)?;

    if start_ms > 0
        && probed
            .info
            .duration_ms
            .is_some_and(|duration| duration > 0 && start_ms >= duration)
    {
        return Err(ApiError::validation("Start time exceeds video duration").field("startMs"));
    }

    let decision = decide(
        &probed,
        &DecisionRequest {
            audio_index: request.audio_index,
            max_height: request.max_height,
            max_bitrate_kbps: request.max_bitrate_kbps,
            mode: request.mode.unwrap_or_default(),
            client: request.client,
        },
    )
    .map_err(decision_error)?;

    let now = clock::now();
    let response = match decision.mode {
        DeliveryMode::Direct => {
            state
                .hardware
                .video
                .close_for_owner_item(current.session_id, id)
                .await;
            let (expires_at, ticket) = issue_ticket_with_lifetime(
                &state.identity,
                &ticket_scope(id),
                current.session_id,
                now,
                VIDEO_TICKET_LIFETIME,
            );
            VideoPlaybackResponse {
                session_id: Uuid::now_v7().to_string(),
                mode: DeliveryMode::Direct,
                protocol: PlaybackProtocol::File,
                url: format!("/api/v1/items/{id}/stream?ticket={ticket}"),
                expires_at,
                start_ms: 0,
                reason: decision.reason,
            }
        }
        DeliveryMode::Remux | DeliveryMode::Transcode => {
            let tier = transcode::tier(&state).await;
            let report = state.hardware.cached().await;
            let started = state
                .hardware
                .video
                .start_hls(
                    StartHlsRequest {
                        owner_user: current.user.id,
                        owner_session: current.session_id,
                        item: id,
                        tier,
                        start_ms,
                        max_bitrate_kbps: request.max_bitrate_kbps,
                        decision: decision.clone(),
                        media: probed,
                    },
                    file,
                    report.as_ref(),
                )
                .await
                .map_err(video_error)?;
            let (expires_at, ticket) =
                issue_hls_ticket(&state.identity, started.session_id, current.session_id, now);
            VideoPlaybackResponse {
                session_id: started.session_id.to_string(),
                mode: decision.mode,
                protocol: PlaybackProtocol::Hls,
                url: format!(
                    "/api/v1/video-sessions/{}/master.m3u8?ticket={ticket}",
                    started.session_id
                ),
                expires_at,
                start_ms: started.start_ms,
                reason: decision.reason,
            }
        }
    };

    Ok(([(header::CACHE_CONTROL, PRIVATE_CACHE)], Json(response)).into_response())
}

/// `GET /api/v1/video-sessions/{sessionId}/master.m3u8`
pub async fn playlist(
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
    query: Result<Query<Access>, QueryRejection>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let session_id: Uuid = path_id(&path?.0, "Video session")?;
    let ticket = authorize_hls(&state, session_id, &headers, query).await?;
    let raw = state
        .hardware
        .video
        .read_playlist(session_id)
        .await
        .map_err(video_error)?;
    let rewritten = rewrite_playlist(&raw, ticket.as_deref());
    Response::builder()
        .header(
            header::CONTENT_TYPE,
            "application/vnd.apple.mpegurl; charset=utf-8",
        )
        .header(header::CACHE_CONTROL, PRIVATE_CACHE)
        .header(header::X_CONTENT_TYPE_OPTIONS, "nosniff")
        .body(Body::from(rewritten))
        .map_err(ApiError::internal)
}

/// `GET /api/v1/video-sessions/{sessionId}/segment/{file}`
pub async fn segment(
    State(state): State<AppState>,
    path: Result<Path<(String, String)>, PathRejection>,
    query: Result<Query<Access>, QueryRejection>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let (raw_session_id, file) = path?.0;
    let session_id: Uuid = path_id(&raw_session_id, "Video session")?;
    if !is_valid_segment_name(&file) {
        return Err(ApiError::validation("Invalid segment filename").field("file"));
    }
    authorize_hls(&state, session_id, &headers, query).await?;
    let (opened, size) = state
        .hardware
        .video
        .open_segment(session_id, &file)
        .await
        .map_err(video_error)?;
    Response::builder()
        .header(header::CONTENT_TYPE, segment_content_type(&file))
        .header(header::CONTENT_LENGTH, size)
        .header(header::CACHE_CONTROL, PRIVATE_CACHE)
        .header(header::X_CONTENT_TYPE_OPTIONS, "nosniff")
        .body(Body::from_stream(ReaderStream::with_capacity(
            opened,
            64 * 1024,
        )))
        .map_err(ApiError::internal)
}

/// `DELETE /api/v1/video-sessions/{sessionId}`
pub async fn delete(
    current: CurrentUser,
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
) -> Result<StatusCode, ApiError> {
    let session_id: Uuid = path_id(&path?.0, "Video session")?;
    state
        .hardware
        .video
        .delete_session(session_id, current.user.id, current.is_admin())
        .await
        .map_err(video_error)?;
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hls_tickets_are_bound_to_video_session_auth_session_issuer_and_expiry() {
        let key = NodeIdentity::generate().unwrap();
        let other_key = NodeIdentity::generate().unwrap();
        let video_session = Uuid::now_v7();
        let other_video_session = Uuid::now_v7();
        let auth_session = SessionId::generate();
        let now = clock::now();

        let (expires_at, token) = issue_hls_ticket(&key, video_session, auth_session, now);
        assert_eq!(
            verify_hls_ticket(&key, video_session, &token, now),
            Some(auth_session)
        );
        // Bound to video_session.
        assert_eq!(
            verify_hls_ticket(&key, other_video_session, &token, now),
            None
        );
        // Bound to issuer key.
        assert_eq!(
            verify_hls_ticket(&other_key, video_session, &token, now),
            None
        );
        // Expires at expires_at.
        assert_eq!(
            verify_hls_ticket(&key, video_session, &token, expires_at),
            None
        );
        // Cannot be reused as a direct stream ticket.
        assert_eq!(
            super::super::verify(&key, ItemId::generate(), &token, now),
            None
        );
        // Tampered auth session fails verification.
        let tampered_session = token.replace(
            &auth_session.to_string(),
            &SessionId::generate().to_string(),
        );
        assert_eq!(
            verify_hls_ticket(&key, video_session, &tampered_session, now),
            None
        );
        // Tampered expiry fails verification.
        let tampered_expiry = token.replace(
            &expires_at.timestamp_millis().to_string(),
            &(expires_at.timestamp_millis() + 1000).to_string(),
        );
        assert_eq!(
            verify_hls_ticket(&key, video_session, &tampered_expiry, now),
            None
        );
    }

    #[test]
    fn response_json_matches_contract_fields() {
        let now = clock::now();
        let resp = VideoPlaybackResponse {
            session_id: Uuid::now_v7().to_string(),
            mode: DeliveryMode::Remux,
            protocol: PlaybackProtocol::Hls,
            url: "/api/v1/video-sessions/abc/master.m3u8?ticket=t".into(),
            expires_at: now,
            start_ms: 4_000,
            reason: Some("container mkv".into()),
        };
        let value = serde_json::to_value(&resp).unwrap();
        assert_eq!(value["mode"], "remux");
        assert_eq!(value["protocol"], "hls");
        assert_eq!(value["startMs"], 4_000);
        assert_eq!(value["reason"], "container mkv");
        assert!(value["sessionId"].is_string());
        assert!(value["expiresAt"].is_string());
    }
}

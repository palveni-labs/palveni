//! Multi-device live playback sessions and remote cast commands
//! (`/api/v1/sessions/playback*`, module `cast`).

use axum::Json;
use axum::extract::rejection::PathRejection;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use palveni_core::ServerEvent;
use palveni_core::sync_cast::{
    PlaybackCommandAck, PlaybackCommandKind, PlaybackCommandRequest, PlaybackHeartbeatRequest,
    PlaybackMediaKind, PlaybackSession, PlaybackSessionList, PlaybackSessionState,
};
use palveni_db::sync_cast::{
    ACTIVE_SESSION_WINDOW_MS, UpsertPlaybackSession, UpsertSessionOutcome,
};
use serde::Deserialize;

use crate::auth::CurrentUser;
use crate::error::ApiError;
use crate::json::ApiJson;
use crate::routes::mqtt::{dispatch_playback_mqtt_and_ambient, normalize_ambient_hex};
use crate::state::AppState;

#[derive(Deserialize)]
pub struct ListSessionsQuery {
    #[serde(default)]
    all: bool,
}

fn clean_opt(
    raw: Option<String>,
    max_chars: usize,
    field: &'static str,
) -> Result<Option<String>, ApiError> {
    let Some(value) = raw else {
        return Ok(None);
    };
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    if trimmed.chars().count() > max_chars || trimmed.chars().any(char::is_control) {
        return Err(ApiError::validation(format!(
            "{field} must be at most {max_chars} printable characters"
        ))
        .field(field));
    }
    Ok(Some(trimmed.to_owned()))
}

/// `POST /api/v1/sessions/playback/heartbeat`
pub async fn heartbeat(
    current: CurrentUser,
    State(state): State<AppState>,
    body: Result<ApiJson<PlaybackHeartbeatRequest>, ApiError>,
) -> Result<Json<PlaybackSession>, ApiError> {
    let ApiJson(req) = body?;

    let session_id = req.session_id.trim();
    if session_id.is_empty()
        || session_id.chars().count() > 64
        || session_id
            .chars()
            .any(|c| c.is_control() || c.is_whitespace())
    {
        return Err(ApiError::validation(
            "sessionId must be 1 to 64 non-whitespace printable characters",
        )
        .field("sessionId"));
    }

    let device_name = req.device_name.trim();
    if device_name.is_empty()
        || device_name.chars().count() > 128
        || device_name.chars().any(char::is_control)
    {
        return Err(
            ApiError::validation("deviceName must be 1 to 128 printable characters")
                .field("deviceName"),
        );
    }

    if req.position_ms < 0 {
        return Err(ApiError::validation("positionMs must be nonnegative").field("positionMs"));
    }
    if req.duration_ms < 0 {
        return Err(ApiError::validation("durationMs must be nonnegative").field("durationMs"));
    }
    if !(0..=100).contains(&req.volume) {
        return Err(ApiError::validation("volume must be between 0 and 100").field("volume"));
    }

    let ambient_hex = match req
        .ambient_hex
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        None => None,
        Some(raw) => Some(normalize_ambient_hex(raw).ok_or_else(|| {
            ApiError::validation("ambientHex must be a #RRGGBB hex color").field("ambientHex")
        })?),
    };

    let item_id = clean_opt(req.item_id, 36, "itemId")?;
    let library_id = clean_opt(req.library_id, 36, "libraryId")?;
    let title = clean_opt(req.title, 255, "title")?;
    let subtitle = clean_opt(req.subtitle, 255, "subtitle")?;
    let artwork_id = clean_opt(req.artwork_id, 80, "artworkId")?;

    let previous_state = state
        .db
        .sync_cast()
        .get_playback_session(session_id)
        .await?
        .map(|s| s.state);

    let outcome = state
        .db
        .sync_cast()
        .upsert_playback_session(
            current.user.id,
            UpsertPlaybackSession {
                session_id: session_id.to_owned(),
                device_name: device_name.to_owned(),
                client_kind: req.client_kind,
                media_kind: req.media_kind,
                item_id,
                library_id,
                title,
                subtitle,
                artwork_id,
                state: req.state,
                position_ms: req.position_ms,
                duration_ms: req.duration_ms,
                volume: req.volume,
                ambient_hex,
            },
        )
        .await?;

    let session = match outcome {
        UpsertSessionOutcome::Saved(session) => *session,
        UpsertSessionOutcome::ForbiddenOwner => {
            return Err(ApiError::new(
                StatusCode::FORBIDDEN,
                "forbidden",
                "Playback session belongs to another user",
            ));
        }
    };

    // Fire smart-home media automation on real state transitions (play / pause / stop).
    if previous_state != Some(session.state) {
        let was_active = matches!(
            previous_state,
            Some(PlaybackSessionState::Playing | PlaybackSessionState::Paused)
        );
        let home_event = match (session.media_kind, session.state) {
            (PlaybackMediaKind::Video, PlaybackSessionState::Playing) => Some("video.play"),
            (PlaybackMediaKind::Video, PlaybackSessionState::Paused) => Some("video.pause"),
            (PlaybackMediaKind::Video, PlaybackSessionState::Stopped) => Some("video.stop"),
            (PlaybackMediaKind::Video, PlaybackSessionState::Idle) if was_active => {
                Some("video.stop")
            }
            (
                PlaybackMediaKind::Audio | PlaybackMediaKind::Audiobook,
                PlaybackSessionState::Playing,
            ) => Some("music.play"),
            (
                PlaybackMediaKind::Audio | PlaybackMediaKind::Audiobook,
                PlaybackSessionState::Paused,
            ) => Some("music.pause"),
            (
                PlaybackMediaKind::Audio | PlaybackMediaKind::Audiobook,
                PlaybackSessionState::Stopped,
            ) => Some("music.stop"),
            (
                PlaybackMediaKind::Audio | PlaybackMediaKind::Audiobook,
                PlaybackSessionState::Idle,
            ) if was_active => Some("music.stop"),
            _ => None,
        };
        if let Some(event_kind) = home_event {
            crate::routes::home::trigger_media_event(&state, event_kind);
        }
    }

    state.bus.publish(ServerEvent::PlaybackSessionUpdated {
        user_id: current.user.id,
        session_id: session.id.clone(),
        device_name: session.device_name.clone(),
        state: session.state.as_str().to_owned(),
        item_id: session.item_id.clone(),
        title: session.title.clone(),
        position_ms: session.position_ms,
        volume: session.volume,
        ambient_hex: session.ambient_hex.clone(),
    });

    // Non-blocking background dispatch to MQTT bridges and ambient smart-home lights.
    let bg_state = state.clone();
    let bg_session = session.clone();
    tokio::spawn(async move {
        dispatch_playback_mqtt_and_ambient(bg_state, bg_session).await;
    });

    Ok(Json(session))
}

/// `GET /api/v1/sessions/playback`
pub async fn list(
    current: CurrentUser,
    State(state): State<AppState>,
    Query(query): Query<ListSessionsQuery>,
) -> Result<Json<PlaybackSessionList>, ApiError> {
    let scope_user = if query.all && current.is_admin() {
        None
    } else {
        Some(current.user.id)
    };
    let sessions = state
        .db
        .sync_cast()
        .list_active_playback_sessions(scope_user, ACTIVE_SESSION_WINDOW_MS)
        .await?;
    Ok(Json(PlaybackSessionList { sessions }))
}

/// `DELETE /api/v1/sessions/playback/{id}`
pub async fn delete(
    current: CurrentUser,
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
) -> Result<StatusCode, ApiError> {
    let id = path?.0;
    let Some(existing) = state.db.sync_cast().get_playback_session(&id).await? else {
        return Err(ApiError::not_found("Playback session not found"));
    };
    if existing.user_id != current.user.id && !current.is_admin() {
        return Err(ApiError::new(
            StatusCode::FORBIDDEN,
            "forbidden",
            "You can only delete your own playback sessions",
        ));
    }
    state.db.sync_cast().delete_playback_session(&id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `POST /api/v1/sessions/playback/{id}/command`
pub async fn send_command(
    current: CurrentUser,
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
    body: Result<ApiJson<PlaybackCommandRequest>, ApiError>,
) -> Result<(StatusCode, Json<PlaybackCommandAck>), ApiError> {
    let id = path?.0;
    let ApiJson(req) = body?;

    match req.command {
        PlaybackCommandKind::Seek => match req.position_ms {
            Some(pos) if pos >= 0 => {}
            _ => {
                return Err(
                    ApiError::validation("seek command requires a nonnegative positionMs")
                        .field("positionMs"),
                );
            }
        },
        PlaybackCommandKind::Volume => match req.volume {
            Some(vol) if (0..=100).contains(&vol) => {}
            _ => {
                return Err(ApiError::validation(
                    "volume command requires volume between 0 and 100",
                )
                .field("volume"));
            }
        },
        PlaybackCommandKind::PlayItem => match req.item_id.as_deref().map(str::trim) {
            Some(item_id) if !item_id.is_empty() && item_id.len() <= 36 => {}
            _ => {
                return Err(
                    ApiError::validation("play_item command requires a valid itemId")
                        .field("itemId"),
                );
            }
        },
        _ => {
            if req.position_ms.is_some_and(|p| p < 0) {
                return Err(
                    ApiError::validation("positionMs must be nonnegative").field("positionMs")
                );
            }
            if req.volume.is_some_and(|v| !(0..=100).contains(&v)) {
                return Err(
                    ApiError::validation("volume must be between 0 and 100").field("volume")
                );
            }
        }
    }

    let Some(target) = state.db.sync_cast().get_playback_session(&id).await? else {
        return Err(ApiError::not_found("Playback session not found"));
    };
    if target.user_id != current.user.id && !current.is_admin() {
        return Err(ApiError::new(
            StatusCode::FORBIDDEN,
            "forbidden",
            "You can only control your own playback sessions",
        ));
    }

    state.bus.publish(ServerEvent::PlaybackCommand {
        user_id: target.user_id,
        target_session_id: target.id.clone(),
        command: req.command.as_str().to_owned(),
        position_ms: req.position_ms,
        volume: req.volume,
        item_id: req.item_id.map(|s| s.trim().to_owned()),
        issued_by: current.user.username,
    });

    Ok((
        StatusCode::ACCEPTED,
        Json(PlaybackCommandAck {
            accepted: true,
            target_session_id: target.id,
            command: req.command,
        }),
    ))
}

use super::path_id;
use crate::{AppState, auth::CurrentUser, error::ApiError};
use axum::{
    Json,
    extract::{Path, State, rejection::PathRejection},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use palveni_db::music::TrackLocation;
use palveni_library::video::subtitles;
use palveni_media::subtitles::SubtitleError;

async fn source(state: &AppState, id: &str) -> Result<TrackLocation, ApiError> {
    state
        .db
        .video()
        .location(path_id(id, "Video")?)
        .await?
        .ok_or_else(|| ApiError::not_found("Video not found"))
}

fn subtitle_error(error: palveni_core::Error) -> ApiError {
    match error {
        palveni_core::Error::Invalid(message) => ApiError::validation(message),
        palveni_core::Error::Internal(_) => ApiError::internal(error),
        _ => ApiError::not_found("Video or subtitle unavailable"),
    }
}

fn embedded_error(error: SubtitleError) -> ApiError {
    match error {
        SubtitleError::Missing => ApiError::not_found("Embedded subtitle unavailable"),
        SubtitleError::Invalid => ApiError::validation("Invalid or oversized embedded subtitle"),
        SubtitleError::Busy | SubtitleError::Timeout => ApiError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "subtitle_unavailable",
            "Subtitle processing temporarily unavailable",
        ),
    }
}

pub async fn list(
    _: CurrentUser,
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
) -> Result<Response, ApiError> {
    let location = source(&state, &path?.0).await?;
    let is_strm = palveni_library::video::strm::is_strm_path(&location.relative_path);
    let (mut list, input) = subtitles::list(location).await.map_err(subtitle_error)?;
    if !is_strm {
        match state.hardware.subtitles.list(input).await {
            Ok(embedded) => {
                for track in embedded {
                    subtitles::append_embedded(
                        &mut list,
                        track.index,
                        track.language.as_deref(),
                        track.forced,
                        track.hearing_impaired,
                    );
                }
            }
            Err(_) if !list.tracks.is_empty() => {}
            Err(error) => return Err(embedded_error(error)),
        }
    }
    Ok(([(header::CACHE_CONTROL, "private, no-store")], Json(list)).into_response())
}

pub async fn read(
    _: CurrentUser,
    State(state): State<AppState>,
    path: Result<Path<(String, String)>, PathRejection>,
) -> Result<Response, ApiError> {
    let (id, track) = path?.0;
    let location = source(&state, &id).await?;
    let body = match subtitles::read(location, &track)
        .await
        .map_err(subtitle_error)?
    {
        subtitles::SubtitleSource::WebVtt(body) => body,
        subtitles::SubtitleSource::Embedded { file, index } => {
            let bytes = state
                .hardware
                .subtitles
                .read(file, index)
                .await
                .map_err(embedded_error)?;
            subtitles::render_embedded(bytes)
                .await
                .map_err(subtitle_error)?
        }
    };
    Ok((
        [
            (header::CONTENT_TYPE, "text/vtt; charset=utf-8"),
            (header::CACHE_CONTROL, "private, no-store"),
            (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
        ],
        body,
    )
        .into_response())
}

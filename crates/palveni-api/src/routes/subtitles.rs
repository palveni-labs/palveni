use super::path_id;
use crate::{AppState, auth::CurrentUser, error::ApiError};
use axum::{
    Json,
    extract::{Path, State, rejection::PathRejection},
    http::header,
    response::{IntoResponse, Response},
};
use palveni_db::music::TrackLocation;
use palveni_library::video::subtitles;

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

pub async fn list(
    _: CurrentUser,
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
) -> Result<Response, ApiError> {
    let location = source(&state, &path?.0).await?;
    let list = subtitles::list(location).await.map_err(subtitle_error)?;
    Ok(([(header::CACHE_CONTROL, "private, no-store")], Json(list)).into_response())
}

pub async fn read(
    _: CurrentUser,
    State(state): State<AppState>,
    path: Result<Path<(String, String)>, PathRejection>,
) -> Result<Response, ApiError> {
    let (id, track) = path?.0;
    let location = source(&state, &id).await?;
    let body = subtitles::read(location, &track)
        .await
        .map_err(subtitle_error)?;
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

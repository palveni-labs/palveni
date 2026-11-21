pub mod search;

use axum::{
    Json,
    extract::{
        Path, Query, State,
        rejection::{PathRejection, QueryRejection},
    },
};
use palveni_core::{LibraryId, music::MusicPage};
use palveni_db::music::MusicCursor;
use serde::Deserialize;

use super::{DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE, parse_limit, path_id};
use crate::{auth::CurrentUser, error::ApiError, state::AppState};

#[derive(Deserialize)]
pub struct Browse {
    artist: Option<String>,
    album: Option<String>,
    cursor: Option<String>,
    limit: Option<String>,
}

pub(super) fn validate_group(value: Option<&str>, field: &'static str) -> Result<(), ApiError> {
    if let Some(key) = value
        && (key.len() != 64
            || !key
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)))
    {
        return Err(ApiError::validation("Invalid group identifier").field(field));
    }
    Ok(())
}

pub async fn list(
    _: CurrentUser,
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
    query: Result<Query<Browse>, QueryRejection>,
) -> Result<Json<MusicPage>, ApiError> {
    let Path(raw) = path?;
    let id: LibraryId = path_id(&raw, "Library")?;
    let Query(query) = query?;
    let artist = query.artist.as_deref();
    let album = query.album.as_deref();
    validate_group(artist, "artist")?;
    validate_group(album, "album")?;
    if album.is_some() && artist.is_none() {
        return Err(ApiError::validation("An album requires its artist group").field("artist"));
    }
    let limit = parse_limit(query.limit.as_deref(), DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE)?;
    let cursor = query
        .cursor
        .as_deref()
        .map(|text| {
            MusicCursor::decode(text, id, artist, album)
                .ok_or_else(|| ApiError::validation("Invalid music cursor").field("cursor"))
        })
        .transpose()?;
    if !state.db.music().has_library(id).await? {
        return Err(ApiError::not_found("Music library not found"));
    }
    Ok(Json(
        state
            .db
            .music()
            .browse(id, artist, album, limit, cursor)
            .await?,
    ))
}

pub async fn analyze_loudness(
    _: crate::auth::AdminUser,
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
) -> Result<(axum::http::StatusCode, Json<super::libraries::ScanAccepted>), ApiError> {
    use palveni_core::{JobSubject, LibraryKind, music::LOUDNESS_JOB_KIND};
    let id: LibraryId = path_id(&path?.0, "Library")?;
    let library = state
        .db
        .libraries()
        .get(id)
        .await?
        .filter(|l| l.kind == LibraryKind::Music)
        .ok_or_else(|| ApiError::not_found("Music library not found"))?;
    let job = state
        .jobs
        .enqueue(palveni_jobs::NewJob {
            kind: LOUDNESS_JOB_KIND.into(),
            payload: serde_json::json!(id),
            subject: Some(JobSubject::library(id, library.name)),
            required_capabilities: Vec::new(),
            dedupe_key: Some(format!("{LOUDNESS_JOB_KIND}:{id}")),
        })
        .await?;
    Ok((
        axum::http::StatusCode::ACCEPTED,
        Json(super::libraries::ScanAccepted { job_id: job.id }),
    ))
}

pub async fn loudness(
    _: CurrentUser,
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
) -> Result<Json<palveni_core::music::TrackLoudness>, ApiError> {
    let id = path_id(&path?.0, "Track")?;
    let measurement = state.db.music().loudness(id).await?;
    Ok(Json(palveni_core::music::TrackLoudness { measurement }))
}

pub async fn lyrics(
    _user: CurrentUser,
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
) -> Result<
    (
        axum::http::HeaderMap,
        Json<palveni_core::music::TrackLyrics>,
    ),
    ApiError,
> {
    let id = path_id(&path?.0, "Track")?;
    let location = state
        .db
        .music()
        .track_location(id)
        .await?
        .ok_or_else(|| ApiError::not_found("Track not found"))?;
    let lyrics = palveni_library::lyrics::read(location)
        .await
        .map_err(|error| match error {
            palveni_core::Error::Internal(detail) => ApiError::internal(detail),
            _ => ApiError::not_found("Track not available"),
        })?;
    let mut headers = axum::http::HeaderMap::new();
    headers.insert(
        axum::http::header::CACHE_CONTROL,
        axum::http::HeaderValue::from_static("private, no-store"),
    );
    Ok((headers, Json(palveni_core::music::TrackLyrics { lyrics })))
}

pub async fn artwork(
    _: CurrentUser,
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
) -> Result<axum::response::Response, ApiError> {
    use axum::{body::Body, http::header, response::Response};
    let id = path_id(&path?.0, "Track")?;
    let (mime, data) = palveni_library::artwork::open(&state.db, id)
        .await
        .map_err(|e| match e {
            palveni_core::Error::Internal(_) => ApiError::internal(e),
            _ => ApiError::not_found("Artwork unavailable"),
        })?;
    Response::builder()
        .header(header::CONTENT_TYPE, mime)
        .header(header::CONTENT_LENGTH, data.len())
        .header(header::CACHE_CONTROL, "private, no-cache")
        .header(header::X_CONTENT_TYPE_OPTIONS, "nosniff")
        .body(Body::from(data))
        .map_err(ApiError::internal)
}

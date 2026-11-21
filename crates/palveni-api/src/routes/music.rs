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

#[derive(Deserialize)]
pub struct Search {
    q: String,
    kind: Option<palveni_core::music::MusicSearchKind>,
    cursor: Option<String>,
    limit: Option<u32>,
}

#[derive(Deserialize, serde::Serialize)]
struct SearchCursor {
    revision: i64,
    position: usize,
    scope: String,
}

pub async fn search(
    _: CurrentUser,
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
    query: Result<Query<Search>, QueryRejection>,
) -> Result<Json<palveni_core::music::MusicSearchPage>, ApiError> {
    use palveni_core::music::{MusicSearchKind, MusicSearchPage};
    let id: LibraryId = path_id(&path?.0, "Library")?;
    let Query(query) = query?;
    let kind = query.kind.unwrap_or(MusicSearchKind::Track);
    let terms = palveni_library::music_search::terms(&query.q)
        .map_err(|_| ApiError::validation("Invalid music search query").field("q"))?;
    let limit = query.limit.unwrap_or(30);
    if !(1..=200).contains(&limit) {
        return Err(ApiError::validation("Invalid limit").field("limit"));
    }
    let scope = hex::encode(ring::digest::digest(
        &ring::digest::SHA256,
        format!("{id}/{kind:?}/{}", terms.join("\0")).as_bytes(),
    ));
    let cursor = query
        .cursor
        .as_deref()
        .map(|text| {
            let cursor: Option<SearchCursor> = (text.len() <= 512)
                .then(|| hex::decode(text).ok())
                .flatten()
                .and_then(|bytes| serde_json::from_slice(&bytes).ok());
            cursor
                .filter(|c| c.scope == scope)
                .map(|c| (c.revision, c.position))
                .ok_or_else(|| ApiError::validation("Invalid music search cursor").field("cursor"))
        })
        .transpose()?;
    let matches = state
        .music_search
        .search(&state.db, id, kind, terms, limit as usize, cursor)
        .await
        .map_err(|error| match error {
            palveni_core::Error::NotFound(_) => ApiError::not_found("Music library not found"),
            palveni_core::Error::Invalid(message) => ApiError::validation(message),
            palveni_core::Error::Conflict(message) => ApiError::conflict(message),
            other => ApiError::internal(other),
        })?;
    let next_cursor = matches
        .next
        .map(|position| {
            serde_json::to_vec(&SearchCursor {
                revision: matches.revision,
                position,
                scope,
            })
            .map(hex::encode)
            .map_err(|err| ApiError::internal(err.to_string()))
        })
        .transpose()?;
    Ok(Json(MusicSearchPage {
        entries: matches.entries,
        next_cursor,
    }))
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

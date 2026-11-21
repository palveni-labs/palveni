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

fn validate_group(value: Option<&str>, field: &'static str) -> Result<(), ApiError> {
    if let Some(key) = value
        && (key.len() != 64
            || !key
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)))
    {
        return Err(ApiError::validation("Invalid music group").field(field));
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

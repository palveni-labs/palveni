use axum::{
    Json,
    extract::{
        Path, Query, State,
        rejection::{PathRejection, QueryRejection},
    },
};
use palveni_core::{LibraryId, spoken::SpokenPage};
use palveni_db::music::MusicCursor;
use serde::Deserialize;

use super::{DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE, music::validate_group, parse_limit, path_id};
use crate::{ApiError, AppState, auth::CurrentUser};

#[derive(Deserialize)]
pub struct Browse {
    collection: Option<String>,
    cursor: Option<String>,
    limit: Option<String>,
}

pub async fn list(
    _: CurrentUser,
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
    query: Result<Query<Browse>, QueryRejection>,
) -> Result<Json<SpokenPage>, ApiError> {
    let id: LibraryId = path_id(&path?.0, "Library")?;
    let Query(query) = query?;
    let collection = query.collection.as_deref();
    validate_group(collection, "collection")?;
    let limit = parse_limit(query.limit.as_deref(), DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE)?;
    let cursor = query
        .cursor
        .as_deref()
        .map(|text| {
            MusicCursor::decode_spoken(text, id, collection).ok_or_else(|| {
                ApiError::validation("Invalid spoken library cursor").field("cursor")
            })
        })
        .transpose()?;
    state
        .db
        .music()
        .browse_spoken(id, collection, limit, cursor)
        .await?
        .map(Json)
        .ok_or_else(|| ApiError::not_found("Spoken library not found"))
}

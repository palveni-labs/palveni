//! Personal media state and music playlists for the authenticated account.
use axum::{
    Json,
    extract::{
        Path, Query, State,
        rejection::{PathRejection, QueryRejection},
    },
    http::StatusCode,
};
use palveni_core::{
    ItemId,
    ids::PlaylistId,
    personal::{
        MediaState, MediaView, PersonalItemPage, PlaybackUpdate, Playlist, PlaylistPage,
        PlaylistTrackPage,
    },
};
use palveni_db::personal::MediaChange;
use serde::Deserialize;
use utoipa::ToSchema;

use super::{DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE, parse_limit, path_id};
use crate::{ApiError, AppState, auth::CurrentUser, json::ApiJson};

type IdPath = Result<Path<String>, PathRejection>;

#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct FavoriteUpdate {
    favorite: bool,
}

#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RatingUpdate {
    #[schema(minimum = 0, maximum = 5)]
    stars: i32,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlaylistUpdate {
    #[schema(minimum = 0)]
    expected_version: i64,
    #[schema(min_length = 1, max_length = 128)]
    name: String,
    #[schema(max_items = 5000)]
    track_ids: Vec<ItemId>,
}

#[derive(Deserialize)]
pub struct PageQuery {
    before: Option<i64>,
    limit: Option<String>,
}

impl PageQuery {
    fn bounds(self) -> Result<(Option<i64>, u32), ApiError> {
        if self.before.is_some_and(|v| v <= 0) {
            return Err(ApiError::validation("before must be a positive revision").field("before"));
        }
        Ok((
            self.before,
            parse_limit(self.limit.as_deref(), DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE)?,
        ))
    }
}

#[derive(Deserialize)]
pub struct MediaQuery {
    view: MediaView,
    before: Option<i64>,
    limit: Option<String>,
}

#[derive(Deserialize)]
pub struct DeleteQuery {
    version: i64,
}

fn media_id(path: IdPath) -> Result<ItemId, ApiError> {
    path_id(&path?.0, "Media item")
}
fn playlist_id(path: IdPath) -> Result<PlaylistId, ApiError> {
    path_id(&path?.0, "Playlist")
}

pub async fn get(
    user: CurrentUser,
    State(state): State<AppState>,
    path: IdPath,
) -> Result<Json<MediaState>, ApiError> {
    Ok(Json(
        state
            .db
            .personal()
            .get(user.user.id, media_id(path)?)
            .await?,
    ))
}

pub async fn favorite(
    user: CurrentUser,
    State(state): State<AppState>,
    path: IdPath,
    ApiJson(body): ApiJson<FavoriteUpdate>,
) -> Result<Json<MediaState>, ApiError> {
    Ok(Json(
        state
            .db
            .personal()
            .update(
                user.user.id,
                media_id(path)?,
                MediaChange::Favorite(body.favorite),
            )
            .await?,
    ))
}

pub async fn rating(
    user: CurrentUser,
    State(state): State<AppState>,
    path: IdPath,
    ApiJson(body): ApiJson<RatingUpdate>,
) -> Result<Json<MediaState>, ApiError> {
    Ok(Json(
        state
            .db
            .personal()
            .update(
                user.user.id,
                media_id(path)?,
                MediaChange::Rating(body.stars),
            )
            .await?,
    ))
}

pub async fn progress(
    user: CurrentUser,
    State(state): State<AppState>,
    path: IdPath,
    ApiJson(body): ApiJson<PlaybackUpdate>,
) -> Result<Json<MediaState>, ApiError> {
    Ok(Json(
        state
            .db
            .personal()
            .update(user.user.id, media_id(path)?, MediaChange::Progress(body))
            .await?,
    ))
}

pub async fn mark_played(
    user: CurrentUser,
    State(state): State<AppState>,
    path: IdPath,
) -> Result<StatusCode, ApiError> {
    state
        .db
        .personal()
        .update(user.user.id, media_id(path)?, MediaChange::Played(true))
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn mark_unplayed(
    user: CurrentUser,
    State(state): State<AppState>,
    path: IdPath,
) -> Result<StatusCode, ApiError> {
    state
        .db
        .personal()
        .update(user.user.id, media_id(path)?, MediaChange::Played(false))
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn list(
    user: CurrentUser,
    State(state): State<AppState>,
    query: Result<Query<MediaQuery>, QueryRejection>,
) -> Result<Json<PersonalItemPage>, ApiError> {
    let Query(q) = query?;
    let (before, limit) = PageQuery {
        before: q.before,
        limit: q.limit,
    }
    .bounds()?;
    Ok(Json(
        state
            .db
            .personal()
            .list(user.user.id, q.view, before, limit)
            .await?,
    ))
}

pub async fn playlists(
    user: CurrentUser,
    State(state): State<AppState>,
    query: Result<Query<PageQuery>, QueryRejection>,
) -> Result<Json<PlaylistPage>, ApiError> {
    let (before, limit) = query?.0.bounds()?;
    Ok(Json(
        state
            .db
            .playlists()
            .list(user.user.id, before, limit)
            .await?,
    ))
}

#[derive(Deserialize)]
pub struct PlaylistTracksQuery {
    version: i64,
    after: Option<i32>,
    limit: Option<String>,
}

pub async fn playlist_tracks(
    user: CurrentUser,
    State(state): State<AppState>,
    path: IdPath,
    query: Result<Query<PlaylistTracksQuery>, QueryRejection>,
) -> Result<Json<PlaylistTrackPage>, ApiError> {
    let q = query?.0;
    if q.version <= 0 {
        return Err(ApiError::validation("version must be positive").field("version"));
    }
    if q.after.is_some_and(|v| v < 0) {
        return Err(ApiError::validation("after must be a nonnegative position").field("after"));
    }
    let limit = parse_limit(q.limit.as_deref(), DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE)?;
    Ok(Json(
        state
            .db
            .playlists()
            .tracks(user.user.id, playlist_id(path)?, q.version, q.after, limit)
            .await?,
    ))
}

pub async fn get_playlist(
    user: CurrentUser,
    State(state): State<AppState>,
    path: IdPath,
) -> Result<Json<Playlist>, ApiError> {
    Ok(Json(
        state
            .db
            .playlists()
            .get(user.user.id, playlist_id(path)?)
            .await?,
    ))
}

pub async fn save_playlist(
    user: CurrentUser,
    State(state): State<AppState>,
    path: IdPath,
    ApiJson(body): ApiJson<PlaylistUpdate>,
) -> Result<Json<Playlist>, ApiError> {
    Ok(Json(
        state
            .db
            .playlists()
            .save(
                user.user.id,
                playlist_id(path)?,
                body.expected_version,
                &body.name,
                body.track_ids,
            )
            .await?,
    ))
}

pub async fn delete_playlist(
    user: CurrentUser,
    State(state): State<AppState>,
    path: IdPath,
    query: Result<Query<DeleteQuery>, QueryRejection>,
) -> Result<StatusCode, ApiError> {
    let version = query?.0.version;
    if version <= 0 {
        return Err(ApiError::validation("version must be positive").field("version"));
    }
    state
        .db
        .playlists()
        .delete(user.user.id, playlist_id(path)?, version)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

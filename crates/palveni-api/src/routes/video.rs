use axum::{
    Json,
    body::Body,
    extract::{
        Path, Query, State,
        rejection::{PathRejection, QueryRejection},
    },
    http::{StatusCode, header},
    response::Response,
};
use palveni_core::video::{
    SortDir, VideoGenreList, VideoHome, VideoOrder, VideoPage, VideoWorkDetail, WorkKind,
};
use palveni_db::video::{PosterLevel, StartsWith, VideoCursor, VideoFilters};
use serde::Deserialize;
use tokio_util::io::ReaderStream;

use super::{MAX_PAGE_SIZE, parse_limit, path_id, query_id};
use crate::{AppState, auth::CurrentUser, error::ApiError};

const DEFAULT_VIDEO_PAGE_SIZE: u32 = 60;

fn is_hex64(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn parse_size(raw: Option<&str>) -> Result<Option<u16>, ApiError> {
    match raw.map(str::trim).filter(|s| !s.is_empty()) {
        None => Ok(None),
        Some(text) => {
            let value: u16 = text
                .parse()
                .ok()
                .filter(|v| (1..=3840).contains(v))
                .ok_or_else(|| {
                    ApiError::validation("size must be an integer between 1 and 3840").field("size")
                })?;
            Ok(Some(value))
        }
    }
}

fn image_response(
    mime: &'static str,
    size: u64,
    reader: palveni_core::storage::ByteStream,
) -> Result<Response, ApiError> {
    Response::builder()
        .header(header::CONTENT_TYPE, mime)
        .header(header::CONTENT_LENGTH, size)
        .header(header::CACHE_CONTROL, "private, no-cache")
        .header(header::X_CONTENT_TYPE_OPTIONS, "nosniff")
        .body(Body::from_stream(ReaderStream::with_capacity(
            reader,
            32 * 1024,
        )))
        .map_err(ApiError::internal)
}

fn map_image_error(err: palveni_core::Error, message: &'static str) -> ApiError {
    match err {
        palveni_core::Error::Internal(_) => ApiError::internal(err),
        _ => ApiError::not_found(message),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Browse {
    series: Option<String>,
    season: Option<i32>,
    cursor: Option<String>,
    limit: Option<String>,
    q: Option<String>,
    kind: Option<WorkKind>,
    #[serde(default)]
    order: VideoOrder,
    dir: Option<SortDir>,
    seed: Option<String>,
    genre: Option<String>,
    year_from: Option<i32>,
    year_to: Option<i32>,
    played: Option<bool>,
    favorite: Option<bool>,
    starts_with: Option<String>,
}

impl Browse {
    fn filters(&self) -> Result<VideoFilters<'_>, ApiError> {
        let title = self.q.as_deref().unwrap_or_default().trim();
        if title.chars().count() > 128 || title.chars().any(char::is_control) {
            return Err(ApiError::validation("Invalid search title").field("q"));
        }
        let genre = self
            .genre
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty());
        if genre.is_some_and(|g| g.chars().count() > 128 || g.chars().any(char::is_control)) {
            return Err(ApiError::validation("Invalid genre filter").field("genre"));
        }
        let seed = self
            .seed
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty());
        if seed.is_some_and(|s| s.chars().count() > 64 || s.chars().any(char::is_control)) {
            return Err(ApiError::validation("Invalid random seed").field("seed"));
        }
        if self.year_from.is_some_and(|y| !(1000..=9999).contains(&y)) {
            return Err(
                ApiError::validation("yearFrom must be a four-digit year").field("yearFrom")
            );
        }
        if self.year_to.is_some_and(|y| !(1000..=9999).contains(&y)) {
            return Err(ApiError::validation("yearTo must be a four-digit year").field("yearTo"));
        }
        let starts_with = self
            .starts_with
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| {
                StartsWith::parse(s).ok_or_else(|| {
                    ApiError::validation("startsWith must be A-Z or #").field("startsWith")
                })
            })
            .transpose()?;

        let order = self.order()?;
        if self.series.is_some() || self.season.is_some() {
            if !title.is_empty() {
                return Err(ApiError::validation("Title search applies only to works").field("q"));
            }
            if self.kind.is_some() {
                return Err(ApiError::validation("Kind filter applies only to works").field("kind"));
            }
            if genre.is_some() {
                return Err(
                    ApiError::validation("Genre filter applies only to works").field("genre")
                );
            }
            if self.year_from.is_some() {
                return Err(
                    ApiError::validation("Year filter applies only to works").field("yearFrom")
                );
            }
            if self.year_to.is_some() {
                return Err(
                    ApiError::validation("Year filter applies only to works").field("yearTo")
                );
            }
            if self.played.is_some() {
                return Err(
                    ApiError::validation("Played filter applies only to works").field("played")
                );
            }
            if self.favorite.is_some() {
                return Err(
                    ApiError::validation("Favorite filter applies only to works").field("favorite"),
                );
            }
            if starts_with.is_some() {
                return Err(
                    ApiError::validation("startsWith applies only to works").field("startsWith")
                );
            }
            if self.dir.is_some_and(|d| d != SortDir::Asc) {
                return Err(
                    ApiError::validation("Sort direction applies only to works").field("dir")
                );
            }
        }
        Ok(VideoFilters {
            order,
            dir: self.dir,
            seed,
            title,
            kind: self.kind,
            genre,
            year_from: self.year_from,
            year_to: self.year_to,
            played: self.played,
            favorite: self.favorite,
            starts_with,
        })
    }

    fn order(&self) -> Result<VideoOrder, ApiError> {
        if self.order != VideoOrder::Title && (self.series.is_some() || self.season.is_some()) {
            return Err(
                ApiError::validation("Custom ordering applies only to works").field("order"),
            );
        }
        Ok(self.order)
    }
}

pub async fn list(
    user: CurrentUser,
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
    params: Result<Query<Browse>, QueryRejection>,
) -> Result<Json<VideoPage>, ApiError> {
    let library = path_id(&path?.0, "Library")?;
    let query = params?.0;
    let series = query.series.as_deref();
    if series.is_some_and(|s| !is_hex64(s)) {
        return Err(ApiError::validation("Invalid series").field("series"));
    }
    if query
        .season
        .is_some_and(|n| !(0..=65_535).contains(&n) || series.is_none())
    {
        return Err(ApiError::validation("A season requires its series").field("season"));
    }
    let filters = query.filters()?;
    let limit = parse_limit(
        query.limit.as_deref(),
        DEFAULT_VIDEO_PAGE_SIZE,
        MAX_PAGE_SIZE,
    )?;
    let cursor = query
        .cursor
        .as_deref()
        .map(|s| {
            VideoCursor::decode_filtered(s, library, series, query.season, filters)
                .ok_or_else(|| ApiError::validation("Invalid video cursor").field("cursor"))
        })
        .transpose()?;
    let videos = state.db.video();
    if !videos.has_library(library).await? {
        return Err(ApiError::not_found("Video library not found"));
    }
    Ok(Json(
        videos
            .browse_for(
                Some(user.user.id),
                library,
                series,
                query.season,
                filters,
                limit,
                cursor,
            )
            .await?,
    ))
}

pub async fn work_detail(
    user: CurrentUser,
    State(state): State<AppState>,
    path: Result<Path<(String, String)>, PathRejection>,
) -> Result<Json<VideoWorkDetail>, ApiError> {
    let (raw_lib, work_id) = path?.0;
    let library = path_id(&raw_lib, "Library")?;
    if !is_hex64(&work_id) {
        return Err(ApiError::not_found("Video work not found"));
    }
    let detail = state
        .db
        .video()
        .work_detail(user.user.id, library, &work_id)
        .await?
        .ok_or_else(|| ApiError::not_found("Video work not found"))?;
    Ok(Json(detail))
}

pub async fn genres(
    _: CurrentUser,
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
) -> Result<Json<VideoGenreList>, ApiError> {
    let library = path_id(&path?.0, "Library")?;
    let videos = state.db.video();
    if !videos.has_library(library).await? {
        return Err(ApiError::not_found("Video library not found"));
    }
    Ok(Json(videos.genres(library).await?))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HomeQuery {
    library_id: Option<String>,
}

pub async fn home(
    user: CurrentUser,
    State(state): State<AppState>,
    params: Result<Query<HomeQuery>, QueryRejection>,
) -> Result<Json<VideoHome>, ApiError> {
    let library_id = query_id(params?.0.library_id.as_deref(), "libraryId")?;
    let result = state
        .db
        .video()
        .home(user.user.id, library_id)
        .await?
        .ok_or_else(|| ApiError::not_found("Video library not found"))?;
    Ok(Json(result))
}

pub async fn mark_work_played(
    user: CurrentUser,
    State(state): State<AppState>,
    path: Result<Path<(String, String)>, PathRejection>,
) -> Result<StatusCode, ApiError> {
    set_work_played(user, state, path, true).await
}

pub async fn mark_work_unplayed(
    user: CurrentUser,
    State(state): State<AppState>,
    path: Result<Path<(String, String)>, PathRejection>,
) -> Result<StatusCode, ApiError> {
    set_work_played(user, state, path, false).await
}

async fn set_work_played(
    user: CurrentUser,
    state: AppState,
    path: Result<Path<(String, String)>, PathRejection>,
    played: bool,
) -> Result<StatusCode, ApiError> {
    let (raw_lib, work_id) = path?.0;
    let library = path_id(&raw_lib, "Library")?;
    if !is_hex64(&work_id) {
        return Err(ApiError::not_found("Video work not found"));
    }
    if !state
        .db
        .video()
        .set_work_played(user.user.id, library, &work_id, played)
        .await?
    {
        return Err(ApiError::not_found("Video work not found"));
    }
    Ok(StatusCode::NO_CONTENT)
}

pub async fn favorite_work(
    user: CurrentUser,
    State(state): State<AppState>,
    path: Result<Path<(String, String)>, PathRejection>,
) -> Result<StatusCode, ApiError> {
    set_work_favorite(user, state, path, true).await
}

pub async fn unfavorite_work(
    user: CurrentUser,
    State(state): State<AppState>,
    path: Result<Path<(String, String)>, PathRejection>,
) -> Result<StatusCode, ApiError> {
    set_work_favorite(user, state, path, false).await
}

async fn set_work_favorite(
    user: CurrentUser,
    state: AppState,
    path: Result<Path<(String, String)>, PathRejection>,
    favorite: bool,
) -> Result<StatusCode, ApiError> {
    let (raw_lib, work_id) = path?.0;
    let library = path_id(&raw_lib, "Library")?;
    if !is_hex64(&work_id) {
        return Err(ApiError::not_found("Video work not found"));
    }
    if !state
        .db
        .video()
        .set_work_favorite(user.user.id, library, &work_id, favorite)
        .await?
    {
        return Err(ApiError::not_found("Video work not found"));
    }
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
pub struct Poster {
    level: Option<String>,
    size: Option<String>,
}

#[derive(Deserialize)]
pub struct ImageQuery {
    size: Option<String>,
}

pub async fn poster(
    _: CurrentUser,
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
    params: Result<Query<Poster>, QueryRejection>,
) -> Result<Response, ApiError> {
    let id = path_id(&path?.0, "Video")?;
    let query = params?.0;
    let level = match query.level.as_deref().unwrap_or("work") {
        "work" => PosterLevel::Work,
        "season" => PosterLevel::Season,
        "episode" => PosterLevel::Episode,
        _ => return Err(ApiError::validation("Unknown poster level").field("level")),
    };
    let size = parse_size(query.size.as_deref())?;
    let loc = state
        .db
        .video()
        .poster_location(id, level)
        .await?
        .ok_or_else(|| ApiError::not_found("Poster not found"))?;
    let (mime, len, reader) = state
        .video_images
        .open_image(loc, size)
        .await
        .map_err(|e| map_image_error(e, "Poster unavailable"))?;
    image_response(mime, len, reader)
}

pub async fn backdrop(
    _: CurrentUser,
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
    params: Result<Query<ImageQuery>, QueryRejection>,
) -> Result<Response, ApiError> {
    let id = path_id(&path?.0, "Video")?;
    let size = parse_size(params?.0.size.as_deref())?;
    let loc = state
        .db
        .video()
        .backdrop_location(id)
        .await?
        .ok_or_else(|| ApiError::not_found("Backdrop not found"))?;
    let (mime, len, reader) = state
        .video_images
        .open_image(loc, size)
        .await
        .map_err(|e| map_image_error(e, "Backdrop unavailable"))?;
    image_response(mime, len, reader)
}

pub async fn thumb(
    _: CurrentUser,
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
    params: Result<Query<ImageQuery>, QueryRejection>,
) -> Result<Response, ApiError> {
    let id = path_id(&path?.0, "Video")?;
    let size = parse_size(params?.0.size.as_deref())?;
    let loc = state
        .db
        .video()
        .thumb_location(id)
        .await?
        .ok_or_else(|| ApiError::not_found("Video not found"))?;
    let (mime, len, reader) = state
        .video_images
        .open_or_grab_thumb(loc, size)
        .await
        .map_err(|e| map_image_error(e, "Thumbnail unavailable"))?;
    image_response(mime, len, reader)
}

pub async fn person_image(
    _: CurrentUser,
    State(state): State<AppState>,
    path: Result<Path<(String, String)>, PathRejection>,
    params: Result<Query<ImageQuery>, QueryRejection>,
) -> Result<Response, ApiError> {
    let (raw_lib, key) = path?.0;
    let library = path_id(&raw_lib, "Library")?;
    if !is_hex64(&key) {
        return Err(ApiError::not_found("Person image not found"));
    }
    let size = parse_size(params?.0.size.as_deref())?;
    let videos = state.db.video();
    if !videos.has_library(library).await? {
        return Err(ApiError::not_found("Video library not found"));
    }
    let loc = videos
        .person_image_location(library, &key)
        .await?
        .ok_or_else(|| ApiError::not_found("Person image not found"))?;
    let (mime, len, reader) = state
        .video_images
        .open_image(loc, size)
        .await
        .map_err(|e| map_image_error(e, "Person image unavailable"))?;
    image_response(mime, len, reader)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{http::StatusCode, response::IntoResponse};

    #[test]
    fn work_filters_validate_search_bounds_and_levels() {
        for query in [
            "kind=episode".to_owned(),
            "q=title&series=abc".to_owned(),
            "kind=movie&series=abc".to_owned(),
            "genre=Drama&series=abc".to_owned(),
            "yearFrom=999".to_owned(),
            "yearTo=10000".to_owned(),
            "startsWith=AB".to_owned(),
            "startsWith=1".to_owned(),
            "startsWith=A&series=abc".to_owned(),
            "played=true&series=abc".to_owned(),
            "favorite=true&series=abc".to_owned(),
            "dir=desc&series=abc".to_owned(),
            "q=A%00B".to_owned(),
            format!("q={}", "a".repeat(129)),
            format!("genre={}", "g".repeat(129)),
            format!("seed={}", "s".repeat(65)),
        ] {
            let uri = format!("/videos?{query}").parse().unwrap();
            let result = Query::<Browse>::try_from_uri(&uri)
                .map_err(ApiError::from)
                .and_then(|q| q.filters().map(|_| ()));
            assert_eq!(
                result.unwrap_err().into_response().status(),
                StatusCode::BAD_REQUEST,
                "expected 400 for {query}"
            );
        }
        let q = Query::<Browse>::try_from_uri(
            &"/videos?q=%20Sea%20&kind=movie&genre=Drama&yearFrom=2000&yearTo=2024&played=false&favorite=true&startsWith=s&order=rating&dir=desc"
                .parse()
                .unwrap(),
        )
        .unwrap();
        let filters = q.filters().unwrap();
        assert_eq!(filters.title, "Sea");
        assert_eq!(filters.kind, Some(WorkKind::Movie));
        assert_eq!(filters.genre, Some("Drama"));
        assert_eq!(filters.year_from, Some(2000));
        assert_eq!(filters.year_to, Some(2024));
        assert_eq!(filters.played, Some(false));
        assert_eq!(filters.favorite, Some(true));
        assert_eq!(filters.starts_with, Some(StartsWith::Letter('s')));
        assert_eq!(filters.order, VideoOrder::Rating);
        assert_eq!(filters.dir, Some(SortDir::Desc));

        let hash =
            Query::<Browse>::try_from_uri(&"/videos?startsWith=%23".parse().unwrap()).unwrap();
        assert_eq!(hash.filters().unwrap().starts_with, Some(StartsWith::Other));
    }

    #[test]
    fn video_order_defaults_to_title_and_rejects_invalid_levels() {
        fn order(query: &str) -> Result<VideoOrder, ApiError> {
            let uri = format!("/videos?{query}").parse().unwrap();
            Query::<Browse>::try_from_uri(&uri)?.order()
        }
        for query in [
            "",
            "order=title",
            "series=abc&season=1",
            "series=abc&order=title",
        ] {
            assert_eq!(order(query).unwrap(), VideoOrder::Title);
        }
        for (query, expected) in [
            ("order=poster", VideoOrder::Poster),
            ("order=year", VideoOrder::Year),
            ("order=added", VideoOrder::Added),
            ("order=rating", VideoOrder::Rating),
            ("order=runtime", VideoOrder::Runtime),
            ("order=random", VideoOrder::Random),
        ] {
            assert_eq!(order(query).unwrap(), expected);
        }
        for query in [
            "order=unknown",
            "order=poster&series=abc",
            "order=year&series=abc",
            "order=poster&series=abc&season=1",
        ] {
            assert_eq!(
                order(query).unwrap_err().into_response().status(),
                StatusCode::BAD_REQUEST
            );
        }
    }

    #[test]
    fn image_size_bounds_validate() {
        assert_eq!(parse_size(None).unwrap(), None);
        assert_eq!(parse_size(Some("")).unwrap(), None);
        assert_eq!(parse_size(Some("480")).unwrap(), Some(480));
        assert_eq!(parse_size(Some("1920")).unwrap(), Some(1920));
        for bad in ["0", "-1", "5000", "abc"] {
            assert_eq!(
                parse_size(Some(bad)).unwrap_err().into_response().status(),
                StatusCode::BAD_REQUEST
            );
        }
    }
}

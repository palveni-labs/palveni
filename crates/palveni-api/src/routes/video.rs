use axum::{
    Json,
    body::Body,
    extract::{
        Path, Query, State,
        rejection::{PathRejection, QueryRejection},
    },
    http::header,
    response::Response,
};
use palveni_core::video::{VideoOrder, VideoPage, WorkKind};
use palveni_db::video::{PosterLevel, VideoCursor, VideoFilters};
use serde::Deserialize;
use tokio_util::io::ReaderStream;

use super::{DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE, parse_limit, path_id};
use crate::{AppState, auth::CurrentUser, error::ApiError};

#[derive(Deserialize)]
pub struct Browse {
    series: Option<String>,
    season: Option<i32>,
    cursor: Option<String>,
    limit: Option<String>,
    q: Option<String>,
    kind: Option<WorkKind>,
    #[serde(default)]
    order: VideoOrder,
}

impl Browse {
    fn filters(&self) -> Result<VideoFilters<'_>, ApiError> {
        let title = self.q.as_deref().unwrap_or_default().trim();
        if title.chars().count() > 128 || title.chars().any(char::is_control) {
            return Err(ApiError::validation("Invalid search title").field("q"));
        }
        if self.series.is_some() || self.season.is_some() {
            if !title.is_empty() {
                return Err(ApiError::validation("Title search applies only to works").field("q"));
            }
            if self.kind.is_some() {
                return Err(ApiError::validation("Kind filter applies only to works").field("kind"));
            }
        }
        Ok(VideoFilters {
            order: self.order()?,
            title,
            kind: self.kind,
        })
    }

    fn order(&self) -> Result<VideoOrder, ApiError> {
        if self.order == VideoOrder::Poster && (self.series.is_some() || self.season.is_some()) {
            return Err(
                ApiError::validation("Poster ordering applies only to works").field("order"),
            );
        }
        Ok(self.order)
    }
}

pub async fn list(
    _: CurrentUser,
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
    params: Result<Query<Browse>, QueryRejection>,
) -> Result<Json<VideoPage>, ApiError> {
    let library = path_id(&path?.0, "Library")?;
    let query = params?.0;
    let series = query.series.as_deref();
    if series.is_some_and(|s| {
        s.len() != 64
            || !s
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    }) {
        return Err(ApiError::validation("Invalid series").field("series"));
    }
    if query
        .season
        .is_some_and(|n| !(0..=65_535).contains(&n) || series.is_none())
    {
        return Err(ApiError::validation("A season requires its series").field("season"));
    }
    let filters = query.filters()?;
    let limit = parse_limit(query.limit.as_deref(), DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE)?;
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
            .browse_filtered(library, series, query.season, filters, limit, cursor)
            .await?,
    ))
}

#[derive(Deserialize)]
pub struct Poster {
    level: Option<String>,
}

pub async fn poster(
    _: CurrentUser,
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
    params: Result<Query<Poster>, QueryRejection>,
) -> Result<Response, ApiError> {
    let id = path_id(&path?.0, "Video")?;
    let level = match params?.level.as_deref().unwrap_or("work") {
        "work" => PosterLevel::Work,
        "season" => PosterLevel::Season,
        "episode" => PosterLevel::Episode,
        _ => return Err(ApiError::validation("Unknown poster level").field("level")),
    };
    let loc = state
        .db
        .video()
        .poster_location(id, level)
        .await?
        .ok_or_else(|| ApiError::not_found("Poster not found"))?;
    let (mime, size, reader) =
        palveni_library::video::open_poster(loc)
            .await
            .map_err(|e| match e {
                palveni_core::Error::Internal(_) => ApiError::internal(e),
                _ => ApiError::not_found("Poster unavailable"),
            })?;
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
            "q=A%00B".to_owned(),
            format!("q={}", "a".repeat(129)),
        ] {
            let uri = format!("/videos?{query}").parse().unwrap();
            let result = Query::<Browse>::try_from_uri(&uri)
                .map_err(ApiError::from)
                .and_then(|q| q.filters().map(|_| ()));
            assert_eq!(
                result.unwrap_err().into_response().status(),
                StatusCode::BAD_REQUEST
            );
        }
        let q = Query::<Browse>::try_from_uri(&"/videos?q=%20Sea%20&kind=movie".parse().unwrap())
            .unwrap();
        let filters = q.filters().unwrap();
        assert_eq!(filters.title, "Sea");
        assert_eq!(filters.kind, Some(WorkKind::Movie));
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
        assert_eq!(order("order=poster").unwrap(), VideoOrder::Poster);
        for query in [
            "order=unknown",
            "order=poster&series=abc",
            "order=poster&series=abc&season=1",
        ] {
            assert_eq!(
                order(query).unwrap_err().into_response().status(),
                StatusCode::BAD_REQUEST
            );
        }
    }
}

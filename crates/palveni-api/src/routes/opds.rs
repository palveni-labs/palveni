//! OPDS 1.2 navigation and acquisition feeds for local book libraries.

mod auth;
pub use auth::{OpdsAccess, access};

use std::fmt::{Display, Write};

use axum::{
    Router,
    extract::{OriginalUri, Path, Query, State},
    http::{HeaderMap, Method, header},
    response::{IntoResponse, Response},
    routing::get,
};
use chrono::{DateTime, Utc};
use palveni_db::items::ItemCursor;
use serde::Deserialize;

use super::xml::escape;
use super::{DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE, books, parse_limit, path_id};
use crate::{ApiError, AppState};
use auth::Reader;

const NAVIGATION: &str = "application/atom+xml;profile=opds-catalog;kind=navigation";
const ACQUISITION: &str = "application/atom+xml;profile=opds-catalog;kind=acquisition";

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/opds", get(root))
        .route("/opds/", get(root))
        .route("/opds/libraries/{id}", get(library))
        .route("/opds/books/{id}", get(download))
}

fn metadata(body: &mut String, id: impl Display, title: &str, updated: DateTime<Utc>) {
    let _ = write!(
        body,
        "<id>urn:uuid:{id}</id><title>{}</title><updated>{}</updated>",
        escape(title),
        updated.to_rfc3339()
    );
}

fn link(body: &mut String, rel: &str, href: &str, mime: &str) {
    let _ = write!(
        body,
        r#"<link rel="{rel}" href="{}" type="{mime}"/>"#,
        escape(href)
    );
}

fn feed(
    id: impl Display,
    title: &str,
    updated: DateTime<Utc>,
    mime: &str,
    start: &str,
    current: &str,
) -> String {
    let mut body = String::from(
        r#"<?xml version="1.0" encoding="UTF-8"?><feed xmlns="http://www.w3.org/2005/Atom">"#,
    );
    metadata(&mut body, id, title, updated);
    body.push_str("<author><name>Palveni</name></author>");
    link(&mut body, "start", start, NAVIGATION);
    link(&mut body, "self", current, mime);
    body
}

fn finish(mut body: String, mime: &'static str) -> Response {
    body.push_str("</feed>");
    (
        [
            (header::CONTENT_TYPE, mime),
            (header::CACHE_CONTROL, "private, no-store"),
            (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
        ],
        body,
    )
        .into_response()
}

async fn root(
    _: Reader,
    State(state): State<AppState>,
    OriginalUri(uri): OriginalUri,
) -> Result<Response, ApiError> {
    let shelves = state.db.libraries().book_shelves(None).await?;
    let updated = shelves
        .iter()
        .map(|s| s.updated_at)
        .max()
        .unwrap_or(DateTime::UNIX_EPOCH);
    // Relative references also work behind a reverse proxy mounted under a path prefix.
    let base = if uri.path().ends_with('/') {
        "./"
    } else {
        "opds/"
    };
    let title = state
        .settings
        .read()
        .await
        .server_name
        .clone()
        .unwrap_or_else(|| state.server.name.clone());
    let mut body = feed(state.server.id, &title, updated, NAVIGATION, base, base);
    for shelf in shelves {
        body.push_str("<entry>");
        metadata(&mut body, shelf.id, &shelf.name, shelf.updated_at);
        body.push_str("<content type=\"text\"/>");
        link(
            &mut body,
            "subsection",
            &format!("{base}libraries/{}", shelf.id),
            ACQUISITION,
        );
        body.push_str("</entry>");
    }
    Ok(finish(body, NAVIGATION))
}

#[derive(Deserialize)]
struct Page {
    limit: Option<String>,
    cursor: Option<String>,
}

async fn library(
    _: Reader,
    State(state): State<AppState>,
    Path(raw): Path<String>,
    Query(query): Query<Page>,
) -> Result<Response, ApiError> {
    let id = path_id(&raw, "Library")?;
    let limit = parse_limit(query.limit.as_deref(), DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE)?;
    let cursor = query
        .cursor
        .as_deref()
        .map(|value| {
            ItemCursor::decode(value)
                .ok_or_else(|| ApiError::validation("Invalid cursor").field("cursor"))
        })
        .transpose()?;
    let shelf = state
        .db
        .libraries()
        .book_shelves(Some(id))
        .await?
        .pop()
        .ok_or_else(|| ApiError::not_found("Book library not found"))?;
    let mut current = format!("{id}?limit={limit}");
    if let Some(cursor) = &cursor {
        let _ = write!(current, "&cursor={}", cursor.encode());
    }
    let page = state.db.items().book_catalog(id, limit, cursor).await?;
    let mut body = feed(
        id,
        &shelf.name,
        shelf.updated_at,
        ACQUISITION,
        "../",
        &current,
    );
    if let Some(cursor) = page.next_cursor {
        link(
            &mut body,
            "next",
            &format!("{id}?limit={limit}&cursor={}", cursor.encode()),
            ACQUISITION,
        );
    }
    for book in page.items {
        let href = format!("../books/{}", book.id);
        let mime = books::media_type(book.path.as_deref().unwrap_or_default());
        body.push_str("<entry>");
        metadata(&mut body, book.id, &book.title, book.updated_at);
        link(&mut body, "http://opds-spec.org/acquisition", &href, mime);
        body.push_str("<content type=\"text\"/>");
        body.push_str("</entry>");
    }
    Ok(finish(body, ACQUISITION))
}

async fn download(
    _: Reader,
    State(state): State<AppState>,
    Path(raw): Path<String>,
    method: Method,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    books::serve_book(&state, path_id(&raw, "Book")?, method, &headers, true).await
}

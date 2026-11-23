//! OPDS 1.2 navigation and acquisition feeds for local book and comic libraries.

mod auth;
pub use auth::{OpdsAccess, access};

use std::fmt::{Display, Write};

use axum::{
    Router,
    extract::{OriginalUri, Path, Query, State},
    http::{HeaderMap, HeaderValue, Method, header},
    response::{IntoResponse, Response},
    routing::get,
};
use chrono::{DateTime, Utc};
use palveni_core::LibraryKind;
use palveni_db::items::ItemCursor;
use palveni_library::reading::{ReadingError, open_reading_file, read_cover, reading_media_type};
use serde::Deserialize;

use super::xml::escape;
use super::{DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE, books, parse_limit, path_id, playback, reading};
use crate::{ApiError, AppState};
use auth::Reader;

const NAVIGATION: &str = "application/atom+xml;profile=opds-catalog;kind=navigation";
const ACQUISITION: &str = "application/atom+xml;profile=opds-catalog;kind=acquisition";

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/opds", get(root))
        .route("/opds/", get(root))
        .route("/opds/libraries/{id}", get(library))
        .route("/opds/covers/{id}", get(cover))
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
        r#"<?xml version="1.0" encoding="UTF-8"?><feed xmlns="http://www.w3.org/2005/Atom" xmlns:dcterms="http://purl.org/dc/terms/">"#,
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
    let shelves = state.db.reading().opds_shelves(None).await?;
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
        let kind_label = match shelf.kind {
            LibraryKind::Comic => "Comics",
            _ => "Books",
        };
        body.push_str("<entry>");
        metadata(&mut body, shelf.id, &shelf.name, shelf.updated_at);
        let _ = write!(
            body,
            r#"<summary type="text">{}</summary>"#,
            escape(kind_label)
        );
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
        .reading()
        .opds_shelves(Some(id))
        .await?
        .pop()
        .ok_or_else(|| ApiError::not_found("Reading library not found"))?;
    let mut current = format!("{id}?limit={limit}");
    if let Some(cursor) = &cursor {
        let _ = write!(current, "&cursor={}", cursor.encode());
    }
    let page = state.db.reading().opds_catalog(id, limit, cursor).await?;
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
    for entry in page.items {
        let href = format!("../books/{}", entry.id);
        let name_or_fmt = entry.path.as_deref().unwrap_or(&entry.format);
        let mime = reading_media_type(name_or_fmt, entry.kind);
        body.push_str("<entry>");
        metadata(&mut body, entry.id, &entry.title, entry.updated_at);
        for author in &entry.authors {
            let _ = write!(body, "<author><name>{}</name></author>", escape(author));
        }
        let mut summary_parts = Vec::new();
        if let Some(ref s) = entry.series_name {
            if let Some(idx) = entry.series_index {
                summary_parts.push(format!("{s} #{idx}"));
            } else {
                summary_parts.push(s.clone());
            }
        }
        if let Some(pages) = entry.page_count {
            let _ = write!(body, "<dcterms:extent>{pages} pages</dcterms:extent>");
            if entry.kind == palveni_core::reading::ReadingKind::Comic {
                summary_parts.push(format!("{pages} pages"));
            }
        }
        if let Some(ref desc) = entry.summary {
            summary_parts.push(desc.clone());
        }
        if !summary_parts.is_empty() {
            let _ = write!(
                body,
                r#"<summary type="text">{}</summary>"#,
                escape(&summary_parts.join(" · "))
            );
        }
        if entry.has_cover {
            let cover_href = format!("../covers/{}", entry.id);
            let thumb_href = format!("../covers/{}?size=240", entry.id);
            link(
                &mut body,
                "http://opds-spec.org/image",
                &cover_href,
                "image/jpeg",
            );
            link(
                &mut body,
                "http://opds-spec.org/image/thumbnail",
                &thumb_href,
                "image/jpeg",
            );
        }
        link(&mut body, "http://opds-spec.org/acquisition", &href, mime);
        body.push_str("<content type=\"text\"/>");
        body.push_str("</entry>");
    }
    Ok(finish(body, ACQUISITION))
}

#[derive(Deserialize)]
struct CoverQuery {
    size: Option<u16>,
}

async fn cover(
    _: Reader,
    State(state): State<AppState>,
    Path(raw): Path<String>,
    Query(query): Query<CoverQuery>,
) -> Result<Response, ApiError> {
    let id = path_id(&raw, "Reading item")?;
    let size = query.size.map(|s| s.clamp(1, 1920));
    let (mime, data) = read_cover(&state.db, id).await.map_err(|e| match e {
        ReadingError::Internal(detail) => ApiError::internal(detail),
        _ => ApiError::not_found("Cover image not available"),
    })?;
    reading::serve_scaled_image(&state, mime, data, size).await
}

async fn download(
    _: Reader,
    State(state): State<AppState>,
    Path(raw): Path<String>,
    method: Method,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let id = path_id(&raw, "Reading item")?;
    let (file, name, _) = open_reading_file(&state.db, id)
        .await
        .map_err(|e| match e {
            ReadingError::Internal(detail) => ApiError::internal(detail),
            _ => ApiError::not_found("Reading item not found"),
        })?;
    let mut response = playback::serve_original(file, method, &headers).await?;
    response
        .headers_mut()
        .insert(header::CONTENT_DISPOSITION, books::attachment(&name));
    response.headers_mut().insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use palveni_core::reading::ReadingKind;

    #[test]
    fn opds_media_types_cover_books_and_comics() {
        assert_eq!(
            reading_media_type("novel.epub", ReadingKind::Book),
            "application/epub+zip"
        );
        assert_eq!(
            reading_media_type("manga.cbz", ReadingKind::Comic),
            "application/vnd.comicbook+zip"
        );
        assert_eq!(
            reading_media_type("manga.cbt", ReadingKind::Comic),
            "application/vnd.comicbook+tar"
        );
    }
}

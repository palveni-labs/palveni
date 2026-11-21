use std::fmt::Write;

use axum::{
    Json,
    extract::{
        Path, Query, State,
        rejection::{PathRejection, QueryRejection},
    },
    http::{HeaderMap, HeaderValue, Method, header},
    response::{IntoResponse, Response},
};
use chrono::{DateTime, Utc};
use palveni_core::{ItemId, LibraryKind, clock};
use palveni_library::local::{TrackFile, open_book};
use serde::{Deserialize, Serialize};

use super::{path_id, playback};
use crate::{AppState, auth::CurrentUser, error::ApiError};

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BookDownload {
    url: String,
    expires_at: DateTime<Utc>,
}

#[derive(Deserialize)]
pub struct Access {
    ticket: Option<String>,
}

fn scope(id: ItemId) -> String {
    format!("palveni:book-download:v1\n{id}")
}

async fn original(state: &AppState, id: ItemId) -> Result<(TrackFile, String), ApiError> {
    let location = state
        .db
        .items()
        .original_location(id, LibraryKind::Book)
        .await?
        .ok_or_else(|| ApiError::not_found("Book not found"))?;
    open_book(location)
        .await
        .map_err(|_| ApiError::not_found("Book file unavailable"))
}

// The ASCII fallback retains the type for older readers. Modern clients use the UTF-8 name.
fn attachment(name: &str) -> HeaderValue {
    let clean: String = name.chars().map(|ch| {
        let directional = matches!(ch, '\u{061c}' | '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}');
        if ch.is_control() || directional || r#"/\:"<>|?*"#.contains(ch) {
            '_'
        } else {
            ch
        }
    }).collect();
    let mut encoded = String::with_capacity(clean.len());
    for byte in clean.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            encoded.push(char::from(byte));
        } else {
            let _ = write!(encoded, "%{byte:02X}");
        }
    }
    let fallback = if clean
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b" _-().".contains(&b))
    {
        clean
    } else {
        let extension = palveni_library::classify::extension_of(name).unwrap_or_default();
        format!("book.{extension}")
    };
    let value = format!("attachment; filename=\"{fallback}\"; filename*=UTF-8''{encoded}");
    HeaderValue::from_str(&value).unwrap_or_else(|_| HeaderValue::from_static("attachment"))
}

pub async fn prepare(
    current: CurrentUser,
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
) -> Result<Response, ApiError> {
    let id = path_id(&path?.0, "Book")?;
    original(&state, id).await?;
    let (expires_at, ticket) = playback::issue_ticket(
        &state.identity,
        &scope(id),
        current.session_id,
        clock::now(),
    );
    let download = BookDownload {
        url: format!("/api/v1/items/{id}/book-download?ticket={ticket}"),
        expires_at,
    };
    Ok((
        [(header::CACHE_CONTROL, "private, no-store")],
        Json(download),
    )
        .into_response())
}

pub async fn download(
    State(state): State<AppState>,
    method: Method,
    path: Result<Path<String>, PathRejection>,
    query: Result<Query<Access>, QueryRejection>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let raw = path?.0;
    let id = if headers.contains_key(header::AUTHORIZATION) {
        CurrentUser::from_headers(&headers, &state).await?;
        path_id(&raw, "Book")?
    } else {
        let access = query.map_err(|_| ApiError::unauthorized())?.0;
        let ticket = access.ticket.ok_or_else(ApiError::unauthorized)?;
        let id = path_id(&raw, "Book")?;
        let session = playback::verify_ticket(&state.identity, &scope(id), &ticket, clock::now())
            .ok_or_else(ApiError::unauthorized)?;
        state
            .db
            .sessions()
            .active_by_id(session)
            .await?
            .ok_or_else(ApiError::unauthorized)?;
        id
    };
    serve_book(&state, id, method, &headers, false).await
}

pub(super) async fn serve_book(
    state: &AppState,
    id: ItemId,
    method: Method,
    headers: &HeaderMap,
    use_book_media_type: bool,
) -> Result<Response, ApiError> {
    let (mut file, name) = original(state, id).await?;
    if use_book_media_type {
        file.content_type = media_type(&name);
    }
    let mut response = playback::serve_original(file, method, headers).await?;
    response
        .headers_mut()
        .insert(header::CONTENT_DISPOSITION, attachment(&name));
    response.headers_mut().insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    Ok(response)
}

pub(super) fn media_type(name: &str) -> &'static str {
    match palveni_library::classify::extension_of(name).as_deref() {
        Some("epub") => "application/epub+zip",
        Some("pdf") => "application/pdf",
        Some("mobi") => "application/x-mobipocket-ebook",
        Some("azw" | "azw3") => "application/vnd.amazon.ebook",
        Some("djvu") => "image/vnd.djvu",
        Some("fb2") => "application/x-fictionbook+xml",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attachments_preserve_unicode_but_replace_unsafe_filename_characters() {
        let header = attachment("读书 1%\r\n/\\\u{202e}.PDF");
        assert_eq!(
            header,
            "attachment; filename=\"book.pdf\"; filename*=UTF-8''%E8%AF%BB%E4%B9%A6%201%25_____.PDF"
        );
    }
}

//! Photos & Albums HTTP endpoints (`/api/v1/photos/*`, `/api/v1/items/{id}/photo*`,
//! `/api/v1/me/photo-albums*`, `/api/v1/libraries/{id}/photos/upload`).

use std::fmt::Write as _;

use axum::{
    Json,
    body::{Body, Bytes},
    extract::{
        Path, Query, State,
        rejection::{PathRejection, QueryRejection},
    },
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::Response,
};
use palveni_core::photos::{
    CreatePhotoAlbumRequest, PhotoAlbumItemsRequest, PhotoAlbumList, PhotoAlbumSummary, PhotoInfo,
    PhotoMemories, PhotoTimelinePage, PhotoUploadRequest, UpdatePhotoAlbumRequest,
};
use palveni_core::{ItemId, LibraryId, PhotoAlbumId, clock};
use palveni_db::audit::AuditEntry;
use palveni_db::photos::{PhotoCursor, PhotoTimelineQuery, parse_place_key, timeline_filter_hash};
use palveni_library::photos::{MAX_PHOTO_UPLOAD_BYTES, PhotoUploadError, upload_photo};
use serde::Deserialize;
use tokio_util::io::ReaderStream;

use super::{MAX_PAGE_SIZE, audit, parse_limit, path_id, query_id};
use crate::{AppState, auth::CurrentUser, error::ApiError, json::ApiJson};

const DEFAULT_PHOTO_PAGE_SIZE: u32 = 60;
type IdPath = Result<Path<String>, PathRejection>;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineParams {
    library_id: Option<String>,
    album_id: Option<String>,
    favorite: Option<bool>,
    has_location: Option<bool>,
    camera: Option<String>,
    month: Option<String>,
    place: Option<String>,
    q: Option<String>,
    cursor: Option<String>,
    limit: Option<String>,
}

fn validate_month(raw: Option<&str>) -> Result<Option<&str>, ApiError> {
    let Some(m) = raw.map(str::trim).filter(|s| !s.is_empty()) else {
        return Ok(None);
    };
    let valid = m.len() == 7
        && m.as_bytes().get(4) == Some(&b'-')
        && m[0..4]
            .parse::<u16>()
            .is_ok_and(|y| (1800..=2999).contains(&y))
        && m[5..7].parse::<u8>().is_ok_and(|mo| (1..=12).contains(&mo));
    if !valid {
        return Err(ApiError::validation("month must match YYYY-MM").field("month"));
    }
    Ok(Some(m))
}

pub async fn timeline(
    user: CurrentUser,
    State(state): State<AppState>,
    params: Result<Query<TimelineParams>, QueryRejection>,
) -> Result<Json<PhotoTimelinePage>, ApiError> {
    let Query(p) = params?;
    let library_id: Option<LibraryId> = query_id(p.library_id.as_deref(), "libraryId")?;
    let album_id: Option<PhotoAlbumId> = query_id(p.album_id.as_deref(), "albumId")?;
    let month = validate_month(p.month.as_deref())?;

    let camera = p.camera.as_deref().map(str::trim).filter(|s| !s.is_empty());
    if camera.is_some_and(|c| c.chars().count() > 256 || c.chars().any(char::is_control)) {
        return Err(ApiError::validation("Invalid camera filter").field("camera"));
    }

    let q = p.q.as_deref().map(str::trim).filter(|s| !s.is_empty());
    if q.is_some_and(|s| s.chars().count() > 128 || s.chars().any(char::is_control)) {
        return Err(ApiError::validation("Invalid search query").field("q"));
    }

    let place = p
        .place
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| {
            parse_place_key(s)
                .ok_or_else(|| ApiError::validation("Invalid place cluster key").field("place"))
        })
        .transpose()?;

    let limit = parse_limit(p.limit.as_deref(), DEFAULT_PHOTO_PAGE_SIZE, MAX_PAGE_SIZE)?;

    let mut db_query = PhotoTimelineQuery {
        library_id,
        album_id,
        favorite: p.favorite,
        has_location: p.has_location,
        camera,
        month,
        place,
        q,
        cursor: None,
        limit,
    };

    if let Some(raw_cursor) = p.cursor.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        let fhash = timeline_filter_hash(&db_query);
        let decoded = PhotoCursor::decode(raw_cursor, &fhash)
            .ok_or_else(|| ApiError::validation("Invalid photo timeline cursor").field("cursor"))?;
        db_query.cursor = Some(decoded);
    }

    let page = state.db.photos().timeline(user.user.id, &db_query).await?;
    Ok(Json(page))
}

pub async fn memories(
    user: CurrentUser,
    State(state): State<AppState>,
) -> Result<Json<PhotoMemories>, ApiError> {
    let data = state
        .db
        .photos()
        .memories(user.user.id, clock::now())
        .await?;
    Ok(Json(data))
}

pub async fn info(
    user: CurrentUser,
    State(state): State<AppState>,
    path: IdPath,
) -> Result<Json<PhotoInfo>, ApiError> {
    let item_id: ItemId = path_id(&path?.0, "Photo")?;
    let info = state
        .db
        .photos()
        .photo_info(user.user.id, item_id, user.is_admin())
        .await?
        .ok_or_else(|| ApiError::not_found("Photo not found"))?;
    Ok(Json(info))
}

#[derive(Deserialize)]
pub struct PhotoImageQuery {
    size: Option<String>,
    original: Option<bool>,
}

fn parse_photo_size(raw: Option<&str>) -> Result<Option<u16>, ApiError> {
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

fn attachment_header(name: &str) -> HeaderValue {
    let clean: String = name
        .chars()
        .map(|ch| {
            let directional = matches!(
                ch,
                '\u{061c}' | '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}'
            );
            if ch.is_control() || directional || r#"/\:"<>|?*"#.contains(ch) {
                '_'
            } else {
                ch
            }
        })
        .collect();
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
        let ext = palveni_library::classify::extension_of(name).unwrap_or_else(|| "jpg".into());
        format!("photo.{ext}")
    };
    let value = format!("attachment; filename=\"{fallback}\"; filename*=UTF-8''{encoded}");
    HeaderValue::from_str(&value).unwrap_or_else(|_| HeaderValue::from_static("attachment"))
}

pub async fn photo(
    _user: CurrentUser,
    State(state): State<AppState>,
    path: IdPath,
    params: Result<Query<PhotoImageQuery>, QueryRejection>,
) -> Result<Response, ApiError> {
    let item_id: ItemId = path_id(&path?.0, "Photo")?;
    let Query(q) = params?;
    let size = parse_photo_size(q.size.as_deref())?;
    let original = q.original.unwrap_or(false);

    let (loc, file_name) = state
        .db
        .photos()
        .photo_location(item_id)
        .await?
        .ok_or_else(|| ApiError::not_found("Photo not found"))?;

    let (mime, len, reader) = state
        .video_images
        .open_photo(loc, size, original)
        .await
        .map_err(|err| match err {
            palveni_core::Error::Internal(_) => ApiError::internal(err),
            _ => ApiError::not_found("Photo file unavailable"),
        })?;

    let mut builder = Response::builder()
        .header(header::CONTENT_TYPE, mime)
        .header(header::CONTENT_LENGTH, len)
        .header(header::CACHE_CONTROL, "private, no-cache")
        .header(header::X_CONTENT_TYPE_OPTIONS, "nosniff");
    if original {
        builder = builder.header(header::CONTENT_DISPOSITION, attachment_header(&file_name));
    }
    builder
        .body(Body::from_stream(ReaderStream::with_capacity(
            reader,
            32 * 1024,
        )))
        .map_err(ApiError::internal)
}

pub async fn list_albums(
    user: CurrentUser,
    State(state): State<AppState>,
) -> Result<Json<PhotoAlbumList>, ApiError> {
    let albums = state.db.photos().list_albums(user.user.id).await?;
    Ok(Json(PhotoAlbumList { albums }))
}

pub async fn create_album(
    user: CurrentUser,
    State(state): State<AppState>,
    ApiJson(body): ApiJson<CreatePhotoAlbumRequest>,
) -> Result<(StatusCode, Json<PhotoAlbumSummary>), ApiError> {
    let created = state
        .db
        .photos()
        .create_album(
            user.user.id,
            &body.title,
            body.description.as_deref(),
            body.cover_item_id,
        )
        .await?;
    Ok((StatusCode::CREATED, Json(created)))
}

pub async fn get_album(
    user: CurrentUser,
    State(state): State<AppState>,
    path: IdPath,
) -> Result<Json<PhotoAlbumSummary>, ApiError> {
    let album_id: PhotoAlbumId = path_id(&path?.0, "Photo album")?;
    let album = state
        .db
        .photos()
        .get_album(user.user.id, album_id)
        .await?
        .ok_or_else(|| ApiError::not_found("Photo album not found"))?;
    Ok(Json(album))
}

pub async fn update_album(
    user: CurrentUser,
    State(state): State<AppState>,
    path: IdPath,
    ApiJson(body): ApiJson<UpdatePhotoAlbumRequest>,
) -> Result<Json<PhotoAlbumSummary>, ApiError> {
    let album_id: PhotoAlbumId = path_id(&path?.0, "Photo album")?;
    if body.title.is_none() && body.description.is_none() && body.cover_item_id.is_none() {
        return Err(ApiError::validation(
            "Provide title, description or coverItemId to update",
        ));
    }
    let updated = state
        .db
        .photos()
        .update_album(
            user.user.id,
            album_id,
            body.title.as_deref(),
            body.description.as_deref(),
            body.cover_item_id,
        )
        .await?;
    Ok(Json(updated))
}

pub async fn delete_album(
    user: CurrentUser,
    State(state): State<AppState>,
    path: IdPath,
) -> Result<StatusCode, ApiError> {
    let album_id: PhotoAlbumId = path_id(&path?.0, "Photo album")?;
    if !state
        .db
        .photos()
        .delete_album(user.user.id, album_id)
        .await?
    {
        return Err(ApiError::not_found("Photo album not found"));
    }
    Ok(StatusCode::NO_CONTENT)
}

pub async fn add_album_items(
    user: CurrentUser,
    State(state): State<AppState>,
    path: IdPath,
    ApiJson(body): ApiJson<PhotoAlbumItemsRequest>,
) -> Result<Json<PhotoAlbumSummary>, ApiError> {
    let album_id: PhotoAlbumId = path_id(&path?.0, "Photo album")?;
    let updated = state
        .db
        .photos()
        .add_album_items(user.user.id, album_id, &body.item_ids)
        .await?;
    Ok(Json(updated))
}

pub async fn remove_album_items(
    user: CurrentUser,
    State(state): State<AppState>,
    path: IdPath,
    ApiJson(body): ApiJson<PhotoAlbumItemsRequest>,
) -> Result<Json<PhotoAlbumSummary>, ApiError> {
    let album_id: PhotoAlbumId = path_id(&path?.0, "Photo album")?;
    let updated = state
        .db
        .photos()
        .remove_album_items(user.user.id, album_id, &body.item_ids)
        .await?;
    Ok(Json(updated))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadQuery {
    file_name: Option<String>,
}

fn decode_base64_val(byte: u8) -> Option<u8> {
    match byte {
        b'A'..=b'Z' => Some(byte - b'A'),
        b'a'..=b'z' => Some(byte - b'a' + 26),
        b'0'..=b'9' => Some(byte - b'0' + 52),
        b'+' | b'-' => Some(62),
        b'/' | b'_' => Some(63),
        _ => None,
    }
}

fn decode_base64(input: &str) -> Result<Vec<u8>, ApiError> {
    let trimmed = input.trim();
    let payload = match trimmed.split_once(";base64,") {
        Some((prefix, rest)) if prefix.starts_with("data:") => rest,
        _ => trimmed,
    };

    let mut clean = Vec::with_capacity(payload.len());
    for b in payload.bytes() {
        if !b.is_ascii_whitespace() {
            clean.push(b);
        }
    }
    while clean.last() == Some(&b'=') {
        clean.pop();
    }
    if clean.is_empty() || clean.len() % 4 == 1 {
        return Err(ApiError::validation("Invalid base64 image data").field("dataBase64"));
    }

    let est_len = (clean.len() * 3) / 4;
    if est_len > MAX_PHOTO_UPLOAD_BYTES {
        return Err(ApiError::new(
            StatusCode::PAYLOAD_TOO_LARGE,
            "payload_too_large",
            "Photo exceeds maximum upload size",
        ));
    }

    let mut out = Vec::with_capacity(est_len);
    for chunk in clean.chunks(4) {
        let b0 = decode_base64_val(chunk[0])
            .ok_or_else(|| ApiError::validation("Invalid base64 character").field("dataBase64"))?;
        let b1 = decode_base64_val(chunk[1])
            .ok_or_else(|| ApiError::validation("Invalid base64 character").field("dataBase64"))?;
        out.push((b0 << 2) | (b1 >> 4));
        if let Some(&c2) = chunk.get(2) {
            let b2 = decode_base64_val(c2).ok_or_else(|| {
                ApiError::validation("Invalid base64 character").field("dataBase64")
            })?;
            out.push((b1 << 4) | (b2 >> 2));
            if let Some(&c3) = chunk.get(3) {
                let b3 = decode_base64_val(c3).ok_or_else(|| {
                    ApiError::validation("Invalid base64 character").field("dataBase64")
                })?;
                out.push((b2 << 6) | b3);
            }
        }
    }
    Ok(out)
}

pub async fn upload(
    user: CurrentUser,
    State(state): State<AppState>,
    path: IdPath,
    query: Result<Query<UploadQuery>, QueryRejection>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<(StatusCode, Json<PhotoInfo>), ApiError> {
    let library_id: LibraryId = path_id(&path?.0, "Library")?;
    let Query(q) = query?;

    if body.is_empty() {
        return Err(ApiError::validation("Empty upload payload"));
    }

    let content_type = headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_ascii_lowercase();

    let is_json = content_type.starts_with("application/json")
        || (content_type.is_empty() && body.first() == Some(&b'{'));

    let (file_name, bytes) = if is_json {
        let mut deserializer = serde_json::Deserializer::from_slice(&body);
        let parsed: PhotoUploadRequest = serde_path_to_error::deserialize(&mut deserializer)
            .map_err(|err| ApiError::validation(format!("Invalid upload JSON: {err}")))?;
        let decoded = decode_base64(&parsed.data_base64)?;
        (parsed.file_name, decoded)
    } else {
        let header_name = headers
            .get("x-file-name")
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned);
        let fname = q
            .file_name
            .or(header_name)
            .unwrap_or_else(|| "upload".into());
        (fname, body.to_vec())
    };

    let info = upload_photo(
        &state.db,
        &state.bus,
        library_id,
        user.user.id,
        user.is_admin(),
        &file_name,
        bytes,
    )
    .await
    .map_err(|err| match err {
        PhotoUploadError::NotFound(what) => ApiError::not_found(format!("{what} not found")),
        PhotoUploadError::Invalid(msg) => ApiError::validation(msg),
        PhotoUploadError::ReadOnly(msg) => ApiError::conflict(msg),
        PhotoUploadError::TooLarge => ApiError::new(
            StatusCode::PAYLOAD_TOO_LARGE,
            "payload_too_large",
            "Photo exceeds maximum upload size",
        ),
        PhotoUploadError::Db(db_err) => ApiError::from(db_err),
        PhotoUploadError::Internal(detail) => ApiError::internal(detail),
    })?;

    audit(
        &state,
        AuditEntry {
            actor: Some(user.user.id),
            target: Some(info.id.to_string()),
            action: "photos.uploaded",
            detail: Some(serde_json::json!({ "fileName": info.file_name })),
        },
    )
    .await;

    Ok((StatusCode::CREATED, Json(info)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_decoder_accepts_standard_and_data_uri() {
        let raw = b"\xFF\xD8\xFF\xE0hello-world-123";
        let b64 = "data:image/jpeg;base64,/9j/4GhlbGxvLXdvcmxkLTEyMw==";
        let out = decode_base64(b64).unwrap();
        assert_eq!(out, raw);
    }

    #[test]
    fn validates_month_format() {
        assert_eq!(validate_month(Some("2025-11")).unwrap(), Some("2025-11"));
        assert_eq!(validate_month(None).unwrap(), None);
        assert!(validate_month(Some("2025-13")).is_err());
        assert!(validate_month(Some("2025/11")).is_err());
    }
}

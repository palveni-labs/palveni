//! Read-only RFC 4918 WebDAV server mounted at `/webdav` and `/webdav/{*path}`.
//!
//! Supports `OPTIONS`, `PROPFIND` (Depth 0 / 1), `GET`, and `HEAD` with HTTP Basic Auth
//! verified against Palveni user accounts (Argon2) and per-library ACL enforcement.
//! All mutating methods return `405 Method Not Allowed`.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path as FsPath, PathBuf};

use axum::Router;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, HeaderValue, Method, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::any;
use chrono::{DateTime, Utc};
use palveni_core::observability::ServerLogLevel;
use palveni_core::storage::{StoragePath, StorageProvider};
use palveni_core::{Item, Library, User};
use palveni_library::local::{LocalStorage, TrackFile};

use super::observability::record_log;
use super::playback;
use super::xml::escape;
use crate::auth::{dummy_hash, verify_password};
use crate::error::ApiError;
use crate::state::AppState;

const ALLOW_HEADER: &str = "OPTIONS, PROPFIND, GET, HEAD";
const DAV_REALM: &str = "Basic realm=\"Palveni WebDAV\", charset=\"UTF-8\"";

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/webdav", any(handle_root))
        .route("/webdav/", any(handle_root))
        .route("/webdav/{*path}", any(handle_path))
}

async fn handle_root(
    State(state): State<AppState>,
    method: Method,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    dispatch(state, method, headers, None).await
}

async fn handle_path(
    State(state): State<AppState>,
    method: Method,
    headers: HeaderMap,
    Path(raw_path): Path<String>,
) -> Result<Response, ApiError> {
    dispatch(state, method, headers, Some(raw_path)).await
}

async fn dispatch(
    state: AppState,
    method: Method,
    headers: HeaderMap,
    raw_path: Option<String>,
) -> Result<Response, ApiError> {
    if method == Method::OPTIONS {
        return Ok(options_response());
    }

    let Some(user) = authenticate_basic(&state, &headers).await? else {
        record_log(
            ServerLogLevel::Warn,
            "webdav.auth",
            "Rejected unauthenticated or invalid WebDAV Basic Auth request",
        );
        return Ok(unauthorized_response());
    };

    let method_str = method.as_str();
    match method_str {
        "PROPFIND" => {
            let depth = match parse_depth(&headers) {
                Ok(d) => d,
                Err(err) => return Ok(err.into_response()),
            };
            handle_propfind(&state, &user, raw_path.as_deref(), depth).await
        }
        "GET" | "HEAD" => {
            handle_get_head(&state, &user, raw_path.as_deref(), method, &headers).await
        }
        "PUT" | "DELETE" | "MKCOL" | "MOVE" | "COPY" | "PROPPATCH" | "LOCK" | "UNLOCK" | "POST"
        | "PATCH" => Ok(method_not_allowed_response()),
        _ => Ok(method_not_allowed_response()),
    }
}

fn options_response() -> Response {
    (
        StatusCode::OK,
        [
            (header::ALLOW, ALLOW_HEADER),
            (header::HeaderName::from_static("dav"), "1"),
            (header::HeaderName::from_static("ms-author-via"), "DAV"),
            (header::CACHE_CONTROL, "private, no-store"),
        ],
    )
        .into_response()
}

fn unauthorized_response() -> Response {
    (
        StatusCode::UNAUTHORIZED,
        [
            (header::WWW_AUTHENTICATE, DAV_REALM),
            (header::CACHE_CONTROL, "private, no-store"),
        ],
    )
        .into_response()
}

fn method_not_allowed_response() -> Response {
    (
        StatusCode::METHOD_NOT_ALLOWED,
        [
            (header::ALLOW, ALLOW_HEADER),
            (header::CACHE_CONTROL, "private, no-store"),
        ],
    )
        .into_response()
}

fn parse_depth(headers: &HeaderMap) -> Result<u8, (StatusCode, &'static str)> {
    let Some(raw) = headers.get("depth").and_then(|v| v.to_str().ok()) else {
        return Ok(1);
    };
    match raw.trim() {
        "0" => Ok(0),
        "1" => Ok(1),
        _ => Err((
            StatusCode::FORBIDDEN,
            "Only Depth 0 and Depth 1 are supported",
        )),
    }
}

async fn authenticate_basic(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<Option<User>, ApiError> {
    let Some((username, password)) = parse_basic_auth(headers) else {
        return Ok(None);
    };
    let creds = state.db.users().find_credentials(&username).await?;
    let Some(creds) = creds else {
        if let Some(dummy) = dummy_hash().await {
            let _ = verify_password(password, dummy).await;
        }
        return Ok(None);
    };
    if creds.disabled {
        return Ok(None);
    }
    let valid = verify_password(password, creds.password_hash).await?;
    Ok(valid.then_some(creds.user))
}

fn parse_basic_auth(headers: &HeaderMap) -> Option<(String, String)> {
    let value = headers.get(header::AUTHORIZATION)?.to_str().ok()?;
    let (scheme, encoded) = value.trim().split_once(' ')?;
    if !scheme.eq_ignore_ascii_case("basic") {
        return None;
    }
    let bytes = decode_base64(encoded.trim())?;
    let text = String::from_utf8(bytes).ok()?;
    let (user, pass) = text.split_once(':')?;
    if user.is_empty() {
        return None;
    }
    Some((user.to_owned(), pass.to_owned()))
}

/// Pure safe-Rust standard RFC 4648 Base64 decoder (zero `unsafe`, zero extra crates).
pub(crate) fn decode_base64(input: &str) -> Option<Vec<u8>> {
    let bytes = input.as_bytes();
    if bytes.is_empty() || !bytes.len().is_multiple_of(4) {
        return None;
    }
    fn val(b: u8) -> Option<u8> {
        match b {
            b'A'..=b'Z' => Some(b - b'A'),
            b'a'..=b'z' => Some(b - b'a' + 26),
            b'0'..=b'9' => Some(b - b'0' + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    }
    let mut out = Vec::with_capacity((bytes.len() / 4) * 3);
    let (chunks, rem) = bytes.as_chunks::<4>();
    if !rem.is_empty() {
        return None;
    }
    let total_chunks = chunks.len();
    for (idx, chunk) in chunks.iter().enumerate() {
        let is_last = idx + 1 == total_chunks;
        let a = val(chunk[0])?;
        let b = val(chunk[1])?;
        match (chunk[2], chunk[3]) {
            (b'=', b'=') if is_last => {
                out.push((a << 2) | (b >> 4));
            }
            (c_byte, b'=') if is_last => {
                let c = val(c_byte)?;
                out.push((a << 2) | (b >> 4));
                out.push((b << 4) | (c >> 2));
            }
            (c_byte, d_byte) => {
                let c = val(c_byte)?;
                let d = val(d_byte)?;
                out.push((a << 2) | (b >> 4));
                out.push((b << 4) | (c >> 2));
                out.push((c << 6) | d);
            }
        }
    }
    Some(out)
}

/// Parses and validates WebDAV virtual path segments, blocking traversal attempts.
fn parse_safe_segments(raw_path: Option<&str>) -> Result<Vec<String>, (StatusCode, &'static str)> {
    let Some(raw) = raw_path else {
        return Ok(Vec::new());
    };
    if raw.contains('\\') || raw.contains('\0') {
        return Err((StatusCode::FORBIDDEN, "Invalid WebDAV path"));
    }
    let mut segments = Vec::new();
    for part in raw.split('/') {
        if part.is_empty() {
            continue;
        }
        let decoded =
            percent_decode(part).ok_or((StatusCode::BAD_REQUEST, "Invalid percent encoding"))?;
        if decoded.is_empty()
            || decoded == "."
            || decoded == ".."
            || decoded.contains('/')
            || decoded.contains('\\')
            || decoded.contains('\0')
        {
            return Err((StatusCode::FORBIDDEN, "Path traversal is forbidden"));
        }
        segments.push(decoded);
    }
    Ok(segments)
}

fn percent_decode(input: &str) -> Option<String> {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            if i + 2 >= bytes.len() {
                return None;
            }
            let hi = hex_nibble(bytes[i + 1])?;
            let lo = hex_nibble(bytes[i + 2])?;
            out.push((hi << 4) | lo);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

fn hex_nibble(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

fn encode_uri_segment(segment: &str) -> String {
    let mut out = String::with_capacity(segment.len());
    for byte in segment.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            out.push(char::from(byte));
        } else {
            let _ = write!(out, "%{byte:02X}");
        }
    }
    out
}

fn build_href(segments: &[String], is_collection: bool) -> String {
    let mut href = String::from("/webdav/");
    for (idx, seg) in segments.iter().enumerate() {
        if idx > 0 {
            href.push('/');
        }
        href.push_str(&encode_uri_segment(seg));
    }
    if is_collection && !href.ends_with('/') {
        href.push('/');
    }
    href
}

#[derive(Clone, Debug)]
struct VirtualFile {
    name: String,
    root: PathBuf,
    relative_components: Vec<String>,
    size_bytes: u64,
    modified_at: DateTime<Utc>,
    content_type: &'static str,
}

#[derive(Default)]
struct VirtualDir {
    dirs: BTreeMap<String, VirtualDir>,
    files: BTreeMap<String, VirtualFile>,
    modified_at: Option<DateTime<Utc>>,
}

impl VirtualDir {
    fn insert_dir(&mut self, parts: &[String], modified: DateTime<Utc>) {
        let mut cursor = self;
        for part in parts {
            cursor = cursor.dirs.entry(part.clone()).or_default();
            cursor.modified_at = Some(
                cursor
                    .modified_at
                    .map_or(modified, |prev| prev.max(modified)),
            );
        }
    }

    fn insert_file(&mut self, parts: &[String], file: VirtualFile) {
        if parts.is_empty() {
            return;
        }
        let mut cursor = self;
        for dir in &parts[..parts.len() - 1] {
            cursor = cursor.dirs.entry(dir.clone()).or_default();
            cursor.modified_at = Some(
                cursor
                    .modified_at
                    .map_or(file.modified_at, |prev| prev.max(file.modified_at)),
            );
        }
        if let Some(name) = parts.last() {
            cursor.files.insert(name.clone(), file);
        }
    }

    fn lookup_dir(&self, parts: &[String]) -> Option<&VirtualDir> {
        let mut cursor = self;
        for part in parts {
            cursor = cursor.dirs.get(part)?;
        }
        Some(cursor)
    }

    fn lookup_file(&self, parts: &[String]) -> Option<&VirtualFile> {
        if parts.is_empty() {
            return None;
        }
        let parent = self.lookup_dir(&parts[..parts.len() - 1])?;
        parent.files.get(parts.last()?)
    }
}

fn build_library_tree(library: &Library, items: &[Item]) -> VirtualDir {
    let mut tree = VirtualDir {
        modified_at: Some(library.updated_at),
        ..Default::default()
    };
    let multi_root = library.paths.len() > 1;
    let root_labels: Vec<(PathBuf, String)> = library
        .paths
        .iter()
        .enumerate()
        .map(|(idx, p)| {
            let path = PathBuf::from(p);
            let label = path
                .file_name()
                .and_then(|n| n.to_str())
                .filter(|s| !s.is_empty())
                .map_or_else(|| format!("root-{idx}"), ToOwned::to_owned);
            (path, label)
        })
        .collect();

    for item in items {
        let Some(ref abs_path) = item.path else {
            continue;
        };
        let item_path = FsPath::new(abs_path);
        let matched = root_labels.iter().find_map(|(root, label)| {
            let rel = item_path.strip_prefix(root).ok()?;
            let mut rel_parts = Vec::new();
            for comp in rel.components() {
                if let std::path::Component::Normal(os) = comp {
                    let s = os.to_str()?;
                    rel_parts.push(s.to_owned());
                } else {
                    return None;
                }
            }
            if rel_parts.is_empty() {
                return None;
            }
            Some((root.clone(), label.clone(), rel_parts))
        });

        let Some((root, root_label, rel_parts)) = matched else {
            continue;
        };

        let mut virtual_parts = Vec::with_capacity(rel_parts.len() + usize::from(multi_root));
        if multi_root {
            virtual_parts.push(root_label);
        }
        virtual_parts.extend(rel_parts.iter().cloned());

        let modified = item.modified_at.unwrap_or(item.updated_at);
        if item.kind.is_folder() {
            tree.insert_dir(&virtual_parts, modified);
        } else {
            let name = rel_parts
                .last()
                .cloned()
                .unwrap_or_else(|| item.title.clone());
            let size_bytes = item
                .size_bytes
                .and_then(|s| u64::try_from(s).ok())
                .unwrap_or(0);
            let content_type = content_type_for_name(&name);
            tree.insert_file(
                &virtual_parts,
                VirtualFile {
                    name,
                    root,
                    relative_components: rel_parts,
                    size_bytes,
                    modified_at: modified,
                    content_type,
                },
            );
        }
    }

    tree
}

async fn accessible_libraries(state: &AppState, user: &User) -> Result<Vec<Library>, ApiError> {
    let all = state.db.libraries().list().await?;
    Ok(state
        .db
        .accounts()
        .filter_libraries(user.id, user.role, all)
        .await?)
}

fn resolve_library<'a>(libs: &'a [Library], segment: &str) -> Option<&'a Library> {
    libs.iter()
        .find(|lib| lib.id.to_string() == segment || lib.name == segment)
}

async fn handle_propfind(
    state: &AppState,
    user: &User,
    raw_path: Option<&str>,
    depth: u8,
) -> Result<Response, ApiError> {
    let segments = match parse_safe_segments(raw_path) {
        Ok(s) => s,
        Err(err) => return Ok(err.into_response()),
    };
    let libs = accessible_libraries(state, user).await?;

    let mut entries_xml =
        String::from(r#"<?xml version="1.0" encoding="UTF-8"?><d:multistatus xmlns:d="DAV:">"#);

    if segments.is_empty() {
        let root_modified = libs
            .iter()
            .map(|l| l.updated_at)
            .max()
            .unwrap_or(DateTime::UNIX_EPOCH);
        push_collection_xml(
            &mut entries_xml,
            "/webdav/",
            "Palveni WebDAV",
            root_modified,
        );
        if depth >= 1 {
            for lib in &libs {
                let lib_segs = vec![lib.name.clone()];
                let href = build_href(&lib_segs, true);
                push_collection_xml(&mut entries_xml, &href, &lib.name, lib.updated_at);
            }
        }
        entries_xml.push_str("</d:multistatus>");
        return Ok(multistatus_response(entries_xml));
    }

    let Some(lib) = resolve_library(&libs, &segments[0]) else {
        return Ok((StatusCode::NOT_FOUND, "Resource not found").into_response());
    };
    let items = state.db.items().list_webdav_items(lib.id).await?;
    let tree = build_library_tree(lib, &items);
    let sub_parts = &segments[1..];
    let lib_segment = segments[0].clone();

    if let Some(dir) = tree.lookup_dir(sub_parts) {
        let display = sub_parts
            .last()
            .map(String::as_str)
            .unwrap_or(lib.name.as_str());
        let href = build_href(&segments, true);
        push_collection_xml(
            &mut entries_xml,
            &href,
            display,
            dir.modified_at.unwrap_or(lib.updated_at),
        );
        if depth >= 1 {
            for (dir_name, child_dir) in &dir.dirs {
                let mut child_segs = Vec::with_capacity(segments.len() + 1);
                child_segs.push(lib_segment.clone());
                child_segs.extend_from_slice(sub_parts);
                child_segs.push(dir_name.clone());
                let child_href = build_href(&child_segs, true);
                push_collection_xml(
                    &mut entries_xml,
                    &child_href,
                    dir_name,
                    child_dir.modified_at.unwrap_or(lib.updated_at),
                );
            }
            for (file_name, file) in &dir.files {
                let mut child_segs = Vec::with_capacity(segments.len() + 1);
                child_segs.push(lib_segment.clone());
                child_segs.extend_from_slice(sub_parts);
                child_segs.push(file_name.clone());
                let child_href = build_href(&child_segs, false);
                push_file_xml(&mut entries_xml, &child_href, file);
            }
        }
        entries_xml.push_str("</d:multistatus>");
        return Ok(multistatus_response(entries_xml));
    }

    if let Some(file) = tree.lookup_file(sub_parts) {
        let href = build_href(&segments, false);
        push_file_xml(&mut entries_xml, &href, file);
        entries_xml.push_str("</d:multistatus>");
        return Ok(multistatus_response(entries_xml));
    }

    Ok((StatusCode::NOT_FOUND, "Resource not found").into_response())
}

async fn handle_get_head(
    state: &AppState,
    user: &User,
    raw_path: Option<&str>,
    method: Method,
    headers: &HeaderMap,
) -> Result<Response, ApiError> {
    let segments = match parse_safe_segments(raw_path) {
        Ok(s) => s,
        Err(err) => return Ok(err.into_response()),
    };
    if segments.len() < 2 {
        return Ok((
            StatusCode::METHOD_NOT_ALLOWED,
            "Cannot GET a WebDAV collection",
        )
            .into_response());
    }

    let libs = accessible_libraries(state, user).await?;
    let Some(lib) = resolve_library(&libs, &segments[0]) else {
        return Ok((StatusCode::NOT_FOUND, "Resource not found").into_response());
    };
    let items = state.db.items().list_webdav_items(lib.id).await?;
    let tree = build_library_tree(lib, &items);
    let sub_parts = &segments[1..];

    let Some(file) = tree.lookup_file(sub_parts) else {
        if tree.lookup_dir(sub_parts).is_some() {
            return Ok((
                StatusCode::METHOD_NOT_ALLOWED,
                "Cannot GET a WebDAV collection",
            )
                .into_response());
        }
        return Ok((StatusCode::NOT_FOUND, "Resource not found").into_response());
    };

    let storage = LocalStorage::open(&file.root)
        .await
        .map_err(|_| ApiError::not_found("Library storage is unavailable"))?;
    let mut storage_path = StoragePath::root();
    for component in &file.relative_components {
        storage_path = storage_path
            .join(component)
            .map_err(|_| ApiError::not_found("Invalid file path"))?;
    }
    let opened = storage
        .open_file(&storage_path)
        .await
        .map_err(|_| ApiError::not_found("File unavailable"))?;

    let size = opened.size();
    let etag = compute_etag(size, file.modified_at);
    let last_modified = format_http_date(file.modified_at);

    let track_file = TrackFile {
        source: opened,
        content_type: file.content_type,
    };
    let mut response = playback::serve_original(track_file, method, headers).await?;
    if let Ok(val) = HeaderValue::from_str(&etag) {
        response.headers_mut().insert(header::ETAG, val);
    }
    if let Ok(val) = HeaderValue::from_str(&last_modified) {
        response.headers_mut().insert(header::LAST_MODIFIED, val);
    }
    Ok(response)
}

fn multistatus_response(xml: String) -> Response {
    Response::builder()
        .status(StatusCode::MULTI_STATUS)
        .header(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/xml; charset=utf-8"),
        )
        .header(
            header::CACHE_CONTROL,
            HeaderValue::from_static("private, no-store"),
        )
        .header(header::CONTENT_LENGTH, xml.len())
        .body(Body::from(xml))
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

fn push_collection_xml(out: &mut String, href: &str, name: &str, modified: DateTime<Utc>) {
    let _ = write!(
        out,
        "<d:response><d:href>{}</d:href><d:propstat><d:prop>\
         <d:displayname>{}</d:displayname>\
         <d:resourcetype><d:collection/></d:resourcetype>\
         <d:getlastmodified>{}</d:getlastmodified>\
         </d:prop><d:status>HTTP/1.1 200 OK</d:status></d:propstat></d:response>",
        escape(href),
        escape(name),
        escape(&format_http_date(modified)),
    );
}

fn push_file_xml(out: &mut String, href: &str, file: &VirtualFile) {
    let etag = compute_etag(file.size_bytes, file.modified_at);
    let _ = write!(
        out,
        "<d:response><d:href>{}</d:href><d:propstat><d:prop>\
         <d:displayname>{}</d:displayname>\
         <d:resourcetype/>\
         <d:getcontentlength>{}</d:getcontentlength>\
         <d:getcontenttype>{}</d:getcontenttype>\
         <d:getetag>{}</d:getetag>\
         <d:getlastmodified>{}</d:getlastmodified>\
         </d:prop><d:status>HTTP/1.1 200 OK</d:status></d:propstat></d:response>",
        escape(href),
        escape(&file.name),
        file.size_bytes,
        escape(file.content_type),
        escape(&etag),
        escape(&format_http_date(file.modified_at)),
    );
}

fn compute_etag(size: u64, modified: DateTime<Utc>) -> String {
    format!("\"{size:x}-{:x}\"", modified.timestamp_millis())
}

fn format_http_date(dt: DateTime<Utc>) -> String {
    dt.format("%a, %d %b %Y %H:%M:%S GMT").to_string()
}

fn content_type_for_name(name: &str) -> &'static str {
    let ext = name.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase());
    match ext.as_deref() {
        Some("mp4" | "m4v") => "video/mp4",
        Some("mkv") => "video/x-matroska",
        Some("webm") => "video/webm",
        Some("mov") => "video/quicktime",
        Some("avi") => "video/x-msvideo",
        Some("ts" | "m2ts") => "video/mp2t",
        Some("mp3") => "audio/mpeg",
        Some("flac") => "audio/flac",
        Some("m4a" | "m4b" | "aac") => "audio/mp4",
        Some("ogg" | "opus") => "audio/ogg",
        Some("wav") => "audio/wav",
        Some("epub") => "application/epub+zip",
        Some("pdf") => "application/pdf",
        Some("cbz" | "zip") => "application/vnd.comicbook+zip",
        Some("cbt" | "tar") => "application/vnd.comicbook+tar",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("png") => "image/png",
        Some("webp") => "image/webp",
        Some("gif") => "image/gif",
        Some("srt") => "application/x-subrip",
        Some("vtt") => "text/vtt; charset=utf-8",
        Some("txt" | "md" | "nfo" | "ass") => "text/plain; charset=utf-8",
        Some("xml") => "application/xml; charset=utf-8",
        Some("json") => "application/json; charset=utf-8",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_base64_decodes_standard_credentials() {
        assert_eq!(
            decode_base64("YWRtaW46cGFzc3dvcmQ=").as_deref(),
            Some(b"admin:password".as_slice())
        );
        assert_eq!(decode_base64("YTpi").as_deref(), Some(b"a:b".as_slice()));
        assert_eq!(
            decode_base64("YWJjZA==").as_deref(),
            Some(b"abcd".as_slice())
        );
        assert_eq!(decode_base64(""), None);
        assert_eq!(decode_base64("not-valid-b64!"), None);
        assert_eq!(decode_base64("abc"), None);
    }

    #[test]
    fn path_validation_blocks_traversal_and_encodes_hrefs() {
        assert!(parse_safe_segments(Some("Movies/../etc/passwd")).is_err());
        assert!(parse_safe_segments(Some("Movies/%2e%2e/secret")).is_err());
        assert!(parse_safe_segments(Some("Movies/sub%2f..")).is_err());
        assert!(parse_safe_segments(Some("Movies\\evil")).is_err());
        assert_eq!(
            parse_safe_segments(Some("Movies/%E6%B2%99%E4%B8%98/Dune%202021.mkv")).unwrap(),
            vec!["Movies", "沙丘", "Dune 2021.mkv"]
        );
        assert_eq!(
            build_href(&["Movies".into(), "A & B.mkv".into()], false),
            "/webdav/Movies/A%20%26%20B.mkv"
        );
    }
}

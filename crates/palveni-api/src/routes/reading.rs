//! Reading HTTP endpoints for ebooks, PDFs and comics.

use axum::{
    Json,
    body::Body,
    extract::{
        Path, Query, State,
        rejection::{PathRejection, QueryRejection},
    },
    http::{HeaderMap, HeaderValue, Method, StatusCode, header},
    response::{IntoResponse, Response},
};
use palveni_core::reading::{
    CreateBookmarkRequest, ReadingBookmark, ReadingChapter, ReadingHome, ReadingInline,
    ReadingKind, ReadingManifest, ReadingPage, ReadingProgressUpdate, ReadingSort, ReadingSortDir,
    ReadingUserState,
};
use palveni_core::{BookmarkId, ItemId, LibraryId, clock};
use palveni_db::reading::{ReadingCursor, ReadingFilters};
use palveni_library::reading::{ReadingError, normalize_archive_path_public};
use serde::Deserialize;
use tokio::sync::Semaphore;

use super::{DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE, parse_limit, path_id, playback, query_id};
use crate::{AppState, auth::CurrentUser, error::ApiError, json::ApiJson};

static IMAGE_SCALER: Semaphore = Semaphore::const_new(2);

fn inline_scope(id: ItemId) -> String {
    format!("palveni:reading-inline:v1\n{id}")
}

fn snap_thumb_tier(size: u16) -> u16 {
    const TIERS: [u16; 5] = [96, 240, 480, 960, 1920];
    TIERS.into_iter().find(|&tier| tier >= size).unwrap_or(1920)
}

fn parse_optional_size(raw: Option<&str>) -> Result<Option<u16>, ApiError> {
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

fn map_reading_err(err: ReadingError, not_found_msg: &'static str) -> ApiError {
    match err {
        ReadingError::NotFound => ApiError::not_found(not_found_msg),
        ReadingError::UnsupportedFormat(fmt) => ApiError::unsupported_media_type(format!(
            "Format `{fmt}` is not supported for online reading; download the original file instead"
        )),
        ReadingError::Corrupt(msg) => {
            ApiError::unsupported_media_type(format!("Archive or book cannot be read: {msg}"))
        }
        ReadingError::Internal(detail) => ApiError::internal(detail),
    }
}

pub(super) async fn serve_scaled_image(
    state: &AppState,
    mut mime: &'static str,
    mut data: Vec<u8>,
    requested_size: Option<u16>,
) -> Result<Response, ApiError> {
    if let Some(tier) = requested_size.map(snap_thumb_tier).filter(|&t| t <= 960)
        && matches!(mime, "image/jpeg" | "image/png" | "image/webp")
        && let Ok(_permit) = IMAGE_SCALER.acquire().await
        && let Ok(scaled) = state.hardware.cover_thumbnail(&data, mime, tier).await
    {
        data = scaled;
        mime = "image/jpeg";
    }
    Response::builder()
        .header(header::CONTENT_TYPE, mime)
        .header(header::CONTENT_LENGTH, data.len())
        .header(header::CACHE_CONTROL, "private, no-cache")
        .header(header::X_CONTENT_TYPE_OPTIONS, "nosniff")
        .body(Body::from(data))
        .map_err(ApiError::internal)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HomeQuery {
    library_id: Option<String>,
}

pub async fn home(
    user: CurrentUser,
    State(state): State<AppState>,
    query: Result<Query<HomeQuery>, QueryRejection>,
) -> Result<Json<ReadingHome>, ApiError> {
    let Query(q) = query?;
    let library_id: Option<LibraryId> = query_id(q.library_id.as_deref(), "libraryId")?;
    let home = state
        .db
        .reading()
        .home(user.user.id, library_id)
        .await?
        .ok_or_else(|| ApiError::not_found("Reading library not found"))?;
    Ok(Json(home))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowseQuery {
    kind: Option<ReadingKind>,
    series: Option<String>,
    author: Option<String>,
    format: Option<String>,
    favorite: Option<bool>,
    completed: Option<bool>,
    q: Option<String>,
    #[serde(default)]
    sort: ReadingSort,
    dir: Option<ReadingSortDir>,
    cursor: Option<String>,
    limit: Option<String>,
}

pub async fn list(
    user: CurrentUser,
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
    query: Result<Query<BrowseQuery>, QueryRejection>,
) -> Result<Json<ReadingPage>, ApiError> {
    let library_id: LibraryId = path_id(&path?.0, "Library")?;
    let Query(q) = query?;

    for (name, val, max_len) in [
        ("series", q.series.as_deref(), 255usize),
        ("author", q.author.as_deref(), 255),
        ("format", q.format.as_deref(), 16),
        ("q", q.q.as_deref(), 128),
    ] {
        if let Some(s) = val
            && (s.chars().count() > max_len || s.chars().any(char::is_control))
        {
            return Err(ApiError::validation(format!("Invalid {name} filter")).field(name));
        }
    }

    let limit = parse_limit(q.limit.as_deref(), DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE)?;
    let cursor = q
        .cursor
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| {
            ReadingCursor::decode(s)
                .ok_or_else(|| ApiError::validation("Invalid reading cursor").field("cursor"))
        })
        .transpose()?;

    let filters = ReadingFilters {
        kind: q.kind,
        series: q.series.as_deref(),
        author: q.author.as_deref(),
        format: q.format.as_deref(),
        favorite: q.favorite,
        completed: q.completed,
        q: q.q.as_deref(),
        sort: q.sort,
        dir: q.dir,
        cursor,
        limit,
    };

    let page = state
        .db
        .reading()
        .list(user.user.id, library_id, &filters)
        .await?
        .ok_or_else(|| ApiError::not_found("Reading library not found"))?;
    Ok(Json(page))
}

pub async fn manifest(
    user: CurrentUser,
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
) -> Result<Json<ReadingManifest>, ApiError> {
    let id: ItemId = path_id(&path?.0, "Reading item")?;
    let chapters = match palveni_library::reading::list_chapters(&state.db, id).await {
        Ok(list) => list,
        Err(ReadingError::NotFound) => return Err(ApiError::not_found("Reading item not found")),
        Err(ReadingError::Internal(detail)) => return Err(ApiError::internal(detail)),
        Err(_) => Vec::new(),
    };
    let manifest = state
        .db
        .reading()
        .manifest(user.user.id, id, chapters)
        .await?
        .ok_or_else(|| ApiError::not_found("Reading item not found"))?;
    Ok(Json(manifest))
}

#[derive(Deserialize)]
pub struct SizeQuery {
    size: Option<String>,
}

pub async fn cover(
    _: CurrentUser,
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
    query: Result<Query<SizeQuery>, QueryRejection>,
) -> Result<Response, ApiError> {
    let id: ItemId = path_id(&path?.0, "Reading item")?;
    let size = parse_optional_size(query?.0.size.as_deref())?;
    let (mime, data) = palveni_library::reading::read_cover(&state.db, id)
        .await
        .map_err(|e| match e {
            ReadingError::Internal(detail) => ApiError::internal(detail),
            _ => ApiError::not_found("Cover image not available"),
        })?;
    serve_scaled_image(&state, mime, data, size).await
}

pub async fn page(
    _: CurrentUser,
    State(state): State<AppState>,
    path: Result<Path<(String, String)>, PathRejection>,
    query: Result<Query<SizeQuery>, QueryRejection>,
) -> Result<Response, ApiError> {
    let (raw_id, raw_page) = path?.0;
    let id: ItemId = path_id(&raw_id, "Comic")?;
    let page_num: u32 = raw_page
        .parse()
        .map_err(|_| ApiError::validation("page must be a nonnegative integer").field("page"))?;
    let size = parse_optional_size(query?.0.size.as_deref())?;

    let (mime, data) = palveni_library::reading::read_page(&state.db, id, page_num)
        .await
        .map_err(|e| map_reading_err(e, "Comic page not found"))?;
    serve_scaled_image(&state, mime, data, size).await
}

pub async fn chapter(
    _: CurrentUser,
    State(state): State<AppState>,
    path: Result<Path<(String, String)>, PathRejection>,
) -> Result<Json<ReadingChapter>, ApiError> {
    let (raw_id, raw_idx) = path?.0;
    let id: ItemId = path_id(&raw_id, "Book")?;
    let index: u32 = raw_idx
        .parse()
        .map_err(|_| ApiError::validation("index must be a nonnegative integer").field("index"))?;

    let ch = palveni_library::reading::read_chapter(&state.db, id, index)
        .await
        .map_err(|e| map_reading_err(e, "Chapter not found"))?;
    Ok(Json(ch))
}

#[derive(Deserialize)]
pub struct AssetQuery {
    path: Option<String>,
}

pub async fn asset(
    _: CurrentUser,
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
    query: Result<Query<AssetQuery>, QueryRejection>,
) -> Result<Response, ApiError> {
    let id: ItemId = path_id(&path?.0, "Book")?;
    let raw_asset = query?
        .0
        .path
        .ok_or_else(|| ApiError::validation("path query parameter is required").field("path"))?;
    let normalized = normalize_archive_path_public(&raw_asset)
        .ok_or_else(|| ApiError::validation("Invalid archive asset path").field("path"))?;

    let (mime, data) = palveni_library::reading::read_asset(&state.db, id, &normalized)
        .await
        .map_err(|e| map_reading_err(e, "Asset not found"))?;

    Response::builder()
        .header(header::CONTENT_TYPE, mime)
        .header(header::CONTENT_LENGTH, data.len())
        .header(header::CACHE_CONTROL, "private, no-cache")
        .header(header::X_CONTENT_TYPE_OPTIONS, "nosniff")
        .body(Body::from(data))
        .map_err(ApiError::internal)
}

#[derive(Deserialize)]
pub struct TicketQuery {
    ticket: Option<String>,
}

pub async fn prepare_inline(
    current: CurrentUser,
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
) -> Result<Response, ApiError> {
    let id: ItemId = path_id(&path?.0, "Reading item")?;
    let (_, name, _) = palveni_library::reading::open_reading_file(&state.db, id)
        .await
        .map_err(|e| map_reading_err(e, "Reading item not found"))?;
    if palveni_library::classify::extension_of(&name).as_deref() != Some("pdf") {
        return Err(ApiError::unsupported_media_type(
            "Inline streaming is only supported for PDF documents",
        ));
    }
    let (expires_at, ticket) = playback::issue_ticket(
        &state.identity,
        &inline_scope(id),
        current.session_id,
        clock::now(),
    );
    let body = ReadingInline {
        url: format!("/api/v1/items/{id}/reading-inline?ticket={ticket}"),
        expires_at,
    };
    Ok(([(header::CACHE_CONTROL, "private, no-store")], Json(body)).into_response())
}

pub async fn stream_inline(
    State(state): State<AppState>,
    method: Method,
    path: Result<Path<String>, PathRejection>,
    query: Result<Query<TicketQuery>, QueryRejection>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let raw_id = path?.0;
    let id: ItemId = if headers.contains_key(header::AUTHORIZATION) {
        CurrentUser::from_headers(&headers, &state).await?;
        path_id(&raw_id, "Reading item")?
    } else {
        let ticket = query
            .map_err(|_| ApiError::unauthorized())?
            .0
            .ticket
            .ok_or_else(ApiError::unauthorized)?;
        let id = path_id(&raw_id, "Reading item")?;
        let session =
            playback::verify_ticket(&state.identity, &inline_scope(id), &ticket, clock::now())
                .ok_or_else(ApiError::unauthorized)?;
        state
            .db
            .sessions()
            .active_by_id(session)
            .await?
            .ok_or_else(ApiError::unauthorized)?;
        id
    };

    let (mut file, name, _) = palveni_library::reading::open_reading_file(&state.db, id)
        .await
        .map_err(|e| map_reading_err(e, "Reading item not found"))?;
    if palveni_library::classify::extension_of(&name).as_deref() != Some("pdf") {
        return Err(ApiError::unsupported_media_type(
            "Inline streaming is only supported for PDF documents",
        ));
    }
    file.content_type = "application/pdf";
    let mut response = playback::serve_original(file, method, &headers).await?;
    response.headers_mut().insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_static("inline"),
    );
    response.headers_mut().insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    Ok(response)
}

pub async fn save_progress(
    user: CurrentUser,
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
    ApiJson(body): ApiJson<ReadingProgressUpdate>,
) -> Result<Json<ReadingUserState>, ApiError> {
    let id: ItemId = path_id(&path?.0, "Reading item")?;
    if body.page < 0 {
        return Err(ApiError::validation("page must be nonnegative").field("page"));
    }
    if body.total_pages.is_some_and(|t| t < 1) {
        return Err(ApiError::validation("totalPages must be at least 1").field("totalPages"));
    }
    if body
        .locator
        .as_deref()
        .is_some_and(|l| l.chars().count() > 512 || l.chars().any(char::is_control))
    {
        return Err(ApiError::validation("Invalid locator").field("locator"));
    }
    let updated = state
        .db
        .reading()
        .save_progress(user.user.id, id, body)
        .await?;
    Ok(Json(updated))
}

pub async fn create_bookmark(
    user: CurrentUser,
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
    ApiJson(body): ApiJson<CreateBookmarkRequest>,
) -> Result<(StatusCode, Json<ReadingBookmark>), ApiError> {
    let id: ItemId = path_id(&path?.0, "Reading item")?;
    if body.page < 0 {
        return Err(ApiError::validation("page must be nonnegative").field("page"));
    }
    let trimmed_label = body.label.trim();
    if trimmed_label.is_empty()
        || trimmed_label.chars().count() > 200
        || trimmed_label.chars().any(char::is_control)
    {
        return Err(ApiError::validation("label must be 1–200 characters").field("label"));
    }
    if body
        .locator
        .as_deref()
        .is_some_and(|l| l.chars().count() > 512 || l.chars().any(char::is_control))
    {
        return Err(ApiError::validation("Invalid locator").field("locator"));
    }
    if body
        .note
        .as_deref()
        .is_some_and(|n| n.chars().count() > 1000)
    {
        return Err(ApiError::validation("note exceeds 1000 characters").field("note"));
    }
    let created = state
        .db
        .reading()
        .create_bookmark(user.user.id, id, body)
        .await?;
    Ok((StatusCode::CREATED, Json(created)))
}

pub async fn delete_bookmark(
    user: CurrentUser,
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
) -> Result<StatusCode, ApiError> {
    let bookmark_id: BookmarkId = path_id(&path?.0, "Bookmark")?;
    if !state
        .db
        .reading()
        .delete_bookmark(user.user.id, bookmark_id)
        .await?
    {
        return Err(ApiError::not_found("Bookmark not found"));
    }
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use axum::body::to_bytes;
    use palveni_core::{FeatureRegistry, ItemId, ItemKind, LibraryKind, PerfTier, Role, ServerId};
    use palveni_db::{Db, DbOptions, items::ItemUpsert, libraries::NewLibrary};
    use palveni_jobs::JobService;
    use palveni_media::{HardwareService, ProbeOptions};
    use palveni_plugin_host::{PluginCatalog, skin_store::SkinStore};
    use tokio_util::sync::CancellationToken;

    use super::*;

    const TINY_PNG: &[u8] = b"\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR\x00\x00\x00\x01\x00\x00\x00\x01\x08\x02\x00\x00\x00\x90wS\xde";

    fn stored_zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut out = Vec::new();
        let mut cd = Vec::new();
        for &(name, data) in entries {
            let offset = u32::try_from(out.len()).unwrap();
            let crc = palveni_library::reading::archive::crc32(data);
            let len = u32::try_from(data.len()).unwrap();
            let nb = name.as_bytes();
            let nl = u16::try_from(nb.len()).unwrap();
            out.extend_from_slice(&[0x50, 0x4b, 0x03, 0x04]);
            out.extend_from_slice(&20u16.to_le_bytes());
            out.extend_from_slice(&[0; 8]);
            out.extend_from_slice(&crc.to_le_bytes());
            out.extend_from_slice(&len.to_le_bytes());
            out.extend_from_slice(&len.to_le_bytes());
            out.extend_from_slice(&nl.to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(nb);
            out.extend_from_slice(data);

            cd.extend_from_slice(&[0x50, 0x4b, 0x01, 0x02]);
            cd.extend_from_slice(&20u16.to_le_bytes());
            cd.extend_from_slice(&20u16.to_le_bytes());
            cd.extend_from_slice(&[0; 8]);
            cd.extend_from_slice(&crc.to_le_bytes());
            cd.extend_from_slice(&len.to_le_bytes());
            cd.extend_from_slice(&len.to_le_bytes());
            cd.extend_from_slice(&nl.to_le_bytes());
            cd.extend_from_slice(&[0; 12]);
            cd.extend_from_slice(&offset.to_le_bytes());
            cd.extend_from_slice(nb);
        }
        let cd_off = u32::try_from(out.len()).unwrap();
        let cd_len = u32::try_from(cd.len()).unwrap();
        let cnt = u16::try_from(entries.len()).unwrap();
        out.extend_from_slice(&cd);
        out.extend_from_slice(&[0x50, 0x4b, 0x05, 0x06, 0, 0, 0, 0]);
        out.extend_from_slice(&cnt.to_le_bytes());
        out.extend_from_slice(&cnt.to_le_bytes());
        out.extend_from_slice(&cd_len.to_le_bytes());
        out.extend_from_slice(&cd_off.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out
    }

    #[tokio::test]
    async fn reading_handlers_cover_manifest_chapters_pages_inline_pdf_and_415() {
        let raw_dir =
            std::env::temp_dir().join(format!("palveni-reading-api-{}", ItemId::generate()));
        std::fs::create_dir_all(&raw_dir).unwrap();
        let dir = raw_dir.canonicalize().unwrap();
        let books_dir = dir.join("books");
        let comics_dir = dir.join("comics");
        std::fs::create_dir_all(&books_dir).unwrap();
        std::fs::create_dir_all(&comics_dir).unwrap();

        std::fs::write(
            books_dir.join("guide.md"),
            "# 阅读指南\n\n欢迎来到书房。\n\n## 第二章\n\n正文内容。\n",
        )
        .unwrap();
        let pdf_bytes = b"%PDF-1.7\n1 0 obj<<>>endobj\ntrailer<<>>\n%%EOF\n";
        std::fs::write(books_dir.join("manual.pdf"), pdf_bytes).unwrap();
        let cbz = stored_zip(&[("cover.png", TINY_PNG), ("page_01.png", TINY_PNG)]);
        std::fs::write(comics_dir.join("vol1.cbz"), &cbz).unwrap();
        std::fs::write(comics_dir.join("legacy.cbr"), b"Rar!\x1a\x07\x00").unwrap();

        let db = Db::open(&DbOptions {
            main_url: format!("sqlite://{}/main.db?mode=rwc", dir.display()),
            ops_url: format!("sqlite://{}/ops.db?mode=rwc", dir.display()),
            max_connections: Some(4),
            min_connections: Some(1),
        })
        .await
        .unwrap();

        let user = db
            .users()
            .create_first_admin("reader", "hash")
            .await
            .unwrap()
            .unwrap();
        let session = db.sessions().create(user.id, "Web", None).await.unwrap();

        let book_lib = db
            .libraries()
            .create(NewLibrary {
                name: "书籍".into(),
                kind: LibraryKind::Book,
                paths: vec![books_dir.display().to_string()],
            })
            .await
            .unwrap();
        let comic_lib = db
            .libraries()
            .create(NewLibrary {
                name: "漫画".into(),
                kind: LibraryKind::Comic,
                paths: vec![comics_dir.display().to_string()],
            })
            .await
            .unwrap();

        let md_id = ItemId::generate();
        let pdf_id = ItemId::generate();
        let cbz_id = ItemId::generate();
        let cbr_id = ItemId::generate();

        db.items()
            .apply_scan_batch(
                book_lib.id,
                &[
                    ItemUpsert {
                        music: None,
                        id: md_id,
                        parent_id: None,
                        kind: ItemKind::Book,
                        title: "guide".into(),
                        sort_title: "guide".into(),
                        search_text: "guide".into(),
                        path: books_dir.join("guide.md").display().to_string(),
                        size_bytes: Some(64),
                        modified_at: Some(1_730_000_000_000),
                    },
                    ItemUpsert {
                        music: None,
                        id: pdf_id,
                        parent_id: None,
                        kind: ItemKind::Book,
                        title: "manual".into(),
                        sort_title: "manual".into(),
                        search_text: "manual".into(),
                        path: books_dir.join("manual.pdf").display().to_string(),
                        size_bytes: Some(pdf_bytes.len() as i64),
                        modified_at: Some(1_730_000_000_000),
                    },
                ],
                &[],
            )
            .await
            .unwrap();
        db.items()
            .apply_scan_batch(
                comic_lib.id,
                &[
                    ItemUpsert {
                        music: None,
                        id: cbz_id,
                        parent_id: None,
                        kind: ItemKind::Comic,
                        title: "vol1".into(),
                        sort_title: "vol1".into(),
                        search_text: "vol1".into(),
                        path: comics_dir.join("vol1.cbz").display().to_string(),
                        size_bytes: Some(cbz.len() as i64),
                        modified_at: Some(1_730_000_000_000),
                    },
                    ItemUpsert {
                        music: None,
                        id: cbr_id,
                        parent_id: None,
                        kind: ItemKind::Comic,
                        title: "legacy".into(),
                        sort_title: "legacy".into(),
                        search_text: "legacy".into(),
                        path: comics_dir.join("legacy.cbr").display().to_string(),
                        size_bytes: Some(7),
                        modified_at: Some(1_730_000_000_000),
                    },
                ],
                &[],
            )
            .await
            .unwrap();

        let bus = db.event_bus(16).await.unwrap();
        let jobs = JobService::new(db.clone(), bus.clone(), 1);
        let state = AppState {
            db: db.clone(),
            music_search: Arc::new(palveni_library::music_search::MusicSearch::new(
                dir.join("search"),
            )),
            video_images: Arc::new(palveni_library::video::VideoImages::new(
                dir.join("cache"),
                None,
                None,
            )),
            identity: Arc::new(palveni_link::NodeIdentity::generate().unwrap()),
            link: Arc::new(crate::LinkPairings::default()),
            login_limit: Arc::new(crate::LoginLimit::new(4)),
            subsonic_limit: Arc::new(crate::LoginLimit::new(4)),
            bus,
            jobs,
            hardware: Arc::new(HardwareService::new(
                db.clone(),
                dir.clone(),
                ProbeOptions::default(),
            )),
            plugins: Arc::new(PluginCatalog::new()),
            settings: Arc::new(tokio::sync::RwLock::new(Default::default())),
            boot_tier: PerfTier::detect(0, 1, false),
            skins: Arc::new(SkinStore::open(&dir.join("skins")).unwrap()),
            skin_writes: Arc::new(tokio::sync::Semaphore::new(1)),
            features: FeatureRegistry::new(),
            server: Arc::new(crate::ServerInfo {
                id: ServerId::generate(),
                name: "Test".into(),
                version: "0.1.0".into(),
            }),
            shutdown: CancellationToken::new(),
        };
        let auth = CurrentUser {
            user: palveni_core::User {
                id: user.id,
                username: user.username,
                role: Role::Admin,
                created_at: user.created_at,
            },
            session_id: session.session_id,
            device_id: None,
        };

        // 1. On-demand backfill via manifest + chapter read
        let md_manifest = manifest(
            auth.clone(),
            State(state.clone()),
            Ok(Path(md_id.to_string())),
        )
        .await
        .unwrap()
        .0;
        assert_eq!(md_manifest.entry.title, "阅读指南");
        assert_eq!(md_manifest.chapters.len(), 2);

        let ch0 = chapter(
            auth.clone(),
            State(state.clone()),
            Ok(Path((md_id.to_string(), "0".into()))),
        )
        .await
        .unwrap()
        .0;
        assert!(ch0.html.contains("欢迎来到书房"));

        // 2. Comic cover & page 0 succeed; CBR page returns 415 Unsupported Media Type
        let page_resp = page(
            auth.clone(),
            State(state.clone()),
            Ok(Path((cbz_id.to_string(), "1".into()))),
            Ok(Query(SizeQuery { size: None })),
        )
        .await
        .unwrap();
        assert_eq!(page_resp.status(), StatusCode::OK);
        assert_eq!(page_resp.headers()[header::CONTENT_TYPE], "image/png");

        let cbr_err = page(
            auth.clone(),
            State(state.clone()),
            Ok(Path((cbr_id.to_string(), "0".into()))),
            Ok(Query(SizeQuery { size: None })),
        )
        .await
        .unwrap_err();
        assert_eq!(cbr_err.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);

        // 3. Inline PDF ticket + HTTP Range 206 + Content-Disposition: inline
        let prep_resp = prepare_inline(
            auth.clone(),
            State(state.clone()),
            Ok(Path(pdf_id.to_string())),
        )
        .await
        .unwrap();
        assert_eq!(prep_resp.status(), StatusCode::OK);
        let prep_json: serde_json::Value =
            serde_json::from_slice(&to_bytes(prep_resp.into_body(), 4096).await.unwrap()).unwrap();
        let url_str = prep_json["url"].as_str().unwrap();
        let ticket = url_str.split("ticket=").nth(1).unwrap().to_owned();

        let mut range_headers = HeaderMap::new();
        range_headers.insert(header::RANGE, HeaderValue::from_static("bytes=0-7"));
        let inline_resp = stream_inline(
            State(state.clone()),
            Method::GET,
            Ok(Path(pdf_id.to_string())),
            Ok(Query(TicketQuery {
                ticket: Some(ticket),
            })),
            range_headers,
        )
        .await
        .unwrap();
        assert_eq!(inline_resp.status(), StatusCode::PARTIAL_CONTENT);
        assert_eq!(inline_resp.headers()[header::CONTENT_DISPOSITION], "inline");
        assert_eq!(
            inline_resp.headers()[header::CONTENT_TYPE],
            "application/pdf"
        );

        // 4. Progress + Bookmark create & delete + Home
        let saved = save_progress(
            auth.clone(),
            State(state.clone()),
            Ok(Path(cbz_id.to_string())),
            ApiJson(ReadingProgressUpdate {
                page: 1,
                total_pages: Some(2),
                locator: Some("page:1".into()),
                completed: false,
            }),
        )
        .await
        .unwrap()
        .0;
        assert_eq!(saved.page, 1);

        let (bm_status, Json(bm)) = create_bookmark(
            auth.clone(),
            State(state.clone()),
            Ok(Path(cbz_id.to_string())),
            ApiJson(CreateBookmarkRequest {
                page: 1,
                locator: Some("page:1".into()),
                label: "精彩跨页".into(),
                note: None,
            }),
        )
        .await
        .unwrap();
        assert_eq!(bm_status, StatusCode::CREATED);

        let home_data = home(
            auth.clone(),
            State(state.clone()),
            Ok(Query(HomeQuery { library_id: None })),
        )
        .await
        .unwrap()
        .0;
        assert_eq!(home_data.continue_reading.len(), 1);
        assert_eq!(home_data.recent_books.len(), 2);
        assert_eq!(home_data.recent_comics.len(), 2);

        let del_status = delete_bookmark(auth, State(state.clone()), Ok(Path(bm.id.to_string())))
            .await
            .unwrap();
        assert_eq!(del_status, StatusCode::NO_CONTENT);

        db.close().await.unwrap();
        let _ = std::fs::remove_dir_all(&dir);
    }
}

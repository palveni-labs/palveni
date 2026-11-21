use super::{DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE, audit, parse_limit};
use crate::{
    AppState,
    auth::{AdminUser, CurrentUser},
    error::ApiError,
    json::ApiJson,
};
use axum::{
    Json,
    extract::{
        Path, Query, State,
        rejection::{PathRejection, QueryRejection},
    },
    http::StatusCode,
};
use palveni_core::broadcast::{
    self, BroadcastEntryPage, BroadcastFavorite, BroadcastImport, BroadcastKind, BroadcastSource,
    BroadcastSourceList, MAX_BROADCAST_BYTES,
};
use palveni_db::audit::AuditEntry;
use palveni_library::broadcast::{fetch_public, parse_catalog, public_media_url};
use reqwest::Url;
use serde::Deserialize;
use std::time::Instant;
use tokio::sync::Semaphore;

static IMPORTS: Semaphore = Semaphore::const_new(2);
type IdPath = Result<Path<String>, PathRejection>;

#[derive(Deserialize)]
pub struct ListQuery {
    kind: Option<BroadcastKind>,
}

pub async fn list(
    user: CurrentUser,
    State(state): State<AppState>,
    query: Result<Query<ListQuery>, QueryRejection>,
) -> Result<Json<BroadcastSourceList>, ApiError> {
    let mut sources = state.db.broadcast_sources(query?.0.kind).await?;
    if !user.is_admin() {
        for source in &mut sources {
            source.feed_url = None;
            source.guide_url = None;
        }
    }
    Ok(Json(BroadcastSourceList { sources }))
}

fn parse_id(raw: &str) -> Result<String, ApiError> {
    broadcast::source_id(raw).ok_or_else(|| ApiError::not_found("Broadcast source not found"))
}
fn url(raw: &Option<String>, field: &str) -> Result<Option<Url>, ApiError> {
    raw.as_deref()
        .map(|s| {
            public_media_url(s, None).ok_or_else(|| {
                ApiError::validation("Expected an HTTP(S) URL without credentials").field(field)
            })
        })
        .transpose()
}
async fn material(
    text: Option<String>,
    location: Option<Url>,
) -> Result<(Option<String>, Option<Url>), ApiError> {
    if let Some(text) = text {
        if text.len() > MAX_BROADCAST_BYTES {
            return Err(ApiError::validation("A catalogue or guide exceeds 1 MiB"));
        }
        return Ok((Some(text), location));
    }
    if let Some(location) = location {
        let (body, final_url) = fetch_public(location).await.map_err(ApiError::validation)?;
        return Ok((Some(body), Some(final_url)));
    }
    Ok((None, None))
}

pub async fn save(
    AdminUser(admin): AdminUser,
    State(state): State<AppState>,
    path: IdPath,
    ApiJson(mut body): ApiJson<BroadcastImport>,
) -> Result<Json<BroadcastSource>, ApiError> {
    let id = parse_id(&path?.0)?;
    let _permit = IMPORTS.try_acquire().map_err(|_| {
        ApiError::new(
            StatusCode::TOO_MANY_REQUESTS,
            "busy",
            "Other subscriptions are being imported",
        )
    })?;
    body.name = body.name.trim().to_owned();
    if body.name.is_empty()
        || body.name.chars().count() > 128
        || body.name.chars().any(char::is_control)
    {
        return Err(
            ApiError::validation("Name must contain 1–128 printable characters").field("name"),
        );
    }
    if body.expected_version < 0 {
        return Err(
            ApiError::validation("Expected version must be nonnegative").field("expectedVersion")
        );
    }
    if body.kind == BroadcastKind::Podcast && (body.guide.is_some() || body.guide_url.is_some()) {
        return Err(ApiError::validation("Podcasts do not use XMLTV").field("guide"));
    }
    let feed_url = url(&body.feed_url, "feedUrl")?;
    let guide_url = url(&body.guide_url, "guideUrl")?;
    if body.content.is_none() && feed_url.is_none() {
        return Err(ApiError::validation("Provide a file, text, or feed URL").field("content"));
    }
    let started = Instant::now();
    let ((content, base), (guide, _)) = tokio::try_join!(
        material(body.content.take(), feed_url),
        material(body.guide.take(), guide_url)
    )?;
    let kind = body.kind;
    let fetched = Instant::now();
    let (entries, parse_time) = tokio::task::spawn_blocking(move || {
        let entered = Instant::now();
        let entries = parse_catalog(
            kind,
            content.as_deref().unwrap_or_default(),
            guide.as_deref(),
            base.as_ref(),
        );
        (entries, entered.elapsed())
    })
    .await
    .map_err(ApiError::internal)?;
    let entries = entries.map_err(ApiError::validation)?;
    let parsed = Instant::now();
    let source = state.db.save_broadcast(&id, &body, entries).await?;
    tracing::info!(
        %kind, entries = source.entry_count,
        read_ms = (fetched - started).as_millis(),
        parse_ms = parse_time.as_millis(),
        worker_wait_ms = (parsed - fetched).saturating_sub(parse_time).as_millis(),
        persist_ms = parsed.elapsed().as_millis(),
        "Imported broadcast subscription"
    );
    audit(
        &state,
        AuditEntry {
            actor: Some(admin.user.id),
            target: Some(id),
            action: "broadcast.source.saved",
            detail: None,
        },
    )
    .await;
    Ok(Json(source))
}

#[derive(Deserialize)]
pub struct VersionQuery {
    version: i64,
}

pub async fn delete(
    AdminUser(admin): AdminUser,
    State(state): State<AppState>,
    path: IdPath,
    query: Result<Query<VersionQuery>, QueryRejection>,
) -> Result<StatusCode, ApiError> {
    let id = parse_id(&path?.0)?;
    let version = query?.0.version;
    if version <= 0 {
        return Err(ApiError::validation("Version must be positive").field("version"));
    }
    state.db.delete_broadcast(&id, version).await?;
    audit(
        &state,
        AuditEntry {
            actor: Some(admin.user.id),
            target: Some(id),
            action: "broadcast.source.deleted",
            detail: None,
        },
    )
    .await;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
pub struct EntriesQuery {
    version: i64,
    offset: Option<u32>,
    limit: Option<String>,
}

pub async fn entries(
    user: CurrentUser,
    State(state): State<AppState>,
    path: IdPath,
    query: Result<Query<EntriesQuery>, QueryRejection>,
) -> Result<Json<BroadcastEntryPage>, ApiError> {
    let q = query?.0;
    if q.version <= 0 {
        return Err(ApiError::validation("Version must be positive").field("version"));
    }
    let limit = parse_limit(q.limit.as_deref(), DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE)?;
    Ok(Json(
        state
            .db
            .broadcast_entries(
                user.user.id,
                &parse_id(&path?.0)?,
                q.version,
                q.offset.unwrap_or_default(),
                limit,
            )
            .await?,
    ))
}

pub async fn favorite(
    user: CurrentUser,
    State(state): State<AppState>,
    path: Result<Path<(String, String)>, PathRejection>,
    ApiJson(body): ApiJson<BroadcastFavorite>,
) -> Result<StatusCode, ApiError> {
    let (id, entry) = path?.0;
    if entry.len() != 64
        || !entry
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(ApiError::not_found("Broadcast entry not found"));
    }
    state
        .db
        .broadcast_favorite(user.user.id, &parse_id(&id)?, &entry, body.favorite)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

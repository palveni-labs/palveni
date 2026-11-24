//! 24x7 Virtual Live TV channels, clock-synced live EPG, M3U playlist export,
//! and XMLTV programme guide export (`module: virtualTv`).

use std::fmt::Write;

use axum::Json;
use axum::extract::rejection::{PathRejection, QueryRejection};
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use chrono::{DateTime, TimeDelta, Utc};
use palveni_core::ids::{JobId, SessionId};
use palveni_core::virtual_tv::{
    CreateVirtualChannelRequest, VirtualChannel, VirtualChannelList, VirtualTvGuide,
};
use palveni_core::{LibraryKind, clock};
use palveni_db::DbError;
use palveni_db::audit::AuditEntry;
use serde::Deserialize;
use serde_json::json;

use crate::auth::{AdminUser, CurrentUser};
use crate::error::ApiError;
use crate::json::ApiJson;
use crate::routes::playback::{issue_ticket_with_lifetime, verify_ticket};
use crate::routes::xml::escape;
use crate::routes::{audit, path_id};
use crate::state::AppState;

const VIRTUAL_FEED_SCOPE: &str = "palveni:virtual-tv-feed:v1";
const FEED_TICKET_LIFETIME: TimeDelta = TimeDelta::days(30);
const STREAM_TICKET_LIFETIME: TimeDelta = TimeDelta::hours(12);
const M3U_MIME: &str = "audio/x-mpegurl; charset=utf-8";
const XMLTV_MIME: &str = "application/xml; charset=utf-8";

#[derive(Deserialize, Default)]
pub struct GuideQuery {
    pub(crate) at: Option<String>,
}

#[derive(Deserialize, Default)]
pub struct FeedAccessQuery {
    pub(crate) ticket: Option<String>,
    pub(crate) at: Option<String>,
}

fn resolve_probe_time(raw: Option<&str>) -> Result<DateTime<Utc>, ApiError> {
    let Some(text) = raw.map(str::trim).filter(|s| !s.is_empty()) else {
        return Ok(clock::now());
    };
    DateTime::parse_from_rfc3339(text)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|_| ApiError::validation("at must be a valid RFC 3339 timestamp").field("at"))
}

async fn authenticate_feed(
    headers: &HeaderMap,
    ticket: Option<&str>,
    state: &AppState,
) -> Result<(SessionId, String), ApiError> {
    let now = clock::now();
    if headers.contains_key(header::AUTHORIZATION) {
        let current = CurrentUser::from_headers(headers, state).await?;
        let (_, issued) = issue_ticket_with_lifetime(
            &state.identity,
            VIRTUAL_FEED_SCOPE,
            current.session_id,
            now,
            FEED_TICKET_LIFETIME,
        );
        return Ok((current.session_id, issued));
    }
    let raw_ticket = ticket
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(ApiError::unauthorized)?;
    let session_id = verify_ticket(&state.identity, VIRTUAL_FEED_SCOPE, raw_ticket, now)
        .ok_or_else(ApiError::unauthorized)?;
    state
        .db
        .sessions()
        .active_by_id(session_id)
        .await?
        .ok_or_else(ApiError::unauthorized)?;
    Ok((session_id, raw_ticket.to_owned()))
}

fn infer_origin(headers: &HeaderMap) -> String {
    let Some(host) = headers
        .get(header::HOST)
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|h| !h.is_empty() && !h.chars().any(char::is_control))
    else {
        return String::new();
    };
    let proto = headers
        .get("x-forwarded-proto")
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|p| matches!(*p, "http" | "https"))
        .unwrap_or("http");
    format!("{proto}://{host}")
}

fn format_xmltv_time(dt: DateTime<Utc>) -> String {
    dt.format("%Y%m%d%H%M%S +0000").to_string()
}

/// `GET /api/v1/virtual-channels`
pub async fn list_channels(
    _: CurrentUser,
    State(state): State<AppState>,
) -> Result<Json<VirtualChannelList>, ApiError> {
    let channels = state.db.virtual_tv().list_channels().await?;
    Ok(Json(VirtualChannelList { channels }))
}

/// `POST /api/v1/virtual-channels`
pub async fn create_channel(
    AdminUser(admin): AdminUser,
    State(state): State<AppState>,
    body: Result<ApiJson<CreateVirtualChannelRequest>, ApiError>,
) -> Result<(StatusCode, Json<VirtualChannel>), ApiError> {
    let ApiJson(req) = body?;
    if !(1..=9999).contains(&req.number) {
        return Err(
            ApiError::validation("Channel number must be between 1 and 9999").field("number"),
        );
    }
    let name = req.name.trim();
    if name.is_empty() || name.chars().count() > 128 || name.chars().any(char::is_control) {
        return Err(ApiError::validation("Name must be 1 to 128 characters").field("name"));
    }
    if let Some(ref genre) = req.genre_filter
        && (genre.chars().count() > 128 || genre.chars().any(char::is_control))
    {
        return Err(
            ApiError::validation("Genre filter must be at most 128 characters")
                .field("genreFilter"),
        );
    }

    let library = state
        .db
        .libraries()
        .get(req.library_id)
        .await?
        .ok_or_else(|| ApiError::not_found("Target video library not found"))?;
    if library.kind != LibraryKind::Video {
        return Err(
            ApiError::validation("Virtual TV channels must bind to a video library")
                .field("libraryId"),
        );
    }

    let channel = state
        .db
        .virtual_tv()
        .create_channel(
            req.number,
            name,
            req.library_id,
            req.genre_filter.as_deref(),
            req.enabled,
        )
        .await
        .map_err(|err| match err {
            DbError::Conflict(msg) => ApiError::conflict(msg).field("number"),
            DbError::Invalid(msg) => ApiError::validation(msg).field("libraryId"),
            other => other.into(),
        })?;

    audit(
        &state,
        AuditEntry {
            action: "virtual_tv.channel_created",
            actor: Some(admin.user.id),
            target: Some(channel.id.clone()),
            detail: Some(json!({
                "number": channel.number,
                "name": channel.name,
                "libraryId": channel.library_id,
                "genreFilter": channel.genre_filter,
            })),
        },
    )
    .await;

    Ok((StatusCode::CREATED, Json(channel)))
}

/// `DELETE /api/v1/virtual-channels/{id}`
pub async fn delete_channel(
    AdminUser(admin): AdminUser,
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
) -> Result<StatusCode, ApiError> {
    let id = path?.0;
    let _parsed: JobId = path_id(&id, "Virtual channel")?;
    if !state.db.virtual_tv().delete_channel(&id).await? {
        return Err(ApiError::not_found("Virtual channel not found"));
    }
    audit(
        &state,
        AuditEntry {
            action: "virtual_tv.channel_deleted",
            actor: Some(admin.user.id),
            target: Some(id),
            detail: None,
        },
    )
    .await;
    Ok(StatusCode::NO_CONTENT)
}

/// `GET /api/v1/virtual-channels/guide`
pub async fn guide(
    current: CurrentUser,
    State(state): State<AppState>,
    query: Result<Query<GuideQuery>, QueryRejection>,
) -> Result<Json<VirtualTvGuide>, ApiError> {
    let q = query.map_err(|e| ApiError::validation(e.body_text()))?.0;
    let now = resolve_probe_time(q.at.as_deref())?;
    let (_, ticket) = issue_ticket_with_lifetime(
        &state.identity,
        VIRTUAL_FEED_SCOPE,
        current.session_id,
        clock::now(),
        FEED_TICKET_LIFETIME,
    );
    let playlist_url = format!("/api/v1/virtual-channels/playlist.m3u?ticket={ticket}");
    let epg_url = format!("/api/v1/virtual-channels/epg.xml?ticket={ticket}");
    let guide = state
        .db
        .virtual_tv()
        .guide_at(now, playlist_url, epg_url)
        .await?;
    Ok(Json(guide))
}

/// `GET /api/v1/virtual-channels/playlist.m3u`
pub async fn playlist_m3u(
    State(state): State<AppState>,
    headers: HeaderMap,
    query: Result<Query<FeedAccessQuery>, QueryRejection>,
) -> Result<Response, ApiError> {
    let q = query.map_err(|_| ApiError::unauthorized())?.0;
    let (session_id, feed_ticket) =
        authenticate_feed(&headers, q.ticket.as_deref(), &state).await?;
    let now = resolve_probe_time(q.at.as_deref())?;
    let origin = infer_origin(&headers);
    let epg_href = format!("{origin}/api/v1/virtual-channels/epg.xml?ticket={feed_ticket}");
    let guide = state
        .db
        .virtual_tv()
        .guide_at(now, String::new(), epg_href.clone())
        .await?;

    let issue_now = clock::now();
    let mut m3u = String::with_capacity(512 + guide.channels.len() * 256);
    let _ = writeln!(
        m3u,
        r#"#EXTM3U url-tvg="{epg_href}" x-tvg-url="{epg_href}""#
    );

    for entry in &guide.channels {
        let Some(ref playing) = entry.now_playing else {
            continue;
        };
        let (_, stream_ticket) = issue_ticket_with_lifetime(
            &state.identity,
            &format!("palveni:music-stream:v1\n{}", playing.item_id),
            session_id,
            issue_now,
            STREAM_TICKET_LIFETIME,
        );
        let clean_name = entry.channel.name.replace(['"', '\r', '\n'], " ");
        let group = entry
            .channel
            .genre_filter
            .as_deref()
            .unwrap_or("Palveni Virtual TV")
            .replace(['"', '\r', '\n'], " ");
        let _ = writeln!(
            m3u,
            r#"#EXTINF:-1 tvg-id="ch-{}" tvg-chno="{}" tvg-name="{}" group-title="{}",{} {}"#,
            entry.channel.id,
            entry.channel.number,
            clean_name,
            group,
            entry.channel.number,
            clean_name
        );
        let offset_secs = playing.offset_ms / 1_000;
        let _ = writeln!(
            m3u,
            "{origin}/api/v1/items/{}/stream?ticket={stream_ticket}#t={offset_secs}",
            playing.item_id
        );
    }

    Ok((
        [
            (header::CONTENT_TYPE, M3U_MIME),
            (header::CACHE_CONTROL, "private, no-store"),
            (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
        ],
        m3u,
    )
        .into_response())
}

/// `GET /api/v1/virtual-channels/epg.xml`
pub async fn epg_xml(
    State(state): State<AppState>,
    headers: HeaderMap,
    query: Result<Query<FeedAccessQuery>, QueryRejection>,
) -> Result<Response, ApiError> {
    let q = query.map_err(|_| ApiError::unauthorized())?.0;
    let _ = authenticate_feed(&headers, q.ticket.as_deref(), &state).await?;
    let now = resolve_probe_time(q.at.as_deref())?;
    let guide = state
        .db
        .virtual_tv()
        .guide_at(now, String::new(), String::new())
        .await?;

    let mut xml = String::with_capacity(1024 + guide.channels.len() * 1024);
    xml.push_str(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
    xml.push_str(r#"<tv generator-info-name="Palveni Server">"#);

    for entry in &guide.channels {
        let _ = write!(
            xml,
            r#"<channel id="ch-{}"><display-name>{}</display-name><lcn>{}</lcn></channel>"#,
            escape(&entry.channel.id),
            escape(&entry.channel.name),
            entry.channel.number
        );
    }

    for entry in &guide.channels {
        let ch_id = format!("ch-{}", escape(&entry.channel.id));
        for prog in entry.now_playing.iter().chain(entry.programmes.iter()) {
            let _ = write!(
                xml,
                r#"<programme start="{}" stop="{}" channel="{}"><title>{}</title>"#,
                format_xmltv_time(prog.started_at),
                format_xmltv_time(prog.ends_at),
                ch_id,
                escape(&prog.work_title),
            );
            if prog.title != prog.work_title {
                let _ = write!(xml, "<sub-title>{}</sub-title>", escape(&prog.title));
            }
            if let (Some(s), Some(e)) = (prog.season, prog.episode) {
                let _ = write!(
                    xml,
                    r#"<episode-num system="onscreen">S{s:02}E{e:02}</episode-num>"#
                );
            }
            xml.push_str("</programme>");
        }
    }

    xml.push_str("</tv>");

    Ok((
        [
            (header::CONTENT_TYPE, XMLTV_MIME),
            (header::CACHE_CONTROL, "private, no-store"),
            (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
        ],
        xml,
    )
        .into_response())
}

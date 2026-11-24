//! Jellyfin-compatible HTTP protocol subset (root PascalCase routes) and
//! `/api/v1/me/api-keys` client API key management.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::net::{IpAddr, Ipv4Addr};
use std::path::Path as FsPath;

use axum::{
    Json, Router,
    body::Body,
    extract::{OriginalUri, Path, State, rejection::PathRejection},
    http::{HeaderMap, Method, StatusCode, Uri, header},
    response::Response,
    routing::{get, post},
};
use chrono::{DateTime, Utc};
use palveni_core::jellyfin::{
    ClientApiKeyKind, ClientApiKeyList, CreateClientApiKeyRequest, CreatedClientApiKey,
    MAX_API_KEY_NAME_CHARS,
};
use palveni_core::personal::MediaState;
use palveni_core::{ItemId, Library, LibraryId, LibraryKind, User, UserId};
use palveni_db::jellyfin::{JellyfinAudioRecord, JellyfinVideoRecord};
use palveni_db::personal::MediaChange;
use palveni_db::video::PosterLevel;
use serde::Deserialize;
use serde_json::{Value, json};
use tokio_util::io::ReaderStream;
use uuid::Uuid;

use crate::auth::{CurrentUser, PASSWORD_MAX_CHARS, dummy_hash, verify_password};
use crate::error::ApiError;
use crate::json::ApiJson;
use crate::routes::playback;
use crate::state::AppState;

const TICKS_PER_MS: i64 = 10_000;
const DEFAULT_LATEST_LIMIT: usize = 20;
const DEFAULT_NEXT_UP_LIMIT: usize = 24;

// ===========================================================================
// Native `/api/v1/me/api-keys` endpoints (module `jellyfin`)
// ===========================================================================

/// `GET /api/v1/me/api-keys`
pub async fn list_api_keys(
    user: CurrentUser,
    State(state): State<AppState>,
) -> Result<Json<ClientApiKeyList>, ApiError> {
    let keys = state.db.jellyfin().list_api_keys(user.user.id).await?;
    Ok(Json(ClientApiKeyList { keys }))
}

/// `POST /api/v1/me/api-keys`
pub async fn create_api_key(
    user: CurrentUser,
    State(state): State<AppState>,
    body: Result<ApiJson<CreateClientApiKeyRequest>, ApiError>,
) -> Result<(StatusCode, Json<CreatedClientApiKey>), ApiError> {
    let ApiJson(req) = body?;
    let trimmed = req.name.trim();
    if trimmed.is_empty() || trimmed.chars().count() > MAX_API_KEY_NAME_CHARS {
        return Err(ApiError::validation(format!(
            "API key name must be 1 to {MAX_API_KEY_NAME_CHARS} characters"
        ))
        .field("name"));
    }
    let kind = req.client_kind.unwrap_or(ClientApiKeyKind::Jellyfin);
    let created = state
        .db
        .jellyfin()
        .create_api_key(user.user.id, trimmed, kind)
        .await?;
    Ok((StatusCode::CREATED, Json(created)))
}

/// `DELETE /api/v1/me/api-keys/{id}`
pub async fn revoke_api_key(
    user: CurrentUser,
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
) -> Result<StatusCode, ApiError> {
    let key_id = path?.0;
    let revoked = state
        .db
        .jellyfin()
        .revoke_api_key(&key_id, user.user.id, user.is_admin())
        .await?;
    if !revoked {
        return Err(ApiError::not_found("API key not found"));
    }
    Ok(StatusCode::NO_CONTENT)
}

// ===========================================================================
// Root-mounted Jellyfin compatible router
// ===========================================================================

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/System/Info/Public", get(system_info_public))
        .route("/System/Info", get(system_info))
        .route("/System/Ping", get(system_ping).post(system_ping))
        .route("/Users/AuthenticateByName", post(authenticate_by_name))
        .route("/Users/Me", get(user_me))
        .route("/Users/{userId}", get(user_by_id))
        .route("/Users/{userId}/Views", get(user_views))
        .route("/Users/{userId}/Items", get(user_items))
        .route("/Items", get(items_root))
        .route("/Users/{userId}/Items/Latest", get(user_items_latest))
        .route("/Users/{userId}/Items/Resume", get(user_items_resume))
        .route("/Users/{userId}/Items/{itemId}", get(user_item_detail))
        .route("/Shows/NextUp", get(shows_next_up))
        .route("/Shows/{seriesId}/Seasons", get(show_seasons))
        .route("/Shows/{seriesId}/Episodes", get(show_episodes))
        .route("/Items/{itemId}/Images/{imageType}", get(item_image))
        .route(
            "/Items/{itemId}/Images/{imageType}/{imageIndex}",
            get(item_image_indexed),
        )
        .route(
            "/Items/{itemId}/PlaybackInfo",
            get(playback_info).post(playback_info),
        )
        .route(
            "/Videos/{itemId}/{target}",
            get(stream_video).head(stream_video),
        )
        .route(
            "/Audio/{itemId}/{target}",
            get(stream_audio).head(stream_audio),
        )
        .route("/Sessions/Playing", post(session_playing))
        .route("/Sessions/Playing/Progress", post(session_playing_progress))
        .route("/Sessions/Playing/Stopped", post(session_playing_stopped))
        .route(
            "/Users/{userId}/PlayedItems/{itemId}",
            post(mark_played).delete(mark_unplayed),
        )
        .route(
            "/Users/{userId}/FavoriteItems/{itemId}",
            post(mark_favorite).delete(unmark_favorite),
        )
        .route("/Library/Refresh", post(library_refresh))
}

// ===========================================================================
// Auth & Query Parsing Helpers
// ===========================================================================

#[derive(Clone, Debug)]
struct JellyfinPrincipal {
    user: User,
    token: String,
    #[allow(dead_code)]
    client: Option<String>,
    #[allow(dead_code)]
    device: Option<String>,
    #[allow(dead_code)]
    device_id: Option<String>,
}

#[derive(Default, Debug)]
struct AuthHeaderMeta {
    client: Option<String>,
    device: Option<String>,
    device_id: Option<String>,
    token: Option<String>,
}

fn parse_mediabrowser_header(value: &str) -> AuthHeaderMeta {
    let mut meta = AuthHeaderMeta::default();
    let trimmed = value.trim();
    let rest = if let Some(s) = strip_prefix_ignore_ascii_case(trimmed, "MediaBrowser ") {
        s
    } else if let Some(s) = strip_prefix_ignore_ascii_case(trimmed, "Emby ") {
        s
    } else {
        return meta;
    };

    let bytes = rest.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        while i < bytes.len() && (bytes[i] == b' ' || bytes[i] == b',' || bytes[i] == b'\t') {
            i += 1;
        }
        let key_start = i;
        while i < bytes.len() && bytes[i] != b'=' && bytes[i] != b',' {
            i += 1;
        }
        if i >= bytes.len() || bytes[i] != b'=' {
            while i < bytes.len() && bytes[i] != b',' {
                i += 1;
            }
            continue;
        }
        let key = rest[key_start..i].trim();
        i += 1; // skip '='
        while i < bytes.len() && bytes[i] == b' ' {
            i += 1;
        }
        let val = if i < bytes.len() && bytes[i] == b'"' {
            i += 1;
            let val_start = i;
            while i < bytes.len() && bytes[i] != b'"' {
                i += 1;
            }
            let v = rest[val_start..i].to_owned();
            if i < bytes.len() && bytes[i] == b'"' {
                i += 1;
            }
            v
        } else {
            let val_start = i;
            while i < bytes.len() && bytes[i] != b',' {
                i += 1;
            }
            rest[val_start..i].trim().to_owned()
        };
        if !val.is_empty() {
            if key.eq_ignore_ascii_case("Token") {
                meta.token = Some(val);
            } else if key.eq_ignore_ascii_case("Client") {
                meta.client = Some(val);
            } else if key.eq_ignore_ascii_case("Device") {
                meta.device = Some(val);
            } else if key.eq_ignore_ascii_case("DeviceId") {
                meta.device_id = Some(val);
            }
        }
    }
    meta
}

fn strip_prefix_ignore_ascii_case<'a>(input: &'a str, prefix: &str) -> Option<&'a str> {
    if input.len() < prefix.len() {
        return None;
    }
    let head = input.get(..prefix.len())?;
    head.eq_ignore_ascii_case(prefix)
        .then(|| &input[prefix.len()..])
}

#[derive(Default, Debug)]
struct QueryParams {
    map: HashMap<String, String>,
}

impl QueryParams {
    fn from_uri(uri: &Uri) -> Self {
        let mut map = HashMap::new();
        if let Some(q) = uri.query() {
            for pair in q.split('&') {
                if pair.is_empty() {
                    continue;
                }
                let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
                let key = percent_decode(k).to_ascii_lowercase();
                let val = percent_decode(v);
                if !key.is_empty() {
                    map.insert(key, val);
                }
            }
        }
        Self { map }
    }

    fn get(&self, key: &str) -> Option<&str> {
        self.map
            .get(&key.to_ascii_lowercase())
            .map(String::as_str)
            .filter(|s| !s.is_empty())
    }

    fn get_bool(&self, key: &str) -> Option<bool> {
        match self.get(key)?.to_ascii_lowercase().as_str() {
            "true" | "1" | "yes" => Some(true),
            "false" | "0" | "no" => Some(false),
            _ => None,
        }
    }

    fn get_usize(&self, key: &str) -> Option<usize> {
        self.get(key)?.parse::<usize>().ok()
    }

    fn get_i32(&self, key: &str) -> Option<i32> {
        self.get(key)?.parse::<i32>().ok()
    }

    fn get_u16(&self, key: &str) -> Option<u16> {
        self.get(key)?.parse::<u16>().ok()
    }
}

fn percent_decode(raw: &str) -> String {
    let bytes = raw.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'+' {
            out.push(b' ');
            i += 1;
        } else if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let (Some(hi), Some(lo)) = (hex_val(bytes[i + 1]), hex_val(bytes[i + 2]))
        {
            out.push((hi << 4) | lo);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

fn extract_auth_meta_and_token(
    headers: &HeaderMap,
    query: &QueryParams,
) -> (AuthHeaderMeta, Option<String>) {
    let mut meta = AuthHeaderMeta::default();
    for header_name in [header::AUTHORIZATION.as_str(), "x-emby-authorization"] {
        if let Some(raw) = headers.get(header_name).and_then(|v| v.to_str().ok()) {
            let parsed = parse_mediabrowser_header(raw);
            if parsed.token.is_some() || parsed.client.is_some() || parsed.device.is_some() {
                meta = parsed;
                break;
            }
            if let Some(bearer) = strip_prefix_ignore_ascii_case(raw.trim(), "Bearer ") {
                let tok = bearer.trim();
                if !tok.is_empty() {
                    meta.token = Some(tok.to_owned());
                    break;
                }
            }
        }
    }

    let token = meta
        .token
        .clone()
        .or_else(|| {
            headers
                .get("x-emby-token")
                .and_then(|v| v.to_str().ok())
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
        })
        .or_else(|| {
            headers
                .get("x-mediabrowser-token")
                .and_then(|v| v.to_str().ok())
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
        })
        .or_else(|| query.get("api_key").map(str::to_owned))
        .or_else(|| query.get("apikey").map(str::to_owned))
        .or_else(|| query.get("token").map(str::to_owned));

    (meta, token)
}

async fn authenticate_jellyfin(
    state: &AppState,
    headers: &HeaderMap,
    query: &QueryParams,
) -> Result<JellyfinPrincipal, ApiError> {
    let (meta, token) = extract_auth_meta_and_token(headers, query);
    let Some(token) = token else {
        return Err(ApiError::unauthorized());
    };

    if let Some((user, _key_id)) = state.db.jellyfin().authenticate_api_key(&token).await? {
        return Ok(JellyfinPrincipal {
            user,
            token,
            client: meta.client,
            device: meta.device,
            device_id: meta.device_id,
        });
    }

    if let Some(session) = state.db.sessions().authenticate(&token).await? {
        return Ok(JellyfinPrincipal {
            user: session.user,
            token,
            client: meta.client,
            device: meta.device,
            device_id: meta.device_id,
        });
    }

    Err(ApiError::unauthorized())
}

fn parse_jellyfin_uuid(raw: &str) -> Option<Uuid> {
    let trimmed = raw.trim();
    if let Ok(u) = Uuid::parse_str(trimmed) {
        return Some(u);
    }
    if trimmed.len() == 32 && trimmed.bytes().all(|b| b.is_ascii_hexdigit()) {
        let hyphenated = format!(
            "{}-{}-{}-{}-{}",
            &trimmed[0..8],
            &trimmed[8..12],
            &trimmed[12..16],
            &trimmed[16..20],
            &trimmed[20..32]
        );
        return Uuid::parse_str(&hyphenated).ok();
    }
    None
}

fn parse_item_id(raw: &str) -> Option<ItemId> {
    parse_jellyfin_uuid(raw).and_then(|u| u.to_string().parse().ok())
}

fn parse_library_id(raw: &str) -> Option<LibraryId> {
    parse_jellyfin_uuid(raw).and_then(|u| u.to_string().parse().ok())
}

fn parse_user_id(raw: &str) -> Option<UserId> {
    parse_jellyfin_uuid(raw).and_then(|u| u.to_string().parse().ok())
}

fn is_hex64(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn encode_season_id(work_key: &str, season: i32) -> String {
    format!("season_{work_key}_{season}")
}

fn parse_season_id(raw: &str) -> Option<(&str, i32)> {
    let rest = raw.strip_prefix("season_")?;
    let (work_key, num_str) = rest.rsplit_once('_')?;
    if !is_hex64(work_key) {
        return None;
    }
    let season: i32 = num_str.parse().ok()?;
    Some((work_key, season))
}

fn resolve_target_user_id(
    principal: &JellyfinPrincipal,
    raw_user_id: &str,
) -> Result<UserId, ApiError> {
    if raw_user_id.eq_ignore_ascii_case("me") {
        return Ok(principal.user.id);
    }
    let requested =
        parse_user_id(raw_user_id).ok_or_else(|| ApiError::not_found("User not found"))?;
    if requested != principal.user.id && !principal.user.is_admin() {
        return Err(ApiError::forbidden());
    }
    Ok(requested)
}

async fn effective_server_name(state: &AppState) -> String {
    state
        .settings
        .read()
        .await
        .server_name
        .clone()
        .unwrap_or_else(|| state.server.name.clone())
}

async fn accessible_media_libraries(
    state: &AppState,
    user: &User,
) -> Result<Vec<Library>, ApiError> {
    let all = state.db.libraries().list().await?;
    let filtered = state
        .db
        .accounts()
        .filter_libraries(user.id, user.role, all)
        .await?;
    Ok(filtered
        .into_iter()
        .filter(|lib| {
            matches!(
                lib.kind,
                LibraryKind::Video
                    | LibraryKind::Music
                    | LibraryKind::Audiobook
                    | LibraryKind::Podcast
            )
        })
        .collect())
}

fn ms_to_ticks(ms: i64) -> i64 {
    ms.max(0).saturating_mul(TICKS_PER_MS)
}

fn ticks_to_ms(ticks: i64) -> i64 {
    ticks.max(0) / TICKS_PER_MS
}

fn file_extension_lower(path: Option<&str>) -> String {
    path.and_then(|p| FsPath::new(p).extension())
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_else(|| "mp4".to_owned())
}

fn jellyfin_user_json(
    server_id: &str,
    user: &User,
    all_libraries: bool,
    allowed_libs: &[Library],
) -> Value {
    let enabled_folders: Vec<String> = allowed_libs.iter().map(|l| l.id.to_string()).collect();
    json!({
        "Name": user.username,
        "ServerId": server_id,
        "Id": user.id.to_string(),
        "HasPassword": true,
        "HasConfiguredPassword": true,
        "HasConfiguredEasyPassword": false,
        "EnableAutoLogin": false,
        "Policy": {
            "IsAdministrator": user.is_admin(),
            "IsHidden": false,
            "IsDisabled": false,
            "EnableAllFolders": user.is_admin() || all_libraries,
            "EnabledFolders": enabled_folders,
            "EnableMediaPlayback": true,
            "EnableAudioPlaybackTranscoding": true,
            "EnableVideoPlaybackTranscoding": true,
            "EnablePlaybackRemuxing": true,
            "EnableContentDownloading": true,
            "EnableRemoteAccess": true
        }
    })
}

fn jellyfin_user_data_json(
    state: &MediaState,
    favorite_override: Option<bool>,
    unplayed_count: Option<u64>,
) -> Value {
    let mut obj = json!({
        "PlaybackPositionTicks": ms_to_ticks(state.position_ms),
        "PlayCount": state.play_count,
        "IsFavorite": favorite_override.unwrap_or(state.favorite),
        "Played": state.completed,
        "Key": ""
    });
    if let Some(last) = state.last_played_at {
        obj["LastPlayedDate"] = json!(last.to_rfc3339());
    }
    if let Some(unplayed) = unplayed_count {
        obj["UnplayedItemCount"] = json!(unplayed);
    }
    obj
}

fn jellyfin_image_tags(
    has_primary: bool,
    has_thumb: bool,
    has_backdrop: bool,
    item_key: &str,
) -> (Value, Vec<String>) {
    let mut tags = serde_json::Map::new();
    let short_tag = item_key.chars().take(12).collect::<String>();
    if has_primary {
        tags.insert("Primary".into(), json!(short_tag));
    }
    if has_thumb {
        tags.insert("Thumb".into(), json!(short_tag));
    }
    let backdrops = if has_backdrop {
        vec![short_tag]
    } else {
        Vec::new()
    };
    (Value::Object(tags), backdrops)
}

struct MediaSourceSpec<'a> {
    item_id: ItemId,
    name: &'a str,
    path: Option<&'a str>,
    size_bytes: Option<i64>,
    runtime_ms: Option<i64>,
    is_audio: bool,
}

async fn build_media_source_json(
    state: &AppState,
    spec: MediaSourceSpec<'_>,
    token: &str,
) -> Value {
    let MediaSourceSpec {
        item_id,
        name,
        path,
        size_bytes,
        runtime_ms,
        is_audio,
    } = spec;
    let ext = file_extension_lower(path);
    let is_strm = path.is_some_and(palveni_library::video::strm::is_strm_name);
    if is_strm
        && let Ok(Some(loc)) = state.db.video().location(item_id).await
        && let Ok(target_url) = palveni_library::video::strm::read_strm_location(loc).await
    {
        let inferred_container = target_url
            .split('?')
            .next()
            .and_then(|u| FsPath::new(u).extension())
            .and_then(|e| e.to_str())
            .map(str::to_ascii_lowercase)
            .filter(|e| e.len() <= 8)
            .unwrap_or_else(|| "mp4".to_owned());
        return json!({
            "Protocol": "Http",
            "Id": item_id.to_string(),
            "Path": target_url,
            "Type": "Default",
            "Container": inferred_container,
            "Name": name,
            "IsRemote": true,
            "RunTimeTicks": runtime_ms.map(ms_to_ticks),
            "SupportsTranscoding": false,
            "SupportsDirectStream": true,
            "SupportsDirectPlay": true,
            "DirectStreamUrl": target_url,
            "MediaStreams": []
        });
    }

    let route_kind = if is_audio { "Audio" } else { "Videos" };
    let direct_stream_url =
        format!("/{route_kind}/{item_id}/stream.{ext}?static=true&api_key={token}");
    json!({
        "Protocol": "File",
        "Id": item_id.to_string(),
        "Path": path.unwrap_or(name),
        "Type": "Default",
        "Container": ext,
        "Size": size_bytes,
        "Name": name,
        "IsRemote": false,
        "RunTimeTicks": runtime_ms.map(ms_to_ticks),
        "SupportsTranscoding": false,
        "SupportsDirectStream": true,
        "SupportsDirectPlay": true,
        "DirectStreamUrl": direct_stream_url,
        "MediaStreams": []
    })
}

#[derive(Clone, Debug)]
struct CatalogItem {
    id: String,
    item_type: &'static str,
    name: String,
    sort_name: String,
    original_title: Option<String>,
    series_name: Option<String>,
    album: Option<String>,
    artists: Vec<String>,
    year: Option<i32>,
    rating: Option<f64>,
    premiered: Option<String>,
    created_at: DateTime<Utc>,
    last_played_at: Option<DateTime<Utc>>,
    played: bool,
    resumable: bool,
    favorite: bool,
    json: Value,
}

fn build_movie_catalog_item(
    server_id: &str,
    records: &[JellyfinVideoRecord],
    media_sources: Option<Vec<Value>>,
) -> CatalogItem {
    let primary = &records[0];
    let id_str = primary.item_id.to_string();
    let has_primary = records.iter().any(|r| r.work_poster || r.thumb);
    let has_backdrop = records.iter().any(|r| r.work_backdrop);
    let has_thumb = records.iter().any(|r| r.thumb);
    let (image_tags, backdrop_tags) =
        jellyfin_image_tags(has_primary, has_thumb, has_backdrop, &id_str);
    let favorite = primary.work_favorite || primary.state.favorite;
    let studios: Vec<Value> = primary
        .work_studios
        .iter()
        .map(|s| json!({ "Name": s }))
        .collect();
    let mut dto = json!({
        "Name": primary.work_title,
        "ServerId": server_id,
        "Id": id_str,
        "Type": "Movie",
        "MediaType": "Video",
        "IsFolder": false,
        "ParentId": primary.library_id.to_string(),
        "OriginalTitle": primary.work_original_title,
        "Overview": primary.work_overview.as_ref().or(primary.overview.as_ref()),
        "ProductionYear": primary.work_year,
        "PremiereDate": primary.work_premiered.as_ref().or(primary.premiered.as_ref()),
        "CommunityRating": primary.work_rating.or(primary.rating),
        "OfficialRating": primary.work_official_rating,
        "RunTimeTicks": primary.runtime_ms.map(ms_to_ticks),
        "DateCreated": primary.created_at.to_rfc3339(),
        "Genres": primary.genres,
        "Studios": studios,
        "ImageTags": image_tags,
        "BackdropImageTags": backdrop_tags,
        "UserData": jellyfin_user_data_json(&primary.state, Some(favorite), None)
    });
    if let Some(sources) = media_sources {
        dto["MediaSources"] = Value::Array(sources);
    }
    CatalogItem {
        id: id_str,
        item_type: "Movie",
        name: primary.work_title.clone(),
        sort_name: primary.work_title.to_lowercase(),
        original_title: primary.work_original_title.clone(),
        series_name: None,
        album: None,
        artists: Vec::new(),
        year: primary.work_year,
        rating: primary.work_rating.or(primary.rating),
        premiered: primary
            .work_premiered
            .clone()
            .or_else(|| primary.premiered.clone()),
        created_at: primary.created_at,
        last_played_at: primary.state.last_played_at,
        played: primary.state.completed,
        resumable: !primary.state.completed && primary.state.position_ms > 0,
        favorite,
        json: dto,
    }
}

fn build_series_catalog_item(server_id: &str, records: &[JellyfinVideoRecord]) -> CatalogItem {
    let primary = &records[0];
    let seasons: HashSet<i32> = records.iter().filter_map(|r| r.season).collect();
    let total = records.len() as u64;
    let completed = records.iter().filter(|r| r.state.completed).count() as u64;
    let unplayed = total.saturating_sub(completed);
    let max_created = records
        .iter()
        .map(|r| r.created_at)
        .max()
        .unwrap_or(primary.created_at);
    let last_played_at = records.iter().filter_map(|r| r.state.last_played_at).max();
    let play_count: i64 = records.iter().map(|r| r.state.play_count).sum();
    let favorite = records.iter().any(|r| r.work_favorite);
    let summary_state = MediaState {
        favorite,
        stars: 0,
        position_ms: 0,
        completed: total > 0 && unplayed == 0,
        play_count,
        last_played_at,
        playback_version: 0,
    };
    let has_primary = records.iter().any(|r| r.work_poster || r.season_poster);
    let has_backdrop = records.iter().any(|r| r.work_backdrop);
    let (image_tags, backdrop_tags) =
        jellyfin_image_tags(has_primary, false, has_backdrop, &primary.work_key);
    let studios: Vec<Value> = primary
        .work_studios
        .iter()
        .map(|s| json!({ "Name": s }))
        .collect();
    let dto = json!({
        "Name": primary.work_title,
        "ServerId": server_id,
        "Id": primary.work_key,
        "Type": "Series",
        "IsFolder": true,
        "ParentId": primary.library_id.to_string(),
        "OriginalTitle": primary.work_original_title,
        "Overview": primary.work_overview,
        "ProductionYear": primary.work_year,
        "PremiereDate": primary.work_premiered,
        "CommunityRating": primary.work_rating,
        "OfficialRating": primary.work_official_rating,
        "ChildCount": seasons.len(),
        "RecursiveItemCount": records.len(),
        "DateCreated": max_created.to_rfc3339(),
        "Genres": primary.genres,
        "Studios": studios,
        "ImageTags": image_tags,
        "BackdropImageTags": backdrop_tags,
        "UserData": jellyfin_user_data_json(&summary_state, Some(favorite), Some(unplayed))
    });
    CatalogItem {
        id: primary.work_key.clone(),
        item_type: "Series",
        name: primary.work_title.clone(),
        sort_name: primary.work_title.to_lowercase(),
        original_title: primary.work_original_title.clone(),
        series_name: Some(primary.work_title.clone()),
        album: None,
        artists: Vec::new(),
        year: primary.work_year,
        rating: primary.work_rating,
        premiered: primary.work_premiered.clone(),
        created_at: max_created,
        last_played_at,
        played: summary_state.completed,
        resumable: false,
        favorite,
        json: dto,
    }
}

fn build_season_catalog_item(
    server_id: &str,
    work_key: &str,
    season_num: i32,
    episodes: &[JellyfinVideoRecord],
) -> CatalogItem {
    let first = &episodes[0];
    let season_id = encode_season_id(work_key, season_num);
    let total = episodes.len() as u64;
    let completed = episodes.iter().filter(|r| r.state.completed).count() as u64;
    let unplayed = total.saturating_sub(completed);
    let max_created = episodes
        .iter()
        .map(|r| r.created_at)
        .max()
        .unwrap_or(first.created_at);
    let last_played_at = episodes.iter().filter_map(|r| r.state.last_played_at).max();
    let name = if season_num == 0 {
        "Specials".to_owned()
    } else {
        format!("Season {season_num}")
    };
    let summary_state = MediaState {
        favorite: false,
        stars: 0,
        position_ms: 0,
        completed: total > 0 && unplayed == 0,
        play_count: episodes.iter().map(|r| r.state.play_count).sum(),
        last_played_at,
        playback_version: 0,
    };
    let has_primary = episodes.iter().any(|r| r.season_poster || r.work_poster);
    let (image_tags, _) = jellyfin_image_tags(has_primary, false, false, &season_id);
    let dto = json!({
        "Name": name,
        "ServerId": server_id,
        "Id": season_id,
        "Type": "Season",
        "IsFolder": true,
        "IndexNumber": season_num,
        "SeriesId": work_key,
        "SeriesName": first.work_title,
        "ParentId": work_key,
        "ChildCount": episodes.len(),
        "DateCreated": max_created.to_rfc3339(),
        "ImageTags": image_tags,
        "UserData": jellyfin_user_data_json(&summary_state, None, Some(unplayed))
    });
    CatalogItem {
        id: season_id,
        item_type: "Season",
        name: name.clone(),
        sort_name: format!("{season_num:06}"),
        original_title: None,
        series_name: Some(first.work_title.clone()),
        album: None,
        artists: Vec::new(),
        year: first.work_year,
        rating: first.work_rating,
        premiered: first.work_premiered.clone(),
        created_at: max_created,
        last_played_at,
        played: summary_state.completed,
        resumable: false,
        favorite: false,
        json: dto,
    }
}

fn build_episode_catalog_item(
    server_id: &str,
    rec: &JellyfinVideoRecord,
    media_sources: Option<Vec<Value>>,
) -> CatalogItem {
    let id_str = rec.item_id.to_string();
    let season_num = rec.season.unwrap_or(1);
    let season_id = encode_season_id(&rec.work_key, season_num);
    let has_primary = rec.thumb || rec.episode_poster || rec.season_poster || rec.work_poster;
    let (image_tags, _) = jellyfin_image_tags(has_primary, rec.thumb, false, &id_str);
    let mut dto = json!({
        "Name": rec.title,
        "ServerId": server_id,
        "Id": id_str,
        "Type": "Episode",
        "MediaType": "Video",
        "IsFolder": false,
        "SeriesId": rec.work_key,
        "SeriesName": rec.work_title,
        "SeasonId": season_id,
        "SeasonName": if season_num == 0 { "Specials".to_owned() } else { format!("Season {season_num}") },
        "ParentId": season_id,
        "ParentIndexNumber": season_num,
        "IndexNumber": rec.episode,
        "Overview": rec.overview,
        "PremiereDate": rec.premiered,
        "CommunityRating": rec.rating,
        "RunTimeTicks": rec.runtime_ms.map(ms_to_ticks),
        "DateCreated": rec.created_at.to_rfc3339(),
        "ImageTags": image_tags,
        "UserData": jellyfin_user_data_json(&rec.state, None, None)
    });
    if let Some(sources) = media_sources {
        dto["MediaSources"] = Value::Array(sources);
    }
    CatalogItem {
        id: id_str,
        item_type: "Episode",
        name: rec.title.clone(),
        sort_name: format!(
            "{} {:04} {:04} {}",
            rec.work_title.to_lowercase(),
            season_num,
            rec.episode.unwrap_or(0),
            rec.title.to_lowercase()
        ),
        original_title: rec.work_original_title.clone(),
        series_name: Some(rec.work_title.clone()),
        album: None,
        artists: Vec::new(),
        year: rec.work_year,
        rating: rec.rating.or(rec.work_rating),
        premiered: rec.premiered.clone(),
        created_at: rec.created_at,
        last_played_at: rec.state.last_played_at,
        played: rec.state.completed,
        resumable: !rec.state.completed && rec.state.position_ms > 0,
        favorite: rec.state.favorite,
        json: dto,
    }
}

fn build_audio_catalog_item(
    server_id: &str,
    rec: &JellyfinAudioRecord,
    media_sources: Option<Vec<Value>>,
) -> CatalogItem {
    let id_str = rec.item_id.to_string();
    let artists: Vec<String> = rec.artist.iter().cloned().collect();
    let (image_tags, _) = jellyfin_image_tags(true, false, false, &id_str);
    let mut dto = json!({
        "Name": rec.title,
        "ServerId": server_id,
        "Id": id_str,
        "Type": "Audio",
        "MediaType": "Audio",
        "IsFolder": false,
        "ParentId": rec.library_id.to_string(),
        "Album": rec.album,
        "AlbumArtist": rec.album_artist.as_ref().or(rec.artist.as_ref()),
        "Artists": artists,
        "IndexNumber": rec.track_number,
        "ParentIndexNumber": rec.disc_number,
        "ProductionYear": rec.year,
        "RunTimeTicks": rec.duration_ms.map(ms_to_ticks),
        "DateCreated": rec.created_at.to_rfc3339(),
        "ImageTags": image_tags,
        "UserData": jellyfin_user_data_json(&rec.state, None, None)
    });
    if let Some(sources) = media_sources {
        dto["MediaSources"] = Value::Array(sources);
    }
    CatalogItem {
        id: id_str,
        item_type: "Audio",
        name: rec.title.clone(),
        sort_name: rec.title.to_lowercase(),
        original_title: None,
        series_name: None,
        album: rec.album.clone(),
        artists,
        year: rec.year.and_then(|y| i32::try_from(y).ok()),
        rating: None,
        premiered: None,
        created_at: rec.created_at,
        last_played_at: rec.state.last_played_at,
        played: rec.state.completed,
        resumable: !rec.state.completed && rec.state.position_ms > 0,
        favorite: rec.state.favorite,
        json: dto,
    }
}

fn build_library_view_json(server_id: &str, lib: &Library) -> Value {
    let collection_type = match lib.kind {
        LibraryKind::Video => "movies",
        LibraryKind::Music | LibraryKind::Audiobook | LibraryKind::Podcast => "music",
        LibraryKind::Photo => "homevideos",
        LibraryKind::Book | LibraryKind::Comic => "books",
    };
    json!({
        "Name": lib.name,
        "ServerId": server_id,
        "Id": lib.id.to_string(),
        "Type": "CollectionFolder",
        "CollectionType": collection_type,
        "IsFolder": true,
        "DateCreated": lib.created_at.to_rfc3339(),
        "ImageTags": {},
        "BackdropImageTags": []
    })
}

// ===========================================================================
// System & Authentication Endpoints
// ===========================================================================

/// `GET /System/Info/Public`
async fn system_info_public(State(state): State<AppState>) -> Json<Value> {
    let name = effective_server_name(&state).await;
    Json(json!({
        "LocalAddress": "",
        "ServerName": name,
        "Version": state.server.version,
        "ProductName": "Palveni",
        "OperatingSystem": std::env::consts::OS,
        "Id": state.server.id.to_string(),
        "StartupWizardCompleted": true
    }))
}

/// `GET /System/Info`
async fn system_info(
    State(state): State<AppState>,
    headers: HeaderMap,
    OriginalUri(uri): OriginalUri,
) -> Result<Json<Value>, ApiError> {
    let query = QueryParams::from_uri(&uri);
    let _principal = authenticate_jellyfin(&state, &headers, &query).await?;
    let name = effective_server_name(&state).await;
    Ok(Json(json!({
        "LocalAddress": "",
        "ServerName": name,
        "Version": state.server.version,
        "ProductName": "Palveni",
        "OperatingSystem": std::env::consts::OS,
        "Id": state.server.id.to_string(),
        "StartupWizardCompleted": true,
        "SupportsLibraryMonitor": true,
        "HasPendingRestart": false,
        "IsShuttingDown": false,
        "CanSelfRestart": false,
        "CanLaunchWebBrowser": false,
        "WebSocketPortNumber": 0
    })))
}

/// `GET /System/Ping` & `POST /System/Ping`
async fn system_ping() -> Json<&'static str> {
    Json("Palveni")
}

#[derive(Deserialize)]
struct AuthenticateByNameRequest {
    #[serde(alias = "Username", alias = "username")]
    username: String,
    #[serde(
        default,
        alias = "Pw",
        alias = "pw",
        alias = "Password",
        alias = "password"
    )]
    pw: String,
}

/// `POST /Users/AuthenticateByName`
async fn authenticate_by_name(
    State(state): State<AppState>,
    headers: HeaderMap,
    OriginalUri(uri): OriginalUri,
    body: Result<ApiJson<AuthenticateByNameRequest>, ApiError>,
) -> Result<Json<Value>, ApiError> {
    let ApiJson(req) = body?;
    let query = QueryParams::from_uri(&uri);
    let (meta, _) = extract_auth_meta_and_token(&headers, &query);
    let attempt = state
        .login_limit
        .begin(IpAddr::V4(Ipv4Addr::LOCALHOST), &req.username)
        .await?;

    let user = if req.pw.chars().count() > PASSWORD_MAX_CHARS {
        None
    } else {
        match state.db.users().find_credentials(&req.username).await? {
            Some(found) if !found.disabled => verify_password(req.pw, found.password_hash)
                .await?
                .then_some(found.user),
            _ => {
                if let Some(dummy) = dummy_hash().await {
                    let _ignored = verify_password(req.pw, dummy).await;
                }
                None
            }
        }
    };
    attempt.finish(user.is_some());

    let Some(user) = user else {
        return Err(ApiError::invalid_credentials());
    };

    let device_name = meta
        .device
        .clone()
        .or_else(|| meta.client.clone())
        .unwrap_or_else(|| "Jellyfin Client".to_owned());
    let issued = state
        .db
        .sessions()
        .create(user.id, &device_name, None)
        .await?;
    let policy = state.db.accounts().get_policy(user.id).await?;
    let allowed = accessible_media_libraries(&state, &user).await?;
    let server_id = state.server.id.to_string();
    let user_dto = jellyfin_user_json(&server_id, &user, policy.all_libraries, &allowed);

    Ok(Json(json!({
        "User": user_dto,
        "SessionInfo": {
            "Id": issued.session_id.to_string(),
            "UserId": user.id.to_string(),
            "UserName": user.username,
            "Client": meta.client.unwrap_or_else(|| "Jellyfin".to_owned()),
            "DeviceName": device_name,
            "DeviceId": meta.device_id.unwrap_or_else(|| issued.device_id.to_string()),
            "ServerId": server_id
        },
        "AccessToken": issued.token,
        "ServerId": server_id
    })))
}

/// `GET /Users/Me`
async fn user_me(
    State(state): State<AppState>,
    headers: HeaderMap,
    OriginalUri(uri): OriginalUri,
) -> Result<Json<Value>, ApiError> {
    let query = QueryParams::from_uri(&uri);
    let principal = authenticate_jellyfin(&state, &headers, &query).await?;
    let policy = state.db.accounts().get_policy(principal.user.id).await?;
    let allowed = accessible_media_libraries(&state, &principal.user).await?;
    let server_id = state.server.id.to_string();
    Ok(Json(jellyfin_user_json(
        &server_id,
        &principal.user,
        policy.all_libraries,
        &allowed,
    )))
}

/// `GET /Users/{userId}`
async fn user_by_id(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(raw_user_id): Path<String>,
    OriginalUri(uri): OriginalUri,
) -> Result<Json<Value>, ApiError> {
    let query = QueryParams::from_uri(&uri);
    let principal = authenticate_jellyfin(&state, &headers, &query).await?;
    let target_id = resolve_target_user_id(&principal, &raw_user_id)?;
    let target_user = if target_id == principal.user.id {
        principal.user.clone()
    } else {
        let managed = state
            .db
            .accounts()
            .get_user(target_id)
            .await?
            .ok_or_else(|| ApiError::not_found("User not found"))?;
        User {
            id: managed.id,
            username: managed.username,
            role: managed.role,
            created_at: managed.created_at,
        }
    };
    let policy = state.db.accounts().get_policy(target_user.id).await?;
    let allowed = accessible_media_libraries(&state, &target_user).await?;
    let server_id = state.server.id.to_string();
    Ok(Json(jellyfin_user_json(
        &server_id,
        &target_user,
        policy.all_libraries,
        &allowed,
    )))
}

// ===========================================================================
// Views, Items, Shows & Detail Endpoints
// ===========================================================================

/// `GET /Users/{userId}/Views`
async fn user_views(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(raw_user_id): Path<String>,
    OriginalUri(uri): OriginalUri,
) -> Result<Json<Value>, ApiError> {
    let query = QueryParams::from_uri(&uri);
    let principal = authenticate_jellyfin(&state, &headers, &query).await?;
    let _target_id = resolve_target_user_id(&principal, &raw_user_id)?;
    let libs = accessible_media_libraries(&state, &principal.user).await?;
    let server_id = state.server.id.to_string();
    let items: Vec<Value> = libs
        .iter()
        .map(|lib| build_library_view_json(&server_id, lib))
        .collect();
    let total = items.len();
    Ok(Json(json!({
        "Items": items,
        "TotalRecordCount": total,
        "StartIndex": 0
    })))
}

/// `GET /Users/{userId}/Items`
async fn user_items(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(raw_user_id): Path<String>,
    OriginalUri(uri): OriginalUri,
) -> Result<Json<Value>, ApiError> {
    let query = QueryParams::from_uri(&uri);
    let principal = authenticate_jellyfin(&state, &headers, &query).await?;
    let target_id = resolve_target_user_id(&principal, &raw_user_id)?;
    query_items(&state, &principal, target_id, &query).await
}

/// `GET /Items`
async fn items_root(
    State(state): State<AppState>,
    headers: HeaderMap,
    OriginalUri(uri): OriginalUri,
) -> Result<Json<Value>, ApiError> {
    let query = QueryParams::from_uri(&uri);
    let principal = authenticate_jellyfin(&state, &headers, &query).await?;
    let target_id = match query.get("userid") {
        Some(raw) => resolve_target_user_id(&principal, raw)?,
        None => principal.user.id,
    };
    query_items(&state, &principal, target_id, &query).await
}

async fn query_items(
    state: &AppState,
    principal: &JellyfinPrincipal,
    user_id: UserId,
    query: &QueryParams,
) -> Result<Json<Value>, ApiError> {
    let server_id = state.server.id.to_string();
    let allowed_libs = accessible_media_libraries(state, &principal.user).await?;
    let allowed_lib_ids: HashSet<LibraryId> = allowed_libs.iter().map(|l| l.id).collect();
    let recursive = query.get_bool("recursive").unwrap_or(false);
    let include_types: HashSet<String> = query
        .get("includeitemtypes")
        .map(|raw| {
            raw.split(',')
                .map(|s| s.trim().to_ascii_lowercase())
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default();

    let mut catalog: Vec<CatalogItem> = Vec::new();

    if let Some(parent_raw) = query.get("parentid") {
        if let Some((work_key, season_num)) = parse_season_id(parent_raw) {
            let records = state
                .db
                .jellyfin()
                .list_video_records_for_work(user_id, work_key)
                .await?;
            for rec in records
                .into_iter()
                .filter(|r| allowed_lib_ids.contains(&r.library_id) && r.season == Some(season_num))
            {
                catalog.push(build_episode_catalog_item(&server_id, &rec, None));
            }
        } else if is_hex64(parent_raw) {
            let records: Vec<JellyfinVideoRecord> = state
                .db
                .jellyfin()
                .list_video_records_for_work(user_id, parent_raw)
                .await?
                .into_iter()
                .filter(|r| allowed_lib_ids.contains(&r.library_id))
                .collect();
            if recursive || include_types.contains("episode") {
                for rec in &records {
                    catalog.push(build_episode_catalog_item(&server_id, rec, None));
                }
            } else {
                let mut by_season: BTreeMap<i32, Vec<JellyfinVideoRecord>> = BTreeMap::new();
                for rec in records {
                    by_season
                        .entry(rec.season.unwrap_or(1))
                        .or_default()
                        .push(rec);
                }
                for (season_num, eps) in by_season {
                    catalog.push(build_season_catalog_item(
                        &server_id, parent_raw, season_num, &eps,
                    ));
                }
            }
        } else if let Some(lib_id) = parse_library_id(parent_raw)
            && let Some(lib) = allowed_libs.iter().find(|l| l.id == lib_id)
        {
            collect_library_catalog(
                state,
                &server_id,
                user_id,
                std::slice::from_ref(lib),
                recursive,
                &include_types,
                &mut catalog,
            )
            .await?;
        }
    } else {
        collect_library_catalog(
            state,
            &server_id,
            user_id,
            &allowed_libs,
            recursive,
            &include_types,
            &mut catalog,
        )
        .await?;
    }

    // Apply IncludeItemTypes filter
    if !include_types.is_empty() {
        catalog.retain(|item| include_types.contains(&item.item_type.to_ascii_lowercase()));
    }

    // Apply SearchTerm filter
    if let Some(term) = query.get("searchterm").map(|s| s.trim().to_lowercase())
        && !term.is_empty()
    {
        catalog.retain(|item| {
            item.name.to_lowercase().contains(&term)
                || item
                    .original_title
                    .as_deref()
                    .is_some_and(|t| t.to_lowercase().contains(&term))
                || item
                    .series_name
                    .as_deref()
                    .is_some_and(|t| t.to_lowercase().contains(&term))
                || item
                    .album
                    .as_deref()
                    .is_some_and(|t| t.to_lowercase().contains(&term))
                || item
                    .artists
                    .iter()
                    .any(|a| a.to_lowercase().contains(&term))
        });
    }

    // Apply IsFavorite filter
    if let Some(fav) = query.get_bool("isfavorite") {
        catalog.retain(|item| item.favorite == fav);
    }

    // Apply Filters (IsUnplayed, IsPlayed, IsResumable)
    if let Some(filters_raw) = query.get("filters") {
        for flag in filters_raw
            .split(',')
            .map(|s| s.trim().to_ascii_lowercase())
        {
            match flag.as_str() {
                "isplayed" => catalog.retain(|item| item.played),
                "isunplayed" => catalog.retain(|item| !item.played),
                "isresumable" => catalog.retain(|item| item.resumable),
                "isfavorite" => catalog.retain(|item| item.favorite),
                _ => {}
            }
        }
    }

    // Sorting
    let sort_by = query
        .get("sortby")
        .and_then(|s| s.split(',').next())
        .map(str::trim)
        .unwrap_or("SortName")
        .to_ascii_lowercase();
    let desc = query
        .get("sortorder")
        .is_some_and(|s| s.eq_ignore_ascii_case("descending"));
    sort_catalog(&mut catalog, &sort_by, desc);

    let total = catalog.len();
    let start_index = query.get_usize("startindex").unwrap_or(0);
    let limit = query.get_usize("limit").unwrap_or(total.max(1));
    let page: Vec<Value> = catalog
        .into_iter()
        .skip(start_index)
        .take(limit)
        .map(|item| item.json)
        .collect();

    Ok(Json(json!({
        "Items": page,
        "TotalRecordCount": total,
        "StartIndex": start_index
    })))
}

async fn collect_library_catalog(
    state: &AppState,
    server_id: &str,
    user_id: UserId,
    libs: &[Library],
    recursive: bool,
    include_types: &HashSet<String>,
    out: &mut Vec<CatalogItem>,
) -> Result<(), ApiError> {
    let video_libs: Vec<LibraryId> = libs
        .iter()
        .filter(|l| l.kind == LibraryKind::Video)
        .map(|l| l.id)
        .collect();
    let audio_libs: Vec<LibraryId> = libs
        .iter()
        .filter(|l| {
            matches!(
                l.kind,
                LibraryKind::Music | LibraryKind::Audiobook | LibraryKind::Podcast
            )
        })
        .map(|l| l.id)
        .collect();

    if !video_libs.is_empty() {
        let records = state
            .db
            .jellyfin()
            .list_video_records(user_id, &video_libs)
            .await?;
        let mut by_work: BTreeMap<String, Vec<JellyfinVideoRecord>> = BTreeMap::new();
        for rec in records {
            by_work.entry(rec.work_key.clone()).or_default().push(rec);
        }
        let want_seasons = include_types.contains("season");
        let want_episodes = include_types.contains("episode") || recursive;
        for (work_key, work_records) in by_work {
            let is_series = work_records.iter().any(|r| r.season.is_some());
            if is_series {
                out.push(build_series_catalog_item(server_id, &work_records));
                if want_seasons {
                    let mut by_season: BTreeMap<i32, Vec<JellyfinVideoRecord>> = BTreeMap::new();
                    for rec in &work_records {
                        by_season
                            .entry(rec.season.unwrap_or(1))
                            .or_default()
                            .push(rec.clone());
                    }
                    for (season_num, eps) in by_season {
                        out.push(build_season_catalog_item(
                            server_id, &work_key, season_num, &eps,
                        ));
                    }
                }
                if want_episodes {
                    for rec in &work_records {
                        out.push(build_episode_catalog_item(server_id, rec, None));
                    }
                }
            } else {
                out.push(build_movie_catalog_item(server_id, &work_records, None));
            }
        }
    }

    if !audio_libs.is_empty() && (include_types.is_empty() || include_types.contains("audio")) {
        let tracks = state
            .db
            .jellyfin()
            .list_audio_records(user_id, &audio_libs)
            .await?;
        for rec in &tracks {
            out.push(build_audio_catalog_item(server_id, rec, None));
        }
    }

    Ok(())
}

fn sort_catalog(items: &mut [CatalogItem], sort_by: &str, desc: bool) {
    items.sort_by(|a, b| {
        let ord = match sort_by {
            "datecreated" => a.created_at.cmp(&b.created_at),
            "productionyear" => a.year.cmp(&b.year),
            "communityrating" => a.rating.unwrap_or(0.0).total_cmp(&b.rating.unwrap_or(0.0)),
            "premieredate" => a.premiered.cmp(&b.premiered),
            "dateplayed" => a.last_played_at.cmp(&b.last_played_at),
            _ => a.sort_name.cmp(&b.sort_name),
        }
        .then_with(|| a.id.cmp(&b.id));
        if desc { ord.reverse() } else { ord }
    });
}

/// `GET /Users/{userId}/Items/Latest` (returns a raw JSON array per Jellyfin spec)
async fn user_items_latest(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(raw_user_id): Path<String>,
    OriginalUri(uri): OriginalUri,
) -> Result<Json<Value>, ApiError> {
    let query = QueryParams::from_uri(&uri);
    let principal = authenticate_jellyfin(&state, &headers, &query).await?;
    let target_id = resolve_target_user_id(&principal, &raw_user_id)?;
    let server_id = state.server.id.to_string();
    let mut libs = accessible_media_libraries(&state, &principal.user).await?;
    if let Some(parent_raw) = query.get("parentid") {
        if let Some(lib_id) = parse_library_id(parent_raw) {
            libs.retain(|l| l.id == lib_id);
        } else {
            libs.clear();
        }
    }
    let include_types: HashSet<String> = query
        .get("includeitemtypes")
        .map(|raw| {
            raw.split(',')
                .map(|s| s.trim().to_ascii_lowercase())
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default();
    let mut catalog = Vec::new();
    collect_library_catalog(
        &state,
        &server_id,
        target_id,
        &libs,
        false,
        &include_types,
        &mut catalog,
    )
    .await?;
    if !include_types.is_empty() {
        catalog.retain(|item| include_types.contains(&item.item_type.to_ascii_lowercase()));
    }
    sort_catalog(&mut catalog, "datecreated", true);
    let limit = query.get_usize("limit").unwrap_or(DEFAULT_LATEST_LIMIT);
    let out: Vec<Value> = catalog.into_iter().take(limit).map(|i| i.json).collect();
    Ok(Json(Value::Array(out)))
}

/// `GET /Users/{userId}/Items/Resume`
async fn user_items_resume(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(raw_user_id): Path<String>,
    OriginalUri(uri): OriginalUri,
) -> Result<Json<Value>, ApiError> {
    let query = QueryParams::from_uri(&uri);
    let principal = authenticate_jellyfin(&state, &headers, &query).await?;
    let target_id = resolve_target_user_id(&principal, &raw_user_id)?;
    let server_id = state.server.id.to_string();
    let mut libs = accessible_media_libraries(&state, &principal.user).await?;
    if let Some(parent_raw) = query.get("parentid") {
        if let Some(lib_id) = parse_library_id(parent_raw) {
            libs.retain(|l| l.id == lib_id);
        } else {
            libs.clear();
        }
    }

    let video_libs: Vec<LibraryId> = libs
        .iter()
        .filter(|l| l.kind == LibraryKind::Video)
        .map(|l| l.id)
        .collect();
    let audio_libs: Vec<LibraryId> = libs
        .iter()
        .filter(|l| {
            matches!(
                l.kind,
                LibraryKind::Music | LibraryKind::Audiobook | LibraryKind::Podcast
            )
        })
        .map(|l| l.id)
        .collect();

    let mut resumable = Vec::new();
    for rec in state
        .db
        .jellyfin()
        .list_video_records(target_id, &video_libs)
        .await?
    {
        if !rec.state.completed && rec.state.position_ms > 0 {
            if rec.season.is_some() {
                resumable.push(build_episode_catalog_item(&server_id, &rec, None));
            } else {
                resumable.push(build_movie_catalog_item(&server_id, &[rec], None));
            }
        }
    }
    for rec in state
        .db
        .jellyfin()
        .list_audio_records(target_id, &audio_libs)
        .await?
    {
        if !rec.state.completed && rec.state.position_ms > 0 {
            resumable.push(build_audio_catalog_item(&server_id, &rec, None));
        }
    }
    sort_catalog(&mut resumable, "dateplayed", true);

    let total = resumable.len();
    let start_index = query.get_usize("startindex").unwrap_or(0);
    let limit = query.get_usize("limit").unwrap_or(DEFAULT_LATEST_LIMIT);
    let page: Vec<Value> = resumable
        .into_iter()
        .skip(start_index)
        .take(limit)
        .map(|i| i.json)
        .collect();

    Ok(Json(json!({
        "Items": page,
        "TotalRecordCount": total,
        "StartIndex": start_index
    })))
}

/// `GET /Shows/NextUp`
async fn shows_next_up(
    State(state): State<AppState>,
    headers: HeaderMap,
    OriginalUri(uri): OriginalUri,
) -> Result<Json<Value>, ApiError> {
    let query = QueryParams::from_uri(&uri);
    let principal = authenticate_jellyfin(&state, &headers, &query).await?;
    let target_id = match query.get("userid") {
        Some(raw) => resolve_target_user_id(&principal, raw)?,
        None => principal.user.id,
    };
    let server_id = state.server.id.to_string();
    let mut libs = accessible_media_libraries(&state, &principal.user).await?;
    libs.retain(|l| l.kind == LibraryKind::Video);
    if let Some(parent_raw) = query.get("parentid") {
        if let Some(lib_id) = parse_library_id(parent_raw) {
            libs.retain(|l| l.id == lib_id);
        } else {
            libs.clear();
        }
    }
    let series_filter = query.get("seriesid");

    let mut next_items: Vec<Value> = Vec::new();
    for lib in libs {
        if let Some(home) = state.db.video().home(target_id, Some(lib.id)).await? {
            for entry in home.next_up {
                if let Some(filter) = series_filter
                    && entry.series_id.as_deref() != Some(filter)
                {
                    continue;
                }
                if let Some(item_id) = entry.item_id
                    && let Some(rec) = state
                        .db
                        .jellyfin()
                        .get_video_record(target_id, item_id)
                        .await?
                {
                    next_items.push(build_episode_catalog_item(&server_id, &rec, None).json);
                }
            }
        }
    }

    let total = next_items.len();
    let start_index = query.get_usize("startindex").unwrap_or(0);
    let limit = query.get_usize("limit").unwrap_or(DEFAULT_NEXT_UP_LIMIT);
    let page: Vec<Value> = next_items
        .into_iter()
        .skip(start_index)
        .take(limit)
        .collect();

    Ok(Json(json!({
        "Items": page,
        "TotalRecordCount": total,
        "StartIndex": start_index
    })))
}

/// `GET /Shows/{seriesId}/Seasons`
async fn show_seasons(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(series_id): Path<String>,
    OriginalUri(uri): OriginalUri,
) -> Result<Json<Value>, ApiError> {
    let query = QueryParams::from_uri(&uri);
    let principal = authenticate_jellyfin(&state, &headers, &query).await?;
    let target_id = match query.get("userid") {
        Some(raw) => resolve_target_user_id(&principal, raw)?,
        None => principal.user.id,
    };
    let records = state
        .db
        .jellyfin()
        .list_video_records_for_work(target_id, &series_id)
        .await?;
    if records.is_empty() {
        return Err(ApiError::not_found("Series not found"));
    }
    if !state
        .db
        .accounts()
        .can_access_library(
            principal.user.id,
            principal.user.role,
            records[0].library_id,
        )
        .await?
    {
        return Err(ApiError::forbidden());
    }
    let server_id = state.server.id.to_string();
    let mut by_season: BTreeMap<i32, Vec<JellyfinVideoRecord>> = BTreeMap::new();
    for rec in records {
        by_season
            .entry(rec.season.unwrap_or(1))
            .or_default()
            .push(rec);
    }
    let items: Vec<Value> = by_season
        .into_iter()
        .map(|(season_num, eps)| {
            build_season_catalog_item(&server_id, &series_id, season_num, &eps).json
        })
        .collect();
    let total = items.len();
    Ok(Json(json!({
        "Items": items,
        "TotalRecordCount": total,
        "StartIndex": 0
    })))
}

/// `GET /Shows/{seriesId}/Episodes`
async fn show_episodes(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(series_id): Path<String>,
    OriginalUri(uri): OriginalUri,
) -> Result<Json<Value>, ApiError> {
    let query = QueryParams::from_uri(&uri);
    let principal = authenticate_jellyfin(&state, &headers, &query).await?;
    let target_id = match query.get("userid") {
        Some(raw) => resolve_target_user_id(&principal, raw)?,
        None => principal.user.id,
    };
    let mut records = state
        .db
        .jellyfin()
        .list_video_records_for_work(target_id, &series_id)
        .await?;
    if records.is_empty() {
        return Err(ApiError::not_found("Series not found"));
    }
    if !state
        .db
        .accounts()
        .can_access_library(
            principal.user.id,
            principal.user.role,
            records[0].library_id,
        )
        .await?
    {
        return Err(ApiError::forbidden());
    }
    let season_filter = query.get_i32("season").or_else(|| {
        query
            .get("seasonid")
            .and_then(parse_season_id)
            .map(|(_, n)| n)
    });
    if let Some(s) = season_filter {
        records.retain(|r| r.season.unwrap_or(1) == s);
    }
    records.sort_by(|a, b| {
        a.season
            .unwrap_or(0)
            .cmp(&b.season.unwrap_or(0))
            .then_with(|| a.episode.unwrap_or(0).cmp(&b.episode.unwrap_or(0)))
            .then_with(|| a.title.cmp(&b.title))
    });
    let server_id = state.server.id.to_string();
    let total = records.len();
    let start_index = query.get_usize("startindex").unwrap_or(0);
    let limit = query.get_usize("limit").unwrap_or(total.max(1));
    let items: Vec<Value> = records
        .into_iter()
        .skip(start_index)
        .take(limit)
        .map(|rec| build_episode_catalog_item(&server_id, &rec, None).json)
        .collect();
    Ok(Json(json!({
        "Items": items,
        "TotalRecordCount": total,
        "StartIndex": start_index
    })))
}

/// `GET /Users/{userId}/Items/{itemId}`
async fn user_item_detail(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((raw_user_id, raw_item_id)): Path<(String, String)>,
    OriginalUri(uri): OriginalUri,
) -> Result<Json<Value>, ApiError> {
    let query = QueryParams::from_uri(&uri);
    let principal = authenticate_jellyfin(&state, &headers, &query).await?;
    let target_id = resolve_target_user_id(&principal, &raw_user_id)?;
    let server_id = state.server.id.to_string();

    // 1. Season ID
    if let Some((work_key, season_num)) = parse_season_id(&raw_item_id) {
        let records = state
            .db
            .jellyfin()
            .list_video_records_for_work(target_id, work_key)
            .await?;
        let eps: Vec<JellyfinVideoRecord> = records
            .into_iter()
            .filter(|r| r.season == Some(season_num))
            .collect();
        let Some(first) = eps.first() else {
            return Err(ApiError::not_found("Season not found"));
        };
        if !state
            .db
            .accounts()
            .can_access_library(principal.user.id, principal.user.role, first.library_id)
            .await?
        {
            return Err(ApiError::forbidden());
        }
        return Ok(Json(
            build_season_catalog_item(&server_id, work_key, season_num, &eps).json,
        ));
    }

    // 2. Series work_key (64-hex)
    if is_hex64(&raw_item_id) {
        let records = state
            .db
            .jellyfin()
            .list_video_records_for_work(target_id, &raw_item_id)
            .await?;
        let Some(first) = records.first() else {
            return Err(ApiError::not_found("Series not found"));
        };
        if !state
            .db
            .accounts()
            .can_access_library(principal.user.id, principal.user.role, first.library_id)
            .await?
        {
            return Err(ApiError::forbidden());
        }
        return Ok(Json(build_series_catalog_item(&server_id, &records).json));
    }

    // 3. Library ID
    if let Some(lib_id) = parse_library_id(&raw_item_id)
        && let Some(lib) = state.db.libraries().get(lib_id).await?
    {
        if !state
            .db
            .accounts()
            .can_access_library(principal.user.id, principal.user.role, lib.id)
            .await?
        {
            return Err(ApiError::forbidden());
        }
        return Ok(Json(build_library_view_json(&server_id, &lib)));
    }

    // 4. Media Item ID (Video or Audio)
    let item_id =
        parse_item_id(&raw_item_id).ok_or_else(|| ApiError::not_found("Item not found"))?;
    if let Some(rec) = state
        .db
        .jellyfin()
        .get_video_record(target_id, item_id)
        .await?
    {
        if !state
            .db
            .accounts()
            .can_access_library(principal.user.id, principal.user.role, rec.library_id)
            .await?
        {
            return Err(ApiError::forbidden());
        }
        if rec.season.is_some() {
            let source = build_media_source_json(
                &state,
                MediaSourceSpec {
                    item_id: rec.item_id,
                    name: &rec.title,
                    path: rec.path.as_deref(),
                    size_bytes: rec.size_bytes,
                    runtime_ms: rec.runtime_ms,
                    is_audio: false,
                },
                &principal.token,
            )
            .await;
            return Ok(Json(
                build_episode_catalog_item(&server_id, &rec, Some(vec![source])).json,
            ));
        }
        // Movie: attach all file versions in MediaSources with requested item first
        let mut versions = state
            .db
            .jellyfin()
            .list_video_records_for_work(target_id, &rec.work_key)
            .await?;
        if versions.is_empty() {
            versions.push(rec.clone());
        } else if let Some(pos) = versions.iter().position(|v| v.item_id == rec.item_id) {
            versions.swap(0, pos);
        }
        let mut sources = Vec::with_capacity(versions.len());
        for v in &versions {
            sources.push(
                build_media_source_json(
                    &state,
                    MediaSourceSpec {
                        item_id: v.item_id,
                        name: &v.title,
                        path: v.path.as_deref(),
                        size_bytes: v.size_bytes,
                        runtime_ms: v.runtime_ms,
                        is_audio: false,
                    },
                    &principal.token,
                )
                .await,
            );
        }
        return Ok(Json(
            build_movie_catalog_item(&server_id, &versions, Some(sources)).json,
        ));
    }

    if let Some(rec) = state
        .db
        .jellyfin()
        .get_audio_record(target_id, item_id)
        .await?
    {
        if !state
            .db
            .accounts()
            .can_access_library(principal.user.id, principal.user.role, rec.library_id)
            .await?
        {
            return Err(ApiError::forbidden());
        }
        let source = build_media_source_json(
            &state,
            MediaSourceSpec {
                item_id: rec.item_id,
                name: &rec.title,
                path: rec.path.as_deref(),
                size_bytes: rec.size_bytes,
                runtime_ms: rec.duration_ms,
                is_audio: true,
            },
            &principal.token,
        )
        .await;
        return Ok(Json(
            build_audio_catalog_item(&server_id, &rec, Some(vec![source])).json,
        ));
    }

    Err(ApiError::not_found("Item not found"))
}

// ===========================================================================
// Images, PlaybackInfo & Streaming
// ===========================================================================

/// `GET /Items/{itemId}/Images/{imageType}`
async fn item_image(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((raw_item_id, image_type)): Path<(String, String)>,
    OriginalUri(uri): OriginalUri,
) -> Result<Response, ApiError> {
    serve_item_image(&state, &headers, &uri, &raw_item_id, &image_type).await
}

/// `GET /Items/{itemId}/Images/{imageType}/{imageIndex}`
async fn item_image_indexed(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((raw_item_id, image_type, _index)): Path<(String, String, String)>,
    OriginalUri(uri): OriginalUri,
) -> Result<Response, ApiError> {
    serve_item_image(&state, &headers, &uri, &raw_item_id, &image_type).await
}

async fn serve_item_image(
    state: &AppState,
    headers: &HeaderMap,
    uri: &Uri,
    raw_item_id: &str,
    image_type: &str,
) -> Result<Response, ApiError> {
    let query = QueryParams::from_uri(uri);
    let principal = authenticate_jellyfin(state, headers, &query).await?;
    let size = query
        .get_u16("maxwidth")
        .or_else(|| query.get_u16("width"))
        .or_else(|| query.get_u16("fillwidth"))
        .filter(|&w| (1..=3840).contains(&w));
    let kind = image_type.to_ascii_lowercase();

    // Resolve representative ItemId and preferred PosterLevel
    let (target_item_id, preferred_level) =
        if let Some((work_key, season_num)) = parse_season_id(raw_item_id) {
            let records = state
                .db
                .jellyfin()
                .list_video_records_for_work(principal.user.id, work_key)
                .await?;
            let ep = records
                .iter()
                .find(|r| r.season == Some(season_num) && r.season_poster)
                .or_else(|| records.iter().find(|r| r.season == Some(season_num)))
                .ok_or_else(|| ApiError::not_found("Image not found"))?;
            if !state
                .db
                .accounts()
                .can_access_library(principal.user.id, principal.user.role, ep.library_id)
                .await?
            {
                return Err(ApiError::forbidden());
            }
            (ep.item_id, PosterLevel::Season)
        } else if is_hex64(raw_item_id) {
            let records = state
                .db
                .jellyfin()
                .list_video_records_for_work(principal.user.id, raw_item_id)
                .await?;
            let chosen = if kind == "backdrop" {
                records
                    .iter()
                    .find(|r| r.work_backdrop)
                    .or_else(|| records.first())
            } else {
                records
                    .iter()
                    .find(|r| r.work_poster)
                    .or_else(|| records.first())
            }
            .ok_or_else(|| ApiError::not_found("Image not found"))?;
            if !state
                .db
                .accounts()
                .can_access_library(principal.user.id, principal.user.role, chosen.library_id)
                .await?
            {
                return Err(ApiError::forbidden());
            }
            (chosen.item_id, PosterLevel::Work)
        } else {
            let item_id =
                parse_item_id(raw_item_id).ok_or_else(|| ApiError::not_found("Image not found"))?;
            let meta = state
                .db
                .jellyfin()
                .get_item_meta(item_id)
                .await?
                .ok_or_else(|| ApiError::not_found("Image not found"))?;
            if !state
                .db
                .accounts()
                .can_access_library(principal.user.id, principal.user.role, meta.library_id)
                .await?
            {
                return Err(ApiError::forbidden());
            }
            // Audio track artwork
            if meta.kind != palveni_core::ItemKind::Video {
                let (mime, data) = palveni_library::artwork::open(&state.db, item_id)
                    .await
                    .map_err(|_| ApiError::not_found("Artwork unavailable"))?;
                return Response::builder()
                    .header(header::CONTENT_TYPE, mime)
                    .header(header::CONTENT_LENGTH, data.len())
                    .header(header::CACHE_CONTROL, "private, no-cache")
                    .header(header::X_CONTENT_TYPE_OPTIONS, "nosniff")
                    .body(Body::from(data))
                    .map_err(ApiError::internal);
            }
            (item_id, PosterLevel::Episode)
        };

    let (mime, len, stream) = match kind.as_str() {
        "backdrop" => {
            if let Some(loc) = state.db.video().backdrop_location(target_item_id).await? {
                state
                    .video_images
                    .open_image(loc, size)
                    .await
                    .map_err(|_| ApiError::not_found("Backdrop unavailable"))?
            } else if let Some(loc) = state
                .db
                .video()
                .poster_location(target_item_id, PosterLevel::Work)
                .await?
            {
                state
                    .video_images
                    .open_image(loc, size)
                    .await
                    .map_err(|_| ApiError::not_found("Backdrop unavailable"))?
            } else {
                return Err(ApiError::not_found("Backdrop not found"));
            }
        }
        "thumb" | "banner" | "logo" => {
            if let Some(loc) = state.db.video().thumb_location(target_item_id).await?
                && let Ok(img) = state.video_images.open_or_grab_thumb(loc, size).await
            {
                img
            } else if let Some(loc) = state
                .db
                .video()
                .poster_location(target_item_id, PosterLevel::Work)
                .await?
            {
                state
                    .video_images
                    .open_image(loc, size)
                    .await
                    .map_err(|_| ApiError::not_found("Thumbnail unavailable"))?
            } else {
                return Err(ApiError::not_found("Thumbnail not found"));
            }
        }
        _ => {
            // Primary poster: try preferred level -> Work -> thumb
            let primary_loc = match state
                .db
                .video()
                .poster_location(target_item_id, preferred_level)
                .await?
            {
                Some(loc) => Some(loc),
                None => {
                    state
                        .db
                        .video()
                        .poster_location(target_item_id, PosterLevel::Work)
                        .await?
                }
            };
            if let Some(loc) = primary_loc {
                state
                    .video_images
                    .open_image(loc, size)
                    .await
                    .map_err(|_| ApiError::not_found("Poster unavailable"))?
            } else if let Some(loc) = state.db.video().thumb_location(target_item_id).await? {
                state
                    .video_images
                    .open_or_grab_thumb(loc, size)
                    .await
                    .map_err(|_| ApiError::not_found("Poster unavailable"))?
            } else {
                return Err(ApiError::not_found("Poster not found"));
            }
        }
    };

    Response::builder()
        .header(header::CONTENT_TYPE, mime)
        .header(header::CONTENT_LENGTH, len)
        .header(header::CACHE_CONTROL, "private, no-cache")
        .header(header::X_CONTENT_TYPE_OPTIONS, "nosniff")
        .body(Body::from_stream(ReaderStream::with_capacity(
            stream,
            32 * 1024,
        )))
        .map_err(ApiError::internal)
}

/// `GET /Items/{itemId}/PlaybackInfo` & `POST /Items/{itemId}/PlaybackInfo`
async fn playback_info(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(raw_item_id): Path<String>,
    OriginalUri(uri): OriginalUri,
) -> Result<Json<Value>, ApiError> {
    let query = QueryParams::from_uri(&uri);
    let principal = authenticate_jellyfin(&state, &headers, &query).await?;
    let item_id =
        parse_item_id(&raw_item_id).ok_or_else(|| ApiError::not_found("Item not found"))?;

    if let Some(rec) = state
        .db
        .jellyfin()
        .get_video_record(principal.user.id, item_id)
        .await?
    {
        if !state
            .db
            .accounts()
            .can_access_library(principal.user.id, principal.user.role, rec.library_id)
            .await?
        {
            return Err(ApiError::forbidden());
        }
        let mut versions = if rec.season.is_none() {
            state
                .db
                .jellyfin()
                .list_video_records_for_work(principal.user.id, &rec.work_key)
                .await?
        } else {
            vec![rec.clone()]
        };
        if versions.is_empty() {
            versions.push(rec.clone());
        } else if let Some(pos) = versions.iter().position(|v| v.item_id == rec.item_id) {
            versions.swap(0, pos);
        }
        let mut sources = Vec::with_capacity(versions.len());
        for v in &versions {
            sources.push(
                build_media_source_json(
                    &state,
                    MediaSourceSpec {
                        item_id: v.item_id,
                        name: &v.title,
                        path: v.path.as_deref(),
                        size_bytes: v.size_bytes,
                        runtime_ms: v.runtime_ms,
                        is_audio: false,
                    },
                    &principal.token,
                )
                .await,
            );
        }
        return Ok(Json(json!({
            "MediaSources": sources,
            "PlaySessionId": Uuid::now_v7().to_string()
        })));
    }

    if let Some(rec) = state
        .db
        .jellyfin()
        .get_audio_record(principal.user.id, item_id)
        .await?
    {
        if !state
            .db
            .accounts()
            .can_access_library(principal.user.id, principal.user.role, rec.library_id)
            .await?
        {
            return Err(ApiError::forbidden());
        }
        let source = build_media_source_json(
            &state,
            MediaSourceSpec {
                item_id: rec.item_id,
                name: &rec.title,
                path: rec.path.as_deref(),
                size_bytes: rec.size_bytes,
                runtime_ms: rec.duration_ms,
                is_audio: true,
            },
            &principal.token,
        )
        .await;
        return Ok(Json(json!({
            "MediaSources": [source],
            "PlaySessionId": Uuid::now_v7().to_string()
        })));
    }

    Err(ApiError::not_found("Item not found"))
}

/// `GET/HEAD /Videos/{itemId}/stream` & `/Videos/{itemId}/stream.{container}`
async fn stream_video(
    State(state): State<AppState>,
    method: Method,
    headers: HeaderMap,
    Path((raw_item_id, target)): Path<(String, String)>,
    OriginalUri(uri): OriginalUri,
) -> Result<Response, ApiError> {
    if target != "stream" && !target.starts_with("stream.") {
        return Err(ApiError::not_found("No such stream endpoint"));
    }
    stream_media_item(&state, method, &headers, &uri, &raw_item_id).await
}

/// `GET/HEAD /Audio/{itemId}/stream` & `/Audio/{itemId}/stream.{container}`
async fn stream_audio(
    State(state): State<AppState>,
    method: Method,
    headers: HeaderMap,
    Path((raw_item_id, target)): Path<(String, String)>,
    OriginalUri(uri): OriginalUri,
) -> Result<Response, ApiError> {
    if target != "stream" && !target.starts_with("stream.") {
        return Err(ApiError::not_found("No such stream endpoint"));
    }
    stream_media_item(&state, method, &headers, &uri, &raw_item_id).await
}

async fn stream_media_item(
    state: &AppState,
    method: Method,
    headers: &HeaderMap,
    uri: &Uri,
    raw_item_id: &str,
) -> Result<Response, ApiError> {
    let query = QueryParams::from_uri(uri);
    let principal = authenticate_jellyfin(state, headers, &query).await?;
    let item_id =
        parse_item_id(raw_item_id).ok_or_else(|| ApiError::not_found("Item not found"))?;
    let meta = state
        .db
        .jellyfin()
        .get_item_meta(item_id)
        .await?
        .ok_or_else(|| ApiError::not_found("Item not found"))?;
    if !state
        .db
        .accounts()
        .can_access_library(principal.user.id, principal.user.role, meta.library_id)
        .await?
    {
        return Err(ApiError::forbidden());
    }

    if let Some(loc) = state.db.video().location(item_id).await? {
        if palveni_library::video::strm::is_strm_path(&loc.relative_path) {
            let target_url = palveni_library::video::strm::read_strm_location(loc)
                .await
                .map_err(|err| match err {
                    palveni_core::Error::Invalid(msg) => ApiError::validation(msg),
                    palveni_core::Error::Internal(_) => ApiError::internal(err),
                    _ => ApiError::not_found("Video file unavailable"),
                })?;
            return Response::builder()
                .status(StatusCode::FOUND)
                .header(header::LOCATION, target_url)
                .header(header::CACHE_CONTROL, "private, no-store")
                .header(header::REFERRER_POLICY, "no-referrer")
                .body(Body::empty())
                .map_err(ApiError::internal);
        }
        let file = palveni_library::local::open_video(loc)
            .await
            .map_err(|err| match err {
                palveni_core::Error::Internal(_) => ApiError::internal(err),
                _ => ApiError::not_found("Video file unavailable"),
            })?;
        return playback::serve_original(file, method, headers).await;
    }

    if let Some(loc) = state.db.music().audio_location(item_id).await? {
        let track = playback::open_audio(loc).await.map_err(|err| match err {
            palveni_core::Error::Internal(_) => ApiError::internal(err),
            _ => ApiError::not_found("Audio file unavailable"),
        })?;
        let key = playback::playback_key(item_id, b"jellyfin", principal.token.as_bytes());
        return playback::serve_track(state, track, method, headers, key).await;
    }

    Err(ApiError::not_found("Media file unavailable"))
}

// ===========================================================================
// Playback Reporting, Played & Favorite Toggles, Library Refresh
// ===========================================================================

#[derive(Deserialize, Default)]
struct PlaybackReportBody {
    #[serde(default, alias = "ItemId", alias = "itemId")]
    item_id: Option<String>,
    #[serde(default, alias = "PositionTicks", alias = "positionTicks")]
    position_ticks: Option<i64>,
}

/// `POST /Sessions/Playing`
async fn session_playing(
    State(state): State<AppState>,
    headers: HeaderMap,
    OriginalUri(uri): OriginalUri,
    body: Option<Json<PlaybackReportBody>>,
) -> Result<StatusCode, ApiError> {
    handle_session_report(&state, &headers, &uri, body.map(|Json(b)| b), false).await
}

/// `POST /Sessions/Playing/Progress`
async fn session_playing_progress(
    State(state): State<AppState>,
    headers: HeaderMap,
    OriginalUri(uri): OriginalUri,
    body: Option<Json<PlaybackReportBody>>,
) -> Result<StatusCode, ApiError> {
    handle_session_report(&state, &headers, &uri, body.map(|Json(b)| b), false).await
}

/// `POST /Sessions/Playing/Stopped`
async fn session_playing_stopped(
    State(state): State<AppState>,
    headers: HeaderMap,
    OriginalUri(uri): OriginalUri,
    body: Option<Json<PlaybackReportBody>>,
) -> Result<StatusCode, ApiError> {
    handle_session_report(&state, &headers, &uri, body.map(|Json(b)| b), true).await
}

async fn handle_session_report(
    state: &AppState,
    headers: &HeaderMap,
    uri: &Uri,
    body: Option<PlaybackReportBody>,
    stopped: bool,
) -> Result<StatusCode, ApiError> {
    let query = QueryParams::from_uri(uri);
    let principal = authenticate_jellyfin(state, headers, &query).await?;
    let report = body.unwrap_or_default();
    let raw_item_id = report
        .item_id
        .or_else(|| query.get("itemid").map(str::to_owned))
        .ok_or_else(|| ApiError::validation("Missing ItemId").field("ItemId"))?;
    let item_id =
        parse_item_id(&raw_item_id).ok_or_else(|| ApiError::not_found("Item not found"))?;

    let meta = state
        .db
        .jellyfin()
        .get_item_meta(item_id)
        .await?
        .ok_or_else(|| ApiError::not_found("Item not found"))?;
    if !state
        .db
        .accounts()
        .can_access_library(principal.user.id, principal.user.role, meta.library_id)
        .await?
    {
        return Err(ApiError::forbidden());
    }

    let position_ticks = report
        .position_ticks
        .or_else(|| {
            query
                .get("positionticks")
                .and_then(|s| s.parse::<i64>().ok())
        })
        .unwrap_or(0);
    let position_ms = ticks_to_ms(position_ticks);

    let mut completed = false;
    if stopped {
        let runtime_ms = if let Some(v) = state
            .db
            .jellyfin()
            .get_video_record(principal.user.id, item_id)
            .await?
        {
            v.runtime_ms
        } else if let Some(a) = state
            .db
            .jellyfin()
            .get_audio_record(principal.user.id, item_id)
            .await?
        {
            a.duration_ms
        } else {
            None
        };
        if let Some(dur) = runtime_ms
            && dur > 0
            && position_ms.saturating_mul(100) >= dur.saturating_mul(90)
        {
            completed = true;
        }
    }

    state
        .db
        .jellyfin()
        .report_playback(
            principal.user.id,
            item_id,
            position_ms,
            completed,
            completed,
        )
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

/// `POST /Users/{userId}/PlayedItems/{itemId}`
async fn mark_played(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((raw_user_id, raw_item_id)): Path<(String, String)>,
    OriginalUri(uri): OriginalUri,
) -> Result<Json<Value>, ApiError> {
    set_played(&state, &headers, &uri, &raw_user_id, &raw_item_id, true).await
}

/// `DELETE /Users/{userId}/PlayedItems/{itemId}`
async fn mark_unplayed(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((raw_user_id, raw_item_id)): Path<(String, String)>,
    OriginalUri(uri): OriginalUri,
) -> Result<Json<Value>, ApiError> {
    set_played(&state, &headers, &uri, &raw_user_id, &raw_item_id, false).await
}

async fn set_played(
    state: &AppState,
    headers: &HeaderMap,
    uri: &Uri,
    raw_user_id: &str,
    raw_item_id: &str,
    played: bool,
) -> Result<Json<Value>, ApiError> {
    let query = QueryParams::from_uri(uri);
    let principal = authenticate_jellyfin(state, headers, &query).await?;
    let target_id = resolve_target_user_id(&principal, raw_user_id)?;

    if is_hex64(raw_item_id) {
        let records = state
            .db
            .jellyfin()
            .list_video_records_for_work(target_id, raw_item_id)
            .await?;
        let Some(first) = records.first() else {
            return Err(ApiError::not_found("Series not found"));
        };
        if !state
            .db
            .accounts()
            .can_access_library(principal.user.id, principal.user.role, first.library_id)
            .await?
        {
            return Err(ApiError::forbidden());
        }
        state
            .db
            .video()
            .set_work_played(target_id, first.library_id, raw_item_id, played)
            .await?;
        return Ok(Json(json!({
            "PlaybackPositionTicks": 0,
            "PlayCount": if played { 1 } else { 0 },
            "IsFavorite": first.work_favorite,
            "Played": played,
            "Key": raw_item_id
        })));
    }

    let item_id =
        parse_item_id(raw_item_id).ok_or_else(|| ApiError::not_found("Item not found"))?;
    let meta = state
        .db
        .jellyfin()
        .get_item_meta(item_id)
        .await?
        .ok_or_else(|| ApiError::not_found("Item not found"))?;
    if !state
        .db
        .accounts()
        .can_access_library(principal.user.id, principal.user.role, meta.library_id)
        .await?
    {
        return Err(ApiError::forbidden());
    }
    let updated = state
        .db
        .personal()
        .update(target_id, item_id, MediaChange::Played(played))
        .await?;
    Ok(Json(jellyfin_user_data_json(&updated, None, None)))
}

/// `POST /Users/{userId}/FavoriteItems/{itemId}`
async fn mark_favorite(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((raw_user_id, raw_item_id)): Path<(String, String)>,
    OriginalUri(uri): OriginalUri,
) -> Result<Json<Value>, ApiError> {
    set_favorite(&state, &headers, &uri, &raw_user_id, &raw_item_id, true).await
}

/// `DELETE /Users/{userId}/FavoriteItems/{itemId}`
async fn unmark_favorite(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((raw_user_id, raw_item_id)): Path<(String, String)>,
    OriginalUri(uri): OriginalUri,
) -> Result<Json<Value>, ApiError> {
    set_favorite(&state, &headers, &uri, &raw_user_id, &raw_item_id, false).await
}

async fn set_favorite(
    state: &AppState,
    headers: &HeaderMap,
    uri: &Uri,
    raw_user_id: &str,
    raw_item_id: &str,
    favorite: bool,
) -> Result<Json<Value>, ApiError> {
    let query = QueryParams::from_uri(uri);
    let principal = authenticate_jellyfin(state, headers, &query).await?;
    let target_id = resolve_target_user_id(&principal, raw_user_id)?;

    if is_hex64(raw_item_id) {
        let records = state
            .db
            .jellyfin()
            .list_video_records_for_work(target_id, raw_item_id)
            .await?;
        let Some(first) = records.first() else {
            return Err(ApiError::not_found("Series not found"));
        };
        if !state
            .db
            .accounts()
            .can_access_library(principal.user.id, principal.user.role, first.library_id)
            .await?
        {
            return Err(ApiError::forbidden());
        }
        state
            .db
            .video()
            .set_work_favorite(target_id, first.library_id, raw_item_id, favorite)
            .await?;
        return Ok(Json(json!({
            "PlaybackPositionTicks": 0,
            "PlayCount": first.state.play_count,
            "IsFavorite": favorite,
            "Played": first.state.completed,
            "Key": raw_item_id
        })));
    }

    let item_id =
        parse_item_id(raw_item_id).ok_or_else(|| ApiError::not_found("Item not found"))?;
    let meta = state
        .db
        .jellyfin()
        .get_item_meta(item_id)
        .await?
        .ok_or_else(|| ApiError::not_found("Item not found"))?;
    if !state
        .db
        .accounts()
        .can_access_library(principal.user.id, principal.user.role, meta.library_id)
        .await?
    {
        return Err(ApiError::forbidden());
    }
    let updated = state
        .db
        .personal()
        .update(target_id, item_id, MediaChange::Favorite(favorite))
        .await?;
    if let Some(work_key) = meta.work_key {
        let _ = state
            .db
            .video()
            .set_work_favorite(target_id, meta.library_id, &work_key, favorite)
            .await;
    }
    Ok(Json(jellyfin_user_data_json(
        &updated,
        Some(favorite),
        None,
    )))
}

/// `POST /Library/Refresh` (Admin only)
async fn library_refresh(
    State(state): State<AppState>,
    headers: HeaderMap,
    OriginalUri(uri): OriginalUri,
) -> Result<StatusCode, ApiError> {
    let query = QueryParams::from_uri(&uri);
    let principal = authenticate_jellyfin(&state, &headers, &query).await?;
    if !principal.user.is_admin() {
        return Err(ApiError::forbidden());
    }
    let libs = state.db.libraries().list().await?;
    for lib in libs {
        let _ = state.jobs.enqueue_library_scan(lib.id, lib.name).await;
    }
    Ok(StatusCode::NO_CONTENT)
}

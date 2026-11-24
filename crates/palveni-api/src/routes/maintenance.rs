//! System maintenance endpoints: security audit log (`/api/v1/system/audit`),
//! portable snapshot backups (`/api/v1/system/backups*`), and external
//! history/playlist migration importer (`/api/v1/system/import`).

use std::collections::HashSet;
use std::time::Duration;

use axum::Json;
use axum::body::Body;
use axum::extract::rejection::{PathRejection, QueryRejection};
use axum::extract::{Path, Query, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::Response;
use palveni_core::accounts::{
    AuditLogPage, BackupList, BackupRestoreResult, BackupSummary, CreateBackupRequest,
    ExternalImportRequest, ExternalImportResult, ExternalImportSource, MAX_BACKUP_NAME_CHARS,
    MAX_IMPORT_BYTES,
};
use palveni_core::settings::RuntimeSettings;
use palveni_core::{BackupId, ServerEvent};
use palveni_db::audit::{AuditEntry, actions};
use palveni_db::settings::KEY_RUNTIME;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::auth::{AdminUser, CurrentUser};
use crate::error::ApiError;
use crate::json::ApiJson;
use crate::routes::downloads::validate_ssrf_safe_url;
use crate::routes::{DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE, audit, parse_limit, path_id};
use crate::state::AppState;

const REMOTE_IMPORT_TIMEOUT: Duration = Duration::from_secs(12);
const REMOTE_PROBE_TIMEOUT: Duration = Duration::from_secs(4);
const MAX_REMOTE_PLAYLISTS: usize = 30;
const JELLYFIN_AUTH_HEADER: &str =
    r#"MediaBrowser Client="Palveni", Device="Server", DeviceId="palveni-migrator", Version="0.1""#;

type IdPath = Result<Path<String>, PathRejection>;

#[derive(Deserialize, Default)]
pub struct AuditQuery {
    pub action: Option<String>,
    pub cursor: Option<String>,
    pub limit: Option<String>,
}

/// `GET /system/audit` (Admin).
pub async fn list_audit(
    _admin: AdminUser,
    State(state): State<AppState>,
    query: Result<Query<AuditQuery>, QueryRejection>,
) -> Result<Json<AuditLogPage>, ApiError> {
    let Query(params) = query?;
    let limit = parse_limit(params.limit.as_deref(), DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE)?;
    if let Some(prefix) = params.action.as_deref()
        && prefix.chars().count() > 64
    {
        return Err(
            ApiError::validation("action filter must be at most 64 characters").field("action"),
        );
    }
    let page = state
        .db
        .accounts()
        .list_audit_logs(params.action.as_deref(), params.cursor.as_deref(), limit)
        .await?;
    Ok(Json(page))
}

/// `GET /system/backups` (Admin).
pub async fn list_backups(
    _admin: AdminUser,
    State(state): State<AppState>,
) -> Result<Json<BackupList>, ApiError> {
    let backups = state.db.accounts().list_backups().await?;
    Ok(Json(BackupList { backups }))
}

/// `POST /system/backups` (Admin).
pub async fn create_backup(
    AdminUser(admin): AdminUser,
    State(state): State<AppState>,
    body: Result<ApiJson<CreateBackupRequest>, ApiError>,
) -> Result<(StatusCode, Json<BackupSummary>), ApiError> {
    let ApiJson(request) = body?;
    if let Some(raw_name) = request.name.as_deref() {
        let trimmed = raw_name.trim();
        if trimmed.chars().count() > MAX_BACKUP_NAME_CHARS || trimmed.chars().any(char::is_control)
        {
            return Err(ApiError::validation(format!(
                "Backup name must be at most {MAX_BACKUP_NAME_CHARS} printable characters"
            ))
            .field("name"));
        }
    }

    let summary = state
        .db
        .accounts()
        .create_backup(Some(admin.user.id), request.name.as_deref())
        .await?;

    audit(
        &state,
        AuditEntry {
            action: actions::BACKUP_CREATED,
            actor: Some(admin.user.id),
            target: Some(summary.id.to_string()),
            detail: Some(json!({
                "name": summary.name,
                "sizeBytes": summary.size_bytes,
                "sha256": summary.sha256,
            })),
        },
    )
    .await;

    Ok((StatusCode::CREATED, Json(summary)))
}

/// `GET /system/backups/{id}/download` (Admin).
pub async fn download_backup(
    _admin: AdminUser,
    State(state): State<AppState>,
    path: IdPath,
) -> Result<Response, ApiError> {
    let id: BackupId = path_id(&path?.0, "Backup")?;
    let (summary, payload_json) = state
        .db
        .accounts()
        .get_backup_payload(id)
        .await?
        .ok_or_else(|| ApiError::not_found("Backup not found"))?;

    let filename = format!("palveni-backup-{}.json", summary.id);
    let disposition = format!("attachment; filename=\"{filename}\"");

    Response::builder()
        .status(StatusCode::OK)
        .header(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/json; charset=utf-8"),
        )
        .header(
            header::CONTENT_DISPOSITION,
            HeaderValue::from_str(&disposition).map_err(ApiError::internal)?,
        )
        .header(
            header::CACHE_CONTROL,
            HeaderValue::from_static("private, no-store"),
        )
        .header(header::CONTENT_LENGTH, payload_json.len())
        .body(Body::from(payload_json))
        .map_err(ApiError::internal)
}

/// `POST /system/backups/{id}/restore` (Admin).
pub async fn restore_backup(
    AdminUser(admin): AdminUser,
    State(state): State<AppState>,
    path: IdPath,
) -> Result<Json<BackupRestoreResult>, ApiError> {
    let id: BackupId = path_id(&path?.0, "Backup")?;
    let result = state.db.accounts().restore_backup(id).await?;

    // Synchronize runtime settings cache if restored.
    if let Ok(Some(raw_runtime)) = state.db.settings().get(KEY_RUNTIME).await
        && let Ok(restored_runtime) = serde_json::from_str::<RuntimeSettings>(&raw_runtime)
    {
        let mut guard = state.settings.write().await;
        if *guard != restored_runtime {
            *guard = restored_runtime.clone();
            state.bus.publish(ServerEvent::SettingsUpdated {
                server_name: restored_runtime.server_name,
                perf_tier: restored_runtime.perf_tier,
            });
        }
    }

    if let Ok(libraries) = state.db.libraries().list().await {
        for library in libraries {
            state.bus.publish(ServerEvent::LibraryUpdated(library));
        }
    }

    audit(
        &state,
        AuditEntry {
            action: actions::BACKUP_RESTORED,
            actor: Some(admin.user.id),
            target: Some(id.to_string()),
            detail: Some(json!({
                "librariesRestored": result.libraries_restored,
                "playlistsRestored": result.playlists_restored,
                "itemStatesRestored": result.item_states_restored,
            })),
        },
    )
    .await;

    Ok(Json(result))
}

/// `DELETE /system/backups/{id}` (Admin).
pub async fn delete_backup(
    AdminUser(admin): AdminUser,
    State(state): State<AppState>,
    path: IdPath,
) -> Result<StatusCode, ApiError> {
    let id: BackupId = path_id(&path?.0, "Backup")?;
    if !state.db.accounts().delete_backup(id).await? {
        return Err(ApiError::not_found("Backup not found"));
    }

    audit(
        &state,
        AuditEntry {
            action: actions::BACKUP_DELETED,
            actor: Some(admin.user.id),
            target: Some(id.to_string()),
            detail: None,
        },
    )
    .await;

    Ok(StatusCode::NO_CONTENT)
}

/// `POST /system/import` (Authenticated).
pub async fn import_external(
    current: CurrentUser,
    State(state): State<AppState>,
    body: Result<ApiJson<ExternalImportRequest>, ApiError>,
) -> Result<Json<ExternalImportResult>, ApiError> {
    let ApiJson(request) = body?;
    let is_remote = request
        .server_url
        .as_deref()
        .map(str::trim)
        .is_some_and(|s| !s.is_empty());

    let effective_request = if is_remote {
        let (detected_source, fetched_json) = fetch_remote_import_payload(&request).await?;
        if fetched_json.len() > MAX_IMPORT_BYTES {
            return Err(ApiError::new(
                StatusCode::PAYLOAD_TOO_LARGE,
                "payload_too_large",
                "Remote server export exceeds 10 MiB",
            ));
        }
        ExternalImportRequest {
            source: detected_source,
            content: fetched_json,
            playlist_name: request.playlist_name.clone(),
            dry_run: request.dry_run,
            ..Default::default()
        }
    } else {
        if request.content.len() > MAX_IMPORT_BYTES {
            return Err(ApiError::new(
                StatusCode::PAYLOAD_TOO_LARGE,
                "payload_too_large",
                "Import content exceeds 10 MiB",
            ));
        }
        request.clone()
    };

    let result = state
        .db
        .accounts()
        .import_external_state(current.user.id, &effective_request)
        .await?;

    if !effective_request.dry_run {
        audit(
            &state,
            AuditEntry {
                action: actions::SYSTEM_IMPORTED,
                actor: Some(current.user.id),
                target: Some(format!("{:?}", result.detected_source).to_lowercase()),
                detail: Some(json!({
                    "mode": if is_remote { "online" } else { "payload" },
                    "matchedItems": result.matched_items,
                    "favoritesImported": result.favorites_imported,
                    "historyImported": result.history_imported,
                    "playlistsImported": result.playlists_imported,
                })),
            },
        )
        .await;
    }

    Ok(Json(result))
}

fn validate_remote_import_params(request: &ExternalImportRequest) -> Result<String, ApiError> {
    if request.source == ExternalImportSource::M3u {
        return Err(ApiError::validation(
            "Direct server pull is only supported for Jellyfin, Emby, or Navidrome",
        )
        .field("source"));
    }
    let raw_url = request
        .server_url
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| ApiError::validation("serverUrl is required").field("serverUrl"))?;
    let validated = validate_ssrf_safe_url(raw_url, "serverUrl", false)?;
    if let Some(u) = request.username.as_deref()
        && u.chars().count() > 128
    {
        return Err(
            ApiError::validation("username must be at most 128 characters").field("username"),
        );
    }
    if let Some(p) = request.password.as_deref()
        && p.chars().count() > 256
    {
        return Err(
            ApiError::validation("password must be at most 256 characters").field("password"),
        );
    }
    if let Some(k) = request.api_key.as_deref()
        && k.chars().count() > 256
    {
        return Err(ApiError::validation("apiKey must be at most 256 characters").field("apiKey"));
    }
    Ok(validated.trim_end_matches('/').to_owned())
}

fn is_safe_path_segment(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

async fn fetch_remote_import_payload(
    request: &ExternalImportRequest,
) -> Result<(ExternalImportSource, String), ApiError> {
    let base_url = validate_remote_import_params(request)?;
    let client = reqwest::Client::builder()
        .timeout(REMOTE_IMPORT_TIMEOUT)
        .redirect(reqwest::redirect::Policy::none())
        .user_agent("Palveni-Server/0.1 (+https://palveni.local)")
        .build()
        .map_err(ApiError::internal)?;

    let username = request
        .username
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let password = request.password.as_deref().filter(|s| !s.is_empty());
    let api_key = request
        .api_key
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());

    let resolved_source = match request.source {
        ExternalImportSource::Jellyfin => ExternalImportSource::Jellyfin,
        ExternalImportSource::Emby => ExternalImportSource::Emby,
        ExternalImportSource::Navidrome => ExternalImportSource::Navidrome,
        ExternalImportSource::Auto => probe_remote_source(&client, &base_url).await,
        ExternalImportSource::M3u => unreachable!(),
    };

    let payload = match resolved_source {
        ExternalImportSource::Jellyfin | ExternalImportSource::Emby => {
            pull_jellyfin_emby(
                &client,
                &base_url,
                resolved_source,
                username,
                password,
                api_key,
            )
            .await?
        }
        ExternalImportSource::Navidrome => {
            pull_navidrome(&client, &base_url, username, password, api_key).await?
        }
        _ => unreachable!(),
    };

    let serialized = serde_json::to_string(&payload).map_err(ApiError::internal)?;
    Ok((resolved_source, serialized))
}

async fn probe_remote_source(client: &reqwest::Client, base_url: &str) -> ExternalImportSource {
    let url = format!("{base_url}/System/Info/Public");
    if let Ok(resp) = client.get(&url).timeout(REMOTE_PROBE_TIMEOUT).send().await
        && resp.status().is_success()
        && let Ok(body) = resp.json::<Value>().await
        && (body.get("ProductName").is_some()
            || body.get("ServerName").is_some()
            || body.get("Id").is_some())
    {
        if body
            .get("ProductName")
            .and_then(Value::as_str)
            .is_some_and(|p| p.to_ascii_lowercase().contains("emby"))
        {
            return ExternalImportSource::Emby;
        }
        return ExternalImportSource::Jellyfin;
    }
    ExternalImportSource::Navidrome
}

async fn pull_jellyfin_emby(
    client: &reqwest::Client,
    base_url: &str,
    source: ExternalImportSource,
    username: Option<&str>,
    password: Option<&str>,
    api_key: Option<&str>,
) -> Result<Value, ApiError> {
    let (token, user_id) = if let Some(key) = api_key {
        let uid = resolve_jellyfin_user_id(client, base_url, key, username).await?;
        (key.to_owned(), uid)
    } else if let Some(user) = username {
        let auth_url = format!("{base_url}/Users/AuthenticateByName");
        let resp = client
            .post(&auth_url)
            .header("Authorization", JELLYFIN_AUTH_HEADER)
            .header("X-Emby-Authorization", JELLYFIN_AUTH_HEADER)
            .json(&json!({
                "Username": user,
                "Pw": password.unwrap_or(""),
            }))
            .send()
            .await
            .map_err(|err| remote_server_error("Jellyfin/Emby", err))?;
        if !resp.status().is_success() {
            return Err(ApiError::new(
                StatusCode::BAD_GATEWAY,
                "unavailable",
                format!(
                    "Remote Jellyfin/Emby authentication failed (HTTP {})",
                    resp.status().as_u16()
                ),
            ));
        }
        let body: Value = resp
            .json()
            .await
            .map_err(|err| remote_server_error("Jellyfin/Emby", err))?;
        let tok = body
            .get("AccessToken")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| {
                ApiError::new(
                    StatusCode::BAD_GATEWAY,
                    "unavailable",
                    "Remote Jellyfin/Emby response did not include an AccessToken",
                )
            })?
            .to_owned();
        let uid = body
            .get("User")
            .and_then(|u| u.get("Id"))
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| is_safe_path_segment(s))
            .ok_or_else(|| {
                ApiError::new(
                    StatusCode::BAD_GATEWAY,
                    "unavailable",
                    "Remote Jellyfin/Emby response did not include a valid User.Id",
                )
            })?
            .to_owned();
        (tok, uid)
    } else {
        return Err(ApiError::validation(
            "Provide an API key or username/password to connect to Jellyfin or Emby",
        )
        .field("apiKey"));
    };

    let favorites = fetch_jellyfin_items(client, base_url, &token, &user_id, "IsFavorite").await?;
    let played = fetch_jellyfin_items(client, base_url, &token, &user_id, "IsPlayed").await?;
    let playlists = fetch_jellyfin_playlists(client, base_url, &token, &user_id).await;

    Ok(merge_jellyfin_payload(source, favorites, played, playlists))
}

async fn resolve_jellyfin_user_id(
    client: &reqwest::Client,
    base_url: &str,
    token: &str,
    username: Option<&str>,
) -> Result<String, ApiError> {
    let me_url = format!("{base_url}/Users/Me");
    if let Ok(resp) = client
        .get(&me_url)
        .header("X-Emby-Token", token)
        .send()
        .await
        && resp.status().is_success()
        && let Ok(body) = resp.json::<Value>().await
        && let Some(id) = body
            .get("Id")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| is_safe_path_segment(s))
    {
        return Ok(id.to_owned());
    }

    let users_url = format!("{base_url}/Users");
    let resp = client
        .get(&users_url)
        .header("X-Emby-Token", token)
        .send()
        .await
        .map_err(|err| remote_server_error("Jellyfin/Emby", err))?;
    if !resp.status().is_success() {
        return Err(ApiError::new(
            StatusCode::BAD_GATEWAY,
            "unavailable",
            format!(
                "Remote Jellyfin/Emby rejected API key (HTTP {})",
                resp.status().as_u16()
            ),
        ));
    }
    let users: Value = resp
        .json()
        .await
        .map_err(|err| remote_server_error("Jellyfin/Emby", err))?;
    let list = users.as_array().ok_or_else(|| {
        ApiError::new(
            StatusCode::BAD_GATEWAY,
            "unavailable",
            "Remote Jellyfin/Emby /Users response was not an array",
        )
    })?;
    let chosen = if let Some(target) = username {
        list.iter().find(|u| {
            u.get("Name")
                .and_then(Value::as_str)
                .is_some_and(|n| n.eq_ignore_ascii_case(target))
        })
    } else {
        list.first()
    };
    chosen
        .and_then(|u| u.get("Id"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| is_safe_path_segment(s))
        .map(ToOwned::to_owned)
        .ok_or_else(|| {
            ApiError::validation(
                "Could not resolve a matching user on the remote Jellyfin/Emby server",
            )
            .field("username")
        })
}

async fn fetch_jellyfin_items(
    client: &reqwest::Client,
    base_url: &str,
    token: &str,
    user_id: &str,
    filter: &str,
) -> Result<Vec<Value>, ApiError> {
    let url = format!("{base_url}/Users/{user_id}/Items");
    let resp = client
        .get(&url)
        .header("X-Emby-Token", token)
        .query(&[
            ("Recursive", "true"),
            ("Filters", filter),
            ("Fields", "Path,ProviderIds,UserData"),
            ("Limit", "5000"),
        ])
        .send()
        .await
        .map_err(|err| remote_server_error("Jellyfin/Emby", err))?;
    if !resp.status().is_success() {
        return Err(ApiError::new(
            StatusCode::BAD_GATEWAY,
            "unavailable",
            format!(
                "Remote Jellyfin/Emby item query failed (HTTP {})",
                resp.status().as_u16()
            ),
        ));
    }
    let body: Value = resp
        .json()
        .await
        .map_err(|err| remote_server_error("Jellyfin/Emby", err))?;
    Ok(body
        .get("Items")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default())
}

async fn fetch_jellyfin_playlists(
    client: &reqwest::Client,
    base_url: &str,
    token: &str,
    user_id: &str,
) -> Vec<Value> {
    let list_url = format!("{base_url}/Users/{user_id}/Items");
    let Ok(resp) = client
        .get(&list_url)
        .header("X-Emby-Token", token)
        .query(&[
            ("Recursive", "true"),
            ("IncludeItemTypes", "Playlist"),
            ("Limit", "50"),
        ])
        .send()
        .await
    else {
        return Vec::new();
    };
    if !resp.status().is_success() {
        return Vec::new();
    }
    let Ok(body) = resp.json::<Value>().await else {
        return Vec::new();
    };
    let Some(items) = body.get("Items").and_then(Value::as_array) else {
        return Vec::new();
    };
    let mut playlists = Vec::new();
    for pl in items.iter().take(MAX_REMOTE_PLAYLISTS) {
        let Some(pl_id) = pl
            .get("Id")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| is_safe_path_segment(s))
        else {
            continue;
        };
        let name = pl
            .get("Name")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or("Imported Playlist");
        let items_url = format!("{base_url}/Playlists/{pl_id}/Items");
        if let Ok(pl_resp) = client
            .get(&items_url)
            .header("X-Emby-Token", token)
            .query(&[("UserId", user_id), ("Fields", "Path"), ("Limit", "2000")])
            .send()
            .await
            && pl_resp.status().is_success()
            && let Ok(pl_body) = pl_resp.json::<Value>().await
            && let Some(tracks) = pl_body.get("Items").and_then(Value::as_array)
            && !tracks.is_empty()
        {
            playlists.push(json!({
                "Name": name,
                "Items": tracks,
            }));
        }
    }
    playlists
}

fn merge_jellyfin_payload(
    source: ExternalImportSource,
    favorites: Vec<Value>,
    played: Vec<Value>,
    playlists: Vec<Value>,
) -> Value {
    let mut seen_ids = HashSet::new();
    let mut merged = Vec::new();
    for item in favorites.into_iter().chain(played) {
        if let Some(id) = item
            .get("Id")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            && !seen_ids.insert(id.to_owned())
        {
            continue;
        }
        merged.push(item);
    }
    json!({
        "ServerType": if source == ExternalImportSource::Emby { "Emby" } else { "Jellyfin" },
        "Items": merged,
        "Playlists": playlists,
    })
}

async fn pull_navidrome(
    client: &reqwest::Client,
    base_url: &str,
    username: Option<&str>,
    password: Option<&str>,
    api_key: Option<&str>,
) -> Result<Value, ApiError> {
    let user = username.ok_or_else(|| {
        ApiError::validation("username is required to connect to Navidrome / Subsonic")
            .field("username")
    })?;
    let secret = password.or(api_key).ok_or_else(|| {
        ApiError::validation("password is required to connect to Navidrome / Subsonic")
            .field("password")
    })?;

    let starred_body =
        subsonic_get(client, base_url, "getStarred2.view", user, secret, &[]).await?;
    let starred_songs = starred_body
        .get("starred2")
        .and_then(|s| s.get("song"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let mut playlists = Vec::new();
    if let Ok(list_body) =
        subsonic_get(client, base_url, "getPlaylists.view", user, secret, &[]).await
        && let Some(arr) = list_body
            .get("playlists")
            .and_then(|p| p.get("playlist"))
            .and_then(Value::as_array)
    {
        for pl in arr.iter().take(MAX_REMOTE_PLAYLISTS) {
            let Some(pl_id) = pl
                .get("id")
                .and_then(|v| {
                    v.as_str()
                        .map(ToOwned::to_owned)
                        .or_else(|| v.as_i64().map(|n| n.to_string()))
                })
                .filter(|s| !s.trim().is_empty())
            else {
                continue;
            };
            if let Ok(detail) = subsonic_get(
                client,
                base_url,
                "getPlaylist.view",
                user,
                secret,
                &[("id", pl_id.as_str())],
            )
            .await
                && let Some(playlist_obj) = detail.get("playlist")
            {
                playlists.push(playlist_obj.clone());
            }
        }
    }

    Ok(build_navidrome_payload(starred_songs, playlists))
}

async fn subsonic_get(
    client: &reqwest::Client,
    base_url: &str,
    endpoint: &str,
    user: &str,
    password: &str,
    extra: &[(&str, &str)],
) -> Result<Value, ApiError> {
    let url = format!("{base_url}/rest/{endpoint}");
    let mut query: Vec<(&str, &str)> = vec![
        ("u", user),
        ("p", password),
        ("v", "1.16.1"),
        ("c", "palveni"),
        ("f", "json"),
    ];
    query.extend_from_slice(extra);
    let resp = client
        .get(&url)
        .query(&query)
        .send()
        .await
        .map_err(|err| remote_server_error("Navidrome/Subsonic", err))?;
    if !resp.status().is_success() {
        return Err(ApiError::new(
            StatusCode::BAD_GATEWAY,
            "unavailable",
            format!(
                "Remote Navidrome/Subsonic server returned HTTP {}",
                resp.status().as_u16()
            ),
        ));
    }
    let body: Value = resp
        .json()
        .await
        .map_err(|err| remote_server_error("Navidrome/Subsonic", err))?;
    let sub = body.get("subsonic-response").ok_or_else(|| {
        ApiError::new(
            StatusCode::BAD_GATEWAY,
            "unavailable",
            "Remote server did not return a valid Subsonic JSON response",
        )
    })?;
    if sub.get("status").and_then(Value::as_str) != Some("ok") {
        let msg = sub
            .get("error")
            .and_then(|e| e.get("message"))
            .and_then(Value::as_str)
            .unwrap_or("Authentication or query failed");
        return Err(ApiError::new(
            StatusCode::BAD_GATEWAY,
            "unavailable",
            format!("Remote Navidrome/Subsonic error: {msg}"),
        ));
    }
    Ok(sub.clone())
}

fn build_navidrome_payload(starred_songs: Vec<Value>, playlists: Vec<Value>) -> Value {
    json!({
        "starred2": { "song": starred_songs },
        "playlists": playlists,
    })
}

fn remote_server_error(kind: &str, err: reqwest::Error) -> ApiError {
    ApiError::new(
        StatusCode::BAD_GATEWAY,
        "unavailable",
        format!("Could not reach remote {kind} server: {err}"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remote_import_validator_blocks_ssrf_and_m3u() {
        let m3u_req = ExternalImportRequest {
            source: ExternalImportSource::M3u,
            server_url: Some("http://192.168.0.10:8096".into()),
            ..Default::default()
        };
        assert!(validate_remote_import_params(&m3u_req).is_err());

        let ssrf_req = ExternalImportRequest {
            source: ExternalImportSource::Jellyfin,
            server_url: Some("http://169.254.169.254/latest".into()),
            ..Default::default()
        };
        assert!(validate_remote_import_params(&ssrf_req).is_err());

        let valid_req = ExternalImportRequest {
            source: ExternalImportSource::Jellyfin,
            server_url: Some("http://192.168.0.10:8096/".into()),
            ..Default::default()
        };
        assert_eq!(
            validate_remote_import_params(&valid_req).unwrap(),
            "http://192.168.0.10:8096"
        );
    }

    #[test]
    fn merge_jellyfin_and_build_navidrome_payloads_deduplicate_and_preserve_playlists() {
        let merged = merge_jellyfin_payload(
            ExternalImportSource::Emby,
            vec![json!({
                "Id": "item-1",
                "Name": "Dune",
                "Path": "/movies/Dune.mkv",
                "UserData": { "IsFavorite": true, "Played": true }
            })],
            vec![
                json!({
                    "Id": "item-1",
                    "Name": "Dune",
                    "Path": "/movies/Dune.mkv",
                    "UserData": { "IsFavorite": true, "Played": true }
                }),
                json!({
                    "Id": "item-2",
                    "Name": "Arrival",
                    "Path": "/movies/Arrival.mkv",
                    "UserData": { "Played": true }
                }),
            ],
            vec![json!({
                "Name": "Sci-Fi",
                "Items": [{ "Name": "Dune", "Path": "/movies/Dune.mkv" }]
            })],
        );
        assert_eq!(merged["ServerType"], "Emby");
        assert_eq!(merged["Items"].as_array().unwrap().len(), 2);
        assert_eq!(merged["Playlists"].as_array().unwrap().len(), 1);

        let navidrome = build_navidrome_payload(
            vec![json!({ "title": "Blue Train", "path": "Coltrane/01 Blue Train.flac" })],
            vec![json!({
                "name": "Jazz",
                "entry": [{ "title": "Blue Train", "path": "Coltrane/01 Blue Train.flac" }]
            })],
        );
        assert_eq!(navidrome["starred2"]["song"].as_array().unwrap().len(), 1);
        assert_eq!(navidrome["playlists"].as_array().unwrap().len(), 1);
    }
}

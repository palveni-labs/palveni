//! External Downloader Bridge (`qBittorrent` / `Transmission` / `aria2`) & RSS/Atom
//! Media Subscriptions (`module: downloads`).

use std::net::IpAddr;
use std::time::Duration;

use axum::Json;
use axum::extract::rejection::PathRejection;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use palveni_core::clock;
use palveni_core::downloads::{
    CreateDownloadTaskRequest, CreateDownloadTaskResponse, CreateDownloaderClientRequest,
    CreateSubscriptionRequest, DownloadTransfer, DownloaderClientStatus, DownloaderClientSummary,
    DownloaderKind, DownloadsOverviewResponse, FeedMatchItem, MediaSubscription,
    SubscriptionCheckResult, TransferState,
};
use palveni_core::ids::ConnectionId;
use palveni_db::audit::{AuditEntry, actions};
use palveni_db::downloads::{DownloaderClientRecord, SaveDownloaderClient};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::auth::{AdminUser, CurrentUser};
use crate::error::ApiError;
use crate::json::ApiJson;
use crate::routes::audit;
use crate::routes::home::{decrypt_secret, encrypt_secret};
use crate::state::AppState;

const POLL_TIMEOUT: Duration = Duration::from_secs(3);
const SUBMIT_TIMEOUT: Duration = Duration::from_secs(6);
const FEED_TIMEOUT: Duration = Duration::from_secs(6);
const MAX_FEED_BYTES: usize = 2 * 1024 * 1024;

const DOWNLOADER_SECRET_SCOPE: &[u8] = b"palveni:downloader-credential:v1";

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct StoredDownloaderCredential {
    #[serde(default)]
    username: Option<String>,
    #[serde(default)]
    password: Option<String>,
}

/// Validates an external downloader or RSS URL, blocking SSRF cloud metadata endpoints,
/// unspecified/multicast addresses, embedded credentials, and non-HTTP(S) schemes.
pub(crate) fn validate_ssrf_safe_url(
    raw: &str,
    field: &'static str,
    allow_query: bool,
) -> Result<String, ApiError> {
    let trimmed = raw.trim().trim_end_matches('/');
    if trimmed.is_empty() || trimmed.len() > 512 || trimmed.chars().any(char::is_control) {
        return Err(ApiError::validation("Invalid URL").field(field));
    }
    let parsed = reqwest::Url::parse(trimmed).map_err(|_| {
        ApiError::validation("Expected a valid http:// or https:// URL").field(field)
    })?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(ApiError::validation("URL scheme must be http or https").field(field));
    }
    if !parsed.username().is_empty() || parsed.password().is_some() {
        return Err(ApiError::validation("URL must not embed credentials").field(field));
    }
    if parsed.fragment().is_some() || (!allow_query && parsed.query().is_some()) {
        return Err(ApiError::validation("URL contains unexpected query or fragment").field(field));
    }
    let Some(host) = parsed.host_str() else {
        return Err(ApiError::validation("URL must include a host").field(field));
    };
    let host_lower = host.trim_matches(['[', ']']).to_ascii_lowercase();
    if matches!(
        host_lower.as_str(),
        "metadata.google.internal" | "metadata" | "instance-data"
    ) {
        return Err(ApiError::validation("Cloud metadata hosts are forbidden").field(field));
    }
    if let Ok(ip) = host_lower.parse::<IpAddr>() {
        let blocked = match ip {
            IpAddr::V4(v4) => {
                v4.is_unspecified()
                    || v4.is_multicast()
                    || v4.is_broadcast()
                    || v4.octets()[..2] == [169, 254]
                    || v4.octets() == [100, 100, 100, 200]
            }
            IpAddr::V6(v6) => {
                v6.is_unspecified() || v6.is_multicast() || (v6.segments()[0] & 0xffc0) == 0xfe80
            }
        };
        if blocked {
            return Err(ApiError::validation(
                "Metadata, link-local, and unspecified IP addresses are forbidden",
            )
            .field(field));
        }
    }
    Ok(if allow_query {
        parsed.as_str().to_owned()
    } else {
        parsed.as_str().trim_end_matches('/').to_owned()
    })
}

fn http_client(timeout: Duration) -> Result<reqwest::Client, ApiError> {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(timeout)
        .build()
        .map_err(ApiError::internal)
}

fn load_credential(
    state: &AppState,
    record: &DownloaderClientRecord,
) -> Result<StoredDownloaderCredential, ApiError> {
    let Some(ref cipher) = record.credential_cipher else {
        return Ok(StoredDownloaderCredential::default());
    };
    if cipher.is_empty() {
        return Ok(StoredDownloaderCredential::default());
    }
    let raw = decrypt_secret(&state.identity, DOWNLOADER_SECRET_SCOPE, &record.id, cipher)?;
    serde_json::from_str(&raw).map_err(|_| ApiError::internal("Corrupt downloader credential"))
}

pub(crate) async fn overview_internal(
    state: &AppState,
    current: &CurrentUser,
) -> Result<DownloadsOverviewResponse, ApiError> {
    let records = state.db.downloads().list_clients().await?;
    let owner_filter = (!current.is_admin()).then_some(current.user.id);
    let subscriptions = state
        .db
        .downloads()
        .list_subscriptions(owner_filter)
        .await?;

    let client = http_client(POLL_TIMEOUT)?;
    let mut summaries = Vec::with_capacity(records.len());
    let mut all_transfers = Vec::new();

    for rec in records {
        let cred_configured = rec
            .credential_cipher
            .as_deref()
            .is_some_and(|s| !s.is_empty());
        if !rec.enabled {
            summaries.push(DownloaderClientSummary {
                id: rec.id,
                name: rec.name,
                kind: rec.kind,
                base_url: rec.base_url,
                save_path: rec.save_path,
                enabled: false,
                credential_configured: cred_configured,
                status: DownloaderClientStatus::Disabled,
                status_message: None,
                created_at: rec.created_at,
            });
            continue;
        }

        let cred = load_credential(state, &rec).unwrap_or_default();
        match poll_client_transfers(&client, &rec, &cred).await {
            Ok(transfers) => {
                all_transfers.extend(transfers);
                summaries.push(DownloaderClientSummary {
                    id: rec.id,
                    name: rec.name,
                    kind: rec.kind,
                    base_url: rec.base_url,
                    save_path: rec.save_path,
                    enabled: true,
                    credential_configured: cred_configured,
                    status: DownloaderClientStatus::Online,
                    status_message: None,
                    created_at: rec.created_at,
                });
            }
            Err(err_msg) => {
                summaries.push(DownloaderClientSummary {
                    id: rec.id,
                    name: rec.name,
                    kind: rec.kind,
                    base_url: rec.base_url,
                    save_path: rec.save_path,
                    enabled: true,
                    credential_configured: cred_configured,
                    status: DownloaderClientStatus::Offline,
                    status_message: Some(err_msg),
                    created_at: rec.created_at,
                });
            }
        }
    }

    Ok(DownloadsOverviewResponse {
        clients: summaries,
        transfers: all_transfers,
        subscriptions,
    })
}

/// `GET /api/v1/downloads`
pub async fn overview(
    current: CurrentUser,
    State(state): State<AppState>,
) -> Result<Json<DownloadsOverviewResponse>, ApiError> {
    Ok(Json(overview_internal(&state, &current).await?))
}

/// `POST /api/v1/downloads/clients`
pub async fn save_client(
    AdminUser(admin): AdminUser,
    State(state): State<AppState>,
    body: Result<ApiJson<CreateDownloaderClientRequest>, ApiError>,
) -> Result<(StatusCode, Json<DownloaderClientSummary>), ApiError> {
    let ApiJson(req) = body?;
    let name = req.name.trim();
    if name.is_empty() || name.chars().count() > 128 || name.chars().any(char::is_control) {
        return Err(
            ApiError::validation("Name must contain 1 to 128 printable characters").field("name"),
        );
    }
    let base_url = validate_ssrf_safe_url(&req.base_url, "baseUrl", false)?;
    let save_path = req
        .save_path
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned);
    if save_path
        .as_deref()
        .is_some_and(|p| p.chars().count() > 512 || p.chars().any(char::is_control))
    {
        return Err(ApiError::validation("Invalid savePath").field("savePath"));
    }

    let target_id = req
        .id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map_or_else(|| ConnectionId::generate().to_string(), str::to_owned);

    let username = req
        .username
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned);
    let password = req
        .password
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned);

    let cipher = if username.is_some() || password.is_some() {
        let raw_json = serde_json::to_string(&StoredDownloaderCredential { username, password })
            .map_err(ApiError::internal)?;
        Some(encrypt_secret(
            &state.identity,
            DOWNLOADER_SECRET_SCOPE,
            &target_id,
            &raw_json,
        )?)
    } else {
        None
    };

    let saved = state
        .db
        .downloads()
        .save_client(SaveDownloaderClient {
            id: Some(&target_id),
            name,
            kind: req.kind,
            base_url: &base_url,
            credential_cipher: cipher,
            keep_existing_credential: true,
            save_path,
            enabled: req.enabled,
        })
        .await?;

    audit(
        &state,
        AuditEntry {
            action: actions::DOWNLOADER_UPDATED,
            actor: Some(admin.user.id),
            target: Some(saved.id.clone()),
            detail: Some(json!({
                "name": saved.name,
                "kind": saved.kind.as_str(),
                "baseUrl": saved.base_url,
            })),
        },
    )
    .await;

    let cred_configured = saved
        .credential_cipher
        .as_deref()
        .is_some_and(|s| !s.is_empty());
    let status = if !saved.enabled {
        DownloaderClientStatus::Disabled
    } else {
        DownloaderClientStatus::Online
    };

    Ok((
        StatusCode::CREATED,
        Json(DownloaderClientSummary {
            id: saved.id,
            name: saved.name,
            kind: saved.kind,
            base_url: saved.base_url,
            save_path: saved.save_path,
            enabled: saved.enabled,
            credential_configured: cred_configured,
            status,
            status_message: None,
            created_at: saved.created_at,
        }),
    ))
}

/// `DELETE /api/v1/downloads/clients/{id}`
pub async fn delete_client(
    AdminUser(admin): AdminUser,
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
) -> Result<StatusCode, ApiError> {
    let id = path?.0;
    if !state.db.downloads().delete_client(&id).await? {
        return Err(ApiError::not_found("Downloader client not found"));
    }
    audit(
        &state,
        AuditEntry {
            action: actions::DOWNLOADER_DELETED,
            actor: Some(admin.user.id),
            target: Some(id),
            detail: None,
        },
    )
    .await;
    Ok(StatusCode::NO_CONTENT)
}

fn validate_download_uri(raw: &str) -> Result<String, ApiError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.len() > 4096 || trimmed.chars().any(char::is_control) {
        return Err(
            ApiError::validation("Download URI must be 1 to 4096 printable characters")
                .field("uri"),
        );
    }
    if trimmed.to_ascii_lowercase().starts_with("magnet:?xt=") {
        return Ok(trimmed.to_owned());
    }
    validate_ssrf_safe_url(trimmed, "uri", true)
}

/// `POST /api/v1/downloads/tasks`
pub async fn create_task(
    AdminUser(admin): AdminUser,
    State(state): State<AppState>,
    body: Result<ApiJson<CreateDownloadTaskRequest>, ApiError>,
) -> Result<Json<CreateDownloadTaskResponse>, ApiError> {
    let ApiJson(req) = body?;
    let uri = validate_download_uri(&req.uri)?;

    let client_rec = if let Some(cid) = req
        .client_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        state
            .db
            .downloads()
            .get_client(cid)
            .await?
            .ok_or_else(|| ApiError::not_found("Downloader client not found"))?
    } else {
        state
            .db
            .downloads()
            .list_clients()
            .await?
            .into_iter()
            .find(|c| c.enabled)
            .ok_or_else(|| {
                ApiError::validation("No enabled downloader client configured").field("clientId")
            })?
    };

    if !client_rec.enabled {
        return Err(ApiError::conflict("Selected downloader client is disabled"));
    }

    let cred = load_credential(&state, &client_rec)?;
    let http = http_client(SUBMIT_TIMEOUT)?;
    let save_dir = req
        .save_path
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .or(client_rec.save_path.as_deref());

    submit_download_uri(
        &http,
        &client_rec,
        &cred,
        &uri,
        save_dir,
        req.category.as_deref(),
    )
    .await
    .map_err(|msg| ApiError::new(StatusCode::BAD_GATEWAY, "unavailable", msg))?;

    audit(
        &state,
        AuditEntry {
            action: actions::DOWNLOAD_SUBMITTED,
            actor: Some(admin.user.id),
            target: Some(client_rec.id.clone()),
            detail: Some(json!({
                "clientName": client_rec.name,
                "kind": client_rec.kind.as_str(),
                "isMagnet": uri.starts_with("magnet:"),
            })),
        },
    )
    .await;

    Ok(Json(CreateDownloadTaskResponse {
        accepted: true,
        client_id: client_rec.id,
        client_name: client_rec.name.clone(),
        message: format!("已投递至下载器「{}」", client_rec.name),
    }))
}

/// `POST /api/v1/downloads/subscriptions`
pub async fn create_subscription(
    current: CurrentUser,
    State(state): State<AppState>,
    body: Result<ApiJson<CreateSubscriptionRequest>, ApiError>,
) -> Result<(StatusCode, Json<MediaSubscription>), ApiError> {
    let ApiJson(mut req) = body?;
    let title = req.title.trim();
    if title.is_empty() || title.chars().count() > 255 || title.chars().any(char::is_control) {
        return Err(ApiError::validation(
            "Subscription title must be 1 to 255 printable characters",
        )
        .field("title"));
    }
    let normalized_feed = req
        .feed_url
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned);
    req.feed_url = match normalized_feed {
        Some(raw_feed) => Some(validate_ssrf_safe_url(&raw_feed, "feedUrl", true)?),
        None => None,
    };
    if let Some(ref qr) = req.quality_rule
        && (qr.chars().count() > 128 || qr.chars().any(char::is_control))
    {
        return Err(ApiError::validation("Invalid qualityRule").field("qualityRule"));
    }

    let created = state
        .db
        .downloads()
        .create_subscription(current.user.id, &req)
        .await?;
    Ok((StatusCode::CREATED, Json(created)))
}

/// `DELETE /api/v1/downloads/subscriptions/{id}`
pub async fn delete_subscription(
    current: CurrentUser,
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
) -> Result<StatusCode, ApiError> {
    let id = path?.0;
    if !state
        .db
        .downloads()
        .delete_subscription(&id, current.user.id, current.is_admin())
        .await?
    {
        return Err(ApiError::not_found("Subscription not found"));
    }
    Ok(StatusCode::NO_CONTENT)
}

/// `POST /api/v1/downloads/subscriptions/{id}/check`
pub async fn check_subscription(
    current: CurrentUser,
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
) -> Result<Json<SubscriptionCheckResult>, ApiError> {
    let id = path?.0;
    let sub = state
        .db
        .downloads()
        .get_subscription(&id)
        .await?
        .ok_or_else(|| ApiError::not_found("Subscription not found"))?;
    if !current.is_admin() && sub.owner_id != current.user.id.to_string() {
        return Err(ApiError::not_found("Subscription not found"));
    }
    let Some(ref raw_feed) = sub.feed_url else {
        return Err(
            ApiError::validation("Subscription has no RSS/Atom feedUrl configured")
                .field("feedUrl"),
        );
    };
    let feed_url = validate_ssrf_safe_url(raw_feed, "feedUrl", true)?;

    let client = http_client(FEED_TIMEOUT)?;
    let resp = client.get(&feed_url).send().await.map_err(|e| {
        ApiError::new(
            StatusCode::BAD_GATEWAY,
            "unavailable",
            format!("Cannot fetch RSS feed: {e}"),
        )
    })?;
    if !resp.status().is_success() {
        return Err(ApiError::new(
            StatusCode::BAD_GATEWAY,
            "unavailable",
            format!("RSS feed returned HTTP {}", resp.status().as_u16()),
        ));
    }
    let bytes = resp.bytes().await.map_err(|_| {
        ApiError::new(
            StatusCode::BAD_GATEWAY,
            "unavailable",
            "Failed to read RSS feed body",
        )
    })?;
    if bytes.len() > MAX_FEED_BYTES {
        return Err(ApiError::validation("RSS feed exceeds 2 MiB limit").field("feedUrl"));
    }
    let xml = String::from_utf8_lossy(&bytes);
    let (total_feed_items, matches) =
        parse_and_match_feed(&xml, &sub.title, sub.quality_rule.as_deref());

    let now_ms = clock::to_millis(clock::now());
    let updated_sub = state
        .db
        .downloads()
        .mark_subscription_checked(&sub.id, now_ms)
        .await?
        .unwrap_or(sub);

    Ok(Json(SubscriptionCheckResult {
        subscription: updated_sub,
        total_feed_items,
        matches,
        submitted_count: 0,
    }))
}

// ---------------------------------------------------------------------------
// External Downloader Protocol Adapters (`qBittorrent`, `Transmission`, `aria2`)
// ---------------------------------------------------------------------------

async fn qbittorrent_cookie(
    client: &reqwest::Client,
    base_url: &str,
    cred: &StoredDownloaderCredential,
) -> Result<Option<String>, String> {
    let (Some(user), Some(pass)) = (&cred.username, &cred.password) else {
        return Ok(None);
    };
    let login_url = format!("{base_url}/api/v2/auth/login");
    let resp = client
        .post(&login_url)
        .header("Referer", base_url)
        .form(&[("username", user.as_str()), ("password", pass.as_str())])
        .send()
        .await
        .map_err(|e| format!("qBittorrent 连接失败: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!(
            "qBittorrent 登录返回 HTTP {}",
            resp.status().as_u16()
        ));
    }
    let cookie = resp
        .headers()
        .get_all(reqwest::header::SET_COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .find_map(|raw| raw.split(';').next().filter(|s| s.starts_with("SID=")))
        .map(str::to_owned);
    Ok(cookie)
}

async fn poll_client_transfers(
    client: &reqwest::Client,
    rec: &DownloaderClientRecord,
    cred: &StoredDownloaderCredential,
) -> Result<Vec<DownloadTransfer>, String> {
    let base_url =
        validate_ssrf_safe_url(&rec.base_url, "baseUrl", false).map_err(|e| e.to_string())?;
    match rec.kind {
        DownloaderKind::Qbittorrent => {
            let sid = qbittorrent_cookie(client, &base_url, cred).await?;
            let url = format!("{base_url}/api/v2/torrents/info");
            let mut req = client.get(&url);
            if let Some(cookie) = sid {
                req = req.header(reqwest::header::COOKIE, cookie);
            }
            let resp = req
                .send()
                .await
                .map_err(|e| format!("qBittorrent 无法访问: {e}"))?;
            if !resp.status().is_success() {
                return Err(format!("qBittorrent HTTP {}", resp.status().as_u16()));
            }
            let items: Vec<Value> = resp
                .json()
                .await
                .map_err(|_| "qBittorrent 响应非有效 JSON".to_owned())?;
            Ok(items
                .into_iter()
                .enumerate()
                .map(|(idx, v)| {
                    let raw_state = v["state"].as_str().unwrap_or("");
                    let progress = v["progress"].as_f64().unwrap_or(0.0).clamp(0.0, 1.0);
                    let state = match raw_state {
                        "downloading" | "metaDL" | "forcedDL" | "stalledDL" => {
                            TransferState::Downloading
                        }
                        "uploading" | "forcedUP" | "stalledUP" => TransferState::Seeding,
                        "pausedDL" | "pausedUP" | "stoppedDL" | "stoppedUP" => {
                            if progress >= 1.0 {
                                TransferState::Completed
                            } else {
                                TransferState::Paused
                            }
                        }
                        "queuedDL" | "queuedUP" | "checkingDL" | "checkingUP" => {
                            TransferState::Queued
                        }
                        "error" | "missingFiles" => TransferState::Error,
                        _ if progress >= 1.0 => TransferState::Completed,
                        _ => TransferState::Downloading,
                    };
                    let eta_raw = v["eta"].as_i64().unwrap_or(-1);
                    DownloadTransfer {
                        id: v["hash"]
                            .as_str()
                            .map_or_else(|| format!("qb-{idx}"), str::to_owned),
                        client_id: rec.id.clone(),
                        client_name: rec.name.clone(),
                        client_kind: rec.kind,
                        name: v["name"].as_str().unwrap_or("未命名任务").to_owned(),
                        state,
                        progress,
                        size_bytes: v["size"]
                            .as_i64()
                            .or_else(|| v["total_size"].as_i64())
                            .unwrap_or(0),
                        downloaded_bytes: v["completed"]
                            .as_i64()
                            .or_else(|| v["downloaded"].as_i64())
                            .unwrap_or(0),
                        download_speed_bps: v["dlspeed"].as_i64().unwrap_or(0),
                        upload_speed_bps: v["upspeed"].as_i64().unwrap_or(0),
                        eta_seconds: (0..8_640_000).contains(&eta_raw).then_some(eta_raw),
                        save_path: v["save_path"].as_str().map(str::to_owned),
                    }
                })
                .collect())
        }
        DownloaderKind::Transmission => {
            let payload = json!({
                "method": "torrent-get",
                "arguments": {
                    "fields": [
                        "id", "hashString", "name", "status", "percentDone",
                        "totalSize", "downloadedEver", "rateDownload", "rateUpload",
                        "eta", "downloadDir"
                    ]
                }
            });
            let resp_val = transmission_rpc(client, &base_url, cred, &payload).await?;
            let list = resp_val["arguments"]["torrents"]
                .as_array()
                .cloned()
                .unwrap_or_default();
            Ok(list
                .into_iter()
                .enumerate()
                .map(|(idx, v)| {
                    let status_num = v["status"].as_i64().unwrap_or(0);
                    let progress = v["percentDone"].as_f64().unwrap_or(0.0).clamp(0.0, 1.0);
                    let state = match status_num {
                        0 => {
                            if progress >= 1.0 {
                                TransferState::Completed
                            } else {
                                TransferState::Paused
                            }
                        }
                        1..=3 => TransferState::Queued,
                        4 => TransferState::Downloading,
                        5 | 6 => TransferState::Seeding,
                        _ => TransferState::Downloading,
                    };
                    let eta_raw = v["eta"].as_i64().unwrap_or(-1);
                    DownloadTransfer {
                        id: v["hashString"]
                            .as_str()
                            .map(str::to_owned)
                            .or_else(|| v["id"].as_i64().map(|n| n.to_string()))
                            .unwrap_or_else(|| format!("tr-{idx}")),
                        client_id: rec.id.clone(),
                        client_name: rec.name.clone(),
                        client_kind: rec.kind,
                        name: v["name"].as_str().unwrap_or("Transmission 任务").to_owned(),
                        state,
                        progress,
                        size_bytes: v["totalSize"].as_i64().unwrap_or(0),
                        downloaded_bytes: v["downloadedEver"].as_i64().unwrap_or(0),
                        download_speed_bps: v["rateDownload"].as_i64().unwrap_or(0),
                        upload_speed_bps: v["rateUpload"].as_i64().unwrap_or(0),
                        eta_seconds: (eta_raw >= 0).then_some(eta_raw),
                        save_path: v["downloadDir"].as_str().map(str::to_owned),
                    }
                })
                .collect())
        }
        DownloaderKind::Aria2 => {
            let endpoint = if base_url.ends_with("/jsonrpc") || base_url.ends_with("/rpc") {
                base_url
            } else {
                format!("{base_url}/jsonrpc")
            };
            let mut params = Vec::new();
            if let Some(ref secret) = cred.password
                && !secret.is_empty()
            {
                params.push(json!(format!("token:{secret}")));
            }
            let payload = json!({
                "jsonrpc": "2.0",
                "id": "palveni-poll",
                "method": "aria2.tellActive",
                "params": params,
            });
            let resp = client
                .post(&endpoint)
                .json(&payload)
                .send()
                .await
                .map_err(|e| format!("aria2 无法访问: {e}"))?;
            if !resp.status().is_success() {
                return Err(format!("aria2 HTTP {}", resp.status().as_u16()));
            }
            let body: Value = resp
                .json()
                .await
                .map_err(|_| "aria2 响应非有效 JSON".to_owned())?;
            if let Some(err_obj) = body.get("error") {
                return Err(format!("aria2 RPC 错误: {err_obj}"));
            }
            let list = body["result"].as_array().cloned().unwrap_or_default();
            Ok(list
                .into_iter()
                .enumerate()
                .map(|(idx, v)| {
                    let total = v["totalLength"]
                        .as_str()
                        .and_then(|s| s.parse::<i64>().ok())
                        .or_else(|| v["totalLength"].as_i64())
                        .unwrap_or(0);
                    let completed = v["completedLength"]
                        .as_str()
                        .and_then(|s| s.parse::<i64>().ok())
                        .or_else(|| v["completedLength"].as_i64())
                        .unwrap_or(0);
                    let dl_speed = v["downloadSpeed"]
                        .as_str()
                        .and_then(|s| s.parse::<i64>().ok())
                        .or_else(|| v["downloadSpeed"].as_i64())
                        .unwrap_or(0);
                    let up_speed = v["uploadSpeed"]
                        .as_str()
                        .and_then(|s| s.parse::<i64>().ok())
                        .or_else(|| v["uploadSpeed"].as_i64())
                        .unwrap_or(0);
                    let progress = if total > 0 {
                        (completed as f64 / total as f64).clamp(0.0, 1.0)
                    } else {
                        0.0
                    };
                    let status_str = v["status"].as_str().unwrap_or("active");
                    let state = match status_str {
                        "active" => TransferState::Downloading,
                        "waiting" => TransferState::Queued,
                        "paused" => TransferState::Paused,
                        "complete" => TransferState::Completed,
                        "error" => TransferState::Error,
                        _ => TransferState::Downloading,
                    };
                    let bt_name = v["bittorrent"]["info"]["name"].as_str();
                    let file_name = v["files"][0]["path"]
                        .as_str()
                        .and_then(|p| p.rsplit('/').next())
                        .filter(|s| !s.is_empty());
                    let eta_seconds = if dl_speed > 0 && total > completed {
                        Some((total - completed) / dl_speed)
                    } else {
                        None
                    };
                    DownloadTransfer {
                        id: v["gid"]
                            .as_str()
                            .map_or_else(|| format!("aria2-{idx}"), str::to_owned),
                        client_id: rec.id.clone(),
                        client_name: rec.name.clone(),
                        client_kind: rec.kind,
                        name: bt_name.or(file_name).unwrap_or("aria2 下载任务").to_owned(),
                        state,
                        progress,
                        size_bytes: total,
                        downloaded_bytes: completed,
                        download_speed_bps: dl_speed,
                        upload_speed_bps: up_speed,
                        eta_seconds,
                        save_path: v["dir"].as_str().map(str::to_owned),
                    }
                })
                .collect())
        }
    }
}

async fn transmission_rpc(
    client: &reqwest::Client,
    base_url: &str,
    cred: &StoredDownloaderCredential,
    payload: &Value,
) -> Result<Value, String> {
    let endpoint = if base_url.ends_with("/transmission/rpc") || base_url.ends_with("/rpc") {
        base_url.to_owned()
    } else {
        format!("{base_url}/transmission/rpc")
    };

    let mut session_id: Option<String> = None;
    for _attempt in 0..2 {
        let mut req = client.post(&endpoint).json(payload);
        if let Some(ref user) = cred.username {
            req = req.basic_auth(user, cred.password.as_deref());
        }
        if let Some(ref sid) = session_id {
            req = req.header("X-Transmission-Session-Id", sid);
        }
        let resp = req
            .send()
            .await
            .map_err(|e| format!("Transmission 无法访问: {e}"))?;
        if resp.status() == reqwest::StatusCode::CONFLICT
            && session_id.is_none()
            && let Some(hdr) = resp
                .headers()
                .get("X-Transmission-Session-Id")
                .and_then(|v| v.to_str().ok())
        {
            session_id = Some(hdr.to_owned());
            continue;
        }
        if !resp.status().is_success() {
            return Err(format!("Transmission HTTP {}", resp.status().as_u16()));
        }
        let body: Value = resp
            .json()
            .await
            .map_err(|_| "Transmission 响应非有效 JSON".to_owned())?;
        if body["result"].as_str() != Some("success") {
            return Err(format!(
                "Transmission RPC 返回: {}",
                body["result"].as_str().unwrap_or("unknown")
            ));
        }
        return Ok(body);
    }
    Err("Transmission 会话握手失败".to_owned())
}

async fn submit_download_uri(
    client: &reqwest::Client,
    rec: &DownloaderClientRecord,
    cred: &StoredDownloaderCredential,
    uri: &str,
    save_dir: Option<&str>,
    category: Option<&str>,
) -> Result<(), String> {
    let base_url =
        validate_ssrf_safe_url(&rec.base_url, "baseUrl", false).map_err(|e| e.to_string())?;
    match rec.kind {
        DownloaderKind::Qbittorrent => {
            let sid = qbittorrent_cookie(client, &base_url, cred).await?;
            let url = format!("{base_url}/api/v2/torrents/add");
            let mut form: Vec<(&str, &str)> = vec![("urls", uri)];
            if let Some(dir) = save_dir {
                form.push(("savepath", dir));
            }
            if let Some(cat) = category.filter(|s| !s.trim().is_empty()) {
                form.push(("category", cat.trim()));
            }
            let mut req = client.post(&url).form(&form);
            if let Some(cookie) = sid {
                req = req.header(reqwest::header::COOKIE, cookie);
            }
            let resp = req
                .send()
                .await
                .map_err(|e| format!("投递至 qBittorrent 失败: {e}"))?;
            if !resp.status().is_success() {
                return Err(format!("qBittorrent 返回 HTTP {}", resp.status().as_u16()));
            }
            Ok(())
        }
        DownloaderKind::Transmission => {
            let mut args = json!({ "filename": uri });
            if let Some(dir) = save_dir {
                args["download-dir"] = json!(dir);
            }
            let payload = json!({
                "method": "torrent-add",
                "arguments": args,
            });
            let _ = transmission_rpc(client, &base_url, cred, &payload).await?;
            Ok(())
        }
        DownloaderKind::Aria2 => {
            let endpoint = if base_url.ends_with("/jsonrpc") || base_url.ends_with("/rpc") {
                base_url
            } else {
                format!("{base_url}/jsonrpc")
            };
            let mut params = Vec::new();
            if let Some(ref secret) = cred.password
                && !secret.is_empty()
            {
                params.push(json!(format!("token:{secret}")));
            }
            params.push(json!([uri]));
            if let Some(dir) = save_dir {
                params.push(json!({ "dir": dir }));
            }
            let payload = json!({
                "jsonrpc": "2.0",
                "id": "palveni-add",
                "method": "aria2.addUri",
                "params": params,
            });
            let resp = client
                .post(&endpoint)
                .json(&payload)
                .send()
                .await
                .map_err(|e| format!("投递至 aria2 失败: {e}"))?;
            if !resp.status().is_success() {
                return Err(format!("aria2 返回 HTTP {}", resp.status().as_u16()));
            }
            let body: Value = resp
                .json()
                .await
                .map_err(|_| "aria2 响应非有效 JSON".to_owned())?;
            if let Some(err) = body.get("error") {
                return Err(format!("aria2 错误: {err}"));
            }
            Ok(())
        }
    }
}

// ---------------------------------------------------------------------------
// Pure Safe Rust RSS 2.0 (`<item>`) & Atom (`<entry>`) Matcher
// ---------------------------------------------------------------------------

pub(crate) fn parse_and_match_feed(
    xml: &str,
    subscription_title: &str,
    quality_rule: Option<&str>,
) -> (usize, Vec<FeedMatchItem>) {
    let mut raw_blocks = collect_tag_blocks(xml, "item");
    if raw_blocks.is_empty() {
        raw_blocks = collect_tag_blocks(xml, "entry");
    }
    let total = raw_blocks.len();

    let title_tokens: Vec<String> = subscription_title
        .split_whitespace()
        .map(|s| s.to_lowercase())
        .filter(|s| !s.is_empty())
        .collect();
    let rule_clean = quality_rule.map(str::trim).filter(|s| !s.is_empty());

    let mut matches = Vec::new();
    for block in raw_blocks {
        let Some(raw_title) = extract_tag_text(block, "title") else {
            continue;
        };
        let decoded_title = decode_xml_text(&raw_title);
        if decoded_title.is_empty() {
            continue;
        }
        let lower_title = decoded_title.to_lowercase();
        if !title_tokens.iter().all(|tok| lower_title.contains(tok)) {
            continue;
        }

        let matched_quality = match rule_clean {
            Some(rule) => matches_quality_rule(&lower_title, rule),
            None => true,
        };
        if !matched_quality {
            continue;
        }

        let link = extract_attr(block, "enclosure", "url")
            .or_else(|| extract_attr(block, "link", "href"))
            .or_else(|| extract_tag_text(block, "link"))
            .map(|s| decode_xml_text(&s))
            .filter(|s| !s.is_empty());

        let pub_date = extract_tag_text(block, "pubDate")
            .or_else(|| extract_tag_text(block, "published"))
            .or_else(|| extract_tag_text(block, "updated"))
            .map(|s| decode_xml_text(&s))
            .filter(|s| !s.is_empty());

        matches.push(FeedMatchItem {
            title: decoded_title,
            link,
            pub_date,
            matched_quality,
        });
    }

    (total, matches)
}

fn matches_quality_rule(lower_title: &str, rule: &str) -> bool {
    // Supports OR groups separated by `|` or `,`, with whitespace-separated AND tokens inside each group.
    rule.split(['|', ','])
        .map(str::trim)
        .filter(|g| !g.is_empty())
        .any(|group| {
            group
                .split_whitespace()
                .map(|tok| tok.to_lowercase())
                .all(|tok| lower_title.contains(&tok))
        })
}

fn collect_tag_blocks<'a>(xml: &'a str, tag: &str) -> Vec<&'a str> {
    let open_exact = format!("<{tag}>");
    let open_attr = format!("<{tag} ");
    let close_tag = format!("</{tag}>");
    let mut out = Vec::new();
    let mut rest = xml;
    while let Some(pos) = find_either(rest, &open_exact, &open_attr) {
        let after_lt = &rest[pos..];
        let Some(gt_offset) = after_lt.find('>') else {
            break;
        };
        let inner_start = pos + gt_offset + 1;
        let after_open = &rest[inner_start..];
        let Some(end_offset) = after_open.find(&close_tag) else {
            break;
        };
        out.push(&after_open[..end_offset]);
        rest = &after_open[end_offset + close_tag.len()..];
    }
    out
}

fn find_either(haystack: &str, a: &str, b: &str) -> Option<usize> {
    match (haystack.find(a), haystack.find(b)) {
        (Some(x), Some(y)) => Some(x.min(y)),
        (Some(x), None) => Some(x),
        (None, Some(y)) => Some(y),
        (None, None) => None,
    }
}

fn extract_tag_text(block: &str, tag: &str) -> Option<String> {
    let items = collect_tag_blocks(block, tag);
    items.first().map(|s| (*s).to_owned())
}

fn extract_attr(block: &str, tag: &str, attr: &str) -> Option<String> {
    let open_prefix = format!("<{tag}");
    let mut rest = block;
    while let Some(start) = rest.find(&open_prefix) {
        let after = &rest[start + open_prefix.len()..];
        if !after.starts_with([' ', '\t', '\r', '\n']) {
            rest = after;
            continue;
        }
        let Some(end_gt) = after.find('>') else {
            break;
        };
        let tag_header = &after[..end_gt];
        for quote in ['"', '\''] {
            let needle = format!("{attr}={quote}");
            if let Some(apos) = tag_header.find(&needle) {
                let val_start = apos + needle.len();
                if let Some(val_end) = tag_header[val_start..].find(quote) {
                    return Some(tag_header[val_start..val_start + val_end].to_owned());
                }
            }
        }
        rest = &after[end_gt + 1..];
    }
    None
}

fn decode_xml_text(raw: &str) -> String {
    let trimmed = raw.trim();
    let inner = if let Some(rest) = trimmed.strip_prefix("<![CDATA[")
        && let Some(cdata) = rest.strip_suffix("]]>")
    {
        cdata.trim()
    } else {
        trimmed
    };
    inner
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&#39;", "'")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_rss_and_atom_feeds_matching_title_and_quality_rule() {
        let rss = r#"<?xml version="1.0" encoding="UTF-8"?>
        <rss version="2.0"><channel>
          <item>
            <title><![CDATA[[SubGroup] 三体 S01E05 2160p WEB-DL HEVC]]></title>
            <enclosure url="magnet:?xt=urn:btih:11111111111111111111" type="application/x-bittorrent"/>
            <pubDate>Mon, 23 Nov 2026 08:00:00 GMT</pubDate>
          </item>
          <item>
            <title>三体 S01E05 720p MP4</title>
            <link>https://example.com/720p.torrent</link>
          </item>
          <item>
            <title>Other Show 2160p HEVC</title>
          </item>
        </channel></rss>"#;

        let (total, hits) = parse_and_match_feed(rss, "三体", Some("2160p HEVC"));
        assert_eq!(total, 3);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].title, "[SubGroup] 三体 S01E05 2160p WEB-DL HEVC");
        assert_eq!(
            hits[0].link.as_deref(),
            Some("magnet:?xt=urn:btih:11111111111111111111")
        );

        let atom = r#"<feed xmlns="http://www.w3.org/2005/Atom">
          <entry>
            <title>Dune Part Two 2024 4K HDR</title>
            <link href="https://example.com/dune-4k.torrent"/>
            <updated>2026-11-23T08:00:00Z</updated>
          </entry>
        </feed>"#;
        let (atom_total, atom_hits) = parse_and_match_feed(atom, "Dune", Some("2160p | 4K"));
        assert_eq!(atom_total, 1);
        assert_eq!(atom_hits.len(), 1);
        assert_eq!(
            atom_hits[0].link.as_deref(),
            Some("https://example.com/dune-4k.torrent")
        );
    }

    #[test]
    fn ssrf_url_validator_blocks_metadata_and_credentials() {
        assert!(validate_ssrf_safe_url("http://169.254.169.254/latest", "url", true).is_err());
        assert!(validate_ssrf_safe_url("http://100.100.100.200/latest", "url", true).is_err());
        assert!(validate_ssrf_safe_url("http://0.0.0.0:8080", "url", false).is_err());
        assert!(validate_ssrf_safe_url("http://user:pass@192.168.1.2:8080", "url", false).is_err());
        assert!(validate_ssrf_safe_url("http://192.168.1.20:8080", "url", false).is_ok());
    }
}

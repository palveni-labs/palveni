//! Notification Center, External Push Channels, Inbound Media Automation Webhooks,
//! Scheduled Routines, and Background Event/Ticker Service (`modules: notifications, routines`).

use std::collections::VecDeque;
use std::net::IpAddr;
use std::time::Duration;

use axum::Json;
use axum::body::Bytes;
use axum::extract::rejection::{PathRejection, QueryRejection};
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use palveni_core::assistant::{RunState, SendMessageRequest};
use palveni_core::ids::{JobId, LibraryId, SessionId, UserId};
use palveni_core::notifications::{
    CreateInboundWebhookRequest, CreateNotificationChannelRequest, CreateRoutineRequest,
    CreatedInboundWebhook, InboundWebhookList, InboundWebhookSourceKind,
    InboundWebhookTriggerResponse, NotificationChannel, NotificationChannelConfig,
    NotificationChannelKind, NotificationChannelList, NotificationDeliveryStatus, NotificationList,
    ReadAllNotificationsResponse, RoutineActionKind, RoutineScheduleKind, RunRoutineResponse,
    ScheduledRoutine, ScheduledRoutineList, TestChannelResponse, UpdateNotificationChannelRequest,
    UpdateRoutineRequest, event_matches, is_in_quiet_hours, is_periodic_routine_due,
    normalize_quiet_hours, normalize_schedule_expr,
};
use palveni_core::{JobState, LibraryKind, ServerEvent, clock};
use palveni_db::audit::AuditEntry;
use palveni_db::notifications::{
    NewNotificationDelivery, NewRoutine, NotificationChannelPatch, NotificationChannelRecord,
    RoutinePatch,
};
use ring::rand::{SecureRandom, SystemRandom};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::auth::{AdminUser, CurrentUser};
use crate::error::ApiError;
use crate::json::ApiJson;
use crate::routes::home::{activate_scene_internal, decrypt_secret, encrypt_secret};
use crate::routes::{DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE, audit, parse_limit, path_id};
use crate::state::AppState;

const NOTIFICATION_SECRET_SCOPE: &[u8] = b"palveni:notification-channel:v1";
const PUSH_TIMEOUT: Duration = Duration::from_secs(8);
const ROUTINE_TICK_INTERVAL: Duration = Duration::from_secs(60);
const ON_EVENT_COOLDOWN_MS: i64 = 30_000;
const SEEN_EVENT_KEYS_MAX: usize = 256;

fn validate_label(raw: &str, field: &'static str) -> Result<String, ApiError> {
    let trimmed = raw.trim();
    let chars = trimmed.chars().count();
    if chars == 0 || chars > 128 || trimmed.chars().any(char::is_control) {
        return Err(
            ApiError::validation(format!("{field} must be 1 to 128 characters")).field(field),
        );
    }
    Ok(trimmed.to_owned())
}

/// Validates an outbound push webhook URL: `http`/`https` only, no userinfo, no fragment,
/// blocks unspecified/multicast/cloud-metadata IP literals while allowing LAN/localhost receivers.
pub(crate) fn validate_push_url(raw: &str, field: &'static str) -> Result<String, ApiError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.len() > 1024 || trimmed.chars().any(char::is_control) {
        return Err(ApiError::validation("Invalid push endpoint URL").field(field));
    }
    let parsed = reqwest::Url::parse(trimmed).map_err(|_| {
        ApiError::validation("Expected a valid http:// or https:// URL").field(field)
    })?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(ApiError::validation("URL scheme must be http or https").field(field));
    }
    if !parsed.username().is_empty() || parsed.password().is_some() {
        return Err(ApiError::validation("URL must not contain credentials").field(field));
    }
    if parsed.fragment().is_some() {
        return Err(ApiError::validation("URL must not include a fragment").field(field));
    }
    let Some(host) = parsed.host_str() else {
        return Err(ApiError::validation("URL must include a host").field(field));
    };
    if let Ok(ip) = host.trim_matches(['[', ']']).parse::<IpAddr>() {
        let blocked = match ip {
            IpAddr::V4(v4) => {
                v4.is_unspecified() || v4.is_multicast() || v4.octets()[..2] == [169, 254]
            }
            IpAddr::V6(v6) => v6.is_unspecified() || v6.is_multicast(),
        };
        if blocked {
            return Err(ApiError::validation(
                "Link-local metadata and unspecified addresses are forbidden",
            )
            .field(field));
        }
    }
    Ok(parsed.as_str().to_owned())
}

fn validate_channel_config(
    kind: NotificationChannelKind,
    cfg: &NotificationChannelConfig,
) -> Result<NotificationChannelConfig, ApiError> {
    let clean_opt = |v: &Option<String>, max_len: usize, field: &'static str| {
        let Some(s) = v.as_deref().map(str::trim).filter(|s| !s.is_empty()) else {
            return Ok(None);
        };
        if s.len() > max_len || s.chars().any(char::is_control) {
            return Err(ApiError::validation(format!("Invalid {field}")).field(field));
        }
        Ok(Some(s.to_owned()))
    };

    let token = clean_opt(&cfg.token, 1024, "config.token")?;
    let target = clean_opt(&cfg.target, 256, "config.target")?;
    let url = match cfg.url.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        Some(raw_url) => Some(validate_push_url(raw_url, "config.url")?),
        None => None,
    };

    match kind {
        NotificationChannelKind::Telegram => {
            let Some(ref bot_token) = token else {
                return Err(
                    ApiError::validation("Telegram bot token is required").field("config.token")
                );
            };
            if bot_token.contains('/') || bot_token.chars().any(char::is_whitespace) {
                return Err(
                    ApiError::validation("Invalid Telegram bot token").field("config.token")
                );
            }
            if target.is_none() {
                return Err(
                    ApiError::validation("Telegram chat_id is required").field("config.target")
                );
            }
        }
        NotificationChannelKind::Webhook
        | NotificationChannelKind::Bark
        | NotificationChannelKind::Ntfy
        | NotificationChannelKind::Wecom
        | NotificationChannelKind::Feishu
        | NotificationChannelKind::Discord => {
            if url.is_none() {
                return Err(
                    ApiError::validation("Push endpoint URL is required").field("config.url")
                );
            }
        }
    }

    Ok(NotificationChannelConfig { url, token, target })
}

fn validate_events(raw: &[String]) -> Result<Vec<String>, ApiError> {
    if raw.is_empty() {
        return Ok(vec!["*".to_owned()]);
    }
    if raw.len() > 32 {
        return Err(ApiError::validation("At most 32 event filters are allowed").field("events"));
    }
    let mut out = Vec::with_capacity(raw.len());
    for ev in raw {
        let t = ev.trim();
        if t.is_empty() || t.len() > 64 || t.chars().any(|c| c.is_control() || c.is_whitespace()) {
            return Err(ApiError::validation("Invalid event filter").field("events"));
        }
        if !out.contains(&t.to_owned()) {
            out.push(t.to_owned());
        }
    }
    Ok(out)
}

fn encrypt_channel_config(
    state: &AppState,
    channel_id: &str,
    cfg: &NotificationChannelConfig,
) -> Result<String, ApiError> {
    let raw = serde_json::to_string(cfg).map_err(ApiError::internal)?;
    encrypt_secret(&state.identity, NOTIFICATION_SECRET_SCOPE, channel_id, &raw)
}

fn decrypt_channel_config(
    state: &AppState,
    channel_id: &str,
    cipher: &str,
) -> Result<NotificationChannelConfig, ApiError> {
    let plain = decrypt_secret(
        &state.identity,
        NOTIFICATION_SECRET_SCOPE,
        channel_id,
        cipher,
    )?;
    serde_json::from_str(&plain).map_err(|_| ApiError::internal("Corrupt channel config"))
}

fn push_http_client(target_url: Option<&str>) -> Result<reqwest::Client, String> {
    let mut builder = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(PUSH_TIMEOUT);
    if target_url
        .and_then(|u| reqwest::Url::parse(u).ok())
        .and_then(|u| {
            u.host_str()
                .and_then(|h| h.trim_matches(['[', ']']).parse::<IpAddr>().ok())
        })
        .is_some_and(|ip| match ip {
            IpAddr::V4(v4) => v4.is_loopback() || v4.is_private(),
            IpAddr::V6(v6) => v6.is_loopback() || v6.is_unique_local(),
        })
    {
        builder = builder.no_proxy();
    }
    builder
        .build()
        .map_err(|e| format!("HTTP client error: {e}"))
}

fn build_external_push_request(
    client: &reqwest::Client,
    kind: NotificationChannelKind,
    cfg: &NotificationChannelConfig,
    event_type: &str,
    title: &str,
    body: &str,
) -> Result<reqwest::Request, String> {
    let now = clock::now();

    let builder = match kind {
        NotificationChannelKind::Webhook => {
            let url = cfg.url.as_deref().ok_or("Missing webhook URL")?;
            validate_push_url(url, "url").map_err(|e| e.to_string())?;
            let mut req = client.post(url).json(&json!({
                "event": event_type,
                "title": title,
                "body": body,
                "timestamp": now.to_rfc3339(),
            }));
            if let Some(secret) = cfg.token.as_deref().filter(|s| !s.is_empty()) {
                req = req.bearer_auth(secret).header("X-Palveni-Token", secret);
            }
            req
        }
        NotificationChannelKind::Telegram => {
            let base = cfg
                .url
                .as_deref()
                .filter(|s| !s.is_empty())
                .unwrap_or("https://api.telegram.org")
                .trim_end_matches('/');
            validate_push_url(base, "url").map_err(|e| e.to_string())?;
            let token = cfg.token.as_deref().ok_or("Missing Telegram bot token")?;
            let chat_id = cfg.target.as_deref().ok_or("Missing Telegram chat_id")?;
            let endpoint = format!("{base}/bot{token}/sendMessage");
            client.post(&endpoint).json(&json!({
                "chat_id": chat_id,
                "text": format!("{title}\n{body}"),
            }))
        }
        NotificationChannelKind::Bark => {
            let base = cfg
                .url
                .as_deref()
                .ok_or("Missing Bark URL")?
                .trim_end_matches('/');
            validate_push_url(base, "url").map_err(|e| e.to_string())?;
            if let Some(device_key) = cfg.token.as_deref().filter(|s| !s.is_empty()) {
                let endpoint = if base.ends_with("/push") {
                    base.to_owned()
                } else {
                    format!("{base}/push")
                };
                client.post(&endpoint).json(&json!({
                    "title": title,
                    "body": body,
                    "device_key": device_key,
                }))
            } else {
                client.post(base).json(&json!({
                    "title": title,
                    "body": body,
                }))
            }
        }
        NotificationChannelKind::Ntfy => {
            let base = cfg
                .url
                .as_deref()
                .ok_or("Missing ntfy URL")?
                .trim_end_matches('/');
            validate_push_url(base, "url").map_err(|e| e.to_string())?;
            let endpoint = match cfg.target.as_deref().filter(|s| !s.is_empty()) {
                Some(topic) if !base.ends_with(&format!("/{topic}")) => {
                    format!("{base}/{}", topic.trim_matches('/'))
                }
                _ => base.to_owned(),
            };
            let safe_title: String = title.chars().filter(|c| !c.is_control()).collect();
            let mut req = client
                .post(&endpoint)
                .header("Title", safe_title)
                .body(body.to_owned());
            if let Some(token) = cfg.token.as_deref().filter(|s| !s.is_empty()) {
                req = req.bearer_auth(token);
            }
            req
        }
        NotificationChannelKind::Wecom => {
            let url = cfg.url.as_deref().ok_or("Missing WeCom webhook URL")?;
            validate_push_url(url, "url").map_err(|e| e.to_string())?;
            client.post(url).json(&json!({
                "msgtype": "text",
                "text": { "content": format!("{title}\n{body}") },
            }))
        }
        NotificationChannelKind::Feishu => {
            let url = cfg.url.as_deref().ok_or("Missing Feishu webhook URL")?;
            validate_push_url(url, "url").map_err(|e| e.to_string())?;
            client.post(url).json(&json!({
                "msg_type": "text",
                "content": { "text": format!("{title}\n{body}") },
            }))
        }
        NotificationChannelKind::Discord => {
            let url = cfg.url.as_deref().ok_or("Missing Discord webhook URL")?;
            validate_push_url(url, "url").map_err(|e| e.to_string())?;
            client.post(url).json(&json!({
                "content": format!("**{title}**\n{body}"),
            }))
        }
    };

    builder
        .build()
        .map_err(|e| format!("Cannot build push request: {e}"))
}

async fn send_external_push(
    kind: NotificationChannelKind,
    cfg: &NotificationChannelConfig,
    event_type: &str,
    title: &str,
    body: &str,
) -> Result<(), String> {
    let client = push_http_client(cfg.url.as_deref())?;
    let request = build_external_push_request(&client, kind, cfg, event_type, title, body)?;
    match client.execute(request).await {
        Ok(resp) if resp.status().is_success() => Ok(()),
        Ok(resp) => Err(format!("Upstream returned HTTP {}", resp.status().as_u16())),
        Err(err) => Err(format!("Push request failed: {}", err.without_url())),
    }
}

/// Records an in-app notification and dispatches matching external push channels (honoring quiet hours).
pub async fn dispatch_notification_event(
    state: &AppState,
    owner_scope: Option<UserId>,
    event_type: &str,
    title: &str,
    body: &str,
) {
    if let Err(err) = state
        .db
        .notifications()
        .record_delivery(NewNotificationDelivery {
            owner_id: owner_scope,
            channel_id: None,
            event_type: event_type.to_owned(),
            title: title.to_owned(),
            body: body.to_owned(),
            status: NotificationDeliveryStatus::InApp,
            error: None,
        })
        .await
    {
        tracing::warn!(error = %err, event_type, "failed to record in-app notification");
    }

    let Ok(channels) = state.db.notifications().list_enabled_channels().await else {
        return;
    };
    let now = clock::now();

    for ch in channels {
        if let Some(target_owner) = owner_scope
            && ch.owner_id != target_owner
        {
            continue;
        }
        if !ch.events.iter().any(|p| event_matches(p, event_type)) {
            continue;
        }
        if let Some(ref qh) = ch.quiet_hours
            && is_in_quiet_hours(qh, now)
        {
            let _ = state
                .db
                .notifications()
                .record_delivery(NewNotificationDelivery {
                    owner_id: Some(ch.owner_id),
                    channel_id: Some(ch.id.clone()),
                    event_type: event_type.to_owned(),
                    title: title.to_owned(),
                    body: body.to_owned(),
                    status: NotificationDeliveryStatus::SkippedQuiet,
                    error: None,
                })
                .await;
            continue;
        }

        let cfg = match decrypt_channel_config(state, &ch.id, &ch.config_cipher) {
            Ok(c) => c,
            Err(err) => {
                let _ = state
                    .db
                    .notifications()
                    .record_delivery(NewNotificationDelivery {
                        owner_id: Some(ch.owner_id),
                        channel_id: Some(ch.id.clone()),
                        event_type: event_type.to_owned(),
                        title: title.to_owned(),
                        body: body.to_owned(),
                        status: NotificationDeliveryStatus::Failed,
                        error: Some(err.to_string()),
                    })
                    .await;
                continue;
            }
        };

        let (status, error) = match send_external_push(ch.kind, &cfg, event_type, title, body).await
        {
            Ok(()) => (NotificationDeliveryStatus::Sent, None),
            Err(err) => (NotificationDeliveryStatus::Failed, Some(err)),
        };

        let _ = state
            .db
            .notifications()
            .record_delivery(NewNotificationDelivery {
                owner_id: Some(ch.owner_id),
                channel_id: Some(ch.id),
                event_type: event_type.to_owned(),
                title: title.to_owned(),
                body: body.to_owned(),
                status,
                error,
            })
            .await;
    }
}

#[derive(Deserialize)]
pub struct ListNotificationsQuery {
    limit: Option<String>,
}

/// `GET /api/v1/notifications`
pub async fn list_notifications(
    current: CurrentUser,
    State(state): State<AppState>,
    query: Result<Query<ListNotificationsQuery>, QueryRejection>,
) -> Result<Json<NotificationList>, ApiError> {
    let q = query.map_err(|e| ApiError::validation(e.body_text()))?.0;
    let limit = parse_limit(q.limit.as_deref(), DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE)?;
    let list = state
        .db
        .notifications()
        .list_deliveries(current.user.id, current.is_admin(), u64::from(limit))
        .await?;
    Ok(Json(list))
}

/// `POST /api/v1/notifications/read-all`
pub async fn mark_all_read(
    current: CurrentUser,
    State(state): State<AppState>,
) -> Result<Json<ReadAllNotificationsResponse>, ApiError> {
    let marked_read = state
        .db
        .notifications()
        .mark_all_read(current.user.id, current.is_admin())
        .await?;
    Ok(Json(ReadAllNotificationsResponse { marked_read }))
}

/// `GET /api/v1/notifications/channels`
pub async fn list_channels(
    current: CurrentUser,
    State(state): State<AppState>,
) -> Result<Json<NotificationChannelList>, ApiError> {
    let records = state
        .db
        .notifications()
        .list_channels(current.user.id, current.is_admin())
        .await?;
    let mut channels = Vec::with_capacity(records.len());
    for rec in records {
        let cfg = decrypt_channel_config(&state, &rec.id, &rec.config_cipher).ok();
        channels.push(rec.to_public(cfg.as_ref())?);
    }
    Ok(Json(NotificationChannelList { channels }))
}

/// `POST /api/v1/notifications/channels`
pub async fn create_channel(
    current: CurrentUser,
    State(state): State<AppState>,
    body: Result<ApiJson<CreateNotificationChannelRequest>, ApiError>,
) -> Result<(StatusCode, Json<NotificationChannel>), ApiError> {
    let ApiJson(req) = body?;
    let name = validate_label(&req.name, "name")?;
    let cfg = validate_channel_config(req.kind, &req.config)?;
    let events = validate_events(&req.events)?;
    let quiet_hours = match req
        .quiet_hours
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        Some(raw) => Some(normalize_quiet_hours(raw).ok_or_else(|| {
            ApiError::validation("quietHours must be HH:MM-HH:MM (e.g. 23:00-07:00)")
                .field("quietHours")
        })?),
        None => None,
    };

    let id = JobId::generate().to_string();
    let config_cipher = encrypt_channel_config(&state, &id, &cfg)?;
    let now_ms = clock::to_millis(clock::now());

    let record = state
        .db
        .notifications()
        .create_channel(NotificationChannelRecord {
            id: id.clone(),
            owner_id: current.user.id,
            name: name.clone(),
            kind: req.kind,
            config_cipher,
            events,
            quiet_hours,
            enabled: req.enabled,
            created_at: now_ms,
            updated_at: now_ms,
        })
        .await?;

    audit(
        &state,
        AuditEntry {
            action: "notification.channel_created",
            actor: Some(current.user.id),
            target: Some(id),
            detail: Some(json!({ "name": name, "kind": req.kind.as_str() })),
        },
    )
    .await;

    Ok((StatusCode::CREATED, Json(record.to_public(Some(&cfg))?)))
}

/// `PATCH /api/v1/notifications/channels/{id}`
pub async fn update_channel(
    current: CurrentUser,
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
    body: Result<ApiJson<UpdateNotificationChannelRequest>, ApiError>,
) -> Result<Json<NotificationChannel>, ApiError> {
    let id = path?.0;
    let _parsed: JobId = path_id(&id, "Notification channel")?;
    let ApiJson(req) = body?;

    let existing = state
        .db
        .notifications()
        .get_channel(current.user.id, current.is_admin(), &id)
        .await?
        .ok_or_else(|| ApiError::not_found("Notification channel not found"))?;

    let name = req
        .name
        .as_deref()
        .map(|n| validate_label(n, "name"))
        .transpose()?;
    let events = req.events.as_deref().map(validate_events).transpose()?;
    let quiet_hours = match req.quiet_hours {
        Some(ref raw) if raw.trim().is_empty() => Some(None),
        Some(ref raw) => Some(Some(normalize_quiet_hours(raw).ok_or_else(|| {
            ApiError::validation("quietHours must be HH:MM-HH:MM (e.g. 23:00-07:00)")
                .field("quietHours")
        })?)),
        None => None,
    };

    let (config_cipher, new_cfg) = if let Some(ref cfg_input) = req.config {
        let prev = decrypt_channel_config(&state, &existing.id, &existing.config_cipher)
            .unwrap_or_default();
        let merged = NotificationChannelConfig {
            url: cfg_input.url.clone().or(prev.url),
            token: cfg_input.token.clone().or(prev.token),
            target: cfg_input.target.clone().or(prev.target),
        };
        let validated = validate_channel_config(existing.kind, &merged)?;
        (
            Some(encrypt_channel_config(&state, &existing.id, &validated)?),
            Some(validated),
        )
    } else {
        (None, None)
    };

    let updated = state
        .db
        .notifications()
        .update_channel(
            current.user.id,
            current.is_admin(),
            &id,
            NotificationChannelPatch {
                name,
                config_cipher,
                events,
                quiet_hours,
                enabled: req.enabled,
            },
        )
        .await?
        .ok_or_else(|| ApiError::not_found("Notification channel not found"))?;

    let final_cfg = new_cfg
        .or_else(|| decrypt_channel_config(&state, &updated.id, &updated.config_cipher).ok());
    Ok(Json(updated.to_public(final_cfg.as_ref())?))
}

/// `DELETE /api/v1/notifications/channels/{id}`
pub async fn delete_channel(
    current: CurrentUser,
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
) -> Result<StatusCode, ApiError> {
    let id = path?.0;
    let _parsed: JobId = path_id(&id, "Notification channel")?;
    if !state
        .db
        .notifications()
        .delete_channel(current.user.id, current.is_admin(), &id)
        .await?
    {
        return Err(ApiError::not_found("Notification channel not found"));
    }
    Ok(StatusCode::NO_CONTENT)
}

/// `POST /api/v1/notifications/channels/{id}/test`
pub async fn test_channel(
    current: CurrentUser,
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
) -> Result<Json<TestChannelResponse>, ApiError> {
    let id = path?.0;
    let _parsed: JobId = path_id(&id, "Notification channel")?;
    let record = state
        .db
        .notifications()
        .get_channel(current.user.id, current.is_admin(), &id)
        .await?
        .ok_or_else(|| ApiError::not_found("Notification channel not found"))?;
    let cfg = decrypt_channel_config(&state, &record.id, &record.config_cipher)?;

    let title = "Palveni 通知测试";
    let body = format!("通知渠道「{}」测试消息发送成功。", record.name);
    let (status, error) =
        match send_external_push(record.kind, &cfg, "notification.test", title, &body).await {
            Ok(()) => (NotificationDeliveryStatus::Sent, None),
            Err(err) => (NotificationDeliveryStatus::Failed, Some(err)),
        };

    let delivery = state
        .db
        .notifications()
        .record_delivery(NewNotificationDelivery {
            owner_id: Some(record.owner_id),
            channel_id: Some(record.id),
            event_type: "notification.test".into(),
            title: title.into(),
            body,
            status,
            error,
        })
        .await?;

    Ok(Json(TestChannelResponse { delivery }))
}

/// `GET /api/v1/webhooks/inbound`
pub async fn list_inbound_webhooks(
    _: AdminUser,
    State(state): State<AppState>,
) -> Result<Json<InboundWebhookList>, ApiError> {
    let webhooks = state.db.notifications().list_inbound_webhooks().await?;
    Ok(Json(InboundWebhookList { webhooks }))
}

fn generate_webhook_token() -> Result<String, ApiError> {
    let mut raw = [0_u8; 24];
    SystemRandom::new()
        .fill(&mut raw)
        .map_err(|_| ApiError::internal("Secure randomness unavailable"))?;
    Ok(format!("pwh_{}", hex::encode(raw)))
}

/// `POST /api/v1/webhooks/inbound`
pub async fn create_inbound_webhook(
    AdminUser(admin): AdminUser,
    State(state): State<AppState>,
    body: Result<ApiJson<CreateInboundWebhookRequest>, ApiError>,
) -> Result<(StatusCode, Json<CreatedInboundWebhook>), ApiError> {
    let ApiJson(req) = body?;
    let name = validate_label(&req.name, "name")?;
    if let Some(lib_id) = req.library_id
        && state.db.libraries().get(lib_id).await?.is_none()
    {
        return Err(ApiError::not_found("Bound media library not found"));
    }

    let token = generate_webhook_token()?;
    let webhook = state
        .db
        .notifications()
        .create_inbound_webhook(&name, req.source_kind, &token, req.library_id, req.enabled)
        .await?;
    let trigger_path = format!("/api/v1/webhooks/inbound/{token}/trigger");

    audit(
        &state,
        AuditEntry {
            action: "webhook.inbound_created",
            actor: Some(admin.user.id),
            target: Some(webhook.id.clone()),
            detail: Some(json!({
                "name": webhook.name,
                "sourceKind": webhook.source_kind.as_str(),
                "libraryId": webhook.library_id,
            })),
        },
    )
    .await;

    Ok((
        StatusCode::CREATED,
        Json(CreatedInboundWebhook {
            webhook,
            token,
            trigger_path,
        }),
    ))
}

/// `DELETE /api/v1/webhooks/inbound/{id}`
pub async fn delete_inbound_webhook(
    AdminUser(admin): AdminUser,
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
) -> Result<StatusCode, ApiError> {
    let id = path?.0;
    let _parsed: JobId = path_id(&id, "Inbound webhook")?;
    if !state.db.notifications().delete_inbound_webhook(&id).await? {
        return Err(ApiError::not_found("Inbound webhook not found"));
    }
    audit(
        &state,
        AuditEntry {
            action: "webhook.inbound_deleted",
            actor: Some(admin.user.id),
            target: Some(id),
            detail: None,
        },
    )
    .await;
    Ok(StatusCode::NO_CONTENT)
}

fn summarize_automation_payload(
    source_kind: InboundWebhookSourceKind,
    payload: &Value,
) -> (bool, String, Option<String>) {
    let event_type = payload
        .get("eventType")
        .or_else(|| payload.get("type"))
        .or_else(|| payload.get("event"))
        .and_then(Value::as_str)
        .unwrap_or("Download");
    let is_test = event_type.eq_ignore_ascii_case("test");

    let hint_path = payload
        .pointer("/movie/folderPath")
        .or_else(|| payload.pointer("/series/path"))
        .or_else(|| payload.get("save_path"))
        .or_else(|| payload.get("content_path"))
        .or_else(|| payload.get("path"))
        .and_then(Value::as_str)
        .map(str::to_owned);

    let media_title = payload
        .pointer("/series/title")
        .or_else(|| payload.pointer("/movie/title"))
        .or_else(|| payload.get("title"))
        .or_else(|| payload.get("torrent_name"))
        .or_else(|| payload.get("name"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty());

    let episode_tag = payload
        .pointer("/episodes/0")
        .and_then(|ep| {
            let s = ep.get("seasonNumber").and_then(Value::as_i64)?;
            let e = ep.get("episodeNumber").and_then(Value::as_i64)?;
            Some(format!(" S{s:02}E{e:02}"))
        })
        .unwrap_or_default();

    let source_label = match source_kind {
        InboundWebhookSourceKind::Sonarr => "Sonarr",
        InboundWebhookSourceKind::Radarr => "Radarr",
        InboundWebhookSourceKind::Moviepilot => "MoviePilot",
        InboundWebhookSourceKind::Qbittorrent => "qBittorrent",
        InboundWebhookSourceKind::Custom => "Webhook",
    };

    let summary = if is_test {
        format!("{source_label} 连通性测试通过")
    } else if let Some(title) = media_title {
        format!("{source_label} 通知入库：《{title}》{episode_tag}（{event_type}）")
    } else {
        format!("{source_label} 触发媒体入库刷新（{event_type}）")
    };

    (is_test, summary, hint_path)
}

/// `POST /api/v1/webhooks/inbound/{token}/trigger` (`Permission::Public`)
pub async fn trigger_inbound_webhook(
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
    body: Bytes,
) -> Result<Json<InboundWebhookTriggerResponse>, ApiError> {
    let token = path?.0;
    if token.trim().is_empty() || token.len() > 256 {
        return Err(ApiError::not_found("Inbound webhook not found"));
    }
    let webhook = state
        .db
        .notifications()
        .find_inbound_webhook_by_token(&token)
        .await?
        .filter(|w| w.enabled)
        .ok_or_else(|| ApiError::not_found("Inbound webhook not found"))?;

    let payload: Value = if body.is_empty() {
        json!({})
    } else {
        serde_json::from_slice(&body)
            .map_err(|_| ApiError::validation("Expected a valid JSON webhook payload"))?
    };

    let now_ms = clock::to_millis(clock::now());
    state
        .db
        .notifications()
        .touch_inbound_webhook(&webhook.id, now_ms)
        .await?;

    let (is_test, summary, hint_path) = summarize_automation_payload(webhook.source_kind, &payload);
    let mut queued_library_ids: Vec<LibraryId> = Vec::new();

    if !is_test {
        if let Some(bound_id) = webhook.library_id {
            if let Some(lib) = state.db.libraries().get(bound_id).await? {
                let _ = state.jobs.enqueue_library_scan(lib.id, lib.name).await?;
                queued_library_ids.push(lib.id);
            }
        } else {
            let all_libs = state.db.libraries().list().await?;
            let path_matched: Vec<_> = hint_path
                .as_deref()
                .map(|hp| {
                    all_libs
                        .iter()
                        .filter(|lib| {
                            lib.paths
                                .iter()
                                .any(|lp| hp == lp || hp.starts_with(&format!("{lp}/")))
                        })
                        .cloned()
                        .collect()
                })
                .unwrap_or_default();

            let targets = if !path_matched.is_empty() {
                path_matched
            } else {
                all_libs
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
                    .collect()
            };

            for lib in targets {
                let _ = state.jobs.enqueue_library_scan(lib.id, lib.name).await?;
                queued_library_ids.push(lib.id);
            }
        }
    }

    let body_text = if is_test {
        format!("入站令牌「{}」已收到测试回调。", webhook.name)
    } else {
        format!(
            "{}，已将 {} 个媒体库加入增量扫描队列。",
            summary,
            queued_library_ids.len()
        )
    };
    dispatch_notification_event(&state, None, "webhook.triggered", &summary, &body_text).await;

    Ok(Json(InboundWebhookTriggerResponse {
        accepted: true,
        source_kind: webhook.source_kind,
        summary,
        queued_library_ids,
    }))
}

// ---------------------------------------------------------------------------
// Scheduled Routines (`module: routines`)
// ---------------------------------------------------------------------------

fn validate_routine_action(
    is_admin: bool,
    kind: RoutineActionKind,
    raw_payload: &str,
) -> Result<String, ApiError> {
    let trimmed = raw_payload.trim();
    match kind {
        RoutineActionKind::ScanAll => {
            if !is_admin {
                return Err(ApiError::forbidden());
            }
            Ok(String::new())
        }
        RoutineActionKind::ScanLibrary => {
            if !is_admin {
                return Err(ApiError::forbidden());
            }
            let lib_str = serde_json::from_str::<Value>(trimmed)
                .ok()
                .and_then(|v| {
                    v.get("libraryId")
                        .and_then(Value::as_str)
                        .map(str::to_owned)
                })
                .unwrap_or_else(|| trimmed.to_owned());
            let parsed: LibraryId = lib_str.parse().map_err(|_| {
                ApiError::validation("actionPayload must be a valid library UUID")
                    .field("actionPayload")
            })?;
            Ok(parsed.to_string())
        }
        RoutineActionKind::HomeScene => {
            let scene_id = serde_json::from_str::<Value>(trimmed)
                .ok()
                .and_then(|v| v.get("sceneId").and_then(Value::as_str).map(str::to_owned))
                .unwrap_or_else(|| trimmed.to_owned());
            if scene_id.is_empty() || scene_id.len() > 128 || scene_id.chars().any(char::is_control)
            {
                return Err(
                    ApiError::validation("actionPayload must specify a target scene ID")
                        .field("actionPayload"),
                );
            }
            Ok(scene_id)
        }
        RoutineActionKind::AssistantPrompt => {
            let prompt = serde_json::from_str::<Value>(trimmed)
                .ok()
                .and_then(|v| v.get("prompt").and_then(Value::as_str).map(str::to_owned))
                .unwrap_or_else(|| trimmed.to_owned());
            let clean = prompt.trim();
            if clean.is_empty() || clean.chars().count() > 2000 {
                return Err(ApiError::validation(
                    "actionPayload must contain an assistant prompt (1-2000 chars)",
                )
                .field("actionPayload"));
            }
            Ok(clean.to_owned())
        }
    }
}

pub(crate) async fn execute_routine_internal(
    state: &AppState,
    routine: &ScheduledRoutine,
) -> (ScheduledRoutine, String, bool) {
    let now_ms = clock::to_millis(clock::now());
    let outcome: Result<String, String> = match routine.action_kind {
        RoutineActionKind::ScanAll => match state.db.libraries().list().await {
            Ok(libs) => {
                let mut count = 0_usize;
                for lib in libs {
                    if state
                        .jobs
                        .enqueue_library_scan(lib.id, lib.name)
                        .await
                        .is_ok()
                    {
                        count += 1;
                    }
                }
                Ok(format!("已触发 {count} 个媒体库扫描"))
            }
            Err(err) => Err(format!("读取媒体库失败: {err}")),
        },
        RoutineActionKind::ScanLibrary => {
            match routine.action_payload.trim().parse::<LibraryId>() {
                Ok(lib_id) => match state.db.libraries().get(lib_id).await {
                    Ok(Some(lib)) => match state
                        .jobs
                        .enqueue_library_scan(lib.id, lib.name.clone())
                        .await
                    {
                        Ok(_) => Ok(format!("已触发媒体库「{}」扫描", lib.name)),
                        Err(err) => Err(format!("入队扫描失败: {err}")),
                    },
                    Ok(None) => Err("目标媒体库不存在".into()),
                    Err(err) => Err(format!("查询媒体库失败: {err}")),
                },
                Err(_) => Err("无效的媒体库 ID".into()),
            }
        }
        RoutineActionKind::HomeScene => {
            let scene_id = routine.action_payload.trim();
            match activate_scene_internal(state, scene_id).await {
                Ok(Some(_)) => Ok(format!("已激活家居场景「{scene_id}」")),
                Ok(None) => Err(format!("家居场景「{scene_id}」不存在")),
                Err(err) => Err(format!("激活场景失败: {err}")),
            }
        }
        RoutineActionKind::AssistantPrompt => match state.db.users().get(routine.owner_id).await {
            Ok(Some(user)) => {
                let caller = CurrentUser {
                    user,
                    session_id: SessionId::generate(),
                    device_id: None,
                };
                match crate::routes::assistant::send_message(
                    caller,
                    State(state.clone()),
                    Ok(ApiJson(SendMessageRequest {
                        text: routine.action_payload.clone(),
                        context: None,
                    })),
                )
                .await
                {
                    Ok(Json(resp)) => Ok(resp.reply.text),
                    Err(err) => Err(format!("助手执行失败: {err}")),
                }
            }
            Ok(None) => Err("例行任务所属账号不存在".into()),
            Err(err) => Err(format!("查询用户失败: {err}")),
        },
    };

    let (succeeded, status_code, detail) = match outcome {
        Ok(msg) => (true, "succeeded".to_owned(), msg),
        Err(err) => (false, format!("failed: {err}"), err),
    };

    let updated = state
        .db
        .notifications()
        .mark_routine_ran(&routine.id, now_ms, &status_code)
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| ScheduledRoutine {
            last_run_at: clock::from_millis(now_ms),
            last_status: Some(status_code),
            ..routine.clone()
        });

    let event_type = if succeeded {
        "routine.completed"
    } else {
        "routine.failed"
    };
    let title = if succeeded {
        format!("例行任务完成：{}", routine.name)
    } else {
        format!("例行任务失败：{}", routine.name)
    };
    dispatch_notification_event(state, Some(routine.owner_id), event_type, &title, &detail).await;

    (updated, detail, succeeded)
}

/// `GET /api/v1/routines`
pub async fn list_routines(
    current: CurrentUser,
    State(state): State<AppState>,
) -> Result<Json<ScheduledRoutineList>, ApiError> {
    let routines = state
        .db
        .notifications()
        .list_routines(current.user.id, current.is_admin())
        .await?;
    Ok(Json(ScheduledRoutineList { routines }))
}

/// `POST /api/v1/routines`
pub async fn create_routine(
    current: CurrentUser,
    State(state): State<AppState>,
    body: Result<ApiJson<CreateRoutineRequest>, ApiError>,
) -> Result<(StatusCode, Json<ScheduledRoutine>), ApiError> {
    let ApiJson(req) = body?;
    let name = validate_label(&req.name, "name")?;
    let schedule_expr =
        normalize_schedule_expr(req.schedule_kind, &req.schedule_expr).ok_or_else(|| {
            ApiError::validation("Invalid schedule expression for the chosen scheduleKind")
                .field("scheduleExpr")
        })?;
    let action_payload =
        validate_routine_action(current.is_admin(), req.action_kind, &req.action_payload)?;

    let routine = state
        .db
        .notifications()
        .create_routine(NewRoutine {
            owner_id: current.user.id,
            name: &name,
            schedule_kind: req.schedule_kind,
            schedule_expr: &schedule_expr,
            action_kind: req.action_kind,
            action_payload: &action_payload,
            enabled: req.enabled,
        })
        .await?;

    Ok((StatusCode::CREATED, Json(routine)))
}

/// `PATCH /api/v1/routines/{id}`
pub async fn update_routine(
    current: CurrentUser,
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
    body: Result<ApiJson<UpdateRoutineRequest>, ApiError>,
) -> Result<Json<ScheduledRoutine>, ApiError> {
    let id = path?.0;
    let _parsed: JobId = path_id(&id, "Routine")?;
    let ApiJson(req) = body?;

    let existing = state
        .db
        .notifications()
        .get_routine(current.user.id, current.is_admin(), &id)
        .await?
        .ok_or_else(|| ApiError::not_found("Scheduled routine not found"))?;

    let name = req
        .name
        .as_deref()
        .map(|n| validate_label(n, "name"))
        .transpose()?;
    let eff_sched_kind = req.schedule_kind.unwrap_or(existing.schedule_kind);
    let schedule_expr = if req.schedule_kind.is_some() || req.schedule_expr.is_some() {
        let raw_expr = req
            .schedule_expr
            .as_deref()
            .unwrap_or(&existing.schedule_expr);
        Some(
            normalize_schedule_expr(eff_sched_kind, raw_expr).ok_or_else(|| {
                ApiError::validation("Invalid schedule expression").field("scheduleExpr")
            })?,
        )
    } else {
        None
    };

    let eff_action_kind = req.action_kind.unwrap_or(existing.action_kind);
    let action_payload = if req.action_kind.is_some() || req.action_payload.is_some() {
        let raw_payload = req
            .action_payload
            .as_deref()
            .unwrap_or(&existing.action_payload);
        Some(validate_routine_action(
            current.is_admin(),
            eff_action_kind,
            raw_payload,
        )?)
    } else {
        None
    };

    let updated = state
        .db
        .notifications()
        .update_routine(
            current.user.id,
            current.is_admin(),
            &id,
            RoutinePatch {
                name,
                schedule_kind: req.schedule_kind,
                schedule_expr,
                action_kind: req.action_kind,
                action_payload,
                enabled: req.enabled,
            },
        )
        .await?
        .ok_or_else(|| ApiError::not_found("Scheduled routine not found"))?;

    Ok(Json(updated))
}

/// `DELETE /api/v1/routines/{id}`
pub async fn delete_routine(
    current: CurrentUser,
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
) -> Result<StatusCode, ApiError> {
    let id = path?.0;
    let _parsed: JobId = path_id(&id, "Routine")?;
    if !state
        .db
        .notifications()
        .delete_routine(current.user.id, current.is_admin(), &id)
        .await?
    {
        return Err(ApiError::not_found("Scheduled routine not found"));
    }
    Ok(StatusCode::NO_CONTENT)
}

/// `POST /api/v1/routines/{id}/run`
pub async fn run_routine(
    current: CurrentUser,
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
) -> Result<Json<RunRoutineResponse>, ApiError> {
    let id = path?.0;
    let _parsed: JobId = path_id(&id, "Routine")?;
    let routine = state
        .db
        .notifications()
        .get_routine(current.user.id, current.is_admin(), &id)
        .await?
        .ok_or_else(|| ApiError::not_found("Scheduled routine not found"))?;

    let (updated, detail, _) = execute_routine_internal(&state, &routine).await;
    Ok(Json(RunRoutineResponse {
        routine: updated,
        detail,
    }))
}

// ---------------------------------------------------------------------------
// Background Event Dispatcher & 60s Scheduled Routine Ticker
// ---------------------------------------------------------------------------

async fn run_due_periodic_routines(state: &AppState) {
    let Ok(stored) = state.db.notifications().list_enabled_routines().await else {
        return;
    };
    let now = clock::now();
    for entry in stored {
        if entry.routine.schedule_kind == RoutineScheduleKind::OnEvent {
            continue;
        }
        if is_periodic_routine_due(
            entry.routine.schedule_kind,
            &entry.routine.schedule_expr,
            entry.last_run_at_ms,
            now,
        ) {
            let _ = execute_routine_internal(state, &entry.routine).await;
        }
    }
}

async fn run_matching_event_routines(
    state: &AppState,
    event_type: &str,
    owner_scope: Option<UserId>,
) {
    let Ok(stored) = state.db.notifications().list_enabled_routines().await else {
        return;
    };
    let now_ms = clock::to_millis(clock::now());
    for entry in stored {
        if entry.routine.schedule_kind != RoutineScheduleKind::OnEvent {
            continue;
        }
        if let Some(owner) = owner_scope
            && entry.routine.owner_id != owner
        {
            continue;
        }
        if !event_matches(&entry.routine.schedule_expr, event_type) {
            continue;
        }
        if entry
            .last_run_at_ms
            .is_some_and(|last| now_ms.saturating_sub(last) < ON_EVENT_COOLDOWN_MS)
        {
            continue;
        }
        let _ = execute_routine_internal(state, &entry.routine).await;
    }
}

fn remember_key(seen: &mut VecDeque<String>, key: String) -> bool {
    if seen.iter().any(|existing| existing == &key) {
        return false;
    }
    if seen.len() >= SEEN_EVENT_KEYS_MAX {
        seen.pop_front();
    }
    seen.push_back(key);
    true
}

async fn handle_bus_event(state: &AppState, event: &ServerEvent, seen: &mut VecDeque<String>) {
    match event {
        ServerEvent::JobUpdated(job) => match job.state {
            JobState::Succeeded => {
                let dedupe = format!("job:{}:succeeded", job.id);
                if !remember_key(seen, dedupe) {
                    return;
                }
                let target_name = job
                    .subject
                    .as_ref()
                    .and_then(|s| s.name.as_deref())
                    .unwrap_or(&job.kind);
                let (event_type, title) = if job.kind == "library.scan" {
                    ("library.scanned", format!("媒体库扫描完成：{target_name}"))
                } else {
                    ("job.succeeded", format!("后台任务完成：{target_name}"))
                };
                let body = job
                    .message
                    .clone()
                    .unwrap_or_else(|| format!("任务 {} 已完成。", job.kind));
                dispatch_notification_event(state, None, event_type, &title, &body).await;
                run_matching_event_routines(state, event_type, None).await;
            }
            JobState::Failed => {
                let dedupe = format!("job:{}:failed", job.id);
                if !remember_key(seen, dedupe) {
                    return;
                }
                let target_name = job
                    .subject
                    .as_ref()
                    .and_then(|s| s.name.as_deref())
                    .unwrap_or(&job.kind);
                let title = format!("后台任务失败：{target_name}");
                let body = job
                    .error
                    .clone()
                    .unwrap_or_else(|| "任务执行异常中止。".into());
                dispatch_notification_event(state, None, "job.failed", &title, &body).await;
                run_matching_event_routines(state, "job.failed", None).await;
            }
            _ => {}
        },
        ServerEvent::LibraryDeleted { id } => {
            let dedupe = format!("library.deleted:{id}");
            if remember_key(seen, dedupe) {
                dispatch_notification_event(
                    state,
                    None,
                    "library.deleted",
                    "媒体库已移除",
                    &format!("媒体库 {id} 已从索引移除。"),
                )
                .await;
                run_matching_event_routines(state, "library.deleted", None).await;
            }
        }
        ServerEvent::AssistantRunUpdated { user_id, run } => {
            if run.state == RunState::AwaitingApproval {
                let dedupe = format!("assistant:{}:approval", run.id);
                if remember_key(seen, dedupe) {
                    let title = run
                        .title
                        .clone()
                        .unwrap_or_else(|| "AI 助手操作等待你的确认".into());
                    let body = run
                        .request
                        .clone()
                        .unwrap_or_else(|| "请前往任务中心或助手页面核对变更预览。".into());
                    dispatch_notification_event(
                        state,
                        Some(*user_id),
                        "assistant.approval_needed",
                        &title,
                        &body,
                    )
                    .await;
                    run_matching_event_routines(state, "assistant.approval_needed", Some(*user_id))
                        .await;
                }
            }
        }
        ServerEvent::HomeDeviceUpdated(_) => {
            run_matching_event_routines(state, "home.device.updated", None).await;
        }
        ServerEvent::LibraryUpdated(_) => {
            run_matching_event_routines(state, "library.updated", None).await;
        }
        ServerEvent::SettingsUpdated { .. }
        | ServerEvent::TaskUpdated { .. }
        | ServerEvent::PlaybackSessionUpdated { .. }
        | ServerEvent::PlaybackCommand { .. } => {}
    }
}

/// Spawns the notification event subscriber and 60-second scheduled routines ticker.
pub fn spawn_background_services(state: AppState) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut bus_sub = state.bus.subscribe();
        let mut cursor = Some(bus_sub.after(None).latest);
        let mut seen = VecDeque::with_capacity(SEEN_EVENT_KEYS_MAX);
        let mut ticker = tokio::time::interval(ROUTINE_TICK_INTERVAL);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        // Consume the immediate initial tick so routines only evaluate on schedule boundaries.
        ticker.tick().await;

        loop {
            tokio::select! {
                () = state.shutdown.cancelled() => break,
                changed = bus_sub.changed() => {
                    if !changed {
                        break;
                    }
                    let replay = bus_sub.after(cursor);
                    cursor = Some(replay.latest);
                    for entry in replay.entries {
                        handle_bus_event(&state, &entry.event, &mut seen).await;
                    }
                }
                _ = ticker.tick() => {
                    run_due_periodic_routines(&state).await;
                    let _ = crate::routes::downloads::poll_due_subscriptions(&state).await;
                }
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use axum::Router;
    use axum::body::to_bytes;
    use axum::http::HeaderMap;
    use axum::routing::post;
    use palveni_core::ids::{ItemId, ServerId};
    use palveni_core::virtual_tv::CreateVirtualChannelRequest;
    use palveni_core::{FeatureRegistry, ItemKind, Job, JobSubject, PerfTier, Role};
    use palveni_db::items::ItemUpsert;
    use palveni_db::libraries::NewLibrary;
    use palveni_db::video::VideoMetadata;
    use palveni_db::{Db, DbOptions};
    use palveni_jobs::JobService;
    use palveni_media::{HardwareService, ProbeOptions};
    use palveni_plugin_host::{PluginCatalog, skin_store::SkinStore};
    use tokio::sync::Mutex;
    use tokio_util::sync::CancellationToken;

    use super::*;
    use crate::routes::virtual_tv::{
        FeedAccessQuery, GuideQuery, create_channel as create_vtv_channel, epg_xml, guide,
        playlist_m3u,
    };

    #[test]
    fn external_push_builders_format_all_seven_provider_payloads() {
        let client = reqwest::Client::new();
        for (kind, cfg, expected_url, expected_fragment) in [
            (
                NotificationChannelKind::Webhook,
                NotificationChannelConfig {
                    url: Some("https://hooks.example/palveni".into()),
                    token: Some("sec-1".into()),
                    target: None,
                },
                "https://hooks.example/palveni",
                "\"event\":\"library.scanned\"",
            ),
            (
                NotificationChannelKind::Telegram,
                NotificationChannelConfig {
                    url: None,
                    token: Some("12345:ABC".into()),
                    target: Some("-10099".into()),
                },
                "https://api.telegram.org/bot12345:ABC/sendMessage",
                "\"chat_id\":\"-10099\"",
            ),
            (
                NotificationChannelKind::Bark,
                NotificationChannelConfig {
                    url: Some("https://api.day.app".into()),
                    token: Some("devkey".into()),
                    target: None,
                },
                "https://api.day.app/push",
                "\"device_key\":\"devkey\"",
            ),
            (
                NotificationChannelKind::Ntfy,
                NotificationChannelConfig {
                    url: Some("https://ntfy.sh".into()),
                    token: Some("tk_1".into()),
                    target: Some("home-media".into()),
                },
                "https://ntfy.sh/home-media",
                "扫描完成",
            ),
            (
                NotificationChannelKind::Wecom,
                NotificationChannelConfig {
                    url: Some("https://qyapi.weixin.qq.com/cgi-bin/webhook/send?key=k1".into()),
                    token: None,
                    target: None,
                },
                "https://qyapi.weixin.qq.com/cgi-bin/webhook/send?key=k1",
                "\"msgtype\":\"text\"",
            ),
            (
                NotificationChannelKind::Feishu,
                NotificationChannelConfig {
                    url: Some("https://open.feishu.cn/open-apis/bot/v2/hook/h1".into()),
                    token: None,
                    target: None,
                },
                "https://open.feishu.cn/open-apis/bot/v2/hook/h1",
                "\"msg_type\":\"text\"",
            ),
            (
                NotificationChannelKind::Discord,
                NotificationChannelConfig {
                    url: Some("https://discord.com/api/webhooks/1/2".into()),
                    token: None,
                    target: None,
                },
                "https://discord.com/api/webhooks/1/2",
                "**标题**",
            ),
        ] {
            let req = build_external_push_request(
                &client,
                kind,
                &cfg,
                "library.scanned",
                "标题",
                "扫描完成",
            )
            .expect("valid request");
            assert_eq!(req.url().as_str(), expected_url);
            let body_str = std::str::from_utf8(req.body().unwrap().as_bytes().unwrap()).unwrap();
            assert!(
                body_str.contains(expected_fragment),
                "kind={kind:?}, body={body_str}"
            );
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn end_to_end_notifications_webhooks_routines_background_and_virtual_tv() {
        let dir = std::env::temp_dir().join(format!("palveni-notif-test-{}", JobId::generate()));
        tokio::fs::create_dir_all(&dir).await.unwrap();

        let db = Db::open(&DbOptions {
            main_url: format!("sqlite://{}/main.db?mode=rwc", dir.display()),
            ops_url: format!("sqlite://{}/ops.db?mode=rwc", dir.display()),
            max_connections: Some(4),
            min_connections: Some(1),
        })
        .await
        .unwrap();

        let admin_user = db
            .users()
            .create("admin", "pw-hash", Role::Admin)
            .await
            .unwrap();
        let session = db
            .sessions()
            .create(admin_user.id, "Workbench", None)
            .await
            .unwrap();

        let video_lib = db
            .libraries()
            .create(NewLibrary {
                name: "4K 影院".into(),
                kind: LibraryKind::Video,
                paths: vec!["/media/movies".into()],
            })
            .await
            .unwrap();

        let movie_id = ItemId::generate();
        db.items()
            .apply_scan_batch(
                video_lib.id,
                &[ItemUpsert {
                    id: movie_id,
                    parent_id: None,
                    kind: ItemKind::Video,
                    title: "Interstellar".into(),
                    sort_title: "interstellar".into(),
                    search_text: "interstellar 星际穿越".into(),
                    path: "/media/movies/Interstellar.mkv".into(),
                    size_bytes: Some(10_000_000),
                    modified_at: Some(1_700_000_000_000),
                    music: None,
                }],
                &[],
            )
            .await
            .unwrap();
        db.video()
            .save_batch(
                video_lib.id,
                vec![VideoMetadata {
                    item_id: movie_id,
                    stamp: "v1".into(),
                    work_group: "interstellar".into(),
                    work_title: "星际穿越".into(),
                    year: Some(2014),
                    work_overview: None,
                    work_poster: None,
                    work_original_title: Some("Interstellar".into()),
                    work_tagline: None,
                    work_rating: Some(92),
                    work_official_rating: None,
                    work_premiered: None,
                    work_studios: Vec::new(),
                    work_backdrop: None,
                    season: None,
                    season_poster: None,
                    episode: None,
                    title: "星际穿越 (IMAX)".into(),
                    overview: None,
                    episode_poster: None,
                    runtime_ms: None, // unprobed -> 45m fallback
                    rating: None,
                    premiered: None,
                    thumb: None,
                    genres: vec!["科幻".into()],
                    people: Vec::new(),
                }],
            )
            .await
            .unwrap();

        let bus = db.event_bus(32).await.unwrap();
        let jobs = JobService::new(db.clone(), bus.clone(), 1);
        let shutdown = CancellationToken::new();
        let state = AppState {
            db: db.clone(),
            music_search: Arc::new(palveni_library::music_search::MusicSearch::new(
                dir.join("search"),
            )),
            video_images: Arc::new(palveni_library::video::VideoImages::new(
                dir.join("cache").join("video"),
                None,
                None,
            )),
            identity: Arc::new(palveni_link::NodeIdentity::generate().unwrap()),
            link: Arc::new(crate::LinkPairings::default()),
            login_limit: Arc::new(crate::LoginLimit::new(4)),
            subsonic_limit: Arc::new(crate::LoginLimit::new(4)),
            bus: bus.clone(),
            jobs: jobs.clone(),
            hardware: Arc::new(HardwareService::new(
                db.clone(),
                dir.clone(),
                ProbeOptions::default(),
            )),
            plugins: Arc::new(PluginCatalog::new()),
            settings: Arc::new(tokio::sync::RwLock::new(Default::default())),
            boot_tier: PerfTier::detect(0, 2, false),
            skins: Arc::new(SkinStore::open(&dir.join("skins")).unwrap()),
            skin_writes: Arc::new(tokio::sync::Semaphore::new(1)),
            features: FeatureRegistry::new(),
            server: Arc::new(crate::ServerInfo {
                id: ServerId::generate(),
                name: "Test".into(),
                version: "0.1.0".into(),
            }),
            shutdown: shutdown.clone(),
        };
        let bg = spawn_background_services(state.clone());

        let auth = CurrentUser {
            user: admin_user,
            session_id: session.session_id,
            device_id: Some(session.device_id),
        };

        // SSRF check: cloud metadata address rejected
        assert!(validate_push_url("http://169.254.169.254/latest", "url").is_err());

        // Local mock webhook server for external push verification (when loopback TCP is allowed)
        let received: Arc<Mutex<Vec<Value>>> = Arc::new(Mutex::new(Vec::new()));
        let loopback_allowed = tokio::net::TcpListener::bind("127.0.0.1:0").await.ok();
        let (hook_url, mock_srv) = if let Some(mock_listener) = loopback_allowed {
            let mock_addr = mock_listener.local_addr().unwrap();
            let received_clone = Arc::clone(&received);
            let srv = tokio::spawn(async move {
                let app = Router::new().route(
                    "/hook",
                    post(move |Json(payload): Json<Value>| {
                        let sink = Arc::clone(&received_clone);
                        async move {
                            sink.lock().await.push(payload);
                            StatusCode::OK
                        }
                    }),
                );
                let _ = axum::serve(mock_listener, app).await;
            });
            let can_connect = tokio::net::TcpStream::connect(mock_addr).await.is_ok();
            (format!("http://{mock_addr}/hook"), Some((srv, can_connect)))
        } else {
            ("http://127.0.0.1:9/hook".to_owned(), None)
        };

        // 1. Create Feishu push channel + run test push
        let (_, Json(channel)) = create_channel(
            auth.clone(),
            State(state.clone()),
            Ok(ApiJson(CreateNotificationChannelRequest {
                name: "本地飞书接收端".into(),
                kind: NotificationChannelKind::Feishu,
                config: NotificationChannelConfig {
                    url: Some(hook_url),
                    token: None,
                    target: None,
                },
                events: vec!["*".into()],
                quiet_hours: None,
                enabled: true,
            })),
        )
        .await
        .unwrap();

        let Json(test_res) = test_channel(
            auth.clone(),
            State(state.clone()),
            Ok(Path(channel.id.clone())),
        )
        .await
        .unwrap();
        if mock_srv
            .as_ref()
            .is_some_and(|(_, can_connect)| *can_connect)
        {
            assert_eq!(test_res.delivery.status, NotificationDeliveryStatus::Sent);
            assert_eq!(received.lock().await.len(), 1);
        } else {
            assert_eq!(test_res.delivery.status, NotificationDeliveryStatus::Failed);
            assert!(test_res.delivery.error.is_some());
        }

        // 2. Inbound webhook creation + public Sonarr trigger
        let (_, Json(created_wh)) = create_inbound_webhook(
            AdminUser(auth.clone()),
            State(state.clone()),
            Ok(ApiJson(CreateInboundWebhookRequest {
                name: "Sonarr 追剧".into(),
                source_kind: InboundWebhookSourceKind::Sonarr,
                library_id: Some(video_lib.id),
                enabled: true,
            })),
        )
        .await
        .unwrap();
        assert!(created_wh.token.starts_with("pwh_"));

        let sonarr_body = Bytes::from(
            serde_json::to_vec(&json!({
                "eventType": "Download",
                "series": { "title": "Severance", "path": "/media/movies/Severance" },
                "episodes": [{ "seasonNumber": 2, "episodeNumber": 1 }]
            }))
            .unwrap(),
        );
        let Json(trig_res) = trigger_inbound_webhook(
            State(state.clone()),
            Ok(Path(created_wh.token.clone())),
            sonarr_body,
        )
        .await
        .unwrap();
        assert!(trig_res.accepted);
        assert_eq!(trig_res.queued_library_ids, vec![video_lib.id]);

        // 3. Scheduled routine creation + immediate execution (`home_scene`)
        let _ = crate::routes::home::snapshot(auth.clone(), State(state.clone()))
            .await
            .unwrap();
        let (_, Json(routine)) = create_routine(
            auth.clone(),
            State(state.clone()),
            Ok(ApiJson(CreateRoutineRequest {
                name: "晚间观影场景".into(),
                schedule_kind: RoutineScheduleKind::Daily,
                schedule_expr: "21:00".into(),
                action_kind: RoutineActionKind::HomeScene,
                action_payload: "movie".into(),
                enabled: true,
            })),
        )
        .await
        .unwrap();
        let Json(run_res) = run_routine(
            auth.clone(),
            State(state.clone()),
            Ok(Path(routine.id.clone())),
        )
        .await
        .unwrap();
        assert_eq!(run_res.routine.last_status.as_deref(), Some("succeeded"));

        // 4. Background bus event -> notification delivery + read-all
        let now = clock::now();
        bus.publish(ServerEvent::JobUpdated(Job {
            id: JobId::generate(),
            kind: "library.scan".into(),
            state: JobState::Succeeded,
            subject: Some(JobSubject {
                subject_type: "library".into(),
                id: video_lib.id.to_string(),
                name: Some("4K 影院".into()),
            }),
            progress: Some(1.0),
            step: None,
            message: Some("1 added".into()),
            error: None,
            created_at: now,
            started_at: Some(now),
            finished_at: Some(now),
        }));
        tokio::time::sleep(Duration::from_millis(120)).await;

        let Json(inbox) = list_notifications(
            auth.clone(),
            State(state.clone()),
            Ok(Query(ListNotificationsQuery { limit: None })),
        )
        .await
        .unwrap();
        assert!(inbox.unread_count >= 2);
        assert!(
            inbox
                .notifications
                .iter()
                .any(|n| n.event_type == "library.scanned")
        );

        let Json(read_res) = mark_all_read(auth.clone(), State(state.clone()))
            .await
            .unwrap();
        assert!(read_res.marked_read >= 2);

        // 5. Virtual Live TV channel + guide + M3U + XMLTV export
        let (_, Json(vtv_ch)) = create_vtv_channel(
            AdminUser(auth.clone()),
            State(state.clone()),
            Ok(ApiJson(CreateVirtualChannelRequest {
                number: 108,
                name: "IMAX 科幻台".into(),
                library_id: video_lib.id,
                genre_filter: Some("科幻".into()),
                enabled: true,
            })),
        )
        .await
        .unwrap();
        assert_eq!(vtv_ch.number, 108);

        let Json(vtv_guide) = guide(
            auth.clone(),
            State(state.clone()),
            Ok(Query(GuideQuery {
                at: Some("2026-11-23T12:10:00Z".into()),
            })),
        )
        .await
        .unwrap();
        assert_eq!(vtv_guide.channels.len(), 1);
        assert!(vtv_guide.channels[0].now_playing.is_some());
        let ticket = vtv_guide
            .playlist_url
            .split_once("ticket=")
            .unwrap()
            .1
            .to_owned();

        let m3u_resp = playlist_m3u(
            State(state.clone()),
            HeaderMap::new(),
            Ok(Query(FeedAccessQuery {
                ticket: Some(ticket.clone()),
                at: Some("2026-11-23T12:10:00Z".into()),
            })),
        )
        .await
        .unwrap();
        let m3u_text = String::from_utf8(
            to_bytes(m3u_resp.into_body(), 65_536)
                .await
                .unwrap()
                .to_vec(),
        )
        .unwrap();
        assert!(m3u_text.contains("#EXTM3U"));
        assert!(m3u_text.contains("IMAX 科幻台"));
        assert!(m3u_text.contains(&movie_id.to_string()));

        let xml_resp = epg_xml(
            State(state.clone()),
            HeaderMap::new(),
            Ok(Query(FeedAccessQuery {
                ticket: Some(ticket),
                at: Some("2026-11-23T12:10:00Z".into()),
            })),
        )
        .await
        .unwrap();
        let xml_text = String::from_utf8(
            to_bytes(xml_resp.into_body(), 262_144)
                .await
                .unwrap()
                .to_vec(),
        )
        .unwrap();
        assert!(xml_text.contains("<tv generator-info-name=\"Palveni Server\">"));
        assert!(xml_text.contains("<title>星际穿越</title>"));

        shutdown.cancel();
        let _ = bg.await;
        if let Some((srv, _)) = mock_srv {
            srv.abort();
        }
        let _ = db.close().await;
        let _ = tokio::fs::remove_dir_all(&dir).await;
    }
}

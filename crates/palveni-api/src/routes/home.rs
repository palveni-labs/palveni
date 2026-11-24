//! Smart Home control, Home Assistant bridge sync, and media-playback linkage (`module: home`).

use std::net::IpAddr;
use std::time::Duration;

use axum::Json;
use axum::extract::rejection::PathRejection;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use chrono::Timelike;
use palveni_core::ServerEvent;
use palveni_core::clock;
use palveni_core::home::{
    AutomationRule, ClimateMode, CreateHomeDeviceRequest, CreateHomeRuleRequest, DevicePatch,
    DeviceState, HomeBridgeList, HomeBridgeSummary, HomeBridgeSyncResult, HomeDevice,
    HomeDeviceKind, HomeSnapshot, MediaPlayerPatchStatus, MediaPlayerStatus, PresetDeviceAction,
    SaveHomeBridgeRequest, SceneIcon, SensorKind, SensorReading, SensorValue, SetHomeRuleRequest,
};
use palveni_core::ids::ConnectionId;
use palveni_db::audit::{AuditEntry, actions};
use palveni_db::home::{HomeBridgeRecord, HomeDeviceRecord, SyncedBridgeDevice, SyncedBridgeScene};
use palveni_link::NodeIdentity;
use ring::aead;
use ring::rand::{SecureRandom, SystemRandom};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::auth::{AdminUser, CurrentUser};
use crate::error::ApiError;
use crate::json::ApiJson;
use crate::routes::audit;
use crate::state::AppState;

const HA_SERVICE_TIMEOUT: Duration = Duration::from_secs(8);
const HA_SYNC_TIMEOUT: Duration = Duration::from_secs(12);

fn bridge_key(identity: &NodeIdentity) -> Result<aead::LessSafeKey, ApiError> {
    let seed = identity.sign(b"palveni:home-bridge-token:v1");
    let key = aead::UnboundKey::new(&aead::AES_256_GCM, &seed[..32])
        .map_err(|_| ApiError::internal("Invalid home bridge key"))?;
    Ok(aead::LessSafeKey::new(key))
}

pub(crate) fn encrypt_secret(
    identity: &NodeIdentity,
    scope: &[u8],
    aad_id: &str,
    secret: &str,
) -> Result<String, ApiError> {
    let seed = identity.sign(scope);
    let unbound = aead::UnboundKey::new(&aead::AES_256_GCM, &seed[..32])
        .map_err(|_| ApiError::internal("Invalid encryption key"))?;
    let key = aead::LessSafeKey::new(unbound);
    let mut nonce = [0u8; aead::NONCE_LEN];
    SystemRandom::new()
        .fill(&mut nonce)
        .map_err(|_| ApiError::internal("Secure randomness unavailable"))?;
    let mut data = secret.as_bytes().to_vec();
    key.seal_in_place_append_tag(
        aead::Nonce::assume_unique_for_key(nonce),
        aead::Aad::from(aad_id.as_bytes()),
        &mut data,
    )
    .map_err(|_| ApiError::internal("Cannot encrypt secret"))?;
    Ok(format!("{}.{}", hex::encode(nonce), hex::encode(data)))
}

pub(crate) fn decrypt_secret(
    identity: &NodeIdentity,
    scope: &[u8],
    aad_id: &str,
    encrypted: &str,
) -> Result<String, ApiError> {
    let corrupt = || ApiError::internal("Unreadable encrypted secret");
    let (nonce, text) = encrypted.split_once('.').ok_or_else(corrupt)?;
    let mut data = hex::decode(text).map_err(|_| corrupt())?;
    let mut iv = [0u8; aead::NONCE_LEN];
    hex::decode_to_slice(nonce, &mut iv).map_err(|_| corrupt())?;
    let seed = identity.sign(scope);
    let unbound = aead::UnboundKey::new(&aead::AES_256_GCM, &seed[..32]).map_err(|_| corrupt())?;
    let key = aead::LessSafeKey::new(unbound);
    let plain = key
        .open_in_place(
            aead::Nonce::assume_unique_for_key(iv),
            aead::Aad::from(aad_id.as_bytes()),
            &mut data,
        )
        .map_err(|_| corrupt())?;
    String::from_utf8(plain.to_vec()).map_err(|_| corrupt())
}

fn encrypt_bridge_token(
    identity: &NodeIdentity,
    bridge_id: &str,
    token: &str,
) -> Result<String, ApiError> {
    let _ = bridge_key(identity)?;
    encrypt_secret(identity, b"palveni:home-bridge-token:v1", bridge_id, token)
}

fn decrypt_bridge_token(
    identity: &NodeIdentity,
    bridge_id: &str,
    encrypted: &str,
) -> Result<String, ApiError> {
    decrypt_secret(
        identity,
        b"palveni:home-bridge-token:v1",
        bridge_id,
        encrypted,
    )
}

/// Validates a Home Assistant base URL: `http` or `https` only, no embedded userinfo,
/// LAN and localhost allowed (HA runs on home networks), cloud metadata IPs blocked.
pub(crate) fn validate_bridge_url(raw: &str, field: &'static str) -> Result<String, ApiError> {
    let trimmed = raw.trim().trim_end_matches('/');
    if trimmed.is_empty() || trimmed.len() > 512 || trimmed.chars().any(char::is_control) {
        return Err(ApiError::validation("Invalid bridge URL").field(field));
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
    if parsed.query().is_some() || parsed.fragment().is_some() {
        return Err(
            ApiError::validation("Base URL must not include query or fragment").field(field),
        );
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
    Ok(parsed.as_str().trim_end_matches('/').to_owned())
}

fn ha_client(timeout: Duration) -> Result<reqwest::Client, ApiError> {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(timeout)
        .build()
        .map_err(ApiError::internal)
}

async fn forward_patch_to_ha(
    state: &AppState,
    record: &HomeDeviceRecord,
    patch: &DevicePatch,
) -> Result<(), ApiError> {
    let (Some(bridge_id), Some(entity_id)) = (&record.bridge_id, &record.external_entity_id) else {
        return Ok(());
    };
    let Some(bridge) = state.db.home().bridge(bridge_id).await? else {
        return Ok(());
    };
    if !bridge.enabled {
        return Ok(());
    }
    let base_url = validate_bridge_url(&bridge.base_url, "baseUrl")?;
    let token = decrypt_bridge_token(&state.identity, &bridge.id, &bridge.encrypted_token)?;
    let client = ha_client(HA_SERVICE_TIMEOUT)?;

    let mut calls: Vec<(&str, &str, Value)> = Vec::new();
    match &record.device.state {
        DeviceState::Light { .. } => {
            if let Some(b) = patch.brightness {
                let pct = b.round().clamp(0.0, 100.0);
                if patch.on == Some(false) || pct <= 0.0 {
                    calls.push(("light", "turn_off", json!({ "entity_id": entity_id })));
                } else {
                    calls.push((
                        "light",
                        "turn_on",
                        json!({ "entity_id": entity_id, "brightness_pct": pct }),
                    ));
                }
            } else if let Some(on) = patch.on {
                calls.push((
                    "light",
                    if on { "turn_on" } else { "turn_off" },
                    json!({ "entity_id": entity_id }),
                ));
            }
        }
        DeviceState::Switch { .. } => {
            if let Some(on) = patch.on {
                calls.push((
                    "switch",
                    if on { "turn_on" } else { "turn_off" },
                    json!({ "entity_id": entity_id }),
                ));
            }
        }
        DeviceState::Climate { .. } => {
            if let Some(on) = patch.on {
                calls.push((
                    "climate",
                    if on { "turn_on" } else { "turn_off" },
                    json!({ "entity_id": entity_id }),
                ));
            }
            if let Some(target) = patch.target {
                calls.push((
                    "climate",
                    "set_temperature",
                    json!({ "entity_id": entity_id, "temperature": target.clamp(16.0, 30.0) }),
                ));
            }
        }
        DeviceState::MediaPlayer { .. } => {
            if let Some(vol) = patch.volume {
                calls.push((
                    "media_player",
                    "volume_set",
                    json!({
                        "entity_id": entity_id,
                        "volume_level": (vol.clamp(0.0, 100.0) / 100.0)
                    }),
                ));
            }
            if let Some(st) = patch.status {
                let service = match st {
                    MediaPlayerPatchStatus::Playing => "media_play",
                    MediaPlayerPatchStatus::Paused => "media_pause",
                };
                calls.push(("media_player", service, json!({ "entity_id": entity_id })));
            }
        }
        DeviceState::Sensor { .. } => {}
    }

    for (domain, service, payload) in calls {
        let url = format!("{base_url}/api/services/{domain}/{service}");
        let resp = client
            .post(&url)
            .bearer_auth(&token)
            .json(&payload)
            .send()
            .await
            .map_err(|_| {
                ApiError::new(
                    StatusCode::BAD_GATEWAY,
                    "unavailable",
                    "Home Assistant bridge did not respond",
                )
            })?;
        if !resp.status().is_success() {
            return Err(ApiError::new(
                StatusCode::BAD_GATEWAY,
                "unavailable",
                format!("Home Assistant returned HTTP {}", resp.status().as_u16()),
            ));
        }
    }
    Ok(())
}

async fn forward_scene_to_ha(
    state: &AppState,
    bridge_id: &str,
    entity_id: &str,
) -> Result<(), ApiError> {
    let Some(bridge) = state.db.home().bridge(bridge_id).await? else {
        return Ok(());
    };
    if !bridge.enabled {
        return Ok(());
    }
    let base_url = validate_bridge_url(&bridge.base_url, "baseUrl")?;
    let token = decrypt_bridge_token(&state.identity, &bridge.id, &bridge.encrypted_token)?;
    let client = ha_client(HA_SERVICE_TIMEOUT)?;
    let url = format!("{base_url}/api/services/scene/turn_on");
    let resp = client
        .post(&url)
        .bearer_auth(&token)
        .json(&json!({ "entity_id": entity_id }))
        .send()
        .await
        .map_err(|_| {
            ApiError::new(
                StatusCode::BAD_GATEWAY,
                "unavailable",
                "Home Assistant bridge did not respond",
            )
        })?;
    if !resp.status().is_success() {
        return Err(ApiError::new(
            StatusCode::BAD_GATEWAY,
            "unavailable",
            format!("Home Assistant returned HTTP {}", resp.status().as_u16()),
        ));
    }
    Ok(())
}

pub(crate) async fn apply_device_patch_internal(
    state: &AppState,
    device_id: &str,
    patch: &DevicePatch,
) -> Result<Option<HomeDevice>, ApiError> {
    let Some(record) = state.db.home().device_record(device_id).await? else {
        return Ok(None);
    };
    if !record.device.online {
        return Err(ApiError::conflict("Device is offline"));
    }
    forward_patch_to_ha(state, &record, patch).await?;
    let Some(updated) = state.db.home().apply_device_patch(device_id, patch).await? else {
        return Ok(None);
    };
    state
        .bus
        .publish(ServerEvent::HomeDeviceUpdated(updated.device.clone()));
    Ok(Some(updated.device))
}

pub(crate) async fn activate_scene_internal(
    state: &AppState,
    scene_id: &str,
) -> Result<Option<HomeSnapshot>, ApiError> {
    let Some(scene) = state.db.home().scene_record(scene_id).await? else {
        return Ok(None);
    };
    if let (Some(bridge_id), Some(entity_id)) = (&scene.bridge_id, &scene.external_entity_id) {
        forward_scene_to_ha(state, bridge_id, entity_id).await?;
    }
    for action in &scene.actions {
        if let Ok(Some(_)) =
            apply_device_patch_internal(state, &action.device_id, &action.patch).await
        {
            // Device updated and published on EventBus.
        }
    }
    state.db.home().set_active_scene(scene_id).await?;
    Ok(Some(state.db.home().snapshot().await?))
}

fn event_kinds_for_media_trigger(
    event_kind: &'static str,
    is_late_night: bool,
) -> Vec<&'static str> {
    let mut kinds = vec![event_kind];
    if event_kind == "video.stop" {
        kinds.push("video.pause");
    } else if event_kind == "music.stop" {
        kinds.push("music.pause");
    }
    if is_late_night && matches!(event_kind, "video.play" | "music.play") {
        kinds.push("night.play");
    }
    kinds
}

/// Non-blocking media-playback linkage trigger invoked on video or audio start/pause/stop.
pub fn trigger_media_event(state: &AppState, event_kind: &'static str) {
    let state = state.clone();
    tokio::spawn(async move {
        let now = clock::now();
        // Respect the host/container configured local timezone (`TZ`) instead of a hardcoded offset.
        let local_now = chrono::Local::now();
        let is_late_night =
            (local_now.hour() == 23 && local_now.minute() >= 30) || local_now.hour() < 6;
        let kinds = event_kinds_for_media_trigger(event_kind, is_late_night);
        let Ok(matching_rules) = state.db.home().enabled_rules_for_events(&kinds).await else {
            return;
        };
        let now_ms = clock::to_millis(now);
        for rule in matching_rules {
            for action in &rule.actions {
                if let Err(err) =
                    apply_device_patch_internal(&state, &action.device_id, &action.patch).await
                {
                    tracing::debug!(
                        rule_id = %rule.rule.id,
                        device_id = %action.device_id,
                        error = %err,
                        "skipping home rule action"
                    );
                }
            }
            let _ = state.db.home().mark_rule_ran(&rule.rule.id, now_ms).await;
        }
    });
}

/// `GET /api/v1/home`
pub async fn snapshot(
    _: CurrentUser,
    State(state): State<AppState>,
) -> Result<Json<HomeSnapshot>, ApiError> {
    Ok(Json(state.db.home().snapshot().await?))
}

/// `POST /api/v1/home/devices/{id}`
pub async fn update_device(
    _: CurrentUser,
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
    body: Result<ApiJson<DevicePatch>, ApiError>,
) -> Result<Json<HomeDevice>, ApiError> {
    let id = path?.0;
    let ApiJson(patch) = body?;
    if patch.is_empty() {
        return Err(ApiError::validation(
            "At least one device property must be provided",
        ));
    }
    let updated = apply_device_patch_internal(&state, &id, &patch)
        .await?
        .ok_or_else(|| ApiError::not_found("Device not found"))?;
    Ok(Json(updated))
}

/// `POST /api/v1/home/scenes/{id}/activate`
pub async fn activate_scene(
    _: CurrentUser,
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
) -> Result<Json<HomeSnapshot>, ApiError> {
    let id = path?.0;
    let snap = activate_scene_internal(&state, &id)
        .await?
        .ok_or_else(|| ApiError::not_found("Scene not found"))?;
    Ok(Json(snap))
}

/// `POST /api/v1/home/rules/{id}`
pub async fn set_rule(
    _: CurrentUser,
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
    body: Result<ApiJson<SetHomeRuleRequest>, ApiError>,
) -> Result<Json<AutomationRule>, ApiError> {
    let id = path?.0;
    let ApiJson(req) = body?;
    let rule = state
        .db
        .home()
        .set_rule_enabled(&id, req.enabled)
        .await?
        .ok_or_else(|| ApiError::not_found("Rule not found"))?;
    Ok(Json(rule))
}

/// `GET /api/v1/home/bridges`
pub async fn list_bridges(
    _: AdminUser,
    State(state): State<AppState>,
) -> Result<Json<HomeBridgeList>, ApiError> {
    let rows = state.db.home().bridges().await?;
    let mut bridges = Vec::with_capacity(rows.len());
    for r in rows {
        bridges.push(r.to_summary()?);
    }
    Ok(Json(HomeBridgeList { bridges }))
}

/// `POST /api/v1/home/bridges`
pub async fn save_bridge(
    AdminUser(admin): AdminUser,
    State(state): State<AppState>,
    body: Result<ApiJson<SaveHomeBridgeRequest>, ApiError>,
) -> Result<(StatusCode, Json<HomeBridgeSummary>), ApiError> {
    let ApiJson(req) = body?;
    let name = req.name.trim();
    if name.is_empty() || name.chars().count() > 100 || name.chars().any(char::is_control) {
        return Err(ApiError::validation("Name must be 1 to 100 characters").field("name"));
    }
    let base_url = validate_bridge_url(&req.base_url, "baseUrl")?;

    let existing = if let Some(ref id) = req.id {
        state.db.home().bridge(id).await?
    } else {
        None
    };

    let id = existing
        .as_ref()
        .map(|b| b.id.clone())
        .unwrap_or_else(|| ConnectionId::generate().to_string());

    let token_clean = req
        .token
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    if token_clean.is_some_and(|t| t.len() > 4096 || t.chars().any(char::is_control)) {
        return Err(ApiError::validation("Invalid access token").field("token"));
    }

    let encrypted_token = match (token_clean, &existing) {
        (Some(raw_token), _) => encrypt_bridge_token(&state.identity, &id, raw_token)?,
        (None, Some(prev)) if !prev.encrypted_token.is_empty() => prev.encrypted_token.clone(),
        _ => {
            return Err(
                ApiError::validation("Long-lived Home Assistant access token is required")
                    .field("token"),
            );
        }
    };

    let record = HomeBridgeRecord {
        id: id.clone(),
        name: name.to_owned(),
        base_url: base_url.clone(),
        encrypted_token,
        enabled: req.enabled,
        last_sync_at: existing.as_ref().and_then(|b| b.last_sync_at),
        created_at: existing
            .as_ref()
            .map_or_else(|| clock::to_millis(clock::now()), |b| b.created_at),
    };
    let summary = record.to_summary()?;
    state.db.home().save_bridge(record).await?;

    audit(
        &state,
        AuditEntry {
            action: actions::HOME_BRIDGE_UPDATED,
            actor: Some(admin.user.id),
            target: Some(id),
            detail: Some(json!({ "name": name, "baseUrl": base_url, "enabled": req.enabled })),
        },
    )
    .await;

    Ok((StatusCode::CREATED, Json(summary)))
}

/// `DELETE /api/v1/home/bridges/{id}`
pub async fn delete_bridge(
    AdminUser(admin): AdminUser,
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
) -> Result<StatusCode, ApiError> {
    let id = path?.0;
    if !state.db.home().delete_bridge(&id).await? {
        return Err(ApiError::not_found("Home bridge not found"));
    }
    audit(
        &state,
        AuditEntry {
            action: actions::HOME_BRIDGE_UPDATED,
            actor: Some(admin.user.id),
            target: Some(id),
            detail: Some(json!({ "deleted": true })),
        },
    )
    .await;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
struct HaEntityState {
    entity_id: String,
    state: String,
    #[serde(default)]
    attributes: serde_json::Map<String, Value>,
}

/// `POST /api/v1/home/bridges/{id}/sync`
pub async fn sync_bridge(
    _: AdminUser,
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
) -> Result<Json<HomeBridgeSyncResult>, ApiError> {
    let id = path?.0;
    let bridge = state
        .db
        .home()
        .bridge(&id)
        .await?
        .ok_or_else(|| ApiError::not_found("Home bridge not found"))?;
    let base_url = validate_bridge_url(&bridge.base_url, "baseUrl")?;
    let token = decrypt_bridge_token(&state.identity, &bridge.id, &bridge.encrypted_token)?;
    let client = ha_client(HA_SYNC_TIMEOUT)?;

    let url = format!("{base_url}/api/states");
    let resp = client
        .get(&url)
        .bearer_auth(&token)
        .send()
        .await
        .map_err(|_| {
            ApiError::new(
                StatusCode::BAD_GATEWAY,
                "unavailable",
                "Cannot reach Home Assistant /api/states",
            )
        })?;
    if !resp.status().is_success() {
        return Err(ApiError::new(
            StatusCode::BAD_GATEWAY,
            "unavailable",
            format!(
                "Home Assistant /api/states returned HTTP {}",
                resp.status().as_u16()
            ),
        ));
    }

    let entities: Vec<HaEntityState> = resp.json().await.map_err(|_| {
        ApiError::new(
            StatusCode::BAD_GATEWAY,
            "unavailable",
            "Invalid JSON payload from Home Assistant",
        )
    })?;

    let (synced_devices, synced_scenes) = map_ha_entities(&bridge.id, &bridge.name, &entities);
    state
        .db
        .home()
        .apply_bridge_sync(&bridge.id, &synced_devices, &synced_scenes)
        .await?;

    let refreshed = state
        .db
        .home()
        .bridge(&bridge.id)
        .await?
        .ok_or_else(|| ApiError::not_found("Home bridge not found"))?
        .to_summary()?;
    let snapshot = state.db.home().snapshot().await?;

    Ok(Json(HomeBridgeSyncResult {
        bridge: refreshed,
        synced_devices: synced_devices.len(),
        synced_scenes: synced_scenes.len(),
        snapshot,
    }))
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn u8_pct(value: f64) -> u8 {
    if !value.is_finite() {
        return 0;
    }
    value.round().clamp(0.0, 100.0) as u8
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn u16_kelvin(value: f64) -> Option<u16> {
    if !value.is_finite() || !(1500.0..=9000.0).contains(&value) {
        return None;
    }
    Some(value.round() as u16)
}

fn infer_room_from_name(friendly: &str, default_room: &str) -> (String, String) {
    for room in [
        "客厅", "书房", "主卧", "次卧", "卧室", "厨房", "餐厅", "阳台", "玄关",
    ] {
        if let Some(rest) = friendly.strip_prefix(room) {
            let trimmed = rest.trim();
            let dev_name = if trimmed.is_empty() {
                friendly.to_owned()
            } else {
                trimmed.to_owned()
            };
            return (room.to_owned(), dev_name);
        }
    }
    (default_room.to_owned(), friendly.to_owned())
}

fn map_ha_entities(
    bridge_id: &str,
    bridge_name: &str,
    entities: &[HaEntityState],
) -> (Vec<SyncedBridgeDevice>, Vec<SyncedBridgeScene>) {
    use ring::digest::{SHA256, digest};

    let mut devices = Vec::new();
    let mut scenes = Vec::new();
    let fallback_room = if bridge_name.trim().is_empty() {
        "Home Assistant"
    } else {
        bridge_name.trim()
    };

    for ent in entities {
        let Some((domain, slug)) = ent.entity_id.split_once('.') else {
            continue;
        };
        let friendly = ent
            .attributes
            .get("friendly_name")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or(slug);
        let area_attr = ent
            .attributes
            .get("area_name")
            .or_else(|| ent.attributes.get("room"))
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty());
        let (inferred_room, clean_name) = infer_room_from_name(friendly, fallback_room);
        let room_name = area_attr.map_or(inferred_room, str::to_owned);
        let display_name = if area_attr.is_some() {
            friendly.to_owned()
        } else {
            clean_name
        };

        let id_digest = hex::encode(digest(
            &SHA256,
            format!("{bridge_id}:{}", ent.entity_id).as_bytes(),
        ));
        let stable_id = format!("ha-{}", &id_digest[..16]);
        let online = !matches!(ent.state.as_str(), "unavailable" | "unknown");

        match domain {
            "scene" => {
                let lower = format!("{} {}", ent.entity_id, friendly).to_lowercase();
                let icon =
                    if lower.contains("movie") || lower.contains("观影") || lower.contains("影院")
                    {
                        SceneIcon::Movie
                    } else if lower.contains("night")
                        || lower.contains("sleep")
                        || lower.contains("晚安")
                    {
                        SceneIcon::Night
                    } else if lower.contains("morning")
                        || lower.contains("wake")
                        || lower.contains("早安")
                    {
                        SceneIcon::Morning
                    } else if lower.contains("away")
                        || lower.contains("leave")
                        || lower.contains("离家")
                    {
                        SceneIcon::Away
                    } else {
                        SceneIcon::Home
                    };
                scenes.push(SyncedBridgeScene {
                    id: stable_id,
                    external_entity_id: ent.entity_id.clone(),
                    name: friendly.to_owned(),
                    icon,
                });
            }
            "light" => {
                let on = ent.state == "on";
                let brightness = ent
                    .attributes
                    .get("brightness")
                    .and_then(Value::as_f64)
                    .map_or(if on { 100 } else { 0 }, |b| u8_pct((b / 255.0) * 100.0));
                let color_temp = ent
                    .attributes
                    .get("color_temp_kelvin")
                    .and_then(Value::as_f64)
                    .and_then(u16_kelvin);
                devices.push(SyncedBridgeDevice {
                    id: stable_id,
                    room_name,
                    external_entity_id: ent.entity_id.clone(),
                    name: display_name,
                    online,
                    state: DeviceState::Light {
                        on,
                        brightness,
                        color_temp,
                    },
                });
            }
            "switch" => {
                let on = ent.state == "on";
                let power = ent
                    .attributes
                    .get("current_power_w")
                    .or_else(|| ent.attributes.get("power"))
                    .and_then(Value::as_f64);
                devices.push(SyncedBridgeDevice {
                    id: stable_id,
                    room_name,
                    external_entity_id: ent.entity_id.clone(),
                    name: display_name,
                    online,
                    state: DeviceState::Switch { on, power },
                });
            }
            "climate" => {
                let on = ent.state != "off" && online;
                let mode = match ent.state.as_str() {
                    "heat" => ClimateMode::Heat,
                    "cool" => ClimateMode::Cool,
                    "dry" => ClimateMode::Dry,
                    "fan_only" | "fan" => ClimateMode::Fan,
                    _ => ClimateMode::Auto,
                };
                let current = ent
                    .attributes
                    .get("current_temperature")
                    .and_then(Value::as_f64)
                    .unwrap_or(24.0);
                let target = ent
                    .attributes
                    .get("temperature")
                    .and_then(Value::as_f64)
                    .unwrap_or(current);
                let humidity = ent
                    .attributes
                    .get("current_humidity")
                    .and_then(Value::as_f64);
                devices.push(SyncedBridgeDevice {
                    id: stable_id,
                    room_name,
                    external_entity_id: ent.entity_id.clone(),
                    name: display_name,
                    online,
                    state: DeviceState::Climate {
                        on,
                        mode,
                        current,
                        target,
                        humidity,
                    },
                });
            }
            "sensor" | "binary_sensor" => {
                let dev_class = ent
                    .attributes
                    .get("device_class")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                let unit = ent
                    .attributes
                    .get("unit_of_measurement")
                    .and_then(Value::as_str)
                    .map(str::to_owned);
                let reading = if domain == "binary_sensor" {
                    let active = ent.state == "on";
                    let kind = match dev_class {
                        "motion" | "occupancy" | "presence" => SensorKind::Motion,
                        _ => SensorKind::Contact,
                    };
                    Some(SensorReading {
                        kind,
                        value: SensorValue::Bool(active),
                        unit: None,
                    })
                } else if let Ok(num) = ent.state.parse::<f64>() {
                    let kind = match dev_class {
                        "temperature" => Some(SensorKind::Temperature),
                        "humidity" => Some(SensorKind::Humidity),
                        "pm25" => Some(SensorKind::Pm25),
                        _ if unit.as_deref() == Some("°C") => Some(SensorKind::Temperature),
                        _ if unit.as_deref() == Some("%") && ent.entity_id.contains("humidity") => {
                            Some(SensorKind::Humidity)
                        }
                        _ => None,
                    };
                    kind.map(|k| SensorReading {
                        kind: k,
                        value: SensorValue::Number(num),
                        unit,
                    })
                } else {
                    None
                };
                if let Some(r) = reading {
                    devices.push(SyncedBridgeDevice {
                        id: stable_id,
                        room_name,
                        external_entity_id: ent.entity_id.clone(),
                        name: display_name,
                        online,
                        state: DeviceState::Sensor { readings: vec![r] },
                    });
                }
            }
            "media_player" => {
                let status = match ent.state.as_str() {
                    "playing" => MediaPlayerStatus::Playing,
                    "paused" => MediaPlayerStatus::Paused,
                    "off" | "standby" => MediaPlayerStatus::Off,
                    _ => MediaPlayerStatus::Idle,
                };
                let title = ent
                    .attributes
                    .get("media_title")
                    .and_then(Value::as_str)
                    .map(str::to_owned);
                let subtitle = ent
                    .attributes
                    .get("media_artist")
                    .or_else(|| ent.attributes.get("media_series_title"))
                    .and_then(Value::as_str)
                    .map(str::to_owned);
                let app = ent
                    .attributes
                    .get("app_name")
                    .or_else(|| ent.attributes.get("source"))
                    .and_then(Value::as_str)
                    .map(str::to_owned);
                let volume = ent
                    .attributes
                    .get("volume_level")
                    .and_then(Value::as_f64)
                    .map_or(35, |v| u8_pct(v * 100.0));
                devices.push(SyncedBridgeDevice {
                    id: stable_id,
                    room_name,
                    external_entity_id: ent.entity_id.clone(),
                    name: display_name,
                    online,
                    state: DeviceState::MediaPlayer {
                        status,
                        title,
                        subtitle,
                        app,
                        volume,
                        progress: None,
                        artwork_id: None,
                    },
                });
            }
            _ => {}
        }
    }

    (devices, scenes)
}

/// `POST /api/v1/home/devices`
pub async fn create_device(
    _: AdminUser,
    State(state): State<AppState>,
    body: Result<ApiJson<CreateHomeDeviceRequest>, ApiError>,
) -> Result<(StatusCode, Json<HomeDevice>), ApiError> {
    let ApiJson(req) = body?;
    let room_name = req.room_name.trim();
    let name = req.name.trim();
    if room_name.is_empty()
        || room_name.chars().count() > 100
        || room_name.chars().any(char::is_control)
    {
        return Err(
            ApiError::validation("Room name must be 1 to 100 characters").field("roomName"),
        );
    }
    if name.is_empty() || name.chars().count() > 128 || name.chars().any(char::is_control) {
        return Err(ApiError::validation("Device name must be 1 to 128 characters").field("name"));
    }

    let initial_state = match req.kind {
        HomeDeviceKind::Light => DeviceState::Light {
            on: false,
            brightness: 75,
            color_temp: Some(3500),
        },
        HomeDeviceKind::Switch => DeviceState::Switch {
            on: false,
            power: None,
        },
        HomeDeviceKind::Climate => DeviceState::Climate {
            on: false,
            mode: ClimateMode::Auto,
            current: 24.0,
            target: 24.0,
            humidity: Some(50.0),
        },
        HomeDeviceKind::Sensor => DeviceState::Sensor {
            readings: vec![SensorReading {
                kind: SensorKind::Temperature,
                value: SensorValue::Number(24.0),
                unit: Some("°C".into()),
            }],
        },
        HomeDeviceKind::MediaPlayer => DeviceState::MediaPlayer {
            status: MediaPlayerStatus::Idle,
            title: None,
            subtitle: None,
            app: Some("Palveni".into()),
            volume: 35,
            progress: None,
            artwork_id: None,
        },
    };

    let id = format!("dev-{}", ConnectionId::generate());
    let device = state
        .db
        .home()
        .create_device(id, room_name, name, initial_state)
        .await?;
    state
        .bus
        .publish(ServerEvent::HomeDeviceUpdated(device.clone()));
    Ok((StatusCode::CREATED, Json(device)))
}

/// `DELETE /api/v1/home/devices/{id}`
pub async fn delete_device(
    _: AdminUser,
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
) -> Result<StatusCode, ApiError> {
    let id = path?.0;
    if !state.db.home().delete_device(&id).await? {
        return Err(ApiError::not_found("Device not found"));
    }
    Ok(StatusCode::NO_CONTENT)
}

/// `POST /api/v1/home/rules`
pub async fn create_rule(
    _: AdminUser,
    State(state): State<AppState>,
    body: Result<ApiJson<CreateHomeRuleRequest>, ApiError>,
) -> Result<(StatusCode, Json<AutomationRule>), ApiError> {
    let ApiJson(req) = body?;
    let trigger = req.trigger.trim();
    let action = req.action.trim();
    if trigger.is_empty() || trigger.chars().count() > 255 || trigger.chars().any(char::is_control)
    {
        return Err(ApiError::validation("Trigger must be 1 to 255 characters").field("trigger"));
    }
    if action.is_empty() || action.chars().count() > 255 || action.chars().any(char::is_control) {
        return Err(ApiError::validation("Action must be 1 to 255 characters").field("action"));
    }
    let event_kind = req
        .event_kind
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("video.play");

    let mut actions_list = Vec::new();
    if let Some(ref scene_id) = req.scene_id
        && let Some(scene) = state.db.home().scene_record(scene_id).await?
    {
        actions_list.extend(scene.actions);
    }
    if let (Some(device_id), Some(patch)) = (req.device_id, req.patch)
        && !device_id.trim().is_empty()
        && !patch.is_empty()
    {
        actions_list.push(PresetDeviceAction {
            device_id: device_id.trim().to_owned(),
            patch,
        });
    }

    let id = format!("rule-{}", ConnectionId::generate());
    let rule = state
        .db
        .home()
        .create_rule(id, event_kind, trigger, action, &actions_list)
        .await?;
    Ok((StatusCode::CREATED, Json(rule)))
}

/// `DELETE /api/v1/home/rules/{id}`
pub async fn delete_rule(
    _: AdminUser,
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
) -> Result<StatusCode, ApiError> {
    let id = path?.0;
    if !state.db.home().delete_rule(&id).await? {
        return Err(ApiError::not_found("Rule not found"));
    }
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bridge_url_validation_allows_lan_and_blocks_metadata() {
        assert_eq!(
            validate_bridge_url("http://192.168.1.25:8123/", "baseUrl").unwrap(),
            "http://192.168.1.25:8123"
        );
        assert_eq!(
            validate_bridge_url("https://ha.local:8123", "baseUrl").unwrap(),
            "https://ha.local:8123"
        );
        assert!(validate_bridge_url("ftp://192.168.1.25:8123", "baseUrl").is_err());
        assert!(validate_bridge_url("http://user:pw@192.168.1.25:8123", "baseUrl").is_err());
        assert!(validate_bridge_url("http://169.254.169.254/latest", "baseUrl").is_err());
        assert!(validate_bridge_url("http://0.0.0.0:8123", "baseUrl").is_err());
    }

    #[test]
    fn ha_entity_mapping_extracts_lights_climate_sensors_and_scenes() {
        let raw: Vec<HaEntityState> = serde_json::from_value(json!([
            {
                "entity_id": "light.living_ceiling",
                "state": "on",
                "attributes": { "friendly_name": "客厅吸顶灯", "brightness": 128 }
            },
            {
                "entity_id": "climate.bedroom_ac",
                "state": "cool",
                "attributes": { "friendly_name": "主卧空调", "current_temperature": 26.5, "temperature": 24.0 }
            },
            {
                "entity_id": "sensor.study_temp",
                "state": "23.4",
                "attributes": { "friendly_name": "书房温度", "device_class": "temperature", "unit_of_measurement": "°C" }
            },
            {
                "entity_id": "scene.movie_night",
                "state": "scening",
                "attributes": { "friendly_name": "客厅观影模式" }
            }
        ]))
        .unwrap();

        let (devs, scs) = map_ha_entities("b1", "HA", &raw);
        assert_eq!(devs.len(), 3);
        assert_eq!(devs[0].room_name, "客厅");
        assert_eq!(devs[0].name, "吸顶灯");
        assert_eq!(scs.len(), 1);
        assert_eq!(scs[0].icon, SceneIcon::Movie);
    }

    #[test]
    fn bridge_token_encryption_is_scoped_to_bridge_and_identity() {
        let node = NodeIdentity::generate().unwrap();
        let other = NodeIdentity::generate().unwrap();
        let ct = encrypt_bridge_token(&node, "bridge-1", "ha_long_lived_secret").unwrap();
        assert!(!ct.contains("ha_long_lived_secret"));
        assert_eq!(
            decrypt_bridge_token(&node, "bridge-1", &ct).unwrap(),
            "ha_long_lived_secret"
        );
        assert!(decrypt_bridge_token(&node, "bridge-2", &ct).is_err());
        assert!(decrypt_bridge_token(&other, "bridge-1", &ct).is_err());
    }

    #[test]
    fn media_trigger_expands_stop_to_pause_and_late_night_play() {
        assert_eq!(
            event_kinds_for_media_trigger("video.stop", false),
            vec!["video.stop", "video.pause"]
        );
        assert_eq!(
            event_kinds_for_media_trigger("music.stop", true),
            vec!["music.stop", "music.pause"]
        );
        assert_eq!(
            event_kinds_for_media_trigger("video.pause", true),
            vec!["video.pause"]
        );
        assert_eq!(
            event_kinds_for_media_trigger("video.play", true),
            vec!["video.play", "night.play"]
        );
        assert_eq!(
            event_kinds_for_media_trigger("music.play", false),
            vec!["music.play"]
        );
    }
}

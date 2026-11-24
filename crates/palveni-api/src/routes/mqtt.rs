//! Pure-Rust MQTT 3.1.1 bridge (Zigbee2MQTT / Tasmota / ESPHome) and
//! cover-color ambient light synchronization (`/api/v1/home/mqtt*`, module `mqtt`).

use std::collections::HashSet;
use std::net::{IpAddr, Ipv6Addr};
use std::time::Duration;

use axum::Json;
use axum::extract::rejection::PathRejection;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use palveni_core::sync_cast::{
    MqttBridgeList, MqttBridgeSummary, MqttTestResponse, PlaybackSession, PlaybackSessionState,
    SaveMqttBridgeRequest,
};
use palveni_core::{ServerEvent, clock};
use palveni_db::audit::{AuditEntry, actions};
use palveni_db::sync_cast::SaveMqttBridgeInput;
use serde_json::json;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpStream, lookup_host};
use tokio::time::timeout;
use uuid::Uuid;

use crate::auth::AdminUser;
use crate::error::ApiError;
use crate::json::ApiJson;
use crate::routes::audit;
use crate::routes::home::{decrypt_secret, encrypt_secret};
use crate::state::AppState;

const MQTT_SECRET_SCOPE: &[u8] = b"palveni:mqtt-bridge-password:v1";
const MQTT_IO_TIMEOUT: Duration = Duration::from_secs(3);
const MQTT_KEEP_ALIVE_SECS: u16 = 30;
const MQTT_MAX_REMAINING_LEN: usize = 268_435_455;
const AWS_IMDS_V6: Ipv6Addr = Ipv6Addr::new(0xfd00, 0x0ec2, 0, 0, 0, 0, 0, 0x0254);

// ---------------------------------------------------------------------------
// SSRF & Input Validation
// ---------------------------------------------------------------------------

fn is_blocked_mqtt_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            v4.is_unspecified()
                || v4.is_multicast()
                || v4.is_broadcast()
                // Link-local 169.254.0.0/16 (includes AWS/GCP/Azure metadata 169.254.169.254)
                || v4.octets()[..2] == [169, 254]
        }
        IpAddr::V6(v6) => {
            if let Some(mapped) = v6.to_ipv4_mapped() {
                return is_blocked_mqtt_ip(IpAddr::V4(mapped));
            }
            let seg0 = v6.segments()[0];
            v6.is_unspecified()
                || v6.is_multicast()
                // Link-local fe80::/10
                || (seg0 & 0xffc0) == 0xfe80
                // AWS IPv6 IMDS
                || v6 == AWS_IMDS_V6
        }
    }
}

/// Validates an MQTT broker host: blocks cloud metadata SSRF targets while
/// permitting LAN brokers (`192.168.x.x`, `10.x.x.x`, `127.0.0.1`, `.local`).
pub(crate) fn validate_mqtt_host(raw: &str) -> Result<String, ApiError> {
    let trimmed = raw.trim();
    if trimmed.is_empty()
        || trimmed.len() > 255
        || trimmed
            .chars()
            .any(|c| c.is_control() || c.is_whitespace() || matches!(c, '/' | '?' | '#' | '@'))
    {
        return Err(ApiError::validation("Invalid MQTT broker host").field("host"));
    }

    let unbracketed = trimmed
        .strip_prefix('[')
        .and_then(|s| s.strip_suffix(']'))
        .unwrap_or(trimmed);
    let lower = unbracketed.trim_end_matches('.').to_ascii_lowercase();

    if matches!(
        lower.as_str(),
        "metadata.google.internal"
            | "metadata.goog"
            | "metadata.azure.com"
            | "169.254.169.254"
            | "::ffff:169.254.169.254"
            | "fd00:ec2::254"
    ) || lower.ends_with(".metadata.google.internal")
    {
        return Err(
            ApiError::validation("Cloud metadata hosts are forbidden for MQTT brokers")
                .field("host"),
        );
    }

    if let Ok(ip) = unbracketed.parse::<IpAddr>() {
        if is_blocked_mqtt_ip(ip) {
            return Err(ApiError::validation(
                "Link-local metadata, multicast and unspecified MQTT addresses are forbidden",
            )
            .field("host"));
        }
        return Ok(unbracketed.to_owned());
    }

    if unbracketed.contains(':') {
        return Err(
            ApiError::validation("Host must not include a port; pass port separately")
                .field("host"),
        );
    }

    Ok(unbracketed.to_owned())
}

fn validate_topic_prefix(raw: &str) -> Result<String, ApiError> {
    let trimmed = raw.trim().trim_matches('/');
    if trimmed.is_empty()
        || trimmed.chars().count() > 128
        || trimmed
            .chars()
            .any(|c| c.is_control() || matches!(c, '+' | '#' | '\0'))
    {
        return Err(ApiError::validation(
            "topicPrefix must be 1 to 128 characters without MQTT wildcards (+ or #)",
        )
        .field("topicPrefix"));
    }
    Ok(trimmed.to_owned())
}

// ---------------------------------------------------------------------------
// Cover-Color Ambient Light Mapper (`#RRGGBB` -> Kelvin + cozy brightness)
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AmbientLightColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub brightness: u8,
    pub color_temp_kelvin: u16,
}

/// Validates and normalizes `#RRGGBB` hex colors to uppercase `#RRGGBB`.
pub(crate) fn normalize_ambient_hex(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    let hex_digits = trimmed.strip_prefix('#').unwrap_or(trimmed);
    if hex_digits.len() != 6 || !hex_digits.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    Some(format!("#{}", hex_digits.to_ascii_uppercase()))
}

/// Maps an `#RRGGBB` album/poster accent color to a comfortable ambient light
/// color temperature (`2700K..=6500K`) and cozy brightness (`25..=75`).
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub(crate) fn map_hex_to_ambient_light(raw: &str) -> Option<AmbientLightColor> {
    let normalized = normalize_ambient_hex(raw)?;
    let digits = &normalized[1..];
    let r = u8::from_str_radix(&digits[0..2], 16).ok()?;
    let g = u8::from_str_radix(&digits[2..4], 16).ok()?;
    let b = u8::from_str_radix(&digits[4..6], 16).ok()?;

    // Rec. 709 perceived luminance in [0.0, 1.0].
    let luminance = (0.2126 * f64::from(r) + 0.7152 * f64::from(g) + 0.0722 * f64::from(b)) / 255.0;
    // Red-vs-blue chromatic balance: warm reds -> 2700K, cool blues -> 6500K.
    let warmth = (f64::from(r) - f64::from(b)) / 255.0;
    let coolness = (1.0 - warmth) * 0.5;
    let color_temp_kelvin = (2700.0 + coolness * 3800.0).round().clamp(2700.0, 6500.0) as u16;
    let brightness = (25.0 + luminance * 50.0).round().clamp(25.0, 75.0) as u8;

    Some(AmbientLightColor {
        r,
        g,
        b,
        brightness,
        color_temp_kelvin,
    })
}

// ---------------------------------------------------------------------------
// Pure-Rust MQTT 3.1.1 Wire Codec & Async TCP Client
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub(crate) struct MqttPublishMessage {
    pub topic: String,
    pub payload: Vec<u8>,
    pub retain: bool,
}

pub(crate) fn encode_remaining_length(mut len: usize, out: &mut Vec<u8>) -> Result<(), String> {
    if len > MQTT_MAX_REMAINING_LEN {
        return Err("MQTT packet exceeds 256 MiB protocol limit".to_owned());
    }
    loop {
        #[allow(clippy::cast_possible_truncation)]
        let mut encoded = (len % 128) as u8;
        len /= 128;
        if len > 0 {
            encoded |= 0x80;
        }
        out.push(encoded);
        if len == 0 {
            break;
        }
    }
    Ok(())
}

fn encode_mqtt_string(value: &str, out: &mut Vec<u8>) -> Result<(), String> {
    if value.as_bytes().contains(&0) {
        return Err("MQTT UTF-8 string must not contain NUL bytes".to_owned());
    }
    let len =
        u16::try_from(value.len()).map_err(|_| "MQTT string exceeds 65535 bytes".to_owned())?;
    out.extend_from_slice(&len.to_be_bytes());
    out.extend_from_slice(value.as_bytes());
    Ok(())
}

fn encode_mqtt_bytes(value: &[u8], out: &mut Vec<u8>) -> Result<(), String> {
    let len = u16::try_from(value.len())
        .map_err(|_| "MQTT binary field exceeds 65535 bytes".to_owned())?;
    out.extend_from_slice(&len.to_be_bytes());
    out.extend_from_slice(value);
    Ok(())
}

pub(crate) fn encode_connect_packet(
    client_id: &str,
    username: Option<&str>,
    password: Option<&str>,
    keep_alive_secs: u16,
) -> Result<Vec<u8>, String> {
    let mut body = Vec::with_capacity(64);
    // Protocol Name "MQTT" + Protocol Level 4 (3.1.1)
    encode_mqtt_string("MQTT", &mut body)?;
    body.push(0x04);

    // MQTT 3.1.1 §3.1.2.9: if Password Flag is 1, User Name Flag MUST be 1.
    let has_password = password.is_some();
    let effective_username = match (username, has_password) {
        (Some(u), _) => Some(u),
        (None, true) => Some(""),
        (None, false) => None,
    };

    let mut connect_flags: u8 = 0x02; // Clean Session = 1
    if effective_username.is_some() {
        connect_flags |= 0x80;
    }
    if has_password {
        connect_flags |= 0x40;
    }
    body.push(connect_flags);
    body.extend_from_slice(&keep_alive_secs.to_be_bytes());

    // Payload: Client ID, optional Username, optional Password
    encode_mqtt_string(client_id, &mut body)?;
    if let Some(u) = effective_username {
        encode_mqtt_string(u, &mut body)?;
    }
    if let Some(p) = password {
        encode_mqtt_bytes(p.as_bytes(), &mut body)?;
    }

    let mut packet = Vec::with_capacity(2 + body.len());
    packet.push(0x10); // CONNECT
    encode_remaining_length(body.len(), &mut packet)?;
    packet.extend_from_slice(&body);
    Ok(packet)
}

pub(crate) fn parse_connack(packet: [u8; 4]) -> Result<(), String> {
    if packet[0] != 0x20 || packet[1] != 0x02 {
        return Err(format!(
            "Invalid MQTT CONNACK header: {:02x} {:02x}",
            packet[0], packet[1]
        ));
    }
    match packet[3] {
        0x00 => Ok(()),
        0x01 => Err("MQTT broker rejected protocol version (3.1.1 required)".to_owned()),
        0x02 => Err("MQTT broker rejected client identifier".to_owned()),
        0x03 => Err("MQTT broker unavailable".to_owned()),
        0x04 => Err("MQTT broker rejected username or password".to_owned()),
        0x05 => Err("MQTT client not authorized by broker".to_owned()),
        code => Err(format!("MQTT broker refused connection (code {code})")),
    }
}

pub(crate) fn encode_publish_qos0(
    topic: &str,
    payload: &[u8],
    retain: bool,
) -> Result<Vec<u8>, String> {
    let mut body = Vec::with_capacity(2 + topic.len() + payload.len());
    encode_mqtt_string(topic, &mut body)?;
    body.extend_from_slice(payload);

    let mut packet = Vec::with_capacity(2 + body.len());
    packet.push(if retain { 0x31 } else { 0x30 }); // PUBLISH QoS 0
    encode_remaining_length(body.len(), &mut packet)?;
    packet.extend_from_slice(&body);
    Ok(packet)
}

pub(crate) const fn encode_disconnect_packet() -> [u8; 2] {
    [0xE0, 0x00]
}

/// Connects to an MQTT 3.1.1 broker over TCP with a 3-second deadline, checks
/// `CONNACK`, publishes QoS 0 message(s), and sends `DISCONNECT`.
pub(crate) async fn publish_messages(
    host: &str,
    port: u16,
    username: Option<&str>,
    password: Option<&str>,
    messages: &[MqttPublishMessage],
) -> Result<(), String> {
    if messages.is_empty() {
        return Ok(());
    }
    let clean_host = validate_mqtt_host(host).map_err(|e| e.to_string())?;
    let client_id = format!("palveni{}", &Uuid::now_v7().simple().to_string()[..12]);
    let connect_pkt = encode_connect_packet(&client_id, username, password, MQTT_KEEP_ALIVE_SECS)?;
    let mut publish_pkts = Vec::with_capacity(messages.len());
    for msg in messages {
        publish_pkts.push(encode_publish_qos0(&msg.topic, &msg.payload, msg.retain)?);
    }
    let disconnect_pkt = encode_disconnect_packet();

    timeout(MQTT_IO_TIMEOUT, async move {
        let resolved: Vec<_> = lookup_host((clean_host.as_str(), port))
            .await
            .map_err(|e| format!("Cannot resolve MQTT host `{clean_host}`: {e}"))?
            .collect();
        if resolved.is_empty() {
            return Err(format!("MQTT host `{clean_host}` resolved to no addresses"));
        }
        for addr in &resolved {
            if is_blocked_mqtt_ip(addr.ip()) {
                return Err(format!(
                    "MQTT host `{clean_host}` resolved to blocked address {}",
                    addr.ip()
                ));
            }
        }
        let target = resolved[0];
        let mut stream = TcpStream::connect(target)
            .await
            .map_err(|e| format!("Cannot connect to MQTT broker {target}: {e}"))?;
        stream
            .write_all(&connect_pkt)
            .await
            .map_err(|e| format!("Failed writing MQTT CONNECT: {e}"))?;
        stream
            .flush()
            .await
            .map_err(|e| format!("Failed flushing MQTT CONNECT: {e}"))?;

        let mut connack = [0u8; 4];
        stream
            .read_exact(&mut connack)
            .await
            .map_err(|e| format!("Failed reading MQTT CONNACK: {e}"))?;
        parse_connack(connack)?;

        for pkt in &publish_pkts {
            stream
                .write_all(pkt)
                .await
                .map_err(|e| format!("Failed writing MQTT PUBLISH: {e}"))?;
        }
        stream
            .write_all(&disconnect_pkt)
            .await
            .map_err(|e| format!("Failed writing MQTT DISCONNECT: {e}"))?;
        stream
            .flush()
            .await
            .map_err(|e| format!("Failed flushing MQTT packets: {e}"))?;
        let _ = stream.shutdown().await;
        Ok(())
    })
    .await
    .map_err(|_| "MQTT broker timed out after 3 seconds".to_owned())?
}

// ---------------------------------------------------------------------------
// Background Playback & Ambient Light Fan-Out
// ---------------------------------------------------------------------------

/// Runs in a detached background task after a playback session heartbeat:
/// 1. Syncs cover-derived `ambientHex` color temperature & brightness to bound
///    smart-home lights (when playing).
/// 2. Publishes playback state and ambient light telemetry to enabled MQTT bridges.
pub(crate) async fn dispatch_playback_mqtt_and_ambient(state: AppState, session: PlaybackSession) {
    let bridges = match state.db.sync_cast().list_enabled_mqtt_bridges().await {
        Ok(list) => list,
        Err(err) => {
            tracing::warn!(error = %err, "cannot load enabled MQTT bridges");
            return;
        }
    };
    if bridges.is_empty() {
        return;
    }

    let ambient = if session.state == PlaybackSessionState::Playing {
        session
            .ambient_hex
            .as_deref()
            .and_then(|hex| map_hex_to_ambient_light(hex).map(|color| (hex, color)))
    } else {
        None
    };

    // Apply cover-derived ambient light to any bound smart-home light device (deduplicated).
    if let Some((_, color)) = ambient {
        let mut updated_lights = HashSet::new();
        for bridge in &bridges {
            let Some(ref light_id) = bridge.ambient_light_device_id else {
                continue;
            };
            if !updated_lights.insert(light_id.clone()) {
                continue;
            }
            match state
                .db
                .home()
                .apply_ambient_light(light_id, color.brightness, color.color_temp_kelvin)
                .await
            {
                Ok(Some(record)) => {
                    state
                        .bus
                        .publish(ServerEvent::HomeDeviceUpdated(record.device));
                }
                Ok(None) => {}
                Err(err) => {
                    tracing::warn!(
                        device_id = %light_id,
                        error = %err,
                        "cannot apply ambient light update"
                    );
                }
            }
        }
    }

    let playback_payload = serde_json::to_vec(&json!({
        "sessionId": session.id,
        "deviceName": session.device_name,
        "clientKind": session.client_kind,
        "mediaKind": session.media_kind,
        "itemId": session.item_id,
        "libraryId": session.library_id,
        "title": session.title,
        "subtitle": session.subtitle,
        "state": session.state,
        "positionMs": session.position_ms,
        "durationMs": session.duration_ms,
        "volume": session.volume,
        "ambientHex": session.ambient_hex,
        "timestamp": clock::to_millis(clock::now()),
    }))
    .unwrap_or_default();

    for bridge in bridges {
        let mut messages = Vec::new();
        if bridge.publish_playback {
            messages.push(MqttPublishMessage {
                topic: format!("{}/playback", bridge.topic_prefix),
                payload: playback_payload.clone(),
                retain: false,
            });
        }
        if let (Some((hex, color)), Some(light_id)) =
            (ambient, bridge.ambient_light_device_id.as_ref())
        {
            let ambient_payload = serde_json::to_vec(&json!({
                "deviceId": light_id,
                "hex": hex,
                "brightness": color.brightness,
                "colorTemp": color.color_temp_kelvin,
                "r": color.r,
                "g": color.g,
                "b": color.b,
                "timestamp": clock::to_millis(clock::now()),
            }))
            .unwrap_or_default();
            messages.push(MqttPublishMessage {
                topic: format!("{}/ambient", bridge.topic_prefix),
                payload: ambient_payload,
                retain: false,
            });
        }
        if messages.is_empty() {
            continue;
        }
        let Ok(port) = u16::try_from(bridge.port) else {
            continue;
        };
        let password = match bridge.password_cipher.as_deref().filter(|s| !s.is_empty()) {
            Some(cipher) => {
                match decrypt_secret(&state.identity, MQTT_SECRET_SCOPE, &bridge.id, cipher) {
                    Ok(plain) => Some(plain),
                    Err(err) => {
                        tracing::warn!(bridge_id = %bridge.id, error = %err, "cannot decrypt MQTT password");
                        continue;
                    }
                }
            }
            None => None,
        };

        match publish_messages(
            &bridge.host,
            port,
            bridge.username.as_deref(),
            password.as_deref(),
            &messages,
        )
        .await
        {
            Ok(()) => {
                let _ = state
                    .db
                    .sync_cast()
                    .touch_mqtt_bridge_published(&bridge.id, clock::to_millis(clock::now()))
                    .await;
            }
            Err(err) => {
                tracing::warn!(
                    bridge_id = %bridge.id,
                    host = %bridge.host,
                    error = %err,
                    "MQTT background publish failed"
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Admin HTTP Routes (`/api/v1/home/mqtt*`)
// ---------------------------------------------------------------------------

/// `GET /api/v1/home/mqtt`
pub async fn list_bridges(
    _: AdminUser,
    State(state): State<AppState>,
) -> Result<Json<MqttBridgeList>, ApiError> {
    let bridges = state
        .db
        .sync_cast()
        .list_mqtt_bridges()
        .await?
        .into_iter()
        .map(|record| record.to_summary())
        .collect();
    Ok(Json(MqttBridgeList { bridges }))
}

/// `POST /api/v1/home/mqtt`
pub async fn save_bridge(
    admin: AdminUser,
    State(state): State<AppState>,
    body: Result<ApiJson<SaveMqttBridgeRequest>, ApiError>,
) -> Result<(StatusCode, Json<MqttBridgeSummary>), ApiError> {
    let ApiJson(req) = body?;
    let name = req.name.trim();
    if name.is_empty() || name.chars().count() > 128 || name.chars().any(char::is_control) {
        return Err(
            ApiError::validation("Bridge name must be 1 to 128 printable characters").field("name"),
        );
    }
    let host = validate_mqtt_host(&req.host)?;
    if !(1..=65535).contains(&req.port) {
        return Err(ApiError::validation("Port must be between 1 and 65535").field("port"));
    }
    let topic_prefix = validate_topic_prefix(&req.topic_prefix)?;

    let username = req
        .username
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned);
    if username
        .as_deref()
        .is_some_and(|u| u.chars().count() > 128 || u.chars().any(char::is_control))
    {
        return Err(
            ApiError::validation("Username must be at most 128 printable characters")
                .field("username"),
        );
    }

    let ambient_light_device_id = req
        .ambient_light_device_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned);
    if ambient_light_device_id
        .as_deref()
        .is_some_and(|id| id.chars().count() > 64 || id.chars().any(char::is_control))
    {
        return Err(
            ApiError::validation("Invalid ambientLightDeviceId").field("ambientLightDeviceId")
        );
    }

    let id = req
        .id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map_or_else(|| Uuid::now_v7().hyphenated().to_string(), str::to_owned);
    if id.chars().count() > 36 || id.chars().any(char::is_control) {
        return Err(ApiError::validation("Invalid MQTT bridge ID").field("id"));
    }

    let raw_password = req
        .password
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let password_cipher = match raw_password {
        Some(secret) => Some(encrypt_secret(
            &state.identity,
            MQTT_SECRET_SCOPE,
            &id,
            secret,
        )?),
        None => None,
    };

    let record = state
        .db
        .sync_cast()
        .save_mqtt_bridge(SaveMqttBridgeInput {
            id: id.clone(),
            name: name.to_owned(),
            host: host.clone(),
            port: req.port,
            username,
            password_cipher,
            keep_existing_password: true,
            topic_prefix: topic_prefix.clone(),
            ambient_light_device_id,
            publish_playback: req.publish_playback,
            enabled: req.enabled,
        })
        .await?;

    audit(
        &state,
        AuditEntry {
            action: actions::HOME_MQTT_UPDATED,
            actor: Some(admin.0.user.id),
            target: Some(id),
            detail: Some(json!({
                "name": record.name,
                "host": host,
                "port": req.port,
                "topicPrefix": topic_prefix,
            })),
        },
    )
    .await;

    Ok((StatusCode::CREATED, Json(record.to_summary())))
}

/// `DELETE /api/v1/home/mqtt/{id}`
pub async fn delete_bridge(
    admin: AdminUser,
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
) -> Result<StatusCode, ApiError> {
    let id = path?.0;
    if !state.db.sync_cast().delete_mqtt_bridge(&id).await? {
        return Err(ApiError::not_found("MQTT bridge not found"));
    }
    audit(
        &state,
        AuditEntry {
            action: actions::HOME_MQTT_DELETED,
            actor: Some(admin.0.user.id),
            target: Some(id),
            detail: None,
        },
    )
    .await;
    Ok(StatusCode::NO_CONTENT)
}

/// `POST /api/v1/home/mqtt/{id}/test`
pub async fn test_bridge(
    _: AdminUser,
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
) -> Result<Json<MqttTestResponse>, ApiError> {
    let id = path?.0;
    let bridge = state
        .db
        .sync_cast()
        .get_mqtt_bridge(&id)
        .await?
        .ok_or_else(|| ApiError::not_found("MQTT bridge not found"))?;
    let topic = format!("{}/status", bridge.topic_prefix);
    let Ok(port) = u16::try_from(bridge.port) else {
        return Ok(Json(MqttTestResponse {
            ok: false,
            topic,
            error: Some("Invalid MQTT port".to_owned()),
        }));
    };

    let password = match bridge.password_cipher.as_deref().filter(|s| !s.is_empty()) {
        Some(cipher) => Some(decrypt_secret(
            &state.identity,
            MQTT_SECRET_SCOPE,
            &bridge.id,
            cipher,
        )?),
        None => None,
    };

    let now_ms = clock::to_millis(clock::now());
    let payload = serde_json::to_vec(&json!({
        "event": "test",
        "server": "palveni",
        "timestamp": now_ms,
    }))
    .unwrap_or_default();

    let message = MqttPublishMessage {
        topic: topic.clone(),
        payload,
        retain: false,
    };

    match publish_messages(
        &bridge.host,
        port,
        bridge.username.as_deref(),
        password.as_deref(),
        &[message],
    )
    .await
    {
        Ok(()) => {
            state
                .db
                .sync_cast()
                .touch_mqtt_bridge_published(&bridge.id, now_ms)
                .await?;
            Ok(Json(MqttTestResponse {
                ok: true,
                topic,
                error: None,
            }))
        }
        Err(err) => Ok(Json(MqttTestResponse {
            ok: false,
            topic,
            error: Some(err),
        })),
    }
}

#[cfg(test)]
mod tests {
    use tokio::net::TcpListener;

    use super::*;

    #[test]
    fn host_validation_blocks_cloud_metadata_and_allows_lan_brokers() {
        assert_eq!(validate_mqtt_host("192.168.1.50").unwrap(), "192.168.1.50");
        assert_eq!(validate_mqtt_host("10.0.0.12").unwrap(), "10.0.0.12");
        assert_eq!(validate_mqtt_host("127.0.0.1").unwrap(), "127.0.0.1");
        assert_eq!(
            validate_mqtt_host("mosquitto.local").unwrap(),
            "mosquitto.local"
        );
        assert!(validate_mqtt_host("169.254.169.254").is_err());
        assert!(validate_mqtt_host("::ffff:169.254.169.254").is_err());
        assert!(validate_mqtt_host("metadata.google.internal").is_err());
        assert!(validate_mqtt_host("0.0.0.0").is_err());
        assert!(validate_mqtt_host("192.168.1.50:1883").is_err());
    }

    #[test]
    fn ambient_hex_maps_warm_and_cool_covers_to_kelvin_and_cozy_brightness() {
        let warm = map_hex_to_ambient_light("#ff5500").expect("warm color");
        assert_eq!(warm.color_temp_kelvin, 2700);
        assert!((25..=75).contains(&warm.brightness));

        let cool = map_hex_to_ambient_light("#0066ff").expect("cool color");
        assert_eq!(cool.color_temp_kelvin, 6500);
        assert!((25..=75).contains(&cool.brightness));

        let dark = map_hex_to_ambient_light("#101010").expect("dark color");
        let bright = map_hex_to_ambient_light("#f0f0e0").expect("bright color");
        assert!(dark.brightness < bright.brightness);
        assert!(dark.brightness >= 25 && bright.brightness <= 75);

        assert!(map_hex_to_ambient_light("#zzz").is_none());
        assert!(map_hex_to_ambient_light("12345").is_none());
    }

    #[test]
    fn mqtt_packet_encoders_produce_spec_compliant_3_1_1_bytes() {
        let mut rem = Vec::new();
        encode_remaining_length(127, &mut rem).unwrap();
        assert_eq!(rem, vec![0x7F]);
        rem.clear();
        encode_remaining_length(128, &mut rem).unwrap();
        assert_eq!(rem, vec![0x80, 0x01]);
        rem.clear();
        encode_remaining_length(16_384, &mut rem).unwrap();
        assert_eq!(rem, vec![0x80, 0x80, 0x01]);

        let connect = encode_connect_packet("palveni1", Some("user"), Some("pass"), 30).unwrap();
        assert_eq!(connect[0], 0x10);
        // Protocol Name "MQTT" + Level 4 + flags (clean|user|pass = 0xC2)
        assert_eq!(&connect[2..9], &[0x00, 0x04, b'M', b'Q', b'T', b'T', 0x04]);
        assert_eq!(connect[9], 0xC2);
        assert_eq!(&connect[10..12], &30u16.to_be_bytes());

        assert!(parse_connack([0x20, 0x02, 0x00, 0x00]).is_ok());
        assert!(parse_connack([0x20, 0x02, 0x00, 0x04]).is_err());

        let pub_pkt = encode_publish_qos0("palveni/status", br#"{"ok":true}"#, false).unwrap();
        assert_eq!(pub_pkt[0], 0x30);
        assert_eq!(encode_disconnect_packet(), [0xE0, 0x00]);
    }

    #[tokio::test]
    async fn mqtt_client_publishes_to_loopback_mock_broker_end_to_end() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let addr = listener.local_addr().unwrap();

        let broker = tokio::spawn(async move {
            let (mut sock, _) = listener.accept().await.unwrap();
            let mut buf = [0u8; 512];
            let n = sock.read(&mut buf).await.unwrap();
            assert!(n > 12);
            assert_eq!(buf[0], 0x10, "expected CONNECT");
            assert_eq!(&buf[2..9], &[0x00, 0x04, b'M', b'Q', b'T', b'T', 0x04]);
            // Reply CONNACK accepted
            sock.write_all(&[0x20, 0x02, 0x00, 0x00]).await.unwrap();

            // Read PUBLISH + DISCONNECT
            let mut rest = Vec::new();
            sock.read_to_end(&mut rest).await.unwrap();
            rest
        });

        publish_messages(
            "127.0.0.1",
            addr.port(),
            Some("demo"),
            Some("secret"),
            &[MqttPublishMessage {
                topic: "palveni/status".into(),
                payload: br#"{"event":"test","server":"palveni"}"#.to_vec(),
                retain: false,
            }],
        )
        .await
        .expect("mqtt publish succeeds");

        let received = broker.await.unwrap();
        assert_eq!(received[0], 0x30, "expected PUBLISH QoS 0");
        assert!(
            received
                .windows(b"palveni/status".len())
                .any(|w| w == b"palveni/status"),
            "topic missing in PUBLISH frame"
        );
        assert!(
            received
                .windows(br#"{"event":"test","server":"palveni"}"#.len())
                .any(|w| w == br#"{"event":"test","server":"palveni"}"#),
            "payload missing in PUBLISH frame"
        );
        assert_eq!(
            &received[received.len() - 2..],
            &[0xE0, 0x00],
            "expected trailing DISCONNECT"
        );
    }
}

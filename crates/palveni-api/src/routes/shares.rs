//! Public and password-protected share link HTTP endpoints (`/api/v1/shares*`,
//! `/api/v1/public/shares/*`) with Ed25519 proof tickets and GPS EXIF stripping.

use std::fmt::Write as _;

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
use chrono::{DateTime, TimeDelta, Utc};
use palveni_core::shares::{
    CreateShareRequest, PublicShareView, ShareLinkList, ShareLinkSummary, ShareMediaKind,
    UnlockShareRequest, UnlockShareResponse,
};
use palveni_core::storage::{ByteRange, StoragePath, StorageProvider};
use palveni_core::{ItemId, LibraryKind, ShareId, clock};
use palveni_db::audit::{AuditEntry, actions as audit_actions};
use palveni_db::shares::{NewShareLink, RevokeOutcome, ShareRecord, ShareUnavailableReason};
use palveni_db::video::PosterLevel;
use palveni_library::local::TrackFile;
use palveni_link::NodeIdentity;
use ring::digest::{SHA256, digest};
use ring::rand::{SecureRandom, SystemRandom};
use serde::Deserialize;
use serde_json::json;
use tokio::io::AsyncReadExt;
use tokio_util::io::ReaderStream;

use super::{audit, path_id, playback};
use crate::{
    AppState,
    auth::{CurrentUser, PASSWORD_MAX_CHARS, hash_password, verify_password},
    error::ApiError,
    json::ApiJson,
};

const SHARE_TOKEN_LEN: usize = 24;
const BASE62_ALPHABET: &[u8; 62] =
    b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
const PROOF_LIFETIME: TimeDelta = TimeDelta::hours(2);
const MAX_EXIF_SCRUB_BYTES: u64 = 32 * 1024 * 1024;

fn generate_share_token() -> Result<String, ApiError> {
    let rng = SystemRandom::new();
    let mut out = String::with_capacity(SHARE_TOKEN_LEN);
    let mut buf = [0u8; 64];
    while out.len() < SHARE_TOKEN_LEN {
        rng.fill(&mut buf)
            .map_err(|_| ApiError::internal("system random generator unavailable"))?;
        for &b in &buf {
            // 248 = 62 * 4: rejection sampling avoids modulo bias.
            if b < 248 {
                out.push(char::from(BASE62_ALPHABET[usize::from(b % 62)]));
                if out.len() == SHARE_TOKEN_LEN {
                    break;
                }
            }
        }
    }
    Ok(out)
}

fn share_proof_scope(share: &ShareRecord) -> String {
    let pw_tag = match share.password_hash.as_deref() {
        Some(hash) => {
            let hex_digest = hex::encode(digest(&SHA256, hash.as_bytes()));
            hex_digest[..16].to_owned()
        }
        None => "open".to_owned(),
    };
    format!(
        "palveni:public-share:v1\n{}\n{}\n{pw_tag}",
        share.id, share.token
    )
}

fn issue_share_proof(
    identity: &NodeIdentity,
    share: &ShareRecord,
    now: DateTime<Utc>,
) -> (DateTime<Utc>, String) {
    let default_exp = now + PROOF_LIFETIME;
    let expiry_ms = match share.expires_at {
        Some(limit_ms) => default_exp.timestamp_millis().min(limit_ms),
        None => default_exp.timestamp_millis(),
    };
    let expires_at = DateTime::from_timestamp_millis(expiry_ms).unwrap_or(default_exp);
    let scope = share_proof_scope(share);
    let sig = identity.sign(format!("{scope}\n{expiry_ms}").as_bytes());
    (expires_at, format!("{expiry_ms}.{}", hex::encode(sig)))
}

fn verify_share_proof(
    identity: &NodeIdentity,
    share: &ShareRecord,
    proof: &str,
    now: DateTime<Utc>,
) -> bool {
    if proof.len() > 192 {
        return false;
    }
    let Some((deadline, sig_hex)) = proof.split_once('.') else {
        return false;
    };
    let Ok(expiry_ms) = deadline.parse::<i64>() else {
        return false;
    };
    if expiry_ms <= now.timestamp_millis() {
        return false;
    }
    let mut sig_bytes = [0u8; 64];
    if hex::decode_to_slice(sig_hex, &mut sig_bytes).is_err() {
        return false;
    }
    let scope = share_proof_scope(share);
    identity
        .node_id()
        .verify(format!("{scope}\n{expiry_ms}").as_bytes(), &sig_bytes)
}

fn unavailable_error(reason: ShareUnavailableReason) -> ApiError {
    let msg = match reason {
        ShareUnavailableReason::Revoked => "Share link has been revoked",
        ShareUnavailableReason::Expired => "Share link has expired",
        ShareUnavailableReason::MaxViewsReached => "Share link has reached its maximum view limit",
    };
    ApiError::new(StatusCode::GONE, "unavailable", msg)
}

/// `GET /api/v1/shares`
pub async fn list(
    user: CurrentUser,
    State(state): State<AppState>,
) -> Result<Json<ShareLinkList>, ApiError> {
    let shares = state
        .db
        .shares()
        .list_for_user(user.user.id, user.is_admin())
        .await?;
    Ok(Json(ShareLinkList { shares }))
}

/// `POST /api/v1/shares`
pub async fn create(
    user: CurrentUser,
    State(state): State<AppState>,
    ApiJson(body): ApiJson<CreateShareRequest>,
) -> Result<Response, ApiError> {
    let now = clock::now();
    let now_ms = clock::to_millis(now);

    let password_hash = match body.password {
        Some(raw) if !raw.is_empty() => {
            if raw.chars().count() > PASSWORD_MAX_CHARS {
                return Err(ApiError::validation(format!(
                    "Share password must be at most {PASSWORD_MAX_CHARS} characters"
                ))
                .field("password"));
            }
            Some(hash_password(raw).await?)
        }
        _ => None,
    };

    let expires_at = if let Some(dt) = body.expires_at {
        let ms = clock::to_millis(dt);
        if ms <= now_ms {
            return Err(ApiError::validation("expiresAt must be in the future").field("expiresAt"));
        }
        Some(ms)
    } else if let Some(hours) = body.expires_in_hours {
        if !(1..=87_600).contains(&hours) {
            return Err(
                ApiError::validation("expiresInHours must be between 1 and 87600")
                    .field("expiresInHours"),
            );
        }
        Some(now_ms.saturating_add(i64::from(hours).saturating_mul(3_600_000)))
    } else {
        None
    };

    if let Some(mv) = body.max_views
        && !(1..=1_000_000).contains(&mv)
    {
        return Err(
            ApiError::validation("maxViews must be between 1 and 1000000").field("maxViews"),
        );
    }

    let token = generate_share_token()?;
    let strip_gps = body.strip_gps.unwrap_or(true);
    let allow_download = body.allow_download.unwrap_or(false);

    let created = state
        .db
        .shares()
        .create(NewShareLink {
            token,
            creator_id: user.user.id,
            target_kind: body.target_kind,
            target_id: body.target_id,
            library_id: body.library_id,
            title: body.title,
            password_hash,
            allow_download,
            strip_gps,
            expires_at,
            max_views: body.max_views,
        })
        .await?;

    audit(
        &state,
        AuditEntry {
            action: audit_actions::SHARE_CREATED,
            actor: Some(user.user.id),
            target: Some(created.id.to_string()),
            detail: Some(json!({
                "targetKind": created.target_kind,
                "targetId": created.target_id,
                "hasPassword": created.has_password,
                "allowDownload": created.allow_download,
                "stripGps": created.strip_gps,
                "maxViews": created.max_views,
            })),
        },
    )
    .await;

    Ok((StatusCode::CREATED, Json::<ShareLinkSummary>(created)).into_response())
}

/// `DELETE /api/v1/shares/{id}`
pub async fn revoke(
    user: CurrentUser,
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
) -> Result<StatusCode, ApiError> {
    let id: ShareId = path_id(&path?.0, "Share link")?;
    match state
        .db
        .shares()
        .revoke(id, user.user.id, user.is_admin())
        .await?
    {
        RevokeOutcome::NotFound => Err(ApiError::not_found("Share link not found")),
        RevokeOutcome::Forbidden => Err(ApiError::forbidden()),
        RevokeOutcome::Revoked => {
            audit(
                &state,
                AuditEntry {
                    action: audit_actions::SHARE_REVOKED,
                    actor: Some(user.user.id),
                    target: Some(id.to_string()),
                    detail: None,
                },
            )
            .await;
            Ok(StatusCode::NO_CONTENT)
        }
    }
}

#[derive(Deserialize)]
pub struct PublicShareQuery {
    proof: Option<String>,
}

/// `GET /api/v1/public/shares/{token}`
pub async fn public_get(
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
    query: Result<Query<PublicShareQuery>, QueryRejection>,
) -> Result<Response, ApiError> {
    let token = path?.0;
    let Query(q) = query?;
    let now = clock::now();
    let now_ms = clock::to_millis(now);

    let share = state
        .db
        .shares()
        .find_by_token(&token)
        .await?
        .ok_or_else(|| ApiError::not_found("Share link not found"))?;

    let has_valid_proof = q
        .proof
        .as_deref()
        .is_some_and(|p| verify_share_proof(&state.identity, &share, p, now));

    if share.password_hash.is_some() && !has_valid_proof {
        share
            .check_availability(now_ms, true)
            .map_err(unavailable_error)?;
        let locked_view = PublicShareView {
            token: share.token,
            title: share.title,
            target_kind: share.target_kind,
            locked: true,
            allow_download: false,
            strip_gps: share.strip_gps,
            expires_at: share.expires_at.and_then(DateTime::from_timestamp_millis),
            proof: None,
            items: Vec::new(),
        };
        return Ok((
            [(header::CACHE_CONTROL, "private, no-store")],
            Json(locked_view),
        )
            .into_response());
    }

    share
        .check_availability(now_ms, !has_valid_proof)
        .map_err(unavailable_error)?;

    let proof = if let Some(existing) = q.proof.filter(|_| has_valid_proof) {
        existing
    } else {
        if !state.db.shares().consume_view(share.id, now_ms).await? {
            return Err(unavailable_error(ShareUnavailableReason::MaxViewsReached));
        }
        issue_share_proof(&state.identity, &share, now).1
    };

    let items = state.db.shares().resolve_items(&share).await?;
    let view = PublicShareView {
        token: share.token,
        title: share.title,
        target_kind: share.target_kind,
        locked: false,
        allow_download: share.allow_download,
        strip_gps: share.strip_gps,
        expires_at: share.expires_at.and_then(DateTime::from_timestamp_millis),
        proof: Some(proof),
        items,
    };
    Ok(([(header::CACHE_CONTROL, "private, no-store")], Json(view)).into_response())
}

/// `POST /api/v1/public/shares/{token}/unlock`
pub async fn public_unlock(
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
    ApiJson(body): ApiJson<UnlockShareRequest>,
) -> Result<Response, ApiError> {
    let token = path?.0;
    let now = clock::now();
    let now_ms = clock::to_millis(now);

    let share = state
        .db
        .shares()
        .find_by_token(&token)
        .await?
        .ok_or_else(|| ApiError::not_found("Share link not found"))?;

    share
        .check_availability(now_ms, true)
        .map_err(unavailable_error)?;

    if let Some(ref hash) = share.password_hash {
        if body.password.chars().count() > PASSWORD_MAX_CHARS {
            return Err(ApiError::new(
                StatusCode::UNAUTHORIZED,
                "invalid_credentials",
                "Invalid share password",
            ));
        }
        let ok = verify_password(body.password, hash.clone()).await?;
        if !ok {
            return Err(ApiError::new(
                StatusCode::UNAUTHORIZED,
                "invalid_credentials",
                "Invalid share password",
            ));
        }
    }

    if !state.db.shares().consume_view(share.id, now_ms).await? {
        return Err(unavailable_error(ShareUnavailableReason::MaxViewsReached));
    }

    let (expires_at, proof) = issue_share_proof(&state.identity, &share, now);
    Ok((
        [(header::CACHE_CONTROL, "private, no-store")],
        Json(UnlockShareResponse { proof, expires_at }),
    )
        .into_response())
}

#[derive(Deserialize)]
pub struct PublicMediaQuery {
    proof: Option<String>,
    size: Option<String>,
    download: Option<bool>,
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

fn content_disposition_attachment(name: &str) -> HeaderValue {
    let clean: String = name
        .chars()
        .map(|ch| {
            let directional = matches!(
                ch,
                '\u{061c}' | '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}'
            );
            if ch.is_control() || directional || r#"/\:"<>|?*"#.contains(ch) {
                '_'
            } else {
                ch
            }
        })
        .collect();
    let mut encoded = String::with_capacity(clean.len());
    for byte in clean.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            encoded.push(char::from(byte));
        } else {
            let _ = write!(encoded, "%{byte:02X}");
        }
    }
    let fallback = if clean
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b" _-().".contains(&b))
    {
        clean
    } else {
        let ext = palveni_library::classify::extension_of(name).unwrap_or_else(|| "bin".into());
        format!("shared.{ext}")
    };
    let value = format!("attachment; filename=\"{fallback}\"; filename*=UTF-8''{encoded}");
    HeaderValue::from_str(&value).unwrap_or_else(|_| HeaderValue::from_static("attachment"))
}

/// `GET /api/v1/public/shares/{token}/media/{itemId}`
pub async fn public_media(
    State(state): State<AppState>,
    method: Method,
    path: Result<Path<(String, String)>, PathRejection>,
    query: Result<Query<PublicMediaQuery>, QueryRejection>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let Path((token, raw_item_id)) = path?;
    let item_id: ItemId = path_id(&raw_item_id, "Shared item")?;
    let Query(q) = query?;
    let size = parse_optional_size(q.size.as_deref())?;
    let want_download = q.download.unwrap_or(false);

    let now = clock::now();
    let now_ms = clock::to_millis(now);
    let share = state
        .db
        .shares()
        .find_by_token(&token)
        .await?
        .ok_or_else(|| ApiError::not_found("Share link not found"))?;
    share
        .check_availability(now_ms, false)
        .map_err(unavailable_error)?;

    let proof = q.proof.as_deref().ok_or_else(ApiError::unauthorized)?;
    if !verify_share_proof(&state.identity, &share, proof, now) {
        return Err(ApiError::unauthorized());
    }

    if want_download && !share.allow_download {
        return Err(ApiError::new(
            StatusCode::FORBIDDEN,
            "forbidden",
            "Downloads are disabled for this share link",
        ));
    }

    let items = state.db.shares().resolve_items(&share).await?;
    let member = items
        .into_iter()
        .find(|i| i.item_id == item_id)
        .ok_or_else(|| ApiError::not_found("Shared item not found"))?;

    match member.media_kind {
        ShareMediaKind::Photo => {
            let (loc, file_name) = state
                .db
                .photos()
                .photo_location(item_id)
                .await?
                .ok_or_else(|| ApiError::not_found("Photo not found"))?;

            // CRITICAL PRIVACY INVARIANT:
            // When `strip_gps` is true, `original` passed to `open_photo` is ALWAYS false,
            // AND we scrub any remaining EXIF/XMP metadata chunks in pure Rust in case `ffmpeg`
            // is absent on the host!
            let open_original = !share.strip_gps && want_download;
            let (mime, len, mut reader) = state
                .video_images
                .open_photo(loc, size, open_original)
                .await
                .map_err(|err| match err {
                    palveni_core::Error::Internal(_) => ApiError::internal(err),
                    _ => ApiError::not_found("Photo unavailable"),
                })?;

            if share.strip_gps && len <= MAX_EXIF_SCRUB_BYTES {
                let mut raw_bytes = Vec::with_capacity(usize::try_from(len).unwrap_or(0));
                reader
                    .read_to_end(&mut raw_bytes)
                    .await
                    .map_err(ApiError::internal)?;
                let scrubbed = strip_image_metadata_bytes(&raw_bytes, mime);
                let mut builder = Response::builder()
                    .header(header::CONTENT_TYPE, mime)
                    .header(header::CONTENT_LENGTH, scrubbed.len())
                    .header(header::CACHE_CONTROL, "private, no-store")
                    .header(header::X_CONTENT_TYPE_OPTIONS, "nosniff");
                if want_download {
                    builder = builder.header(
                        header::CONTENT_DISPOSITION,
                        content_disposition_attachment(&file_name),
                    );
                }
                return builder
                    .body(Body::from(scrubbed))
                    .map_err(ApiError::internal);
            }

            let mut builder = Response::builder()
                .header(header::CONTENT_TYPE, mime)
                .header(header::CONTENT_LENGTH, len)
                .header(header::CACHE_CONTROL, "private, no-store")
                .header(header::X_CONTENT_TYPE_OPTIONS, "nosniff");
            if want_download {
                builder = builder.header(
                    header::CONTENT_DISPOSITION,
                    content_disposition_attachment(&file_name),
                );
            }
            builder
                .body(Body::from_stream(ReaderStream::with_capacity(
                    reader,
                    32 * 1024,
                )))
                .map_err(ApiError::internal)
        }
        ShareMediaKind::Audio => {
            let loc = state
                .db
                .music()
                .audio_location(item_id)
                .await?
                .ok_or_else(|| ApiError::not_found("Audio track not found"))?;
            let file_name = loc
                .relative_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("track.flac")
                .to_owned();
            let track = playback::open_audio(loc)
                .await
                .map_err(|_| ApiError::not_found("Audio file unavailable"))?;
            let key = playback::playback_key(item_id, b"share", proof.as_bytes());
            let mut resp = playback::serve_track(&state, track, method, &headers, key).await?;
            if want_download {
                resp.headers_mut().insert(
                    header::CONTENT_DISPOSITION,
                    content_disposition_attachment(&file_name),
                );
            }
            Ok(resp)
        }
        ShareMediaKind::Video => {
            let loc = state
                .db
                .video()
                .location(item_id)
                .await?
                .ok_or_else(|| ApiError::not_found("Video not found"))?;
            let file_name = loc
                .relative_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("video.mp4")
                .to_owned();
            let file = palveni_library::local::open_video(loc)
                .await
                .map_err(|_| ApiError::not_found("Video file unavailable"))?;
            let mut resp = playback::serve_original(file, method, &headers).await?;
            if want_download {
                resp.headers_mut().insert(
                    header::CONTENT_DISPOSITION,
                    content_disposition_attachment(&file_name),
                );
            }
            Ok(resp)
        }
        ShareMediaKind::Book | ShareMediaKind::Comic => {
            if !share.allow_download {
                return Err(ApiError::new(
                    StatusCode::FORBIDDEN,
                    "forbidden",
                    "Downloads are disabled for this share link",
                ));
            }
            let lib_kind = if member.media_kind == ShareMediaKind::Comic {
                LibraryKind::Comic
            } else {
                LibraryKind::Book
            };
            let loc = state
                .db
                .items()
                .original_location(item_id, lib_kind)
                .await?
                .ok_or_else(|| ApiError::not_found("File not found"))?;
            let (file, file_name) = open_reading_archive(loc)
                .await
                .map_err(|_| ApiError::not_found("Reading file unavailable"))?;
            let mut resp = playback::serve_original(file, method, &headers).await?;
            resp.headers_mut().insert(
                header::CONTENT_DISPOSITION,
                content_disposition_attachment(&file_name),
            );
            resp.headers_mut().insert(
                header::X_CONTENT_TYPE_OPTIONS,
                HeaderValue::from_static("nosniff"),
            );
            Ok(resp)
        }
    }
}

/// `GET /api/v1/public/shares/{token}/cover/{itemId}`
pub async fn public_cover(
    State(state): State<AppState>,
    path: Result<Path<(String, String)>, PathRejection>,
    query: Result<Query<PublicMediaQuery>, QueryRejection>,
) -> Result<Response, ApiError> {
    static COVER_SCALER: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(2);
    let Path((token, raw_item_id)) = path?;
    let item_id: ItemId = path_id(&raw_item_id, "Shared item")?;
    let Query(q) = query?;
    let size = parse_optional_size(q.size.as_deref())?.unwrap_or(480);

    let now = clock::now();
    let now_ms = clock::to_millis(now);
    let share = state
        .db
        .shares()
        .find_by_token(&token)
        .await?
        .ok_or_else(|| ApiError::not_found("Share link not found"))?;
    share
        .check_availability(now_ms, false)
        .map_err(unavailable_error)?;

    let proof = q.proof.as_deref().ok_or_else(ApiError::unauthorized)?;
    if !verify_share_proof(&state.identity, &share, proof, now) {
        return Err(ApiError::unauthorized());
    }

    let items = state.db.shares().resolve_items(&share).await?;
    let member = items
        .into_iter()
        .find(|i| i.item_id == item_id)
        .ok_or_else(|| ApiError::not_found("Shared item not found"))?;

    match member.media_kind {
        ShareMediaKind::Video => {
            if let Some(loc) = state
                .db
                .video()
                .poster_location(item_id, PosterLevel::Work)
                .await?
                && let Ok((mime, len, reader)) =
                    state.video_images.open_image(loc, Some(size)).await
            {
                return stream_cover(mime, len, reader);
            }
            let thumb_loc = state
                .db
                .video()
                .thumb_location(item_id)
                .await?
                .ok_or_else(|| ApiError::not_found("Cover not found"))?;
            let (mime, len, reader) = state
                .video_images
                .open_or_grab_thumb(thumb_loc, Some(size))
                .await
                .map_err(|_| ApiError::not_found("Cover unavailable"))?;
            stream_cover(mime, len, reader)
        }
        ShareMediaKind::Audio => {
            let (mut mime, mut data) = palveni_library::artwork::open(&state.db, item_id)
                .await
                .map_err(|_| ApiError::not_found("Cover unavailable"))?;
            if let Ok(_permit) = COVER_SCALER.acquire().await
                && let Ok(scaled) = state.hardware.cover_thumbnail(&data, mime, size).await
            {
                data = scaled;
                mime = "image/jpeg";
            }
            bytes_cover(mime, data)
        }
        ShareMediaKind::Photo => {
            let (loc, _) = state
                .db
                .photos()
                .photo_location(item_id)
                .await?
                .ok_or_else(|| ApiError::not_found("Photo not found"))?;
            let (mime, len, mut reader) = state
                .video_images
                .open_photo(loc, Some(size), false)
                .await
                .map_err(|_| ApiError::not_found("Photo cover unavailable"))?;
            if share.strip_gps && len <= MAX_EXIF_SCRUB_BYTES {
                let mut raw = Vec::with_capacity(usize::try_from(len).unwrap_or(0));
                reader
                    .read_to_end(&mut raw)
                    .await
                    .map_err(ApiError::internal)?;
                return bytes_cover(mime, strip_image_metadata_bytes(&raw, mime));
            }
            stream_cover(mime, len, reader)
        }
        ShareMediaKind::Book | ShareMediaKind::Comic => {
            let (mut mime, mut data) = palveni_library::reading::read_cover(&state.db, item_id)
                .await
                .map_err(|_| ApiError::not_found("Cover unavailable"))?;
            if let Ok(_permit) = COVER_SCALER.acquire().await
                && let Ok(scaled) = state.hardware.cover_thumbnail(&data, mime, size).await
            {
                data = scaled;
                mime = "image/jpeg";
            }
            bytes_cover(mime, data)
        }
    }
}

fn stream_cover(
    mime: &'static str,
    len: u64,
    reader: palveni_core::storage::ByteStream,
) -> Result<Response, ApiError> {
    Response::builder()
        .header(header::CONTENT_TYPE, mime)
        .header(header::CONTENT_LENGTH, len)
        .header(header::CACHE_CONTROL, "private, no-store")
        .header(header::X_CONTENT_TYPE_OPTIONS, "nosniff")
        .body(Body::from_stream(ReaderStream::with_capacity(
            reader,
            32 * 1024,
        )))
        .map_err(ApiError::internal)
}

fn bytes_cover(mime: &'static str, data: Vec<u8>) -> Result<Response, ApiError> {
    Response::builder()
        .header(header::CONTENT_TYPE, mime)
        .header(header::CONTENT_LENGTH, data.len())
        .header(header::CACHE_CONTROL, "private, no-store")
        .header(header::X_CONTENT_TYPE_OPTIONS, "nosniff")
        .body(Body::from(data))
        .map_err(ApiError::internal)
}

async fn open_reading_archive(
    location: palveni_db::music::TrackLocation,
) -> palveni_core::Result<(TrackFile, String)> {
    let storage = palveni_library::LocalStorage::open(&location.root).await?;
    let mut path = StoragePath::root();
    for part in &location.relative_path {
        let seg = part
            .to_str()
            .ok_or_else(|| palveni_core::Error::invalid("invalid file path"))?;
        path = path.join(seg)?;
    }
    let name = path.file_name().unwrap_or("archive.bin").to_owned();
    let content_type = match palveni_library::classify::extension_of(&name).as_deref() {
        Some("epub") => "application/epub+zip",
        Some("pdf") => "application/pdf",
        Some("cbz" | "zip") => "application/zip",
        Some("cbt" | "tar") => "application/x-tar",
        Some("txt" | "md") => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    };
    let source = storage.open_file(&path).await?;
    let _ = ByteRange {
        start: 0,
        end: None,
    };
    Ok((
        TrackFile {
            source,
            content_type,
        },
        name,
    ))
}

/// Pure-Rust EXIF/XMP metadata stripper for JPEG, PNG, and WebP so `strip_gps = true`
/// never exposes raw EXIF GPS bytes even when `ffmpeg` is unavailable on the host.
pub(crate) fn strip_image_metadata_bytes(data: &[u8], mime: &str) -> Vec<u8> {
    if mime == "image/jpeg" || data.starts_with(&[0xFF, 0xD8, 0xFF]) {
        if let Some(stripped) = strip_jpeg_app_metadata(data) {
            return stripped;
        }
    } else if mime == "image/png" || data.starts_with(b"\x89PNG\r\n\x1a\n") {
        if let Some(stripped) = strip_png_metadata(data) {
            return stripped;
        }
    } else if (mime == "image/webp"
        || (data.starts_with(b"RIFF") && data.get(8..12) == Some(b"WEBP")))
        && let Some(stripped) = strip_webp_metadata(data)
    {
        return stripped;
    }
    data.to_vec()
}

fn strip_jpeg_app_metadata(data: &[u8]) -> Option<Vec<u8>> {
    if data.len() < 4 || data[0] != 0xFF || data[1] != 0xD8 {
        return None;
    }
    let mut out = Vec::with_capacity(data.len());
    out.extend_from_slice(&data[..2]);
    let mut pos = 2usize;

    while pos < data.len() {
        if data[pos] != 0xFF {
            out.extend_from_slice(&data[pos..]);
            break;
        }
        let mut marker_idx = pos + 1;
        while marker_idx < data.len() && data[marker_idx] == 0xFF {
            marker_idx += 1;
        }
        if marker_idx >= data.len() {
            break;
        }
        let marker = data[marker_idx];
        pos = marker_idx + 1;

        // Standalone markers without length
        if marker == 0xD9 || marker == 0x01 || (0xD0..=0xD7).contains(&marker) {
            out.extend_from_slice(&[0xFF, marker]);
            if marker == 0xD9 {
                break;
            }
            continue;
        }
        if pos + 2 > data.len() {
            return None;
        }
        let seg_len = usize::from(u16::from_be_bytes([data[pos], data[pos + 1]]));
        if seg_len < 2 || pos + seg_len > data.len() {
            return None;
        }
        let payload = &data[pos + 2..pos + seg_len];

        // Drop APP1..APP15 (Exif, XMP, IPTC, vendor location tags) & COM comments, keep APP0 (JFIF) & APP2 ICC_PROFILE.
        let keep = match marker {
            0xE0 => true,
            0xE2 => payload.starts_with(b"ICC_PROFILE\0"),
            0xE1 | 0xE3..=0xEF | 0xFE => false,
            _ => true,
        };
        if keep {
            out.extend_from_slice(&[0xFF, marker]);
            out.extend_from_slice(&data[pos..pos + seg_len]);
        }
        pos += seg_len;

        // Start of Scan (SOS): copy entropy-coded scan body to EOI verbatim.
        if marker == 0xDA {
            out.extend_from_slice(&data[pos..]);
            break;
        }
    }
    Some(out)
}

fn strip_png_metadata(data: &[u8]) -> Option<Vec<u8>> {
    const SIG: &[u8; 8] = b"\x89PNG\r\n\x1a\n";
    if !data.starts_with(SIG) {
        return None;
    }
    let mut out = Vec::with_capacity(data.len());
    out.extend_from_slice(SIG);
    let mut pos = 8usize;
    while pos + 12 <= data.len() {
        let len = usize::try_from(u32::from_be_bytes([
            data[pos],
            data[pos + 1],
            data[pos + 2],
            data[pos + 3],
        ]))
        .ok()?;
        let total = 12usize.checked_add(len)?;
        if pos + total > data.len() {
            return None;
        }
        let chunk_type = &data[pos + 4..pos + 8];
        let drop_chunk = matches!(chunk_type, b"eXIf" | b"tEXt" | b"iTXt" | b"zTXt");
        if !drop_chunk {
            out.extend_from_slice(&data[pos..pos + total]);
        }
        pos += total;
        if chunk_type == b"IEND" {
            break;
        }
    }
    Some(out)
}

fn strip_webp_metadata(data: &[u8]) -> Option<Vec<u8>> {
    if data.len() < 12 || &data[0..4] != b"RIFF" || &data[8..12] != b"WEBP" {
        return None;
    }
    let mut chunks = Vec::with_capacity(data.len());
    let mut pos = 12usize;
    while pos + 8 <= data.len() {
        let fourcc = &data[pos..pos + 4];
        let size = usize::try_from(u32::from_le_bytes([
            data[pos + 4],
            data[pos + 5],
            data[pos + 6],
            data[pos + 7],
        ]))
        .ok()?;
        let padded = size.checked_add(size & 1)?;
        let end = pos.checked_add(8)?.checked_add(padded)?;
        if end > data.len() {
            return None;
        }
        if !matches!(fourcc, b"EXIF" | b"XMP ") {
            chunks.extend_from_slice(&data[pos..end]);
        }
        pos = end;
    }
    let riff_size = u32::try_from(4usize.checked_add(chunks.len())?).ok()?;
    let mut out = Vec::with_capacity(12 + chunks.len());
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&riff_size.to_le_bytes());
    out.extend_from_slice(b"WEBP");
    out.extend_from_slice(&chunks);
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_jpeg_exif_gps_segments_completely() {
        // Minimal JPEG: SOI + APP1(Exif + GPS coordinates ASCII/binary marker) + DQT + EOI
        let exif_payload = b"Exif\0\0GPSLatitude\x0033.2794\x00131.5006";
        let app1_len = u16::try_from(exif_payload.len() + 2).unwrap();
        let mut jpeg = vec![0xFF, 0xD8, 0xFF, 0xE1];
        jpeg.extend_from_slice(&app1_len.to_be_bytes());
        jpeg.extend_from_slice(exif_payload);
        jpeg.extend_from_slice(&[0xFF, 0xDB, 0x00, 0x04, 0x00, 0x10, 0xFF, 0xD9]);

        let scrubbed = strip_image_metadata_bytes(&jpeg, "image/jpeg");
        assert!(scrubbed.starts_with(&[0xFF, 0xD8]));
        assert!(scrubbed.ends_with(&[0xFF, 0xD9]));
        assert!(!scrubbed.windows(4).any(|w| w == b"Exif"));
        assert!(!scrubbed.windows(3).any(|w| w == b"GPS"));
    }

    #[test]
    fn share_proof_is_bound_to_share_and_password_hash() {
        let identity = NodeIdentity::generate().unwrap();
        let now = clock::now();
        let mut rec = ShareRecord {
            id: ShareId::generate(),
            token: "abc123def456ghi789jkl012".into(),
            creator_id: palveni_core::UserId::generate(),
            target_kind: palveni_core::shares::ShareTargetKind::PhotoAlbum,
            target_id: "album-1".into(),
            library_id: None,
            title: "Test".into(),
            password_hash: Some("$argon2id$v=19$hash1".into()),
            allow_download: false,
            strip_gps: true,
            expires_at: None,
            max_views: None,
            view_count: 0,
            revoked_at: None,
            created_at: clock::to_millis(now),
        };
        let (exp, proof) = issue_share_proof(&identity, &rec, now);
        assert!(verify_share_proof(&identity, &rec, &proof, now));
        assert!(!verify_share_proof(&identity, &rec, &proof, exp));

        rec.password_hash = Some("$argon2id$v=19$hash2".into());
        assert!(!verify_share_proof(&identity, &rec, &proof, now));
    }

    #[tokio::test]
    async fn end_to_end_shares_collections_related_and_global_search_routes() {
        use axum::body::to_bytes;
        use palveni_core::collections::MediaCollectionEntryKind;
        use palveni_core::music::{AudioProperties, TrackTags};
        use palveni_core::reading::ReadingKind;
        use palveni_core::shares::ShareTargetKind;
        use palveni_core::{
            FeatureRegistry, ItemId, ItemKind, LibraryKind, PerfTier, Role, ServerId,
        };
        use palveni_db::items::ItemUpsert;
        use palveni_db::libraries::NewLibrary;
        use palveni_db::photos::PhotoMetadataUpsert;
        use palveni_db::reading::ReadingMetadataUpsert;
        use palveni_db::video::VideoMetadata;
        use palveni_db::{Db, DbOptions};
        use palveni_jobs::JobService;
        use palveni_media::{HardwareService, ProbeOptions};
        use palveni_plugin_host::{PluginCatalog, skin_store::SkinStore};
        use std::sync::Arc;
        use tokio_util::sync::CancellationToken;

        let tmp =
            std::env::temp_dir().join(format!("palveni-shares-api-test-{}", ShareId::generate()));
        std::fs::create_dir_all(&tmp).unwrap();
        let movies_dir = tmp.join("movies");
        let music_dir = tmp.join("music");
        let books_dir = tmp.join("books");
        let photos_dir = tmp.join("photos");
        for d in [&movies_dir, &music_dir, &books_dir, &photos_dir] {
            std::fs::create_dir_all(d).unwrap();
        }

        // Write a real JPEG containing raw APP1 Exif GPS bytes on disk
        let exif_payload = b"Exif\0\0GPSLatitude\x0031.2304\x00121.4737";
        let app1_len = u16::try_from(exif_payload.len() + 2).unwrap();
        let mut jpeg = vec![0xFF, 0xD8, 0xFF, 0xE1];
        jpeg.extend_from_slice(&app1_len.to_be_bytes());
        jpeg.extend_from_slice(exif_payload);
        jpeg.extend_from_slice(&[0xFF, 0xDB, 0x00, 0x04, 0x00, 0x10, 0xFF, 0xD9]);
        let photo_path = photos_dir.join("bund.jpg");
        std::fs::write(&photo_path, &jpeg).unwrap();

        let book_path = books_dir.join("Dune.epub");
        std::fs::write(&book_path, b"PK\x03\x04dune-ebook-bytes").unwrap();

        let movie_path = movies_dir.join("Dune.mp4");
        std::fs::write(&movie_path, b"\x00\x00\x00\x18ftypmp42dune-video").unwrap();

        let track_path = music_dir.join("01.flac");
        std::fs::write(&track_path, b"fLaC-dune-ost").unwrap();

        let db = Db::open(&DbOptions {
            main_url: format!("sqlite://{}/main.db?mode=rwc", tmp.display()),
            ops_url: format!("sqlite://{}/ops.db?mode=rwc", tmp.display()),
            max_connections: Some(4),
            min_connections: Some(1),
        })
        .await
        .unwrap();

        let user = db.users().create("alice", "pw", Role::Admin).await.unwrap();
        let session = db
            .sessions()
            .create(user.id, "TestWeb", None)
            .await
            .unwrap();
        let auth = CurrentUser {
            user: user.clone(),
            session_id: session.session_id,
            device_id: Some(session.device_id),
        };

        let vlib = db
            .libraries()
            .create(NewLibrary {
                name: "Movies".into(),
                kind: LibraryKind::Video,
                paths: vec![movies_dir.canonicalize().unwrap().display().to_string()],
            })
            .await
            .unwrap();
        let mlib = db
            .libraries()
            .create(NewLibrary {
                name: "Music".into(),
                kind: LibraryKind::Music,
                paths: vec![music_dir.canonicalize().unwrap().display().to_string()],
            })
            .await
            .unwrap();
        let blib = db
            .libraries()
            .create(NewLibrary {
                name: "Books".into(),
                kind: LibraryKind::Book,
                paths: vec![books_dir.canonicalize().unwrap().display().to_string()],
            })
            .await
            .unwrap();
        let plib = db
            .libraries()
            .create(NewLibrary {
                name: "Photos".into(),
                kind: LibraryKind::Photo,
                paths: vec![photos_dir.canonicalize().unwrap().display().to_string()],
            })
            .await
            .unwrap();

        let vid_id = ItemId::generate();
        db.items()
            .apply_scan_batch(
                vlib.id,
                &[ItemUpsert {
                    music: None,
                    id: vid_id,
                    parent_id: None,
                    kind: ItemKind::Video,
                    title: "Dune: Part Two".into(),
                    sort_title: "dune part two".into(),
                    search_text: "dune part two".into(),
                    path: movie_path.canonicalize().unwrap().display().to_string(),
                    size_bytes: Some(26),
                    modified_at: Some(1_760_000_000_000),
                }],
                &[],
            )
            .await
            .unwrap();
        db.video()
            .save_batch(
                vlib.id,
                vec![VideoMetadata {
                    item_id: vid_id,
                    stamp: "v1".into(),
                    work_group: "movie:dune".into(),
                    work_title: "Dune: Part Two".into(),
                    year: Some(2024),
                    work_overview: None,
                    work_poster: None,
                    work_original_title: None,
                    work_tagline: None,
                    work_rating: Some(86),
                    work_official_rating: None,
                    work_premiered: None,
                    work_studios: vec![],
                    work_backdrop: None,
                    season: None,
                    season_poster: None,
                    episode: None,
                    title: "Dune: Part Two".into(),
                    overview: None,
                    episode_poster: None,
                    runtime_ms: Some(9_960_000),
                    rating: None,
                    premiered: None,
                    thumb: None,
                    genres: vec![],
                    people: vec![],
                }],
            )
            .await
            .unwrap();

        let trk_id = ItemId::generate();
        db.items()
            .apply_scan_batch(
                mlib.id,
                &[ItemUpsert {
                    music: Some(TrackTags {
                        title: Some("Beginnings".into()),
                        artist: Some("Hans Zimmer".into()),
                        album_artist: Some("Hans Zimmer".into()),
                        album: Some("Dune (Original Motion Picture Soundtrack)".into()),
                        disc_number: Some(1),
                        track_number: Some(1),
                        duration_ms: Some(180_000),
                        year: Some(2024),
                        audio: AudioProperties {
                            codec: Some("flac".into()),
                            ..Default::default()
                        },
                        ..Default::default()
                    }),
                    id: trk_id,
                    parent_id: None,
                    kind: ItemKind::Track,
                    title: "Beginnings".into(),
                    sort_title: "beginnings".into(),
                    search_text: "beginnings hans zimmer dune".into(),
                    path: track_path.canonicalize().unwrap().display().to_string(),
                    size_bytes: Some(13),
                    modified_at: Some(1_760_000_000_000),
                }],
                &[],
            )
            .await
            .unwrap();

        let bk_id = ItemId::generate();
        db.items()
            .apply_scan_batch(
                blib.id,
                &[ItemUpsert {
                    music: None,
                    id: bk_id,
                    parent_id: None,
                    kind: ItemKind::Book,
                    title: "Dune - Book 1".into(),
                    sort_title: "dune 01".into(),
                    search_text: "dune frank herbert".into(),
                    path: book_path.canonicalize().unwrap().display().to_string(),
                    size_bytes: Some(22),
                    modified_at: Some(1_760_000_000_000),
                }],
                &[],
            )
            .await
            .unwrap();
        db.reading()
            .upsert_batch(&[ReadingMetadataUpsert {
                item_id: bk_id,
                library_id: blib.id,
                stamp: "r1".into(),
                kind: ReadingKind::Book,
                format: "epub".into(),
                title: "Dune - Book 1".into(),
                sort_title: "dune 01".into(),
                authors: vec!["Frank Herbert".into()],
                series_name: None,
                series_key: None,
                series_index: None,
                summary: None,
                publisher: None,
                language: Some("en".into()),
                page_count: Some(512),
                has_cover: false,
                tags: vec![],
            }])
            .await
            .unwrap();

        let ph_id = ItemId::generate();
        db.items()
            .apply_scan_batch(
                plib.id,
                &[ItemUpsert {
                    music: None,
                    id: ph_id,
                    parent_id: None,
                    kind: ItemKind::Photo,
                    title: "Shanghai Bund".into(),
                    sort_title: "shanghai bund".into(),
                    search_text: "shanghai bund".into(),
                    path: photo_path.canonicalize().unwrap().display().to_string(),
                    size_bytes: Some(i64::try_from(jpeg.len()).unwrap()),
                    modified_at: Some(1_760_000_000_000),
                }],
                &[],
            )
            .await
            .unwrap();
        db.photos()
            .upsert_batch(
                plib.id,
                &[PhotoMetadataUpsert {
                    item_id: ph_id,
                    stamp: "p1".into(),
                    taken_at: 1_760_000_000_000,
                    day_bucket: "2025-10-09".into(),
                    month_bucket: "2025-10".into(),
                    width: Some(1920),
                    height: Some(1080),
                    orientation: 1,
                    camera_make: Some("Leica".into()),
                    camera_model: Some("Q3".into()),
                    lens_model: None,
                    iso: Some(100),
                    f_number: None,
                    exposure_time: None,
                    focal_length: None,
                    latitude: Some(31.2304),
                    longitude: Some(121.4737),
                    content_hash: Some("e".repeat(64)),
                }],
            )
            .await
            .unwrap();
        let album = db
            .photos()
            .create_album(user.id, "Bund Walk", None, None)
            .await
            .unwrap();
        db.photos()
            .add_album_items(user.id, album.id, &[ph_id])
            .await
            .unwrap();

        let bus = db.event_bus(16).await.unwrap();
        let jobs = JobService::new(db.clone(), bus.clone(), 1);
        let state = AppState {
            db: db.clone(),
            music_search: Arc::new(palveni_library::music_search::MusicSearch::new(
                tmp.join("search"),
            )),
            video_images: Arc::new(palveni_library::video::VideoImages::new(
                tmp.join("cache"),
                None,
                None,
            )),
            identity: Arc::new(NodeIdentity::generate().unwrap()),
            link: Arc::new(crate::LinkPairings::default()),
            login_limit: Arc::new(crate::LoginLimit::new(4)),
            subsonic_limit: Arc::new(crate::LoginLimit::new(4)),
            bus,
            jobs,
            hardware: Arc::new(HardwareService::new(
                db.clone(),
                tmp.clone(),
                ProbeOptions::default(),
            )),
            plugins: Arc::new(PluginCatalog::new()),
            settings: Arc::new(tokio::sync::RwLock::new(Default::default())),
            boot_tier: PerfTier::detect(0, 1, false),
            skins: Arc::new(SkinStore::open(&tmp.join("skins")).unwrap()),
            skin_writes: Arc::new(tokio::sync::Semaphore::new(1)),
            features: FeatureRegistry::new(),
            server: Arc::new(crate::ServerInfo {
                id: ServerId::generate(),
                name: "Test".into(),
                version: "0.1.0".into(),
            }),
            shutdown: CancellationToken::new(),
        };

        let _router = crate::router(state.clone());

        // 1. Create password-protected photo album share with `stripGps: true`
        let create_resp = create(
            auth.clone(),
            State(state.clone()),
            ApiJson(CreateShareRequest {
                target_kind: ShareTargetKind::PhotoAlbum,
                target_id: album.id.to_string(),
                library_id: None,
                title: None,
                password: Some("secret-pin".into()),
                allow_download: Some(false),
                strip_gps: Some(true),
                expires_at: None,
                expires_in_hours: None,
                max_views: Some(1),
            }),
        )
        .await
        .unwrap();
        assert_eq!(create_resp.status(), StatusCode::CREATED);
        let created_share: ShareLinkSummary =
            serde_json::from_slice(&to_bytes(create_resp.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(created_share.token.len(), 24);
        assert!(created_share.has_password);

        // Audit log records `share.created`
        let audits = db.audit().recent(10).await.unwrap();
        assert!(audits.iter().any(|a| a.action == "share.created"));

        // 2. Public GET without proof returns `locked: true` without leaking items
        let get_locked = public_get(
            State(state.clone()),
            Ok(Path(created_share.token.clone())),
            Ok(Query(PublicShareQuery { proof: None })),
        )
        .await
        .unwrap();
        assert_eq!(get_locked.status(), StatusCode::OK);
        let locked_view: PublicShareView =
            serde_json::from_slice(&to_bytes(get_locked.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert!(locked_view.locked);
        assert!(locked_view.items.is_empty());

        // 3. Wrong password returns 401 invalid_credentials; right password returns signed proof
        let bad_unlock = public_unlock(
            State(state.clone()),
            Ok(Path(created_share.token.clone())),
            ApiJson(UnlockShareRequest {
                password: "wrong".into(),
            }),
        )
        .await
        .unwrap_err();
        assert_eq!(bad_unlock.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(bad_unlock.code(), "invalid_credentials");

        let ok_unlock = public_unlock(
            State(state.clone()),
            Ok(Path(created_share.token.clone())),
            ApiJson(UnlockShareRequest {
                password: "secret-pin".into(),
            }),
        )
        .await
        .unwrap();
        assert_eq!(ok_unlock.status(), StatusCode::OK);
        let unlock_body: UnlockShareResponse =
            serde_json::from_slice(&to_bytes(ok_unlock.into_body(), usize::MAX).await.unwrap())
                .unwrap();

        // 4. Unlocked GET returns items with GPS stripped, and media bytes strip APP1 Exif GPS!
        let unlocked_resp = public_get(
            State(state.clone()),
            Ok(Path(created_share.token.clone())),
            Ok(Query(PublicShareQuery {
                proof: Some(unlock_body.proof.clone()),
            })),
        )
        .await
        .unwrap();
        assert_eq!(unlocked_resp.status(), StatusCode::OK);
        let unlocked_view: PublicShareView = serde_json::from_slice(
            &to_bytes(unlocked_resp.into_body(), usize::MAX)
                .await
                .unwrap(),
        )
        .unwrap();
        assert!(!unlocked_view.locked);
        assert_eq!(unlocked_view.items.len(), 1);
        assert_eq!(unlocked_view.items[0].latitude, None);
        assert_eq!(unlocked_view.items[0].longitude, None);

        let photo_resp = public_media(
            State(state.clone()),
            Method::GET,
            Ok(Path((created_share.token.clone(), ph_id.to_string()))),
            Ok(Query(PublicMediaQuery {
                proof: Some(unlock_body.proof.clone()),
                size: None,
                download: None,
            })),
            HeaderMap::new(),
        )
        .await
        .unwrap();
        assert_eq!(photo_resp.status(), StatusCode::OK);
        let served_bytes = to_bytes(photo_resp.into_body(), usize::MAX).await.unwrap();
        assert!(served_bytes.starts_with(&[0xFF, 0xD8]));
        assert!(!served_bytes.windows(4).any(|w| w == b"Exif"));
        assert!(!served_bytes.windows(3).any(|w| w == b"GPS"));

        // 5. Second viewer without proof hits `maxViews = 1` -> 410 Gone
        let second_viewer = public_get(
            State(state.clone()),
            Ok(Path(created_share.token.clone())),
            Ok(Query(PublicShareQuery { proof: None })),
        )
        .await
        .unwrap_err();
        assert_eq!(second_viewer.status(), StatusCode::GONE);

        // 6. Related works endpoint & Global Search endpoint
        let q_rel = Query::try_from_uri(&axum::http::Uri::from_static(
            "/api/v1/related?title=Dune%3A%20Part%20Two&excludeKind=video",
        ))
        .unwrap();
        let Json(rel_json) =
            crate::routes::collections::related(auth.clone(), State(state.clone()), Ok(q_rel))
                .await
                .unwrap();
        assert_eq!(rel_json.works.len(), 2);
        assert!(
            rel_json
                .works
                .iter()
                .any(|w| w.entry_kind == MediaCollectionEntryKind::MusicAlbum)
        );
        assert!(
            rel_json
                .works
                .iter()
                .any(|w| w.entry_kind == MediaCollectionEntryKind::Book)
        );

        let q_search = Query::try_from_uri(&axum::http::Uri::from_static(
            "/api/v1/search?q=Dune&limit=10",
        ))
        .unwrap();
        let Json(search_json) =
            crate::routes::search::search_all(auth.clone(), State(state.clone()), Ok(q_search))
                .await
                .unwrap();
        assert!(search_json.hits.len() >= 3);

        let _ = std::fs::remove_dir_all(&tmp);
    }
}

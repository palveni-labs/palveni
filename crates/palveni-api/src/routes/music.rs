pub mod library;
pub mod search;

use axum::{
    Json,
    extract::{
        Path, Query, State,
        rejection::{PathRejection, QueryRejection},
    },
};
use palveni_core::{LibraryId, music::MusicPage};
use palveni_db::music::MusicCursor;
use serde::Deserialize;

use super::{DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE, parse_limit, path_id};
use crate::{auth::CurrentUser, error::ApiError, state::AppState};

#[derive(Deserialize)]
pub struct Browse {
    artist: Option<String>,
    album: Option<String>,
    cursor: Option<String>,
    limit: Option<String>,
}

pub(super) fn validate_group(value: Option<&str>, field: &'static str) -> Result<(), ApiError> {
    if let Some(key) = value
        && (key.len() != 64
            || !key
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)))
    {
        return Err(ApiError::validation("Invalid group identifier").field(field));
    }
    Ok(())
}

pub async fn list(
    _: CurrentUser,
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
    query: Result<Query<Browse>, QueryRejection>,
) -> Result<Json<MusicPage>, ApiError> {
    let Path(raw) = path?;
    let id: LibraryId = path_id(&raw, "Library")?;
    let Query(query) = query?;
    let artist = query.artist.as_deref();
    let album = query.album.as_deref();
    validate_group(artist, "artist")?;
    validate_group(album, "album")?;
    if album.is_some() && artist.is_none() {
        return Err(ApiError::validation("An album requires its artist group").field("artist"));
    }
    let limit = parse_limit(query.limit.as_deref(), DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE)?;
    let cursor = query
        .cursor
        .as_deref()
        .map(|text| {
            MusicCursor::decode(text, id, artist, album)
                .ok_or_else(|| ApiError::validation("Invalid music cursor").field("cursor"))
        })
        .transpose()?;
    if !state.db.music().has_library(id).await? {
        return Err(ApiError::not_found("Music library not found"));
    }
    Ok(Json(
        state
            .db
            .music()
            .browse(id, artist, album, limit, cursor)
            .await?,
    ))
}

pub async fn analyze_loudness(
    _: crate::auth::AdminUser,
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
) -> Result<(axum::http::StatusCode, Json<super::libraries::ScanAccepted>), ApiError> {
    use palveni_core::{JobSubject, LibraryKind, music::LOUDNESS_JOB_KIND};
    let id: LibraryId = path_id(&path?.0, "Library")?;
    let library = state
        .db
        .libraries()
        .get(id)
        .await?
        .filter(|l| l.kind == LibraryKind::Music)
        .ok_or_else(|| ApiError::not_found("Music library not found"))?;
    let job = state
        .jobs
        .enqueue(palveni_jobs::NewJob {
            kind: LOUDNESS_JOB_KIND.into(),
            payload: serde_json::json!(id),
            subject: Some(JobSubject::library(id, library.name)),
            required_capabilities: Vec::new(),
            dedupe_key: Some(format!("{LOUDNESS_JOB_KIND}:{id}")),
        })
        .await?;
    Ok((
        axum::http::StatusCode::ACCEPTED,
        Json(super::libraries::ScanAccepted { job_id: job.id }),
    ))
}

pub async fn loudness(
    _: CurrentUser,
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
) -> Result<Json<palveni_core::music::TrackLoudness>, ApiError> {
    let id = path_id(&path?.0, "Track")?;
    let measurement = state.db.music().loudness(id).await?;
    Ok(Json(palveni_core::music::TrackLoudness { measurement }))
}

pub async fn lyrics(
    user: CurrentUser,
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
) -> Result<
    (
        axum::http::HeaderMap,
        Json<palveni_core::music::TrackLyrics>,
    ),
    ApiError,
> {
    let id = path_id(&path?.0, "Track")?;
    let location = state
        .db
        .music()
        .track_location(id)
        .await?
        .ok_or_else(|| ApiError::not_found("Track not found"))?;
    let mut lyrics =
        palveni_library::lyrics::read(location)
            .await
            .map_err(|error| match error {
                palveni_core::Error::Internal(detail) => ApiError::internal(detail),
                _ => ApiError::not_found("Track not available"),
            })?;
    if lyrics.is_none() {
        lyrics = fetch_lrclib_lyrics(&state, user.user.id, id).await;
    }
    let mut headers = axum::http::HeaderMap::new();
    headers.insert(
        axum::http::header::CACHE_CONTROL,
        axum::http::HeaderValue::from_static("private, no-store"),
    );
    Ok((headers, Json(palveni_core::music::TrackLyrics { lyrics })))
}

async fn fetch_lrclib_lyrics(
    state: &AppState,
    user_id: palveni_core::UserId,
    track_id: palveni_core::ItemId,
) -> Option<palveni_core::music::Lyrics> {
    let installed = state
        .db
        .plugins_photo_ai()
        .get_installed_plugin("palveni.recipe.lrclib")
        .await
        .ok()??;
    if !installed.enabled
        || !installed
            .granted_permissions
            .iter()
            .any(|perm| perm == "network:lrclib.net")
    {
        return None;
    }

    let prefer_synced = installed
        .settings
        .get("preferSynced")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(true);
    let fallback_plain = installed
        .settings
        .get("fallbackPlain")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(true);
    if !prefer_synced && !fallback_plain {
        return None;
    }

    let track = state
        .db
        .music()
        .lookup_tracks(user_id, &[track_id])
        .await
        .ok()?
        .into_iter()
        .next()?;
    let title = track.title.trim();
    if title.is_empty() {
        return None;
    }

    let mut params: Vec<(&str, String)> = vec![("track_name", title.to_owned())];
    if let Some(artist) = track
        .artist
        .as_deref()
        .or(track.album_artist.as_deref())
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        params.push(("artist_name", artist.to_owned()));
    }
    if let Some(album) = track
        .album
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        params.push(("album_name", album.to_owned()));
    }
    if let Some(duration_ms) = track.duration_ms.filter(|ms| *ms > 0) {
        let secs = (duration_ms + 500) / 1000;
        if secs > 0 {
            params.push(("duration", secs.to_string()));
        }
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .user_agent("Palveni-Server/0.1 (+https://palveni.local)")
        .build()
        .ok()?;
    let response = client
        .get("https://lrclib.net/api/get")
        .query(&params)
        .send()
        .await
        .ok()?;
    if !response.status().is_success() {
        return None;
    }
    let payload: serde_json::Value = response.json().await.ok()?;
    parse_lrclib_response(&payload, prefer_synced, fallback_plain)
}

fn parse_lrclib_response(
    payload: &serde_json::Value,
    prefer_synced: bool,
    fallback_plain: bool,
) -> Option<palveni_core::music::Lyrics> {
    use palveni_core::music::{Lyrics, LyricsFormat, LyricsSource};
    let synced = payload
        .get("syncedLyrics")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let plain = payload
        .get("plainLyrics")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty());

    if prefer_synced {
        if let Some(text) = synced {
            return Some(Lyrics {
                source: LyricsSource::Online,
                format: LyricsFormat::Lrc,
                text: text.to_owned(),
            });
        }
        if fallback_plain && let Some(text) = plain {
            return Some(Lyrics {
                source: LyricsSource::Online,
                format: LyricsFormat::Plain,
                text: text.to_owned(),
            });
        }
        return None;
    }

    if fallback_plain && let Some(text) = plain {
        return Some(Lyrics {
            source: LyricsSource::Online,
            format: LyricsFormat::Plain,
            text: text.to_owned(),
        });
    }
    None
}

#[derive(Deserialize)]
pub struct ArtworkQuery {
    size: Option<u16>,
}

fn artwork_tier(size: u16) -> u16 {
    const TIERS: [u16; 5] = [96, 240, 480, 960, 1920];
    TIERS
        .into_iter()
        .min_by_key(|tier| tier.abs_diff(size))
        .unwrap_or(1920)
}

pub async fn artwork(
    _: CurrentUser,
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
    query: Result<Query<ArtworkQuery>, QueryRejection>,
) -> Result<axum::response::Response, ApiError> {
    use axum::{body::Body, http::header, response::Response};
    static SCALER: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(2);
    let id = path_id(&path?.0, "Track")?;
    let requested = query?.0.size;
    if let Some(size) = requested
        && !(1..=4096).contains(&size)
    {
        return Err(ApiError::validation("size must be between 1 and 4096").field("size"));
    }
    let (mut mime, mut data) = palveni_library::artwork::open(&state.db, id)
        .await
        .map_err(|e| match e {
            palveni_core::Error::Internal(_) => ApiError::internal(e),
            _ => ApiError::not_found("Artwork unavailable"),
        })?;
    if let Some(tier) = requested.map(artwork_tier).filter(|t| *t <= 960)
        && let Ok(_permit) = SCALER.acquire().await
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

#[cfg(test)]
mod tests {
    use palveni_core::music::{LyricsFormat, LyricsSource};
    use serde_json::json;

    use super::parse_lrclib_response;

    #[test]
    fn lrclib_parser_honors_synced_preference_and_plain_fallback() {
        let both = json!({
            "syncedLyrics": "[00:10.00] First line\n[00:15.00] Second line",
            "plainLyrics": "First line\nSecond line"
        });
        let preferred = parse_lrclib_response(&both, true, true).unwrap();
        assert_eq!(preferred.source, LyricsSource::Online);
        assert_eq!(preferred.format, LyricsFormat::Lrc);
        assert!(preferred.text.starts_with("[00:10.00]"));

        let plain_pref = parse_lrclib_response(&both, false, true).unwrap();
        assert_eq!(plain_pref.source, LyricsSource::Online);
        assert_eq!(plain_pref.format, LyricsFormat::Plain);
        assert_eq!(plain_pref.text, "First line\nSecond line");

        let plain_only = json!({
            "syncedLyrics": "   ",
            "plainLyrics": "Plain verse"
        });
        let fallback = parse_lrclib_response(&plain_only, true, true).unwrap();
        assert_eq!(fallback.format, LyricsFormat::Plain);
        assert!(parse_lrclib_response(&plain_only, true, false).is_none());
        assert!(parse_lrclib_response(&both, false, false).is_none());
    }
}

use super::*;
use futures_util::{StreamExt, stream};
use palveni_core::PerfTier;
use palveni_media::audio::{AudioError, AudioFormat, Profile};

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Parameters {
    format: Option<String>,
    bitrate_kbps: Option<u16>,
    start_ms: Option<u64>,
    ticket: Option<String>,
}

impl Parameters {
    pub(super) fn profile(&self) -> Result<Option<Profile>, ApiError> {
        let format = match self.format.as_deref() {
            None | Some("original") => {
                return if self.bitrate_kbps.is_none() && self.start_ms.is_none() {
                    Ok(None)
                } else {
                    Err(
                        ApiError::validation("Encoding options require an audio format")
                            .field("format"),
                    )
                };
            }
            Some("mp3") => AudioFormat::Mp3,
            Some("aac") => AudioFormat::Aac,
            Some("opus") => AudioFormat::Opus,
            _ => return Err(ApiError::validation("Unknown audio format").field("format")),
        };
        let bitrate_kbps = self.bitrate_kbps.unwrap_or(128);
        if !(32..=320).contains(&bitrate_kbps) {
            return Err(
                ApiError::validation("Audio bitrate must be 32–320 kbit/s").field("bitrateKbps")
            );
        }
        let start_ms = self.start_ms.unwrap_or_default();
        if start_ms > 7 * 24 * 3600 * 1000 {
            return Err(ApiError::validation("Start time exceeds seven days").field("startMs"));
        }
        Ok(Some(Profile {
            format,
            bitrate_kbps,
            start_ms,
        }))
    }
}

fn scope(id: ItemId, p: Profile) -> String {
    format!(
        "palveni:audio-transcode:v1\n{id}\n{}\n{}\n{}",
        p.format.name(),
        p.bitrate_kbps,
        p.start_ms
    )
}
pub(super) async fn tier(state: &AppState) -> PerfTier {
    let configured = state.settings.read().await.perf_tier;
    match configured {
        Some(tier) => tier,
        None => state
            .hardware
            .cached()
            .await
            .map_or(state.boot_tier, |r| r.perf_tier),
    }
}
async fn input(
    state: &AppState,
    id: ItemId,
    start_ms: u64,
) -> Result<(std::fs::File, Option<std::ops::Range<u64>>), ApiError> {
    let location = state
        .db
        .music()
        .audio_location(id)
        .await?
        .ok_or_else(|| ApiError::not_found("Audio item not found"))?;
    let window = location
        .cue_frames
        .as_ref()
        .map(|r| {
            let start = r.start * 588 + start_ms * 44_100 / 1000;
            let end = r.end * 588;
            if start >= end {
                Err(ApiError::validation("Start time exceeds track duration").field("startMs"))
            } else {
                Ok(start..end)
            }
        })
        .transpose()?;
    palveni_library::local::open_audio_input(location)
        .await
        .map(|file| (file, window))
        .map_err(|_| ApiError::not_found("Audio file unavailable"))
}
pub(super) fn problem(error: AudioError) -> ApiError {
    match error {
        AudioError::Invalid => ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "audio_unsupported",
            error.to_string(),
        ),
        AudioError::Disabled | AudioError::Unavailable => ApiError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "transcode_unavailable",
            error.to_string(),
        ),
        AudioError::UnsupportedFormat => ApiError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "transcode_format_unavailable",
            error.to_string(),
        ),
        AudioError::Busy => ApiError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "transcode_busy",
            error.to_string(),
        ),
    }
}
pub(super) async fn prepare(
    state: &AppState,
    id: ItemId,
    session: SessionId,
    p: Profile,
) -> Result<Response, ApiError> {
    input(state, id, p.start_ms).await?;
    state
        .hardware
        .audio
        .check_format(tier(state).await, p.format)
        .await
        .map_err(problem)?;
    let (expires_at, ticket) = issue_ticket(&state.identity, &scope(id, p), session, clock::now());
    let url = format!(
        "/api/v1/items/{id}/audio?format={}&bitrateKbps={}&startMs={}&ticket={ticket}",
        p.format.name(),
        p.bitrate_kbps,
        p.start_ms
    );
    Ok((
        [(header::CACHE_CONTROL, PRIVATE_CACHE)],
        Json(PlaybackInfo {
            mode: PlaybackMode::Transcode,
            url,
            expires_at,
        }),
    )
        .into_response())
}

pub async fn audio(
    State(state): State<AppState>,
    method: Method,
    path: Result<Path<String>, PathRejection>,
    query: Result<Query<Parameters>, QueryRejection>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let parameters = query?;
    let id = path_id(&path?.0, "Audio item")?;
    let profile = parameters
        .profile()?
        .ok_or_else(|| ApiError::validation("Expected an audio format").field("format"))?;
    if headers.contains_key(header::AUTHORIZATION) {
        CurrentUser::from_headers(&headers, &state).await?;
    } else {
        let ticket = parameters
            .ticket
            .as_deref()
            .ok_or_else(ApiError::unauthorized)?;
        let session = verify_ticket(&state.identity, &scope(id, profile), ticket, clock::now())
            .ok_or_else(ApiError::unauthorized)?;
        state
            .db
            .sessions()
            .active_by_id(session)
            .await?
            .ok_or_else(ApiError::unauthorized)?;
    }
    let (file, window) = input(&state, id, profile.start_ms).await?;
    let tier = tier(&state).await;
    state
        .hardware
        .audio
        .check_format(tier, profile.format)
        .await
        .map_err(problem)?;
    let body = if method == Method::HEAD {
        Body::empty()
    } else {
        let (first, chunks) = state
            .hardware
            .audio
            .start(tier, profile, file, window, None)
            .await
            .map_err(problem)?;
        let rest = stream::unfold(chunks, |mut rx| async {
            rx.recv().await.map(|chunk| (chunk, rx))
        });
        Body::from_stream(stream::once(async { Ok(first) }).chain(rest))
    };
    Response::builder()
        .header(header::ACCEPT_RANGES, "none")
        .header(header::CONTENT_TYPE, profile.format.content_type())
        .header(header::CACHE_CONTROL, PRIVATE_CACHE)
        .body(body)
        .map_err(ApiError::internal)
}

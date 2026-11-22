//! Direct play, remux or transcode, from the file and what the browser reports.
use serde::{Deserialize, Serialize};

use super::info::{AudioStreamInfo, ProbedMedia};

#[derive(utoipa::ToSchema, Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HlsSupport {
    Native,
    Mse,
    #[default]
    None,
}

#[derive(utoipa::ToSchema, Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RequestedMode {
    #[default]
    Auto,
    Direct,
    Transcode,
}

#[derive(utoipa::ToSchema, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DeliveryMode {
    Direct,
    Remux,
    Transcode,
}

#[derive(utoipa::ToSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientProfile {
    #[serde(default)]
    pub containers: Vec<String>,
    #[serde(default)]
    pub video_codecs: Vec<String>,
    #[serde(default)]
    pub audio_codecs: Vec<String>,
    #[serde(default)]
    pub hls: HlsSupport,
}

#[derive(Clone, Debug, Default)]
pub struct DecisionRequest {
    pub audio_index: Option<u32>,
    pub max_height: Option<u32>,
    pub max_bitrate_kbps: Option<u32>,
    pub mode: RequestedMode,
    pub client: ClientProfile,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Decision {
    pub mode: DeliveryMode,
    /// What prevents playing the original file, if anything.
    pub reason: Option<String>,
    /// File index of the audio stream to deliver.
    pub audio: Option<u32>,
    /// The chosen audio is already AAC and can be copied into HLS.
    pub copy_audio: bool,
    /// Output height when the source must be scaled down.
    pub scale_to: Option<u32>,
}

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum DecisionError {
    #[error("No audio stream {0} in this file")]
    UnknownAudio(u32),
    #[error("Transcoding needs a player with HLS support")]
    NoHls,
}

/// Video codecs that can be copied into fragmented MP4 HLS segments.
const COPYABLE_VIDEO: [&str; 3] = ["h264", "hevc", "av1"];

fn canonical(codec: &str) -> String {
    let lower = codec.trim().to_ascii_lowercase();
    match lower.as_str() {
        "avc" | "avc1" | "x264" => "h264".into(),
        "h265" | "hvc1" | "hev1" | "x265" => "hevc".into(),
        "av01" => "av1".into(),
        "mp4a" => "aac".into(),
        "ac-3" => "ac3".into(),
        "e-ac-3" | "ec-3" => "eac3".into(),
        "matroska" => "mkv".into(),
        "m4v" => "mp4".into(),
        _ => lower,
    }
}

fn supports(list: &[String], value: &str) -> bool {
    let value = canonical(value);
    list.iter().any(|entry| canonical(entry) == value)
}

pub fn decide(media: &ProbedMedia, request: &DecisionRequest) -> Result<Decision, DecisionError> {
    let info = &media.info;
    let client = &request.client;
    let default_audio = media.default_audio();
    let audio: Option<&AudioStreamInfo> = match request.audio_index {
        Some(index) => Some(
            info.audio
                .iter()
                .find(|a| a.index == index)
                .ok_or(DecisionError::UnknownAudio(index))?,
        ),
        None => default_audio,
    };
    let height = info.video.as_ref().map(|v| v.height);
    let scale_to = request
        .max_height
        .filter(|max| height.is_some_and(|h| h > *max))
        .map(|max| max & !1);
    let over_bitrate = matches!(
        (request.max_bitrate_kbps, info.bitrate_kbps),
        (Some(max), Some(rate)) if rate > max
    );

    // Each entry blocks direct play; `video_blocked` entries also block remuxing.
    let mut reasons = Vec::new();
    let mut video_blocked = false;
    let container = info.container.as_deref().unwrap_or("unknown");
    if !supports(&client.containers, container) {
        reasons.push(format!("container {container}"));
    }
    if let Some(video) = &info.video {
        if !supports(&client.video_codecs, &video.codec) {
            reasons.push(format!("video codec {}", video.codec));
            video_blocked = true;
        } else if video.codec == "h264" && video.bit_depth.is_some_and(|d| d > 8) {
            reasons.push(format!("video h264 {}-bit", video.bit_depth.unwrap_or(10)));
            video_blocked = true;
        } else if !COPYABLE_VIDEO.contains(&canonical(&video.codec).as_str()) {
            video_blocked = true;
        }
    }
    if let Some(audio) = audio {
        if !supports(&client.audio_codecs, &audio.codec) {
            reasons.push(format!("audio codec {}", audio.codec));
        }
        if default_audio.is_some_and(|d| d.index != audio.index) {
            reasons.push(format!("audio track {}", audio.index));
        }
    }
    if let (Some(max), Some(height)) = (scale_to, height) {
        reasons.push(format!("height {height} > {max}"));
        video_blocked = true;
    }
    if over_bitrate {
        reasons.push(format!(
            "bitrate {} kbps > {} kbps",
            info.bitrate_kbps.unwrap_or_default(),
            request.max_bitrate_kbps.unwrap_or_default()
        ));
        video_blocked = true;
    }
    let reason = (!reasons.is_empty()).then(|| reasons.join(", "));
    let copy_audio = audio.is_some_and(|a| a.codec == "aac");
    let decision = |mode, reason| Decision {
        mode,
        reason,
        audio: audio.map(|a| a.index),
        copy_audio,
        scale_to,
    };
    match request.mode {
        RequestedMode::Direct => return Ok(decision(DeliveryMode::Direct, reason)),
        RequestedMode::Transcode if client.hls == HlsSupport::None => {
            return Err(DecisionError::NoHls);
        }
        RequestedMode::Transcode => {
            return Ok(decision(
                DeliveryMode::Transcode,
                Some(reason.unwrap_or_else(|| "transcode requested".into())),
            ));
        }
        RequestedMode::Auto => {}
    }
    if reason.is_none() {
        return Ok(decision(DeliveryMode::Direct, None));
    }
    if client.hls == HlsSupport::None {
        // Nothing else this player can open; let it try the original.
        let reason = reason.map(|r| format!("{r}, no HLS support"));
        return Ok(decision(DeliveryMode::Direct, reason));
    }
    let mode = if video_blocked {
        DeliveryMode::Transcode
    } else {
        DeliveryMode::Remux
    };
    Ok(decision(mode, reason))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::video::info::{MediaInfo, VideoStreamInfo};

    fn media(container: &str, video: &str, depth: u8, height: u32, audio: &[&str]) -> ProbedMedia {
        ProbedMedia {
            info: MediaInfo {
                container: Some(container.into()),
                duration_ms: Some(60_000),
                size_bytes: Some(60_000_000),
                bitrate_kbps: Some(8_000),
                video: Some(VideoStreamInfo {
                    codec: video.into(),
                    profile: None,
                    width: height * 16 / 9,
                    height,
                    frame_rate: Some(24.0),
                    bit_depth: Some(depth),
                    hdr: None,
                }),
                audio: audio
                    .iter()
                    .zip(1..)
                    .map(|(codec, index)| AudioStreamInfo {
                        index,
                        codec: (*codec).into(),
                        language: None,
                        title: None,
                        channels: Some(2),
                        channel_layout: None,
                        default: index == 1,
                    })
                    .collect(),
                subtitles: vec![],
                chapters: vec![],
            },
            video_index: Some(0),
            start_time_ms: 0,
        }
    }

    fn chrome() -> ClientProfile {
        ClientProfile {
            containers: vec!["mp4".into(), "webm".into()],
            video_codecs: vec!["h264".into(), "vp9".into(), "av1".into()],
            audio_codecs: vec!["aac".into(), "mp3".into(), "opus".into(), "flac".into()],
            hls: HlsSupport::Mse,
        }
    }

    #[test]
    fn decision_table() {
        use DeliveryMode::*;
        struct Case {
            name: &'static str,
            media: ProbedMedia,
            request: DecisionRequest,
            mode: DeliveryMode,
            reason: Option<&'static str>,
            audio: Option<u32>,
            copy_audio: bool,
            scale_to: Option<u32>,
        }
        let auto = DecisionRequest {
            client: chrome(),
            ..DecisionRequest::default()
        };
        let with = |f: &dyn Fn(&mut DecisionRequest)| {
            let mut request = auto.clone();
            f(&mut request);
            request
        };
        let cases = [
            Case {
                name: "mp4 h264 aac plays directly",
                media: media("mp4", "h264", 8, 1080, &["aac"]),
                request: auto.clone(),
                mode: Direct,
                reason: None,
                audio: Some(1),
                copy_audio: true,
                scale_to: None,
            },
            Case {
                name: "mkv h264 aac is remuxed",
                media: media("mkv", "h264", 8, 1080, &["aac"]),
                request: auto.clone(),
                mode: Remux,
                reason: Some("container mkv"),
                audio: Some(1),
                copy_audio: true,
                scale_to: None,
            },
            Case {
                name: "mp4 hevc on a client without hevc is transcoded",
                media: media("mp4", "hevc", 10, 2160, &["aac"]),
                request: auto.clone(),
                mode: Transcode,
                reason: Some("video codec hevc"),
                audio: Some(1),
                copy_audio: true,
                scale_to: None,
            },
            Case {
                name: "mp4 hevc on a client with hevc plays directly",
                media: media("mp4", "hevc", 10, 2160, &["aac"]),
                request: with(&|r| r.client.video_codecs.push("h265".into())),
                mode: Direct,
                reason: None,
                audio: Some(1),
                copy_audio: true,
                scale_to: None,
            },
            Case {
                name: "unsupported audio is converted while video is copied",
                media: media("mp4", "h264", 8, 1080, &["ac3"]),
                request: auto.clone(),
                mode: Remux,
                reason: Some("audio codec ac3"),
                audio: Some(1),
                copy_audio: false,
                scale_to: None,
            },
            Case {
                name: "a non-default audio track needs remuxing",
                media: media("mp4", "h264", 8, 1080, &["aac", "aac"]),
                request: with(&|r| r.audio_index = Some(2)),
                mode: Remux,
                reason: Some("audio track 2"),
                audio: Some(2),
                copy_audio: true,
                scale_to: None,
            },
            Case {
                name: "selecting the default track explicitly still plays directly",
                media: media("mp4", "h264", 8, 1080, &["aac", "ac3"]),
                request: with(&|r| r.audio_index = Some(1)),
                mode: Direct,
                reason: None,
                audio: Some(1),
                copy_audio: true,
                scale_to: None,
            },
            Case {
                name: "mkv with ac3 default and aac second track",
                media: media("mkv", "h264", 8, 1080, &["ac3", "aac"]),
                request: with(&|r| r.audio_index = Some(2)),
                mode: Remux,
                reason: Some("container mkv, audio track 2"),
                audio: Some(2),
                copy_audio: true,
                scale_to: None,
            },
            Case {
                name: "a height limit below the source is transcoded",
                media: media("mp4", "h264", 8, 1080, &["aac"]),
                request: with(&|r| r.max_height = Some(720)),
                mode: Transcode,
                reason: Some("height 1080 > 720"),
                audio: Some(1),
                copy_audio: true,
                scale_to: Some(720),
            },
            Case {
                name: "a height limit above the source changes nothing",
                media: media("mp4", "h264", 8, 720, &["aac"]),
                request: with(&|r| r.max_height = Some(1080)),
                mode: Direct,
                reason: None,
                audio: Some(1),
                copy_audio: true,
                scale_to: None,
            },
            Case {
                name: "a bitrate cap below the source is transcoded",
                media: media("mp4", "h264", 8, 1080, &["aac"]),
                request: with(&|r| r.max_bitrate_kbps = Some(4_000)),
                mode: Transcode,
                reason: Some("bitrate 8000 kbps > 4000 kbps"),
                audio: Some(1),
                copy_audio: true,
                scale_to: None,
            },
            Case {
                name: "10-bit h264 cannot be decoded by browsers",
                media: media("mkv", "h264", 10, 1080, &["aac"]),
                request: auto.clone(),
                mode: Transcode,
                reason: Some("container mkv, video h264 10-bit"),
                audio: Some(1),
                copy_audio: true,
                scale_to: None,
            },
            Case {
                name: "an old codec in a supported container",
                media: media("avi", "mpeg4", 8, 480, &["mp3"]),
                request: with(&|r| r.client.video_codecs.push("mpeg4".into())),
                mode: Transcode,
                reason: Some("container avi"),
                audio: Some(1),
                copy_audio: false,
                scale_to: None,
            },
            Case {
                name: "forced transcoding",
                media: media("mp4", "h264", 8, 1080, &["aac"]),
                request: with(&|r| r.mode = RequestedMode::Transcode),
                mode: Transcode,
                reason: Some("transcode requested"),
                audio: Some(1),
                copy_audio: true,
                scale_to: None,
            },
            Case {
                name: "forced direct play reports what may fail",
                media: media("mkv", "hevc", 10, 2160, &["dts"]),
                request: with(&|r| r.mode = RequestedMode::Direct),
                mode: Direct,
                reason: Some("container mkv, video codec hevc, audio codec dts"),
                audio: Some(1),
                copy_audio: false,
                scale_to: None,
            },
            Case {
                name: "a player without HLS falls back to the original",
                media: media("mkv", "h264", 8, 1080, &["aac"]),
                request: with(&|r| r.client.hls = HlsSupport::None),
                mode: Direct,
                reason: Some("container mkv, no HLS support"),
                audio: Some(1),
                copy_audio: true,
                scale_to: None,
            },
            Case {
                name: "codec and container aliases",
                media: media("mkv", "hevc", 8, 1080, &["eac3"]),
                request: with(&|r| {
                    r.client.containers.push("matroska".into());
                    r.client.video_codecs.push("HVC1".into());
                    r.client.audio_codecs.push("ec-3".into());
                }),
                mode: Direct,
                reason: None,
                audio: Some(1),
                copy_audio: false,
                scale_to: None,
            },
        ];
        for case in cases {
            let decision = decide(&case.media, &case.request).unwrap();
            assert_eq!(decision.mode, case.mode, "{}", case.name);
            assert_eq!(decision.reason.as_deref(), case.reason, "{}", case.name);
            assert_eq!(decision.audio, case.audio, "{}", case.name);
            assert_eq!(decision.copy_audio, case.copy_audio, "{}", case.name);
            assert_eq!(decision.scale_to, case.scale_to, "{}", case.name);
        }
    }

    #[test]
    fn invalid_requests() {
        let file = media("mp4", "h264", 8, 1080, &["aac"]);
        let request = DecisionRequest {
            audio_index: Some(7),
            client: chrome(),
            ..DecisionRequest::default()
        };
        assert_eq!(decide(&file, &request), Err(DecisionError::UnknownAudio(7)));
        let request = DecisionRequest {
            mode: RequestedMode::Transcode,
            ..DecisionRequest::default()
        };
        assert_eq!(decide(&file, &request), Err(DecisionError::NoHls));
    }

    #[test]
    fn audio_only_and_odd_heights() {
        let mut file = media("mp4", "h264", 8, 1080, &["aac"]);
        file.info.video = None;
        file.video_index = None;
        let decision = decide(
            &file,
            &DecisionRequest {
                max_height: Some(480),
                client: chrome(),
                ..DecisionRequest::default()
            },
        )
        .unwrap();
        assert_eq!(decision.mode, DeliveryMode::Direct);
        let file = media("mp4", "h264", 8, 1080, &[]);
        let decision = decide(
            &file,
            &DecisionRequest {
                max_height: Some(719),
                client: chrome(),
                ..DecisionRequest::default()
            },
        )
        .unwrap();
        assert_eq!(
            (decision.mode, decision.scale_to, decision.audio),
            (DeliveryMode::Transcode, Some(718), None)
        );
    }
}

//! Media information of a video file from one bounded ffprobe run.
use std::{collections::BTreeMap, fs::File, path::Path, process::Stdio, time::Duration};

use serde::{Deserialize, Serialize};
use tokio::process::Command;

use crate::process::{FILE_INPUT, bounded_output};

/// Demuxers a library video may need. ffprobe and ffmpeg refuse anything else.
pub const VIDEO_DEMUXERS: &str = "mov,matroska,avi,mpegts,mpeg,asf,flv,ogg,mxf,rm";
const MAX_PROBE_BYTES: usize = 2 * 1024 * 1024;
const PROBE_TIMEOUT: Duration = Duration::from_secs(20);
const TEXT_SUBTITLES: [&str; 6] = ["subrip", "ass", "ssa", "mov_text", "webvtt", "text"];

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaInfo {
    /// Normalized container: `mp4`, `mov`, `mkv`, `webm`, `ts`, `avi`, …
    #[schema(required = true)]
    pub container: Option<String>,
    #[schema(required = true)]
    pub duration_ms: Option<u64>,
    #[schema(required = true)]
    pub size_bytes: Option<u64>,
    #[schema(required = true)]
    pub bitrate_kbps: Option<u32>,
    #[schema(required = true)]
    pub video: Option<VideoStreamInfo>,
    pub audio: Vec<AudioStreamInfo>,
    pub subtitles: Vec<SubtitleStreamInfo>,
    pub chapters: Vec<ChapterInfo>,
}

#[derive(utoipa::ToSchema, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum HdrFormat {
    Sdr,
    Hdr10,
    Hlg,
    DolbyVision,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoStreamInfo {
    pub codec: String,
    #[schema(required = true)]
    pub profile: Option<String>,
    pub width: u32,
    pub height: u32,
    #[schema(required = true)]
    pub frame_rate: Option<f64>,
    #[schema(required = true)]
    pub bit_depth: Option<u8>,
    #[schema(required = true)]
    pub hdr: Option<HdrFormat>,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioStreamInfo {
    /// Stream index in the file; pass it back as `audioIndex`.
    pub index: u32,
    pub codec: String,
    #[schema(required = true)]
    pub language: Option<String>,
    #[schema(required = true)]
    pub title: Option<String>,
    #[schema(required = true)]
    pub channels: Option<u32>,
    #[schema(required = true)]
    pub channel_layout: Option<String>,
    pub default: bool,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubtitleStreamInfo {
    pub index: u32,
    pub codec: String,
    #[schema(required = true)]
    pub language: Option<String>,
    #[schema(required = true)]
    pub title: Option<String>,
    pub forced: bool,
    pub default: bool,
    /// Can be converted to WebVTT by `GET /items/{id}/subtitles/{track}`.
    pub text_based: bool,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChapterInfo {
    pub start_ms: u64,
    #[schema(required = true)]
    pub title: Option<String>,
}

/// The public information plus what playback decisions need.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProbedMedia {
    pub info: MediaInfo,
    /// File index of the main (non cover art) video stream.
    pub video_index: Option<u32>,
    /// Container start offset, which `-ss` adds implicitly.
    pub start_time_ms: u64,
}

impl ProbedMedia {
    /// The track a player uses when none is chosen: flagged default, else the first.
    pub fn default_audio(&self) -> Option<&AudioStreamInfo> {
        let audio = &self.info.audio;
        audio.iter().find(|a| a.default).or_else(|| audio.first())
    }
}

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum ProbeFailure {
    #[error("FFprobe is not available")]
    Unavailable,
    #[error("Media probing is busy")]
    Busy,
    #[error("Media probing timed out")]
    Timeout,
    #[error("The file could not be read as a video")]
    Invalid,
}

#[derive(Deserialize, Default)]
struct Raw {
    #[serde(default)]
    format: RawFormat,
    #[serde(default)]
    streams: Vec<RawStream>,
    #[serde(default)]
    chapters: Vec<RawChapter>,
}

#[derive(Deserialize, Default)]
struct RawFormat {
    format_name: Option<String>,
    duration: Option<String>,
    size: Option<String>,
    bit_rate: Option<String>,
    start_time: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct RawStream {
    index: Option<u32>,
    codec_type: Option<String>,
    codec_name: Option<String>,
    codec_tag_string: Option<String>,
    profile: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
    pix_fmt: Option<String>,
    bits_per_raw_sample: Option<String>,
    color_transfer: Option<String>,
    avg_frame_rate: Option<String>,
    r_frame_rate: Option<String>,
    channels: Option<u32>,
    channel_layout: Option<String>,
    disposition: BTreeMap<String, i64>,
    tags: BTreeMap<String, String>,
    side_data_list: Vec<BTreeMap<String, serde_json::Value>>,
}

#[derive(Deserialize, Default)]
struct RawChapter {
    start_time: Option<String>,
    #[serde(default)]
    tags: BTreeMap<String, String>,
}

impl RawStream {
    fn flag(&self, name: &str) -> bool {
        self.disposition.get(name).is_some_and(|v| *v != 0)
    }

    fn tag(&self, name: &str) -> Option<String> {
        let value = self
            .tags
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))?
            .1
            .trim();
        (!value.is_empty() && value != "und").then(|| value.chars().take(128).collect())
    }
}

fn seconds_ms(text: Option<&str>) -> Option<u64> {
    let seconds: f64 = text?.trim().parse().ok()?;
    (seconds.is_finite() && seconds >= 0.0).then(|| {
        #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let ms = (seconds * 1000.0).round() as u64;
        ms
    })
}

fn rate(text: Option<&str>) -> Option<f64> {
    let (num, den) = text?.split_once('/')?;
    let (num, den): (f64, f64) = (num.parse().ok()?, den.parse().ok()?);
    (num > 0.0 && den > 0.0).then(|| (num / den * 1000.0).round() / 1000.0)
}

fn bit_depth(stream: &RawStream) -> Option<u8> {
    let pix = stream.pix_fmt.as_deref().unwrap_or_default();
    for (marker, depth) in [
        ("p16", 16),
        ("p14", 14),
        ("p12", 12),
        ("p10", 10),
        ("p9", 9),
    ] {
        if pix.contains(marker) {
            return Some(depth);
        }
    }
    if matches!(
        pix,
        "yuv420p" | "yuvj420p" | "yuv422p" | "yuvj422p" | "yuv444p" | "yuvj444p" | "nv12" | "nv21"
    ) {
        return Some(8);
    }
    stream.bits_per_raw_sample.as_deref()?.parse().ok()
}

fn hdr(stream: &RawStream) -> HdrFormat {
    let dolby = stream.side_data_list.iter().any(|entry| {
        entry
            .get("side_data_type")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|kind| kind.contains("DOVI"))
    }) || matches!(
        stream.codec_tag_string.as_deref(),
        Some("dvh1" | "dvhe" | "dva1" | "dvav")
    );
    if dolby {
        return HdrFormat::DolbyVision;
    }
    match stream.color_transfer.as_deref() {
        Some("smpte2084") => HdrFormat::Hdr10,
        Some("arib-std-b67") => HdrFormat::Hlg,
        _ => HdrFormat::Sdr,
    }
}

/// Short container name from ffprobe's demuxer list and the file extension.
pub fn container_name(format_name: &str, extension: Option<&str>) -> String {
    let ext = extension.map(str::to_ascii_lowercase);
    let first = format_name.split(',').next().unwrap_or_default();
    match first {
        "matroska" if ext.as_deref() == Some("webm") => "webm",
        "matroska" => "mkv",
        "mov" if matches!(ext.as_deref(), Some("mov" | "qt")) => "mov",
        "mov" => "mp4",
        "mpegts" => "ts",
        "asf" => "wmv",
        other => other,
    }
    .to_owned()
}

fn parse(data: &[u8], extension: Option<&str>) -> Result<ProbedMedia, ProbeFailure> {
    let raw: Raw = serde_json::from_slice(data).map_err(|_| ProbeFailure::Invalid)?;
    let format = &raw.format;
    let duration_ms = seconds_ms(format.duration.as_deref());
    let mut video_index = None;
    let mut video = None;
    let mut audio = Vec::new();
    let mut subtitles = Vec::new();
    for stream in &raw.streams {
        let (Some(index), Some(codec)) = (stream.index, stream.codec_name.clone()) else {
            continue;
        };
        match stream.codec_type.as_deref() {
            Some("video") if video.is_none() && !stream.flag("attached_pic") => {
                let (Some(width), Some(height)) = (stream.width, stream.height) else {
                    continue;
                };
                video_index = Some(index);
                video = Some(VideoStreamInfo {
                    codec,
                    profile: stream.profile.clone(),
                    width,
                    height,
                    frame_rate: rate(stream.avg_frame_rate.as_deref())
                        .or_else(|| rate(stream.r_frame_rate.as_deref())),
                    bit_depth: bit_depth(stream),
                    hdr: Some(hdr(stream)),
                });
            }
            Some("audio") => audio.push(AudioStreamInfo {
                index,
                codec,
                language: stream.tag("language"),
                title: stream.tag("title"),
                channels: stream.channels,
                channel_layout: stream.channel_layout.clone(),
                default: stream.flag("default"),
            }),
            Some("subtitle") => subtitles.push(SubtitleStreamInfo {
                index,
                text_based: TEXT_SUBTITLES.contains(&codec.as_str()),
                codec,
                language: stream.tag("language"),
                title: stream.tag("title"),
                forced: stream.flag("forced"),
                default: stream.flag("default"),
            }),
            _ => {}
        }
    }
    let mut chapters: Vec<ChapterInfo> = raw
        .chapters
        .iter()
        .filter_map(|chapter| {
            Some(ChapterInfo {
                start_ms: seconds_ms(chapter.start_time.as_deref())?,
                title: chapter
                    .tags
                    .get("title")
                    .map(|t| t.trim().chars().take(256).collect::<String>())
                    .filter(|t| !t.is_empty()),
            })
        })
        .collect();
    chapters.sort_by_key(|c| c.start_ms);
    chapters.dedup_by_key(|c| c.start_ms);
    chapters.truncate(1024);
    let size_bytes: Option<u64> = format.size.as_deref().and_then(|s| s.parse().ok());
    let bitrate_kbps = format
        .bit_rate
        .as_deref()
        .and_then(|b| b.parse::<u64>().ok())
        .map(|bps| u32::try_from(bps / 1000).unwrap_or(u32::MAX))
        .or_else(|| {
            let bits = size_bytes?.checked_mul(8)?;
            let ms = duration_ms.filter(|ms| *ms > 0)?;
            u32::try_from(bits / ms).ok()
        });
    Ok(ProbedMedia {
        info: MediaInfo {
            container: format
                .format_name
                .as_deref()
                .map(|name| container_name(name, extension)),
            duration_ms,
            size_bytes,
            bitrate_kbps,
            video,
            audio,
            subtitles,
            chapters,
        },
        video_index,
        start_time_ms: seconds_ms(format.start_time.as_deref()).unwrap_or(0),
    })
}

fn probe_command(ffprobe: &Path, file: File) -> Command {
    let mut cmd = Command::new(ffprobe);
    cmd.args([
        "-v",
        "error",
        "-threads",
        "1",
        "-max_alloc",
        "134217728",
        "-max_streams",
        "256",
        "-probesize",
        "10000000",
        "-analyzeduration",
        "5000000",
        "-protocol_whitelist",
        FILE_INPUT,
        "-format_whitelist",
        VIDEO_DEMUXERS,
    ])
    .stdin(Stdio::from(file));
    cmd
}

pub(crate) async fn probe(
    ffprobe: &Path,
    file: File,
    extension: Option<&str>,
) -> Result<ProbedMedia, ProbeFailure> {
    let mut cmd = probe_command(ffprobe, file);
    cmd.args([
        "-show_format",
        "-show_streams",
        "-show_chapters",
        "-of",
        "json",
        "-i",
        &format!("{FILE_INPUT}:"),
    ]);
    match bounded_output(cmd, MAX_PROBE_BYTES, PROBE_TIMEOUT).await {
        Ok(data) => parse(&data, extension),
        Err(err) if err.kind() == std::io::ErrorKind::TimedOut => Err(ProbeFailure::Timeout),
        Err(_) => Err(ProbeFailure::Invalid),
    }
}

/// Position (ms from the file start) of the video keyframe at or before `start_ms`.
pub(crate) async fn keyframe_before(
    ffprobe: &Path,
    file: File,
    start_ms: u64,
    media: &ProbedMedia,
) -> Option<u64> {
    let index = media.video_index?;
    let absolute = start_ms.saturating_add(media.start_time_ms);
    let mut cmd = probe_command(ffprobe, file);
    cmd.args([
        "-select_streams",
        &index.to_string(),
        "-read_intervals",
        &format!("{}.{:03}%+#1", absolute / 1000, absolute % 1000),
        "-show_entries",
        "packet=pts_time,dts_time",
        "-of",
        "json",
        "-i",
        &format!("{FILE_INPUT}:"),
    ]);
    #[derive(Deserialize)]
    struct Packets {
        #[serde(default)]
        packets: Vec<Packet>,
    }
    #[derive(Deserialize)]
    struct Packet {
        pts_time: Option<String>,
        dts_time: Option<String>,
    }
    let data = bounded_output(cmd, 64 * 1024, Duration::from_secs(10))
        .await
        .ok()?;
    let packet = serde_json::from_slice::<Packets>(&data)
        .ok()?
        .packets
        .into_iter()
        .next()?;
    let found = seconds_ms(packet.pts_time.or(packet.dts_time).as_deref())?;
    Some(found.saturating_sub(media.start_time_ms).min(start_ms))
}

#[cfg(test)]
mod tests {
    use super::*;

    const MKV: &str = include_str!("../../fixtures/ffprobe_video.json");

    #[test]
    fn parses_streams_chapters_and_hdr() {
        let media = parse(MKV.as_bytes(), Some("mkv")).unwrap();
        let info = &media.info;
        assert_eq!(info.container.as_deref(), Some("mkv"));
        assert_eq!(info.duration_ms, Some(5_400_250));
        assert_eq!(info.bitrate_kbps, Some(18_000));
        assert_eq!(media.video_index, Some(1));
        let video = info.video.as_ref().unwrap();
        assert_eq!(
            (video.codec.as_str(), video.width, video.height),
            ("hevc", 3840, 2160)
        );
        assert_eq!(video.frame_rate, Some(23.976));
        assert_eq!(video.bit_depth, Some(10));
        assert_eq!(video.hdr, Some(HdrFormat::Hdr10));
        assert_eq!(info.audio.len(), 2);
        assert_eq!(info.audio[0].index, 2);
        assert!(info.audio[0].default);
        assert_eq!(info.audio[0].language.as_deref(), Some("eng"));
        assert_eq!(info.audio[1].language, None);
        assert_eq!(info.audio[1].title.as_deref(), Some("Commentary"));
        assert_eq!(media.default_audio().unwrap().index, 2);
        assert_eq!(info.subtitles.len(), 2);
        assert!(info.subtitles[0].text_based && info.subtitles[0].forced);
        assert!(!info.subtitles[1].text_based);
        assert_eq!(
            info.chapters,
            vec![
                ChapterInfo {
                    start_ms: 0,
                    title: Some("Opening".into())
                },
                ChapterInfo {
                    start_ms: 600_500,
                    title: None
                }
            ]
        );
        let json = serde_json::to_value(info).unwrap();
        assert_eq!(json["video"]["hdr"], "hdr10");
        assert_eq!(json["audio"][1]["channelLayout"], "5.1(side)");
        assert_eq!(json["subtitles"][0]["textBased"], true);
    }

    #[test]
    fn container_names_follow_extension_where_demuxers_are_shared() {
        assert_eq!(
            container_name("mov,mp4,m4a,3gp,3g2,mj2", Some("m4v")),
            "mp4"
        );
        assert_eq!(
            container_name("mov,mp4,m4a,3gp,3g2,mj2", Some("MOV")),
            "mov"
        );
        assert_eq!(container_name("matroska,webm", Some("webm")), "webm");
        assert_eq!(container_name("matroska,webm", None), "mkv");
        assert_eq!(container_name("mpegts", Some("m2ts")), "ts");
        assert_eq!(container_name("avi", None), "avi");
    }

    #[test]
    fn dolby_vision_and_cover_art() {
        let data = br#"{"format":{"format_name":"mov,mp4","duration":"10.0","size":"1000"},
            "streams":[{"index":0,"codec_type":"video","codec_name":"mjpeg","width":600,"height":600,"disposition":{"attached_pic":1}},
            {"index":1,"codec_type":"video","codec_name":"hevc","codec_tag_string":"dvh1","width":1920,"height":1080,"pix_fmt":"yuv420p10le","avg_frame_rate":"0/0","r_frame_rate":"25/1"}]}"#;
        let media = parse(data, Some("mp4")).unwrap();
        assert_eq!(media.video_index, Some(1));
        let video = media.info.video.unwrap();
        assert_eq!(video.hdr, Some(HdrFormat::DolbyVision));
        assert_eq!(video.frame_rate, Some(25.0));
        assert_eq!(media.info.bitrate_kbps, Some(0));
        assert!(media.info.audio.is_empty());
        assert!(parse(b"not json", None).is_err());
    }
}

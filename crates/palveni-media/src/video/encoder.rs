//! H.264 encoder selection, FFmpeg HLS argument construction, and playlist rewriting.
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

use palveni_core::PerfTier;

use crate::{
    encoders::EncodeMethod,
    process::{FILE_INPUT, os_args},
    report::HardwareReport,
};

use super::{decision::DeliveryMode, info::VIDEO_DEMUXERS};

pub const SEGMENT_SECONDS: u32 = 4;
pub const PLAYLIST_FILE: &str = "index.m3u8";
pub const INIT_SEGMENT_FILE: &str = "init.mp4";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectedEncoder {
    pub id: String,
    pub method: EncodeMethod,
}

impl SelectedEncoder {
    pub fn software() -> Self {
        Self {
            id: "libx264".to_owned(),
            method: EncodeMethod::Software,
        }
    }

    pub fn is_hardware(&self) -> bool {
        self.method != EncodeMethod::Software
    }
}

fn encoder_rank(method: EncodeMethod) -> usize {
    match method {
        EncodeMethod::Videotoolbox => 0,
        EncodeMethod::Nvenc => 1,
        EncodeMethod::Qsv => 2,
        EncodeMethod::Vaapi => 3,
        EncodeMethod::Amf => 4,
        EncodeMethod::Rkmpp => 5,
        EncodeMethod::V4l2m2m => 6,
        EncodeMethod::Software => 7,
    }
}

/// Picks the best verified H.264 encoder from the hardware report, falling back to `libx264`.
pub fn select_h264_encoder(report: Option<&HardwareReport>) -> SelectedEncoder {
    let Some(report) = report else {
        return SelectedEncoder::software();
    };
    report
        .encoders
        .iter()
        .filter(|e| e.codec == "h264" && e.verified)
        .min_by_key(|e| (encoder_rank(e.method), e.id.as_str()))
        .map_or_else(SelectedEncoder::software, |e| SelectedEncoder {
            id: e.id.clone(),
            method: e.method,
        })
}

#[derive(Clone, Debug)]
pub(crate) struct HlsPlan {
    pub mode: DeliveryMode,
    pub video_index: u32,
    pub video_codec: Option<String>,
    pub video_width: u32,
    pub video_height: u32,
    pub frame_rate: Option<f64>,
    pub container: Option<String>,
    pub audio_index: Option<u32>,
    pub copy_audio: bool,
    pub scale_to: Option<u32>,
    pub max_bitrate_kbps: Option<u32>,
    pub start_ms: u64,
    pub tier: PerfTier,
    pub encoder: SelectedEncoder,
    pub render_node: Option<PathBuf>,
    pub use_readrate_burst: bool,
}

fn tier_threads(tier: PerfTier) -> u32 {
    match tier {
        PerfTier::Tiny => 1,
        PerfTier::Lite => 2,
        PerfTier::Standard => 4,
        PerfTier::Performance => 6,
    }
}

fn default_video_cap_kbps(height: u32) -> u32 {
    if height > 1080 {
        16_000
    } else if height > 720 {
        8_000
    } else if height > 480 {
        4_000
    } else {
        1_800
    }
}

fn target_audio_kbps(plan: &HlsPlan) -> u32 {
    if plan.audio_index.is_none() {
        0
    } else if plan.max_bitrate_kbps.is_some_and(|max| max <= 800) {
        128
    } else {
        192
    }
}

pub(crate) fn build_hls_args(plan: &HlsPlan, output_dir: &Path) -> Vec<OsString> {
    let threads = tier_threads(plan.tier);
    let filter_threads = threads.min(2);
    let mut args = os_args([
        "-nostdin",
        "-hide_banner",
        "-v",
        "error",
        "-threads",
        &threads.to_string(),
        "-filter_threads",
        &filter_threads.to_string(),
        "-max_alloc",
        "268435456",
        "-max_streams",
        "256",
        "-protocol_whitelist",
        FILE_INPUT,
        "-format_whitelist",
        VIDEO_DEMUXERS,
    ]);

    if plan.mode == DeliveryMode::Transcode && plan.encoder.method == EncodeMethod::Vaapi {
        let node = plan.render_node.as_deref().map_or_else(
            || "/dev/dri/renderD128".to_owned(),
            |p| p.display().to_string(),
        );
        args.extend(os_args([
            "-init_hw_device".to_owned(),
            format!("vaapi=va:{node}"),
            "-filter_hw_device".to_owned(),
            "va".to_owned(),
        ]));
    }

    if plan.use_readrate_burst {
        let (rate, burst) = if plan.mode == DeliveryMode::Remux {
            ("4", "24")
        } else {
            ("2.5", "16")
        };
        args.extend(os_args([
            "-readrate",
            rate,
            "-readrate_initial_burst",
            burst,
        ]));
    }

    if plan.start_ms > 0 {
        if plan.mode == DeliveryMode::Remux {
            args.push(OsString::from("-noaccurate_seek"));
        }
        args.extend(os_args(["-ss".to_owned(), format!("{}ms", plan.start_ms)]));
    }

    args.extend(os_args([
        "-i".to_owned(),
        format!("{FILE_INPUT}:"),
        "-map".to_owned(),
        format!("0:{}", plan.video_index),
    ]));
    if let Some(audio) = plan.audio_index {
        args.extend(os_args(["-map".to_owned(), format!("0:{audio}")]));
    } else {
        args.push(OsString::from("-an"));
    }
    args.extend(os_args([
        "-sn",
        "-dn",
        "-map_metadata",
        "-1",
        "-map_chapters",
        "-1",
    ]));

    let audio_kbps = target_audio_kbps(plan);
    match plan.mode {
        DeliveryMode::Remux | DeliveryMode::Direct => {
            args.extend(os_args(["-c:v", "copy"]));
            if plan
                .video_codec
                .as_deref()
                .is_some_and(|c| c.eq_ignore_ascii_case("hevc"))
            {
                args.extend(os_args(["-tag:v", "hvc1"]));
            }
        }
        DeliveryMode::Transcode => {
            let mut filters = Vec::new();
            if let Some(max_h) = plan.scale_to {
                filters.push(format!("scale=-2:{max_h}"));
            } else if !plan.video_width.is_multiple_of(2) || !plan.video_height.is_multiple_of(2) {
                filters.push("scale=trunc(iw/2)*2:trunc(ih/2)*2".to_owned());
            }
            let pix_filter = match plan.encoder.method {
                EncodeMethod::Vaapi => "format=nv12,hwupload",
                EncodeMethod::Qsv
                | EncodeMethod::Amf
                | EncodeMethod::Rkmpp
                | EncodeMethod::Videotoolbox => "format=nv12",
                EncodeMethod::Software | EncodeMethod::Nvenc | EncodeMethod::V4l2m2m => {
                    "format=yuv420p"
                }
            };
            filters.push(pix_filter.to_owned());
            args.extend(os_args(["-vf".to_owned(), filters.join(",")]));

            let effective_height = plan.scale_to.unwrap_or(plan.video_height);
            let video_max_kbps = match plan.max_bitrate_kbps {
                Some(total) => total.saturating_sub(audio_kbps).max(200),
                None => default_video_cap_kbps(effective_height),
            };
            let video_target_kbps = (video_max_kbps.saturating_mul(4) / 5).max(200);
            let bufsize_kbps = video_max_kbps.saturating_mul(2);

            #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let fps = plan
                .frame_rate
                .filter(|f| f.is_finite() && *f > 0.0)
                .map_or(24, |f| f.round() as u32)
                .clamp(15, 60);
            let gop = fps.saturating_mul(SEGMENT_SECONDS);

            match plan.encoder.method {
                EncodeMethod::Software => {
                    args.extend(os_args([
                        "-c:v".to_owned(),
                        plan.encoder.id.clone(),
                        "-preset".to_owned(),
                        "veryfast".to_owned(),
                        "-profile:v".to_owned(),
                        "high".to_owned(),
                        "-crf".to_owned(),
                        "23".to_owned(),
                        "-maxrate".to_owned(),
                        format!("{video_max_kbps}k"),
                        "-bufsize".to_owned(),
                        format!("{bufsize_kbps}k"),
                        "-threads:v".to_owned(),
                        threads.to_string(),
                        "-sc_threshold".to_owned(),
                        "0".to_owned(),
                    ]));
                }
                EncodeMethod::Videotoolbox => {
                    args.extend(os_args([
                        "-c:v".to_owned(),
                        plan.encoder.id.clone(),
                        "-profile:v".to_owned(),
                        "high".to_owned(),
                        "-b:v".to_owned(),
                        format!("{video_target_kbps}k"),
                        "-maxrate".to_owned(),
                        format!("{video_max_kbps}k"),
                        "-bufsize".to_owned(),
                        format!("{bufsize_kbps}k"),
                    ]));
                }
                EncodeMethod::Nvenc => {
                    args.extend(os_args([
                        "-c:v".to_owned(),
                        plan.encoder.id.clone(),
                        "-preset".to_owned(),
                        "p4".to_owned(),
                        "-profile:v".to_owned(),
                        "high".to_owned(),
                        "-b:v".to_owned(),
                        format!("{video_target_kbps}k"),
                        "-maxrate".to_owned(),
                        format!("{video_max_kbps}k"),
                        "-bufsize".to_owned(),
                        format!("{bufsize_kbps}k"),
                    ]));
                }
                EncodeMethod::Qsv => {
                    args.extend(os_args([
                        "-c:v".to_owned(),
                        plan.encoder.id.clone(),
                        "-preset".to_owned(),
                        "veryfast".to_owned(),
                        "-b:v".to_owned(),
                        format!("{video_target_kbps}k"),
                        "-maxrate".to_owned(),
                        format!("{video_max_kbps}k"),
                        "-bufsize".to_owned(),
                        format!("{bufsize_kbps}k"),
                    ]));
                }
                EncodeMethod::Vaapi
                | EncodeMethod::Amf
                | EncodeMethod::Rkmpp
                | EncodeMethod::V4l2m2m => {
                    args.extend(os_args([
                        "-c:v".to_owned(),
                        plan.encoder.id.clone(),
                        "-b:v".to_owned(),
                        format!("{video_target_kbps}k"),
                        "-maxrate".to_owned(),
                        format!("{video_max_kbps}k"),
                        "-bufsize".to_owned(),
                        format!("{bufsize_kbps}k"),
                    ]));
                }
            }
            args.extend(os_args([
                "-g".to_owned(),
                gop.to_string(),
                "-force_key_frames".to_owned(),
                format!("expr:gte(t,n_forced*{SEGMENT_SECONDS})"),
            ]));
        }
    }

    if plan.audio_index.is_some() {
        if plan.copy_audio {
            args.extend(os_args(["-c:a", "copy"]));
            if plan.container.as_deref() == Some("ts") {
                args.extend(os_args(["-bsf:a", "aac_adtstoasc"]));
            }
        } else {
            args.extend(os_args([
                "-c:a".to_owned(),
                "aac".to_owned(),
                "-ac".to_owned(),
                "2".to_owned(),
                "-ar".to_owned(),
                "48000".to_owned(),
                "-b:a".to_owned(),
                format!("{audio_kbps}k"),
            ]));
        }
    }

    args.extend(os_args([
        "-avoid_negative_ts",
        "make_zero",
        "-f",
        "hls",
        "-hls_time",
        &SEGMENT_SECONDS.to_string(),
        "-hls_list_size",
        "0",
        "-hls_playlist_type",
        "event",
        "-hls_segment_type",
        "fmp4",
        "-hls_fmp4_init_filename",
        INIT_SEGMENT_FILE,
        "-hls_flags",
        "independent_segments+temp_file",
        "-hls_segment_filename",
    ]));
    args.push(output_dir.join("seg%05d.m4s").into_os_string());
    args.push(output_dir.join(PLAYLIST_FILE).into_os_string());
    args
}

/// Validates a single-segment HLS file name requested from `/segment/{file}`.
pub fn is_valid_segment_name(name: &str) -> bool {
    if name.is_empty() || name.len() > 64 || name.starts_with('.') || name.contains("..") {
        return false;
    }
    let valid_ext = name.ends_with(".m4s") || name.ends_with(".mp4") || name.ends_with(".ts");
    valid_ext
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
}

pub fn segment_content_type(name: &str) -> &'static str {
    if name.ends_with(".ts") {
        "video/mp2t"
    } else {
        "video/mp4"
    }
}

/// Rewrites ffmpeg's local playlist URIs to `segment/<file>?ticket=<ticket>` (or `segment/<file>`).
pub fn rewrite_playlist(raw: &str, ticket: Option<&str>) -> String {
    let suffix = ticket.map_or_else(String::new, |t| format!("?ticket={t}"));
    let mut out = String::with_capacity(raw.len() + 256);
    for line in raw.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("#EXT-X-MAP:") {
            out.push_str("#EXT-X-MAP:");
            if let Some((before, after_uri)) = rest.split_once("URI=\"")
                && let Some((name, tail)) = after_uri.split_once('"')
            {
                out.push_str(before);
                out.push_str("URI=\"segment/");
                out.push_str(name);
                out.push_str(&suffix);
                out.push('"');
                out.push_str(tail);
            } else {
                out.push_str(rest);
            }
        } else if trimmed.is_empty() || trimmed.starts_with('#') {
            out.push_str(trimmed);
        } else {
            out.push_str("segment/");
            out.push_str(trimmed);
            out.push_str(&suffix);
        }
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use palveni_core::clock;

    use super::*;
    use crate::report::{CpuInfo, EncoderCapability, ToolInfo};

    fn empty_report(encoders: Vec<EncoderCapability>) -> HardwareReport {
        HardwareReport {
            perf_tier: PerfTier::Standard,
            cpu: CpuInfo {
                model: "test".into(),
                cores: 4,
                arch: "aarch64".into(),
            },
            memory_bytes: 8 * 1024 * 1024 * 1024,
            ffmpeg: ToolInfo {
                found: true,
                path: None,
                version: None,
                source: None,
            },
            ffprobe: ToolInfo {
                found: true,
                path: None,
                version: None,
                source: None,
            },
            hwaccels: vec![],
            encoders,
            probed_at: clock::now(),
        }
    }

    fn cap(id: &str, codec: &str, method: EncodeMethod, verified: bool) -> EncoderCapability {
        EncoderCapability {
            id: id.into(),
            codec: codec.into(),
            method,
            listed: true,
            verified,
            error: None,
        }
    }

    #[test]
    fn prefers_verified_hardware_h264_over_software() {
        let report = empty_report(vec![
            cap("libx264", "h264", EncodeMethod::Software, true),
            cap(
                "hevc_videotoolbox",
                "hevc",
                EncodeMethod::Videotoolbox,
                true,
            ),
            cap(
                "h264_videotoolbox",
                "h264",
                EncodeMethod::Videotoolbox,
                false,
            ),
            cap("h264_vaapi", "h264", EncodeMethod::Vaapi, true),
        ]);
        let selected = select_h264_encoder(Some(&report));
        assert_eq!(selected.id, "h264_vaapi");
        assert_eq!(selected.method, EncodeMethod::Vaapi);

        let fallback = select_h264_encoder(Some(&empty_report(vec![cap(
            "h264_videotoolbox",
            "h264",
            EncodeMethod::Videotoolbox,
            false,
        )])));
        assert_eq!(fallback, SelectedEncoder::software());
        assert_eq!(select_h264_encoder(None), SelectedEncoder::software());
    }

    #[test]
    fn validates_segment_names_against_traversal() {
        assert!(is_valid_segment_name("init.mp4"));
        assert!(is_valid_segment_name("seg00000.m4s"));
        assert!(is_valid_segment_name("seg00001.ts"));
        for bad in [
            "",
            ".hidden.m4s",
            "../init.mp4",
            "a/b.m4s",
            "a\\b.m4s",
            "seg00000.m4s.tmp",
            "index.m3u8",
            "seg 0.m4s",
        ] {
            assert!(!is_valid_segment_name(bad), "{bad}");
        }
    }

    #[test]
    fn rewrites_fmp4_playlist_with_ticket() {
        let raw = "#EXTM3U\n#EXT-X-VERSION:7\n#EXT-X-PLAYLIST-TYPE:EVENT\n#EXT-X-MAP:URI=\"init.mp4\"\n#EXTINF:4.000000,\nseg00000.m4s\n#EXTINF:4.000000,\nseg00001.m4s\n";
        let rewritten = rewrite_playlist(raw, Some("tok.123.abc"));
        assert!(rewritten.contains("#EXT-X-MAP:URI=\"segment/init.mp4?ticket=tok.123.abc\""));
        assert!(rewritten.contains("\nsegment/seg00000.m4s?ticket=tok.123.abc\n"));
        assert!(rewritten.contains("\nsegment/seg00001.m4s?ticket=tok.123.abc\n"));

        let no_ticket = rewrite_playlist(raw, None);
        assert!(no_ticket.contains("#EXT-X-MAP:URI=\"segment/init.mp4\""));
        assert!(no_ticket.contains("\nsegment/seg00000.m4s\n"));
    }

    #[test]
    fn builds_remux_and_transcode_args() {
        let dir = Path::new("/tmp/session");
        let remux = build_hls_args(
            &HlsPlan {
                mode: DeliveryMode::Remux,
                video_index: 0,
                video_codec: Some("hevc".into()),
                video_width: 3840,
                video_height: 2160,
                frame_rate: Some(24.0),
                container: Some("mkv".into()),
                audio_index: Some(2),
                copy_audio: false,
                scale_to: None,
                max_bitrate_kbps: None,
                start_ms: 4_000,
                tier: PerfTier::Standard,
                encoder: SelectedEncoder::software(),
                render_node: None,
                use_readrate_burst: true,
            },
            dir,
        );
        let joined: Vec<String> = remux
            .into_iter()
            .map(|s| s.into_string().unwrap())
            .collect();
        let line = joined.join(" ");
        assert!(line.contains("-noaccurate_seek -ss 4000ms -i"));
        assert!(line.contains("-map 0:0 -map 0:2"));
        assert!(line.contains("-c:v copy -tag:v hvc1"));
        assert!(line.contains("-c:a aac -ac 2 -ar 48000 -b:a 192k"));
        assert!(line.contains("-hls_playlist_type event -hls_segment_type fmp4"));

        let transcode = build_hls_args(
            &HlsPlan {
                mode: DeliveryMode::Transcode,
                video_index: 0,
                video_codec: Some("hevc".into()),
                video_width: 1920,
                video_height: 1080,
                frame_rate: Some(23.976),
                container: Some("mp4".into()),
                audio_index: Some(1),
                copy_audio: true,
                scale_to: Some(720),
                max_bitrate_kbps: Some(2_500),
                start_ms: 10_000,
                tier: PerfTier::Lite,
                encoder: SelectedEncoder {
                    id: "h264_vaapi".into(),
                    method: EncodeMethod::Vaapi,
                },
                render_node: Some(PathBuf::from("/dev/dri/renderD128")),
                use_readrate_burst: false,
            },
            dir,
        );
        let joined: Vec<String> = transcode
            .into_iter()
            .map(|s| s.into_string().unwrap())
            .collect();
        let line = joined.join(" ");
        assert!(line.contains("-init_hw_device vaapi=va:/dev/dri/renderD128"));
        assert!(line.contains("-vf scale=-2:720,format=nv12,hwupload"));
        assert!(line.contains("-c:v h264_vaapi"));
        assert!(line.contains("-maxrate 2308k"));
        assert!(line.contains("-c:a copy"));
    }
}

//! Encoder candidates and runtime verification with short test encodes.

use std::ffi::OsString;
use std::path::Path;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::devices::DeviceNodes;
use crate::parse::ListedEncoder;
use crate::process::{os_args, run, summarize_stderr};

#[derive(
    utoipa::ToSchema,
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
)]
#[serde(rename_all = "lowercase")]
pub enum EncodeMethod {
    Software,
    Vaapi,
    Qsv,
    Nvenc,
    Amf,
    Rkmpp,
    V4l2m2m,
    Videotoolbox,
}

impl EncodeMethod {
    pub const fn as_str(self) -> &'static str {
        match self {
            EncodeMethod::Software => "software",
            EncodeMethod::Vaapi => "vaapi",
            EncodeMethod::Qsv => "qsv",
            EncodeMethod::Nvenc => "nvenc",
            EncodeMethod::Amf => "amf",
            EncodeMethod::Rkmpp => "rkmpp",
            EncodeMethod::V4l2m2m => "v4l2m2m",
            EncodeMethod::Videotoolbox => "videotoolbox",
        }
    }

    /// Discrete-GPU class encoders; used for the `performance` tier.
    pub const fn is_dedicated_gpu(self) -> bool {
        matches!(self, EncodeMethod::Nvenc | EncodeMethod::Amf)
    }
}

/// Video codecs the server will transcode to.
pub const TARGET_CODECS: [&str; 3] = ["h264", "hevc", "av1"];

/// Software fallbacks, always reported (as unlisted if missing).
pub const SOFTWARE_ENCODERS: [(&str, &str); 3] = [
    ("libx264", "h264"),
    ("libx265", "hevc"),
    ("libsvtav1", "av1"),
];

/// Hardware method from an encoder name suffix, e.g. `hevc_vaapi`.
pub fn hardware_method(encoder: &str) -> Option<EncodeMethod> {
    let (_, suffix) = encoder.rsplit_once('_')?;
    match suffix {
        "vaapi" => Some(EncodeMethod::Vaapi),
        "qsv" => Some(EncodeMethod::Qsv),
        "nvenc" => Some(EncodeMethod::Nvenc),
        "amf" => Some(EncodeMethod::Amf),
        "rkmpp" => Some(EncodeMethod::Rkmpp),
        "v4l2m2m" => Some(EncodeMethod::V4l2m2m),
        "videotoolbox" => Some(EncodeMethod::Videotoolbox),
        _ => None,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Candidate {
    pub id: String,
    pub codec: String,
    pub method: EncodeMethod,
    pub listed: bool,
}

/// Encoders worth testing: every listed hardware encoder for a target
/// codec, plus the software encoders (listed or not).
pub fn candidates(listed: &[ListedEncoder]) -> Vec<Candidate> {
    let mut out: Vec<Candidate> = SOFTWARE_ENCODERS
        .iter()
        .map(|(name, codec)| Candidate {
            id: (*name).to_owned(),
            codec: (*codec).to_owned(),
            method: EncodeMethod::Software,
            listed: listed.iter().any(|e| e.name == *name),
        })
        .collect();
    for encoder in listed {
        if encoder.media_type != 'V' || !TARGET_CODECS.contains(&encoder.codec.as_str()) {
            continue;
        }
        if let Some(method) = hardware_method(&encoder.name) {
            out.push(Candidate {
                id: encoder.name.clone(),
                codec: encoder.codec.clone(),
                method,
                listed: true,
            });
        }
    }
    out.sort_by(|a, b| {
        (a.method, codec_rank(&a.codec), &a.id).cmp(&(b.method, codec_rank(&b.codec), &b.id))
    });
    out
}

fn codec_rank(codec: &str) -> usize {
    TARGET_CODECS
        .iter()
        .position(|c| *c == codec)
        .unwrap_or(TARGET_CODECS.len())
}

/// Why a candidate cannot be tested on this machine, if it cannot.
pub fn missing_prerequisite(candidate: &Candidate, devices: &DeviceNodes) -> Option<String> {
    if !candidate.listed {
        return Some("not included in this ffmpeg build".to_owned());
    }
    if !cfg!(target_os = "linux") {
        return None;
    }
    match candidate.method {
        EncodeMethod::Vaapi | EncodeMethod::Qsv => {
            if devices.dri_render.is_empty() {
                Some("no /dev/dri render node (GPU not passed through?)".to_owned())
            } else if devices.usable_render_node().is_none() {
                Some("/dev/dri render node exists but is not accessible (check the render/video group)".to_owned())
            } else {
                None
            }
        }
        EncodeMethod::Nvenc if devices.nvidia.is_empty() => {
            Some("no /dev/nvidia* device nodes".to_owned())
        }
        EncodeMethod::Rkmpp if devices.rockchip.is_empty() => {
            Some("no /dev/mpp_service device node".to_owned())
        }
        EncodeMethod::V4l2m2m if devices.v4l2.is_empty() => {
            Some("no /dev/video* device nodes".to_owned())
        }
        _ => None,
    }
}

/// ffmpeg arguments for a short synthetic encode with `candidate`.
///
/// 640x360 at 15 frames is enough to open the encoder and push real frames
/// through it while staying cheap on the weakest supported hardware.
pub fn test_encode_args(candidate: &Candidate, render_node: Option<&Path>) -> Vec<OsString> {
    let mut args = os_args(["-hide_banner", "-nostdin", "-loglevel", "error"]);
    if candidate.method == EncodeMethod::Vaapi {
        let node = render_node.map_or_else(
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
    args.extend(os_args([
        "-f",
        "lavfi",
        "-i",
        "testsrc2=size=640x360:rate=30",
        "-frames:v",
        "15",
    ]));
    let filter = match candidate.method {
        EncodeMethod::Vaapi => "format=nv12,hwupload",
        EncodeMethod::Qsv
        | EncodeMethod::Amf
        | EncodeMethod::Rkmpp
        | EncodeMethod::Videotoolbox => "format=nv12",
        EncodeMethod::Software | EncodeMethod::Nvenc | EncodeMethod::V4l2m2m => "format=yuv420p",
    };
    args.extend(os_args(["-vf", filter]));
    args.extend(os_args(["-c:v", candidate.id.as_str(), "-f", "null", "-"]));
    args
}

/// Runs the test encode. `Ok(())` means frames were actually encoded.
pub async fn verify(
    ffmpeg: &Path,
    candidate: &Candidate,
    devices: &DeviceNodes,
    timeout: Duration,
) -> Result<(), String> {
    if let Some(reason) = missing_prerequisite(candidate, devices) {
        return Err(reason);
    }
    let args = test_encode_args(candidate, devices.usable_render_node());
    match run(ffmpeg, &args, timeout).await {
        Ok(output) if output.success => Ok(()),
        Ok(output) => Err(summarize_stderr(&output.stderr)
            .unwrap_or_else(|| format!("ffmpeg exited with status {}", output.code.unwrap_or(-1)))),
        Err(err) => Err(err.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse_encoders;

    const ENCODERS: &str = include_str!("../fixtures/encoders.txt");

    #[test]
    fn methods_from_names() {
        assert_eq!(hardware_method("h264_vaapi"), Some(EncodeMethod::Vaapi));
        assert_eq!(hardware_method("hevc_v4l2m2m"), Some(EncodeMethod::V4l2m2m));
        assert_eq!(hardware_method("libx264"), None);
        assert_eq!(hardware_method("libfdk_aac"), None);
        assert_eq!(
            serde_json::to_string(&EncodeMethod::V4l2m2m).unwrap(),
            "\"v4l2m2m\""
        );
        assert_eq!(
            serde_json::to_string(&EncodeMethod::Videotoolbox).unwrap(),
            "\"videotoolbox\""
        );
    }

    #[test]
    fn candidate_matrix_from_listing() {
        let listed = parse_encoders(ENCODERS);
        let candidates = candidates(&listed);
        let ids: Vec<&str> = candidates.iter().map(|c| c.id.as_str()).collect();
        // Software first, in codec order.
        assert_eq!(&ids[..3], &["libx264", "libx265", "libsvtav1"]);
        assert!(candidates[..3].iter().all(|c| c.listed));
        // Hardware video encoders for target codecs only.
        for expected in [
            "h264_vaapi",
            "hevc_vaapi",
            "av1_vaapi",
            "h264_qsv",
            "av1_nvenc",
            "h264_amf",
            "h264_v4l2m2m",
        ] {
            assert!(ids.contains(&expected), "{expected}");
        }
        for excluded in ["mjpeg_vaapi", "vp9_qsv", "libx264rgb", "aac", "libvpx-vp9"] {
            assert!(!ids.contains(&excluded), "{excluded}");
        }
    }

    #[test]
    fn missing_software_encoders_are_reported_unlisted() {
        let candidates = candidates(&[]);
        assert_eq!(candidates.len(), 3);
        assert!(candidates.iter().all(|c| !c.listed));
        assert!(missing_prerequisite(&candidates[0], &DeviceNodes::default()).is_some());
    }

    #[test]
    fn vaapi_args_initialise_the_device() {
        let candidate = Candidate {
            id: "hevc_vaapi".into(),
            codec: "hevc".into(),
            method: EncodeMethod::Vaapi,
            listed: true,
        };
        let args: Vec<String> =
            test_encode_args(&candidate, Some(Path::new("/dev/dri/renderD129")))
                .into_iter()
                .map(|a| a.into_string().unwrap())
                .collect();
        let joined = args.join(" ");
        assert!(joined.contains("-init_hw_device vaapi=va:/dev/dri/renderD129"));
        assert!(joined.contains("-vf format=nv12,hwupload"));
        assert!(joined.ends_with("-c:v hevc_vaapi -f null -"));
        // Device init must precede the input.
        let init = args.iter().position(|a| a == "-init_hw_device").unwrap();
        let input = args.iter().position(|a| a == "-i").unwrap();
        assert!(init < input);
    }
}

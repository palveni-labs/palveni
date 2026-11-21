//! `ffprobe` wrapper.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::Path;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::process::{RunError, os_args, run, summarize_stderr};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ProbeResult {
    #[serde(default)]
    pub format: Option<ProbeFormat>,
    #[serde(default)]
    pub streams: Vec<ProbeStream>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ProbeFormat {
    #[serde(default)]
    pub format_name: Option<String>,
    /// Seconds as a decimal string, as ffprobe prints it.
    #[serde(default)]
    pub duration: Option<String>,
    #[serde(default)]
    pub size: Option<String>,
    #[serde(default)]
    pub bit_rate: Option<String>,
    #[serde(default)]
    pub tags: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ProbeStream {
    #[serde(default)]
    pub index: Option<i64>,
    #[serde(default)]
    pub codec_type: Option<String>,
    #[serde(default)]
    pub codec_name: Option<String>,
    #[serde(default)]
    pub profile: Option<String>,
    #[serde(default)]
    pub width: Option<i64>,
    #[serde(default)]
    pub height: Option<i64>,
    #[serde(default)]
    pub pix_fmt: Option<String>,
    #[serde(default)]
    pub color_transfer: Option<String>,
    #[serde(default)]
    pub sample_rate: Option<String>,
    #[serde(default)]
    pub channels: Option<i64>,
    #[serde(default)]
    pub channel_layout: Option<String>,
    #[serde(default)]
    pub bit_rate: Option<String>,
    #[serde(default)]
    pub duration: Option<String>,
    #[serde(default)]
    pub tags: BTreeMap<String, String>,
}

impl ProbeFormat {
    pub fn duration_seconds(&self) -> Option<f64> {
        self.duration.as_deref()?.trim().parse().ok()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ProbeError {
    #[error("path must be absolute: {0}")]
    RelativePath(String),
    #[error(transparent)]
    Run(#[from] RunError),
    #[error("ffprobe failed: {0}")]
    Failed(String),
    #[error("cannot parse ffprobe output: {0}")]
    Parse(String),
}

pub fn parse_probe_json(text: &str) -> Result<ProbeResult, ProbeError> {
    serde_json::from_str(text).map_err(|e| ProbeError::Parse(e.to_string()))
}

/// Probes a local media file.
///
/// The path is passed as `file:<absolute path>` so names that look like
/// options or protocol URLs (`-x.mkv`, `http:…`, `concat:…`) are never
/// interpreted as such, and only the `file` protocol is whitelisted so a
/// playlist cannot make ffprobe fetch remote or other local resources.
pub async fn probe(
    ffprobe: &Path,
    file: &Path,
    timeout: Duration,
) -> Result<ProbeResult, ProbeError> {
    if !file.is_absolute() {
        return Err(ProbeError::RelativePath(file.display().to_string()));
    }
    let mut input = OsString::from("file:");
    input.push(file.as_os_str());
    let mut args = os_args([
        "-v",
        "error",
        "-print_format",
        "json",
        "-show_format",
        "-show_streams",
        "-protocol_whitelist",
        "file",
        "-i",
    ]);
    args.push(input);
    let output = run(ffprobe, &args, timeout).await?;
    if !output.success {
        return Err(ProbeError::Failed(
            summarize_stderr(&output.stderr).unwrap_or_else(|| "unknown error".to_owned()),
        ));
    }
    parse_probe_json(&output.stdout)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MKV: &str = include_str!("../fixtures/ffprobe_mkv.json");

    #[test]
    fn parses_realistic_output() {
        let result = parse_probe_json(MKV).unwrap();
        let format = result.format.as_ref().unwrap();
        assert_eq!(format.format_name.as_deref(), Some("matroska,webm"));
        assert_eq!(format.duration_seconds(), Some(7083.076));
        assert_eq!(
            format.tags.get("title").map(String::as_str),
            Some("Example")
        );
        assert_eq!(result.streams.len(), 3);
        let video = &result.streams[0];
        assert_eq!(video.codec_type.as_deref(), Some("video"));
        assert_eq!(video.codec_name.as_deref(), Some("hevc"));
        assert_eq!(video.width, Some(3840));
        assert_eq!(video.color_transfer.as_deref(), Some("smpte2084"));
        let audio = &result.streams[1];
        assert_eq!(audio.channels, Some(6));
        assert_eq!(audio.tags.get("language").map(String::as_str), Some("eng"));
        assert_eq!(
            result.streams[2].tags.get("title").map(String::as_str),
            Some("简体中文")
        );
    }

    #[test]
    fn tolerates_empty_output() {
        let result = parse_probe_json("{}").unwrap();
        assert!(result.format.is_none());
        assert!(result.streams.is_empty());
        assert!(parse_probe_json("not json").is_err());
    }

    #[tokio::test]
    async fn rejects_relative_paths() {
        let err = probe(
            Path::new("/usr/bin/ffprobe"),
            Path::new("movie.mkv"),
            Duration::from_secs(1),
        )
        .await
        .unwrap_err();
        assert!(matches!(err, ProbeError::RelativePath(_)));
    }
}

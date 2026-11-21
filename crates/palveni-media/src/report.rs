//! The hardware/ffmpeg capability report (`GET /system/hardware`).

use std::path::PathBuf;
use std::time::Duration;

use chrono::{DateTime, Utc};
use palveni_core::{HostInfo, PerfTier, clock};
use serde::{Deserialize, Serialize};

use crate::devices::DeviceNodes;
use crate::discovery::{LocatedTool, ToolSource, locate};
use crate::encoders::{EncodeMethod, candidates, verify};
use crate::parse::{parse_encoders, parse_hwaccels, parse_version};
use crate::process::{os_args, run};

const QUERY_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolInfo {
    pub found: bool,
    #[schema(required = true)]
    pub path: Option<String>,
    #[schema(required = true)]
    pub version: Option<String>,
    #[schema(required = true)]
    pub source: Option<ToolSource>,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CpuInfo {
    pub model: String,
    pub cores: u32,
    pub arch: String,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EncoderCapability {
    /// ffmpeg encoder name, e.g. `h264_vaapi`, `libx264`.
    pub id: String,
    /// `h264`, `hevc`, `av1`.
    pub codec: String,
    pub method: EncodeMethod,
    /// Present in `ffmpeg -encoders`.
    pub listed: bool,
    /// A real test encode succeeded.
    pub verified: bool,
    /// Why the test encode failed or was skipped.
    #[schema(required = true)]
    pub error: Option<String>,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HardwareReport {
    pub perf_tier: PerfTier,
    pub cpu: CpuInfo,
    pub memory_bytes: u64,
    pub ffmpeg: ToolInfo,
    pub ffprobe: ToolInfo,
    pub hwaccels: Vec<String>,
    pub encoders: Vec<EncoderCapability>,
    pub probed_at: DateTime<Utc>,
}

impl HardwareReport {
    pub fn has_verified(&self, method: EncodeMethod) -> bool {
        self.encoders
            .iter()
            .any(|e| e.method == method && e.verified)
    }
}

#[derive(Clone, Debug)]
pub struct ProbeOptions {
    /// `ffmpeg_path` from the configuration.
    pub ffmpeg_path: Option<PathBuf>,
    pub perf_tier_override: Option<PerfTier>,
    /// Per test encode.
    pub encode_timeout: Duration,
}

impl Default for ProbeOptions {
    fn default() -> Self {
        ProbeOptions {
            ffmpeg_path: None,
            perf_tier_override: None,
            encode_timeout: Duration::from_secs(20),
        }
    }
}

/// Everything the probe found, including details the HTTP report omits.
#[derive(Clone, Debug)]
pub struct ProbeDetails {
    pub report: HardwareReport,
    pub devices: DeviceNodes,
    pub ffmpeg_path: Option<PathBuf>,
    pub ffprobe_path: Option<PathBuf>,
}

/// Runs the full probe. Test encodes run one after another so a weak
/// machine is never asked to run several encoders at once.
pub async fn probe_hardware(options: &ProbeOptions) -> HardwareReport {
    probe_hardware_detailed(options).await.report
}

pub async fn probe_hardware_detailed(options: &ProbeOptions) -> ProbeDetails {
    let configured = options.ffmpeg_path.clone();
    let blocking = tokio::task::spawn_blocking(move || {
        let host = HostInfo::detect();
        let devices = DeviceNodes::scan();
        let ffmpeg = locate("ffmpeg", configured.as_deref());
        let ffprobe = locate("ffprobe", configured.as_deref());
        (host, devices, ffmpeg, ffprobe)
    })
    .await;
    let (host, devices, ffmpeg, ffprobe) = match blocking {
        Ok(found) => found,
        Err(err) => {
            tracing::error!(error = %err, "hardware detection task failed");
            (HostInfo::detect(), DeviceNodes::default(), None, None)
        }
    };

    let ffmpeg_info = tool_info(ffmpeg.as_ref()).await;
    let ffprobe_info = tool_info(ffprobe.as_ref()).await;

    let mut hwaccels = Vec::new();
    let mut encoders = Vec::new();
    if let Some(tool) = &ffmpeg {
        if let Some(text) = query(tool, &["-hide_banner", "-hwaccels"]).await {
            hwaccels = parse_hwaccels(&text);
        }
        let listed = match query(tool, &["-hide_banner", "-encoders"]).await {
            Some(text) => parse_encoders(&text),
            None => Vec::new(),
        };
        for candidate in candidates(&listed) {
            let result = verify(&tool.path, &candidate, &devices, options.encode_timeout).await;
            if let Err(reason) = &result {
                tracing::debug!(encoder = %candidate.id, %reason, "encoder not usable");
            }
            encoders.push(EncoderCapability {
                id: candidate.id,
                codec: candidate.codec,
                method: candidate.method,
                listed: candidate.listed,
                verified: result.is_ok(),
                error: result.err(),
            });
        }
    }

    let dedicated = encoders
        .iter()
        .any(|e| e.verified && e.method.is_dedicated_gpu());
    let perf_tier = options
        .perf_tier_override
        .unwrap_or_else(|| PerfTier::detect(host.memory_bytes, host.cores, dedicated));

    ProbeDetails {
        report: HardwareReport {
            perf_tier,
            cpu: CpuInfo {
                model: host.cpu_model.clone(),
                cores: u32::try_from(host.cores).unwrap_or(u32::MAX),
                arch: host.arch.clone(),
            },
            memory_bytes: host.memory_bytes,
            ffmpeg: ffmpeg_info,
            ffprobe: ffprobe_info,
            hwaccels,
            encoders,
            probed_at: clock::now(),
        },
        devices,
        ffmpeg_path: ffmpeg.map(|t| t.path),
        ffprobe_path: ffprobe.map(|t| t.path),
    }
}

async fn tool_info(tool: Option<&LocatedTool>) -> ToolInfo {
    let Some(tool) = tool else {
        return ToolInfo {
            found: false,
            path: None,
            version: None,
            source: None,
        };
    };
    let version = query(tool, &["-version"])
        .await
        .and_then(|text| parse_version(&text));
    ToolInfo {
        found: true,
        path: Some(tool.path.display().to_string()),
        version,
        source: Some(tool.source),
    }
}

/// Runs a listing command and returns stdout on success.
async fn query(tool: &LocatedTool, args: &[&str]) -> Option<String> {
    let args = os_args(args.iter().copied());
    match run(&tool.path, &args, QUERY_TIMEOUT).await {
        Ok(output) if output.success => Some(output.stdout),
        Ok(output) => {
            tracing::warn!(tool = %tool.path.display(), ?args, code = ?output.code, "ffmpeg query failed");
            None
        }
        Err(err) => {
            tracing::warn!(tool = %tool.path.display(), error = %err, "ffmpeg query failed");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_json_matches_contract_shape() {
        let report = HardwareReport {
            perf_tier: PerfTier::Standard,
            cpu: CpuInfo {
                model: "Intel(R) N100".into(),
                cores: 4,
                arch: "x86_64".into(),
            },
            memory_bytes: 16 * 1024 * 1024 * 1024,
            ffmpeg: ToolInfo {
                found: true,
                path: Some("/usr/lib/jellyfin-ffmpeg/ffmpeg".into()),
                version: Some("7.1.1-Jellyfin".into()),
                source: Some(ToolSource::Bundled),
            },
            ffprobe: ToolInfo {
                found: false,
                path: None,
                version: None,
                source: None,
            },
            hwaccels: vec!["vaapi".into(), "qsv".into()],
            encoders: vec![EncoderCapability {
                id: "h264_vaapi".into(),
                codec: "h264".into(),
                method: EncodeMethod::Vaapi,
                listed: true,
                verified: false,
                error: Some("no /dev/dri render node".into()),
            }],
            probed_at: clock::now(),
        };
        let value = serde_json::to_value(&report).unwrap();
        let obj = value.as_object().unwrap();
        for key in [
            "perfTier",
            "cpu",
            "memoryBytes",
            "ffmpeg",
            "ffprobe",
            "hwaccels",
            "encoders",
            "probedAt",
        ] {
            assert!(obj.contains_key(key), "{key}");
        }
        assert_eq!(value["ffmpeg"]["source"], "bundled");
        assert!(value["ffprobe"]["path"].is_null());
        assert!(value["ffprobe"]["source"].is_null());
        assert_eq!(value["encoders"][0]["method"], "vaapi");
        assert_eq!(value["perfTier"], "standard");
        let back: HardwareReport = serde_json::from_value(value).unwrap();
        assert_eq!(back, report);
    }
}

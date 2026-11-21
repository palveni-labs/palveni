//! Host detection and performance tiers.
//!
//! The tier decides feature defaults and concurrency. It is derived from
//! memory and core count, bumped to `performance` only when a dedicated GPU
//! encoder has actually been verified, and can always be overridden by the
//! administrator.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

const MIB: u64 = 1024 * 1024;
const GIB: u64 = 1024 * MIB;

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
pub enum PerfTier {
    Tiny,
    Lite,
    Standard,
    Performance,
}

impl PerfTier {
    /// Picks a tier from host resources.
    ///
    /// `memory_bytes == 0` means "unknown" and only the core count is used.
    /// Thresholds sit slightly below the nominal sizes because the kernel and
    /// firmware reserve memory (an "8 GB" box reports roughly 7.6 GiB).
    pub fn detect(memory_bytes: u64, cores: usize, has_dedicated_gpu_encoder: bool) -> Self {
        let mem_known = memory_bytes > 0;
        if cores <= 1 || (mem_known && memory_bytes < 1536 * MIB) {
            PerfTier::Tiny
        } else if cores < 4 || (mem_known && memory_bytes < 6 * GIB) {
            PerfTier::Lite
        } else if has_dedicated_gpu_encoder
            && cores >= 8
            && (!mem_known || memory_bytes >= 15 * GIB)
        {
            PerfTier::Performance
        } else {
            PerfTier::Standard
        }
    }

    /// Default number of background jobs that may run at the same time.
    pub const fn job_concurrency(self) -> usize {
        match self {
            PerfTier::Tiny | PerfTier::Lite => 1,
            PerfTier::Standard => 2,
            PerfTier::Performance => 4,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            PerfTier::Tiny => "tiny",
            PerfTier::Lite => "lite",
            PerfTier::Standard => "standard",
            PerfTier::Performance => "performance",
        }
    }
}

impl fmt::Display for PerfTier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for PerfTier {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "tiny" => Ok(PerfTier::Tiny),
            "lite" => Ok(PerfTier::Lite),
            "standard" => Ok(PerfTier::Standard),
            "performance" => Ok(PerfTier::Performance),
            other => Err(crate::Error::invalid(format!(
                "unknown performance tier `{other}` (expected tiny, lite, standard or performance)"
            ))),
        }
    }
}

/// Basic facts about the machine the server runs on.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostInfo {
    pub cpu_model: String,
    pub cores: usize,
    /// Rust target architecture name, e.g. `x86_64`, `aarch64`, `arm`.
    pub arch: String,
    /// Usable memory in bytes (container limit if lower); 0 when unknown.
    pub memory_bytes: u64,
}

impl HostInfo {
    /// Detects host facts. Performs small blocking reads (and on macOS a
    /// `sysctl` call), so call it from a blocking context.
    pub fn detect() -> Self {
        let cores = std::thread::available_parallelism()
            .map(std::num::NonZeroUsize::get)
            .unwrap_or(1);
        let (cpu_model, memory_bytes) = platform::detect();
        HostInfo {
            cpu_model: cpu_model.unwrap_or_else(|| "unknown".to_owned()),
            cores,
            arch: std::env::consts::ARCH.to_owned(),
            memory_bytes: memory_bytes.unwrap_or(0),
        }
    }
}

/// Parses `MemTotal` from `/proc/meminfo` (value is in KiB).
pub fn parse_meminfo_total(meminfo: &str) -> Option<u64> {
    meminfo.lines().find_map(|line| {
        let rest = line.strip_prefix("MemTotal:")?;
        let kib: u64 = rest.split_whitespace().next()?.parse().ok()?;
        kib.checked_mul(1024)
    })
}

/// Parses a cgroup memory limit file (`memory.max` or
/// `memory.limit_in_bytes`). `max` and absurdly large values mean "no limit".
pub fn parse_cgroup_limit(contents: &str) -> Option<u64> {
    let value = contents.trim();
    if value == "max" {
        return None;
    }
    let bytes: u64 = value.parse().ok()?;
    // cgroup v1 reports "unlimited" as a huge page-aligned number.
    if bytes >= (1u64 << 60) {
        None
    } else {
        Some(bytes)
    }
}

/// Extracts a human-readable CPU name from `/proc/cpuinfo`.
pub fn parse_cpuinfo_model(cpuinfo: &str) -> Option<String> {
    const KEYS: [&str; 4] = ["model name", "Model", "Hardware", "cpu model"];
    for key in KEYS {
        let found = cpuinfo.lines().find_map(|line| {
            let (k, v) = line.split_once(':')?;
            if k.trim() == key {
                let v = v.trim();
                (!v.is_empty()).then(|| v.to_owned())
            } else {
                None
            }
        });
        if found.is_some() {
            return found;
        }
    }
    None
}

/// Names an ARM core from the `CPU implementer` / `CPU part` fields that
/// aarch64 kernels print instead of a model name (cloud VMs and boards
/// without a device-tree model). Unknown parts are reported by number.
pub fn parse_cpuinfo_arm(cpuinfo: &str) -> Option<String> {
    let field = |key: &str| {
        cpuinfo.lines().find_map(|line| {
            let (k, v) = line.split_once(':')?;
            let v = v.trim();
            (k.trim() == key).then(|| v.trim_start_matches("0x").to_ascii_lowercase())
        })
    };
    let implementer = u32::from_str_radix(&field("CPU implementer")?, 16).ok()?;
    let part = u32::from_str_radix(&field("CPU part")?, 16).ok()?;
    let known = match (implementer, part) {
        (0x41, 0xd03) => Some("ARM Cortex-A53"),
        (0x41, 0xd04) => Some("ARM Cortex-A35"),
        (0x41, 0xd05) => Some("ARM Cortex-A55"),
        (0x41, 0xd07) => Some("ARM Cortex-A57"),
        (0x41, 0xd08) => Some("ARM Cortex-A72"),
        (0x41, 0xd09) => Some("ARM Cortex-A73"),
        (0x41, 0xd0a) => Some("ARM Cortex-A75"),
        (0x41, 0xd0b) => Some("ARM Cortex-A76"),
        (0x41, 0xd0c) => Some("ARM Neoverse-N1"),
        (0x41, 0xd0d) => Some("ARM Cortex-A77"),
        (0x41, 0xd40) => Some("ARM Neoverse-V1"),
        (0x41, 0xd41) => Some("ARM Cortex-A78"),
        (0x41, 0xd46) => Some("ARM Cortex-A510"),
        (0x41, 0xd47) => Some("ARM Cortex-A710"),
        (0x41, 0xd49) => Some("ARM Neoverse-N2"),
        (0x41, 0xd4f) => Some("ARM Neoverse-V2"),
        (0x41, 0xd80) => Some("ARM Cortex-A520"),
        (0x41, 0xd81) => Some("ARM Cortex-A720"),
        _ => None,
    };
    Some(match known {
        Some(name) => name.to_owned(),
        None => format!("ARM implementer 0x{implementer:02x} part 0x{part:03x}"),
    })
}

#[cfg(target_os = "linux")]
mod platform {
    use super::{parse_cgroup_limit, parse_cpuinfo_arm, parse_cpuinfo_model, parse_meminfo_total};

    pub(super) fn detect() -> (Option<String>, Option<u64>) {
        let cpuinfo = std::fs::read_to_string("/proc/cpuinfo").ok();
        let cpu = cpuinfo
            .as_deref()
            .and_then(parse_cpuinfo_model)
            .or_else(device_tree_model)
            .or_else(|| cpuinfo.as_deref().and_then(parse_cpuinfo_arm));

        let total = std::fs::read_to_string("/proc/meminfo")
            .ok()
            .and_then(|text| parse_meminfo_total(&text));
        let limit = [
            "/sys/fs/cgroup/memory.max",
            "/sys/fs/cgroup/memory/memory.limit_in_bytes",
        ]
        .iter()
        .find_map(|path| {
            std::fs::read_to_string(path)
                .ok()
                .and_then(|text| parse_cgroup_limit(&text))
        });
        let memory = match (total, limit) {
            (Some(t), Some(l)) => Some(t.min(l)),
            (t, l) => t.or(l),
        };
        (cpu, memory)
    }

    /// ARM boards often have no model line in cpuinfo but expose the board
    /// name through the device tree.
    fn device_tree_model() -> Option<String> {
        let raw = std::fs::read("/sys/firmware/devicetree/base/model").ok()?;
        let text = String::from_utf8_lossy(&raw);
        let model = text.trim_end_matches('\0').trim();
        (!model.is_empty()).then(|| model.to_owned())
    }
}

#[cfg(target_os = "macos")]
mod platform {
    use std::process::{Command, Stdio};

    fn sysctl(name: &str) -> Option<String> {
        let output = Command::new("/usr/sbin/sysctl")
            .args(["-n", name])
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        let text = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        (!text.is_empty()).then_some(text)
    }

    pub(super) fn detect() -> (Option<String>, Option<u64>) {
        let cpu = sysctl("machdep.cpu.brand_string");
        let memory = sysctl("hw.memsize").and_then(|v| v.parse().ok());
        (cpu, memory)
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
mod platform {
    pub(super) fn detect() -> (Option<String>, Option<u64>) {
        (None, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tiers_for_typical_devices() {
        // 1 GB ARM box
        assert_eq!(PerfTier::detect(925 * MIB, 4, false), PerfTier::Tiny);
        // single-core VPS
        assert_eq!(PerfTier::detect(4 * GIB, 1, false), PerfTier::Tiny);
        // 2 GB quad-core ARM NAS
        assert_eq!(PerfTier::detect(1900 * MIB, 4, false), PerfTier::Lite);
        // 4 GB Celeron NAS
        assert_eq!(PerfTier::detect(3800 * MIB, 4, false), PerfTier::Lite);
        // 8 GB x86 NAS
        assert_eq!(PerfTier::detect(7800 * MIB, 4, false), PerfTier::Standard);
        // 32 GB desktop with a discrete GPU
        assert_eq!(PerfTier::detect(31 * GIB, 16, true), PerfTier::Performance);
        // same desktop before the encoder has been verified
        assert_eq!(PerfTier::detect(31 * GIB, 16, false), PerfTier::Standard);
        // unknown memory falls back to cores
        assert_eq!(PerfTier::detect(0, 8, false), PerfTier::Standard);
    }

    #[test]
    fn parses_tier_names() {
        assert_eq!(
            "Performance".parse::<PerfTier>().unwrap(),
            PerfTier::Performance
        );
        assert!("huge".parse::<PerfTier>().is_err());
        assert_eq!(serde_json::to_string(&PerfTier::Lite).unwrap(), "\"lite\"");
    }

    #[test]
    fn meminfo() {
        let text = "MemTotal:       16310412 kB\nMemFree:         1234567 kB\n";
        assert_eq!(parse_meminfo_total(text), Some(16_310_412 * 1024));
        assert_eq!(parse_meminfo_total("garbage"), None);
    }

    #[test]
    fn cgroup_limits() {
        assert_eq!(parse_cgroup_limit("max\n"), None);
        assert_eq!(parse_cgroup_limit("2147483648\n"), Some(2_147_483_648));
        assert_eq!(parse_cgroup_limit("9223372036854771712"), None);
    }

    #[test]
    fn cpu_model_x86_and_arm() {
        let x86 = "processor\t: 0\nvendor_id\t: GenuineIntel\nmodel name\t: Intel(R) Celeron(R) N5105 @ 2.00GHz\n";
        assert_eq!(
            parse_cpuinfo_model(x86).as_deref(),
            Some("Intel(R) Celeron(R) N5105 @ 2.00GHz")
        );
        let pi = "processor\t: 0\nBogoMIPS\t: 108.00\n\nHardware\t: BCM2835\nModel\t\t: Raspberry Pi 4 Model B Rev 1.4\n";
        assert_eq!(
            parse_cpuinfo_model(pi).as_deref(),
            Some("Raspberry Pi 4 Model B Rev 1.4")
        );
        assert_eq!(parse_cpuinfo_model("processor : 0\n"), None);
    }

    #[test]
    fn cpu_model_from_arm_part_numbers() {
        let ampere = "processor\t: 0\nBogoMIPS\t: 50.00\nCPU implementer\t: 0x41\nCPU architecture: 8\nCPU variant\t: 0x3\nCPU part\t: 0xd0c\nCPU revision\t: 1\n";
        assert_eq!(parse_cpuinfo_model(ampere), None);
        assert_eq!(
            parse_cpuinfo_arm(ampere).as_deref(),
            Some("ARM Neoverse-N1")
        );
        let unknown = "CPU implementer\t: 0x61\nCPU part\t: 0x022\n";
        assert_eq!(
            parse_cpuinfo_arm(unknown).as_deref(),
            Some("ARM implementer 0x61 part 0x022")
        );
        assert_eq!(parse_cpuinfo_arm("model name : x86\n"), None);
    }
}

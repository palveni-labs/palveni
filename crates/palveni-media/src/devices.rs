//! Hardware device nodes relevant to video encoding (Linux only).
//!
//! Inside NAS packages and containers the nodes often exist but are not
//! accessible to the service user, which is the most common reason a
//! listed hardware encoder fails; `accessible` captures that separately.

use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeviceNode {
    pub path: PathBuf,
    /// `None` when not checked (opening some nodes has side effects).
    pub accessible: Option<bool>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DeviceNodes {
    /// `/dev/dri/renderD*` (VAAPI, QSV).
    pub dri_render: Vec<DeviceNode>,
    /// `/dev/nvidia*` control and GPU nodes (NVENC).
    pub nvidia: Vec<DeviceNode>,
    /// `/dev/mpp_service`, `/dev/rga` (Rockchip MPP).
    pub rockchip: Vec<DeviceNode>,
    /// `/dev/video*` (V4L2 mem2mem, e.g. Raspberry Pi).
    pub v4l2: Vec<DeviceNode>,
}

impl DeviceNodes {
    /// Scans `/dev`. Blocking; cheap.
    pub fn scan() -> Self {
        if !cfg!(target_os = "linux") {
            return DeviceNodes::default();
        }
        let mut nodes = DeviceNodes::default();
        for path in list_prefixed(Path::new("/dev/dri"), "renderD") {
            nodes.dri_render.push(checked(path));
        }
        for path in list_prefixed(Path::new("/dev"), "nvidia") {
            // nvidia-uvm and friends are not needed for NVENC itself.
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default();
            if name == "nvidiactl" || name.trim_start_matches("nvidia").parse::<u32>().is_ok() {
                nodes.nvidia.push(checked(path));
            }
        }
        for name in ["/dev/mpp_service", "/dev/rga"] {
            let path = PathBuf::from(name);
            if path.exists() {
                nodes.rockchip.push(checked(path));
            }
        }
        for path in list_prefixed(Path::new("/dev"), "video") {
            nodes.v4l2.push(DeviceNode {
                path,
                accessible: None,
            });
        }
        nodes
    }

    /// First render node the current user can open.
    pub fn usable_render_node(&self) -> Option<&Path> {
        self.dri_render
            .iter()
            .find(|n| n.accessible == Some(true))
            .map(|n| n.path.as_path())
    }

    pub fn has_usable(nodes: &[DeviceNode]) -> bool {
        nodes.iter().any(|n| n.accessible != Some(false))
    }

    pub fn all(&self) -> impl Iterator<Item = &DeviceNode> {
        self.dri_render
            .iter()
            .chain(&self.nvidia)
            .chain(&self.rockchip)
            .chain(&self.v4l2)
    }
}

fn list_prefixed(dir: &Path, prefix: &str) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut paths: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .filter(|e| {
            e.file_name()
                .to_str()
                .is_some_and(|n| n.starts_with(prefix))
        })
        .map(|e| e.path())
        .collect();
    paths.sort();
    paths
}

/// Opening DRM render, NVIDIA and MPP nodes read-write is side-effect free
/// and is exactly what the encoders will do.
fn checked(path: PathBuf) -> DeviceNode {
    let accessible = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&path)
        .is_ok();
    DeviceNode {
        path,
        accessible: Some(accessible),
    }
}

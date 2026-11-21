//! Locating ffmpeg and ffprobe.
//!
//! Search order:
//! 1. the configured path (`ffmpeg_path`: the ffmpeg binary, or a directory
//!    containing both tools; ffprobe is looked up next to ffmpeg);
//! 2. bundled locations, in order: `<executable dir>/ffmpeg/`,
//!    `/usr/lib/palveni/ffmpeg/`, `/usr/lib/jellyfin-ffmpeg/` (the Docker
//!    image installs the jellyfin-ffmpeg package there);
//! 3. `PATH`.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(utoipa::ToSchema, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ToolSource {
    Config,
    Bundled,
    Path,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocatedTool {
    pub path: PathBuf,
    pub source: ToolSource,
}

/// Bundled directories in search order.
pub fn bundled_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(exe_dir) = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf))
    {
        dirs.push(exe_dir.join("ffmpeg"));
    }
    dirs.push(PathBuf::from("/usr/lib/palveni/ffmpeg"));
    dirs.push(PathBuf::from("/usr/lib/jellyfin-ffmpeg"));
    dirs
}

/// File name of a tool on this platform (`ffmpeg` / `ffmpeg.exe`).
pub fn tool_file_name(tool: &str) -> String {
    format!("{tool}{}", std::env::consts::EXE_SUFFIX)
}

/// Locates `tool` ("ffmpeg" or "ffprobe") on the real filesystem.
pub fn locate(tool: &str, configured_ffmpeg: Option<&Path>) -> Option<LocatedTool> {
    let path_var = std::env::var_os("PATH");
    let found = locate_with(
        tool,
        configured_ffmpeg,
        &bundled_dirs(),
        path_var.as_deref(),
        &is_executable_file,
    );
    if let Some(configured) = configured_ffmpeg
        && found.as_ref().map(|t| t.source) != Some(ToolSource::Config)
    {
        tracing::warn!(
            tool,
            configured = %configured.display(),
            "configured ffmpeg path does not provide this tool; falling back to bundled/PATH"
        );
    }
    found
}

/// Pure search logic; `is_exec` decides whether a candidate is usable.
pub fn locate_with(
    tool: &str,
    configured_ffmpeg: Option<&Path>,
    bundled: &[PathBuf],
    path_var: Option<&OsStr>,
    is_exec: &dyn Fn(&Path) -> bool,
) -> Option<LocatedTool> {
    let file_name = tool_file_name(tool);

    if let Some(configured) = configured_ffmpeg {
        let mut candidates = Vec::new();
        if tool == "ffmpeg" {
            candidates.push(configured.to_path_buf());
        } else if let Some(dir) = configured.parent() {
            candidates.push(dir.join(&file_name));
        }
        // The configured path may also be a directory holding both tools.
        candidates.push(configured.join(&file_name));
        if let Some(path) = candidates.into_iter().find(|c| is_exec(c)) {
            return Some(LocatedTool {
                path,
                source: ToolSource::Config,
            });
        }
    }

    if let Some(path) = bundled
        .iter()
        .map(|dir| dir.join(&file_name))
        .find(|c| is_exec(c))
    {
        return Some(LocatedTool {
            path,
            source: ToolSource::Bundled,
        });
    }

    let path_var = path_var?;
    std::env::split_paths(path_var)
        .filter(|dir| !dir.as_os_str().is_empty())
        .map(|dir| dir.join(&file_name))
        .find(|c| is_exec(c))
        .map(|path| LocatedTool {
            path,
            source: ToolSource::Path,
        })
}

/// A regular file with an execute bit (any regular file on non-Unix).
pub fn is_executable_file(path: &Path) -> bool {
    let Ok(meta) = std::fs::metadata(path) else {
        return false;
    };
    meta.is_file() && has_exec_permission(&meta)
}

#[cfg(unix)]
fn has_exec_permission(meta: &std::fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt;
    meta.permissions().mode() & 0o111 != 0
}

#[cfg(not(unix))]
fn has_exec_permission(_meta: &std::fs::Metadata) -> bool {
    true
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::ffi::OsString;

    use super::*;

    fn fs(paths: &[&str]) -> impl Fn(&Path) -> bool {
        let set: HashSet<PathBuf> = paths
            .iter()
            .map(|p| PathBuf::from(format!("{p}{}", std::env::consts::EXE_SUFFIX)))
            .collect();
        move |p: &Path| set.contains(p)
    }

    fn bundled() -> Vec<PathBuf> {
        vec![
            PathBuf::from("/opt/palveni/ffmpeg"),
            PathBuf::from("/usr/lib/palveni/ffmpeg"),
            PathBuf::from("/usr/lib/jellyfin-ffmpeg"),
        ]
    }

    fn path_var() -> OsString {
        std::env::join_paths(["/usr/local/bin", "/usr/bin"]).unwrap()
    }

    #[test]
    fn config_wins_and_ffprobe_is_a_sibling() {
        let exists = fs(&[
            "/custom/bin/ffmpeg",
            "/custom/bin/ffprobe",
            "/usr/bin/ffmpeg",
        ]);
        let configured = PathBuf::from(format!("/custom/bin/{}", tool_file_name("ffmpeg")));
        let pv = path_var();
        let ffmpeg =
            locate_with("ffmpeg", Some(&configured), &bundled(), Some(&pv), &exists).unwrap();
        assert_eq!(ffmpeg.source, ToolSource::Config);
        let ffprobe =
            locate_with("ffprobe", Some(&configured), &bundled(), Some(&pv), &exists).unwrap();
        assert_eq!(ffprobe.source, ToolSource::Config);
        assert_eq!(
            ffprobe.path,
            PathBuf::from(format!("/custom/bin/{}", tool_file_name("ffprobe")))
        );
    }

    #[test]
    fn configured_directory_works() {
        let exists = fs(&["/custom/ffmpeg"]);
        let found = locate_with("ffmpeg", Some(Path::new("/custom")), &[], None, &exists).unwrap();
        assert_eq!(found.source, ToolSource::Config);
    }

    #[test]
    fn bundled_order_then_path() {
        let exists = fs(&[
            "/usr/lib/jellyfin-ffmpeg/ffmpeg",
            "/usr/lib/palveni/ffmpeg/ffmpeg",
            "/usr/bin/ffmpeg",
        ]);
        let pv = path_var();
        let found = locate_with("ffmpeg", None, &bundled(), Some(&pv), &exists).unwrap();
        assert_eq!(found.source, ToolSource::Bundled);
        assert_eq!(
            found.path,
            PathBuf::from(format!(
                "/usr/lib/palveni/ffmpeg/{}",
                tool_file_name("ffmpeg")
            ))
        );

        let only_path = fs(&["/usr/bin/ffprobe"]);
        let found = locate_with("ffprobe", None, &bundled(), Some(&pv), &only_path).unwrap();
        assert_eq!(found.source, ToolSource::Path);

        assert_eq!(
            locate_with("ffprobe", None, &bundled(), Some(&pv), &fs(&[])),
            None
        );
    }

    #[test]
    fn broken_config_falls_back() {
        let exists = fs(&["/usr/bin/ffmpeg"]);
        let pv = path_var();
        let found = locate_with(
            "ffmpeg",
            Some(Path::new("/missing/ffmpeg")),
            &bundled(),
            Some(&pv),
            &exists,
        )
        .unwrap();
        assert_eq!(found.source, ToolSource::Path);
    }

    #[test]
    fn source_serialises_like_the_contract() {
        assert_eq!(
            serde_json::to_string(&ToolSource::Bundled).unwrap(),
            "\"bundled\""
        );
    }
}

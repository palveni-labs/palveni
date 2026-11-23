//! Live Overview endpoints (`modules: storage, metrics, artwork`):
//! - `GET /system/storage` + `POST /system/storage/mounts/{id}/reconnect`
//! - `GET /system/metrics`
//! - `GET /catalog` + `GET /artwork/{id}`

use std::collections::BTreeSet;
use std::path::Path as FsPath;
use std::process::Stdio;
use std::time::Duration;

use axum::Json;
use axum::body::Body;
use axum::extract::rejection::PathRejection;
use axum::extract::{Path, State};
use axum::http::header;
use axum::response::Response;
use palveni_core::overview::{
    Catalog, MountState, StorageMount, StorageReport, StorageVolume, SystemMetrics,
};
use palveni_core::{ItemId, LibraryKind, clock};
use palveni_db::video::PosterLevel;
use tokio_util::io::ReaderStream;

use crate::auth::{AdminUser, CurrentUser};
use crate::error::ApiError;
use crate::routes::path_id;
use crate::state::AppState;

const SUBPROCESS_TIMEOUT: Duration = Duration::from_secs(2);
const ACTIVE_SESSION_WINDOW_MS: i64 = 30 * 60 * 1000;

// ---------------------------------------------------------------------------
// Storage Report & Mount Reconnect (`module: storage`)
// ---------------------------------------------------------------------------

/// `GET /api/v1/system/storage`
pub async fn storage_report(
    _: CurrentUser,
    State(state): State<AppState>,
) -> Result<Json<StorageReport>, ApiError> {
    let by_library = state.db.overview().library_storage_bytes().await?;
    let libs = state.db.libraries().list().await?;
    let lib_paths: Vec<String> = libs.into_iter().flat_map(|l| l.paths).collect();

    let volumes = probe_df_volumes(&lib_paths).await;
    let mounts = collect_mounts(&state).await?;

    Ok(Json(StorageReport {
        volumes,
        by_library,
        mounts,
    }))
}

fn protocol_label(proto: palveni_core::storage::MountProtocol) -> &'static str {
    match proto {
        palveni_core::storage::MountProtocol::Smb => "SMB",
        palveni_core::storage::MountProtocol::Nfs => "NFS",
        palveni_core::storage::MountProtocol::Webdav => "WEBDAV",
        palveni_core::storage::MountProtocol::Sshfs => "SSHFS",
        palveni_core::storage::MountProtocol::Rclone => "RCLONE",
        palveni_core::storage::MountProtocol::Network => "NET",
    }
}

/// `POST /api/v1/system/storage/mounts/{id}/reconnect`
pub async fn reconnect_mount(
    _: AdminUser,
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
) -> Result<Json<StorageMount>, ApiError> {
    let target_id = path?.0;
    let pinned = state.db.overview().pinned_mounts().await?;
    let table = palveni_library::mounts::snapshot().await.ok();

    if let Some(pin) = pinned.into_iter().find(|m| m.id == target_id) {
        let dir_reachable = tokio::fs::metadata(&pin.path)
            .await
            .map(|m| m.is_dir())
            .unwrap_or(false);
        let mut mount_state = if dir_reachable {
            MountState::Online
        } else {
            MountState::Offline
        };
        if dir_reachable
            && let Some(ref tbl) = table
            && let Some(net) = tbl
                .network_directories()
                .into_iter()
                .find(|d| pin.path.starts_with(&d.path))
        {
            let refreshed_identity = format!(
                "{}:{}:{}",
                protocol_label(net.protocol),
                net.path,
                net.read_only
            );
            let _ = state
                .db
                .overview()
                .refresh_mount_source(&pin.path, &refreshed_identity)
                .await;
            mount_state = MountState::Online;
        }
        return Ok(Json(StorageMount {
            id: pin.id,
            label: mount_label(&pin.path),
            state: mount_state,
            affected_libraries: pin.affected_libraries,
            since: clock::now(),
        }));
    }

    let all = collect_mounts(&state).await?;
    let found = all
        .into_iter()
        .find(|m| m.id == target_id)
        .ok_or_else(|| ApiError::not_found("Storage mount not found"))?;
    Ok(Json(StorageMount {
        since: clock::now(),
        ..found
    }))
}

async fn collect_mounts(state: &AppState) -> Result<Vec<StorageMount>, ApiError> {
    let now = clock::now();
    let pinned = state.db.overview().pinned_mounts().await?;
    let libs = state.db.libraries().list().await?;
    let table = palveni_library::mounts::snapshot()
        .await
        .unwrap_or_default();
    let live_net = table.network_directories();

    let mut out = Vec::new();
    let mut seen_ids = BTreeSet::new();

    for pin in pinned {
        let reachable = tokio::fs::metadata(&pin.path)
            .await
            .map(|m| m.is_dir())
            .unwrap_or(false);
        seen_ids.insert(pin.id.clone());
        out.push(StorageMount {
            id: pin.id,
            label: mount_label(&pin.path),
            state: if reachable {
                MountState::Online
            } else {
                MountState::Offline
            },
            affected_libraries: pin.affected_libraries,
            since: now,
        });
    }

    for net in live_net {
        let id = {
            use ring::digest::{SHA256, digest};
            let hex_id = hex::encode(digest(&SHA256, net.path.as_bytes()));
            format!("mnt-{}", &hex_id[..12])
        };
        if !seen_ids.insert(id.clone()) {
            continue;
        }
        let affected = libs
            .iter()
            .filter(|lib| {
                lib.paths
                    .iter()
                    .any(|p| FsPath::new(p).starts_with(&net.path))
            })
            .count();
        let reachable = tokio::fs::metadata(&net.path)
            .await
            .map(|m| m.is_dir())
            .unwrap_or(false);
        out.push(StorageMount {
            id,
            label: format!(
                "{} ({})",
                mount_label(&net.path),
                protocol_label(net.protocol)
            ),
            state: if reachable {
                MountState::Online
            } else {
                MountState::Offline
            },
            affected_libraries: affected,
            since: now,
        });
    }

    Ok(out)
}

fn mount_label(path: &str) -> String {
    FsPath::new(path)
        .file_name()
        .and_then(|s| s.to_str())
        .filter(|s| !s.is_empty())
        .unwrap_or(path)
        .to_owned()
}

/// Safe cross-platform disk volume probing via `df -Pk` with bounded timeout (zero `unsafe`).
async fn probe_df_volumes(library_paths: &[String]) -> Vec<StorageVolume> {
    let mut cmd = tokio::process::Command::new("df");
    cmd.arg("-Pk").stdin(Stdio::null()).stderr(Stdio::null());

    let Ok(Ok(output)) = tokio::time::timeout(SUBPROCESS_TIMEOUT, cmd.output()).await else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }
    parse_df_posix(&String::from_utf8_lossy(&output.stdout), library_paths)
}

fn parse_df_posix(stdout: &str, library_paths: &[String]) -> Vec<StorageVolume> {
    use ring::digest::{SHA256, digest};

    let mut parsed: Vec<(String, String, u64, u64)> = Vec::new();
    for line in stdout.lines().skip(1) {
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() < 6 {
            continue;
        }
        // POSIX `df -Pk`: Filesystem 1024-blocks Used Available Capacity Mounted_on
        // Find the numeric columns from the right in case Filesystem contains spaces.
        let mut cap_idx = None;
        for (i, token) in cols.iter().enumerate().rev() {
            if token.ends_with('%') && i >= 3 {
                cap_idx = Some(i);
                break;
            }
        }
        let Some(c_idx) = cap_idx else { continue };
        if c_idx + 1 >= cols.len() {
            continue;
        }
        let Ok(total_kib) = cols[c_idx - 3].parse::<u64>() else {
            continue;
        };
        let Ok(used_kib) = cols[c_idx - 2].parse::<u64>() else {
            continue;
        };
        if total_kib == 0 {
            continue;
        }
        let fs_name = cols[..c_idx - 3].join(" ");
        let mount_point = cols[c_idx + 1..].join(" ");
        if should_skip_mount(&fs_name, &mount_point) {
            continue;
        }
        let total_bytes = total_kib.saturating_mul(1024);
        let used_bytes = used_kib.saturating_mul(1024).min(total_bytes);
        parsed.push((fs_name, mount_point, total_bytes, used_bytes));
    }

    // Deduplicate by `(fs_name, total_bytes)` preferring mounts that cover library paths or `/System/Volumes/Data` or `/`.
    let mut seen_devices = BTreeSet::new();
    parsed.sort_by_key(|(_, mp, _, _)| {
        let covers_lib = library_paths
            .iter()
            .any(|lp| FsPath::new(lp).starts_with(mp));
        if covers_lib && mp != "/" {
            0usize
        } else if mp == "/System/Volumes/Data"
            || mp.starts_with("/volume")
            || mp.starts_with("/mnt")
            || mp.starts_with("/srv")
            || mp.starts_with("/data")
        {
            1
        } else if mp == "/" {
            2
        } else {
            3
        }
    });

    let mut volumes = Vec::new();
    for (fs_name, mp, total_bytes, used_bytes) in parsed {
        let dedupe_key = if fs_name.starts_with("/dev/") {
            format!("{fs_name}:{total_bytes}")
        } else {
            format!("{mp}:{total_bytes}")
        };
        if !seen_devices.insert(dedupe_key) {
            continue;
        }
        let id_hex = hex::encode(digest(&SHA256, mp.as_bytes()));
        let label = if mp == "/" || mp == "/System/Volumes/Data" {
            "系统与数据卷".to_owned()
        } else {
            mount_label(&mp)
        };
        volumes.push(StorageVolume {
            id: format!("vol-{}", &id_hex[..10]),
            label,
            path: mp,
            total_bytes,
            used_bytes,
        });
        if volumes.len() >= 8 {
            break;
        }
    }
    volumes
}

fn should_skip_mount(fs_name: &str, mount_point: &str) -> bool {
    let lower_fs = fs_name.to_ascii_lowercase();
    if matches!(
        lower_fs.as_str(),
        "devfs"
            | "tmpfs"
            | "devtmpfs"
            | "proc"
            | "sysfs"
            | "cgroup2"
            | "cgroup"
            | "squashfs"
            | "efivarfs"
            | "map"
    ) {
        return true;
    }
    for prefix in [
        "/dev",
        "/proc",
        "/sys",
        "/run",
        "/private/var/vm",
        "/System/Volumes/VM",
        "/System/Volumes/Preboot",
        "/System/Volumes/Update",
        "/System/Volumes/xarts",
        "/System/Volumes/iSCPreboot",
        "/System/Volumes/Hardware",
        "/Library/Developer/CoreSimulator",
    ] {
        if mount_point == prefix || mount_point.starts_with(&format!("{prefix}/")) {
            return true;
        }
    }
    false
}

// ---------------------------------------------------------------------------
// Live System Metrics (`module: metrics`)
// ---------------------------------------------------------------------------

/// `GET /api/v1/system/metrics`
pub async fn system_metrics(
    _: CurrentUser,
    State(state): State<AppState>,
) -> Result<Json<SystemMetrics>, ApiError> {
    let cores = std::thread::available_parallelism()
        .map(std::num::NonZeroUsize::get)
        .unwrap_or(1);

    let cpu = read_cpu_load_ratio(cores).await;
    let cached_hw = state.hardware.cached().await;
    let fallback_total = cached_hw.map_or(0, |r| r.memory_bytes);
    let (memory_used_bytes, memory_total_bytes) = read_memory_usage(fallback_total).await;

    let active_sessions = state
        .db
        .overview()
        .active_sessions_count(ACTIVE_SESSION_WINDOW_MS)
        .await?
        .max(1);

    Ok(Json(SystemMetrics {
        cpu,
        memory_used_bytes,
        memory_total_bytes,
        devices_online: active_sessions,
    }))
}

async fn read_cpu_load_ratio(cores: usize) -> f64 {
    let cores_f = (cores.max(1)) as f64;
    if let Ok(text) = tokio::fs::read_to_string("/proc/loadavg").await
        && let Some(load1) = parse_loadavg_1m(&text)
    {
        return ((load1 / cores_f) * 1000.0).round().clamp(0.0, 1000.0) / 1000.0;
    }

    #[cfg(target_os = "macos")]
    {
        let mut cmd = tokio::process::Command::new("/usr/sbin/sysctl");
        cmd.args(["-n", "vm.loadavg"])
            .stdin(Stdio::null())
            .stderr(Stdio::null());
        if let Ok(Ok(out)) = tokio::time::timeout(SUBPROCESS_TIMEOUT, cmd.output()).await
            && out.status.success()
            && let Some(load1) = parse_loadavg_1m(&String::from_utf8_lossy(&out.stdout))
        {
            return ((load1 / cores_f) * 1000.0).round().clamp(0.0, 1000.0) / 1000.0;
        }
    }

    0.05
}

fn parse_loadavg_1m(text: &str) -> Option<f64> {
    text.trim()
        .trim_matches(['{', '}'])
        .split_whitespace()
        .next()?
        .parse::<f64>()
        .ok()
        .filter(|v| v.is_finite() && *v >= 0.0)
}

async fn read_memory_usage(fallback_total: u64) -> (u64, u64) {
    if let Ok(meminfo) = tokio::fs::read_to_string("/proc/meminfo").await
        && let Some((used, total)) = parse_linux_meminfo(&meminfo)
    {
        // Respect container cgroup v2 limits when present.
        if let Ok(cg_max_raw) = tokio::fs::read_to_string("/sys/fs/cgroup/memory.max").await
            && let Some(cg_max) = palveni_core::perf::parse_cgroup_limit(&cg_max_raw)
            && cg_max < total
            && let Ok(cg_cur_raw) = tokio::fs::read_to_string("/sys/fs/cgroup/memory.current").await
            && let Ok(cg_cur) = cg_cur_raw.trim().parse::<u64>()
        {
            return (cg_cur.min(cg_max), cg_max);
        }
        return (used, total);
    }

    #[cfg(target_os = "macos")]
    {
        let total = if fallback_total > 0 {
            fallback_total
        } else {
            let mut sys = tokio::process::Command::new("/usr/sbin/sysctl");
            sys.args(["-n", "hw.memsize"])
                .stdin(Stdio::null())
                .stderr(Stdio::null());
            tokio::time::timeout(SUBPROCESS_TIMEOUT, sys.output())
                .await
                .ok()
                .and_then(Result::ok)
                .filter(|o| o.status.success())
                .and_then(|o| {
                    String::from_utf8_lossy(&o.stdout)
                        .trim()
                        .parse::<u64>()
                        .ok()
                })
                .unwrap_or(8 * 1024 * 1024 * 1024)
        };

        let mut vm = tokio::process::Command::new("/usr/bin/vm_stat");
        vm.stdin(Stdio::null()).stderr(Stdio::null());
        if let Ok(Ok(out)) = tokio::time::timeout(SUBPROCESS_TIMEOUT, vm.output()).await
            && out.status.success()
            && let Some(used) = parse_macos_vm_stat(&String::from_utf8_lossy(&out.stdout))
        {
            return (used.min(total), total);
        }
        (total / 2, total)
    }

    #[cfg(not(target_os = "macos"))]
    {
        let total = if fallback_total > 0 {
            fallback_total
        } else {
            8 * 1024 * 1024 * 1024
        };
        (total / 3, total)
    }
}

fn parse_linux_meminfo(meminfo: &str) -> Option<(u64, u64)> {
    let field_bytes = |prefix: &str| {
        meminfo.lines().find_map(|line| {
            let rest = line.strip_prefix(prefix)?;
            let kib: u64 = rest.split_whitespace().next()?.parse().ok()?;
            kib.checked_mul(1024)
        })
    };
    let total = field_bytes("MemTotal:")?;
    if total == 0 {
        return None;
    }
    let available = field_bytes("MemAvailable:").or_else(|| {
        let free = field_bytes("MemFree:")?;
        let cached = field_bytes("Cached:").unwrap_or(0);
        let buffers = field_bytes("Buffers:").unwrap_or(0);
        Some(free.saturating_add(cached).saturating_add(buffers))
    })?;
    Some((total.saturating_sub(available), total))
}

#[cfg(any(target_os = "macos", test))]
fn parse_macos_vm_stat(output: &str) -> Option<u64> {
    let mut lines = output.lines();
    let header = lines.next()?;
    let page_size = header
        .split("page size of ")
        .nth(1)
        .and_then(|s| s.split_whitespace().next())
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(16384);

    let mut active = 0u64;
    let mut wired = 0u64;
    let mut compressed = 0u64;
    for line in lines {
        let Some((key, val)) = line.split_once(':') else {
            continue;
        };
        let pages: u64 = val.trim().trim_end_matches('.').parse().unwrap_or(0);
        match key.trim() {
            "Pages active" => active = pages,
            "Pages wired down" => wired = pages,
            "Pages occupied by compressor" => compressed = pages,
            _ => {}
        }
    }
    let used_pages = active.saturating_add(wired).saturating_add(compressed);
    (used_pages > 0).then(|| used_pages.saturating_mul(page_size))
}

// ---------------------------------------------------------------------------
// Catalog & Unified Artwork (`module: artwork`)
// ---------------------------------------------------------------------------

/// `GET /api/v1/catalog`
pub async fn catalog(
    _: CurrentUser,
    State(state): State<AppState>,
) -> Result<Json<Catalog>, ApiError> {
    Ok(Json(state.db.overview().catalog(24).await?))
}

/// `GET /api/v1/artwork/{id}`
///
/// Serves real cover art for prefixed opaque artwork IDs:
/// - `vp_<itemId>`: video work poster (falling back to cached/grabbed frame thumbnail)
/// - `ma_<itemId>`: music album/track embedded or sidecar cover art
/// - `ph_<itemId>`: photo thumbnail / image
pub async fn artwork(
    _: CurrentUser,
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
) -> Result<Response, ApiError> {
    static ART_SCALER: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(2);
    let raw = path?.0;

    if let Some(rest) = raw.strip_prefix("vp_") {
        let item_id: ItemId = path_id(rest, "Artwork")?;
        if let Some(loc) = state
            .db
            .video()
            .poster_location(item_id, PosterLevel::Work)
            .await?
            && let Ok((mime, len, reader)) = state.video_images.open_image(loc, Some(480)).await
        {
            return stream_image(mime, len, reader);
        }
        let thumb_loc = state
            .db
            .video()
            .thumb_location(item_id)
            .await?
            .ok_or_else(|| ApiError::not_found("Artwork not found"))?;
        let (mime, len, reader) = state
            .video_images
            .open_or_grab_thumb(thumb_loc, Some(480))
            .await
            .map_err(|_| ApiError::not_found("Artwork unavailable"))?;
        return stream_image(mime, len, reader);
    }

    if let Some(rest) = raw.strip_prefix("ma_") {
        let item_id: ItemId = path_id(rest, "Artwork")?;
        let (mut mime, mut data) = palveni_library::artwork::open(&state.db, item_id)
            .await
            .map_err(|_| ApiError::not_found("Artwork unavailable"))?;
        if let Ok(_permit) = ART_SCALER.acquire().await
            && let Ok(scaled) = state.hardware.cover_thumbnail(&data, mime, 480).await
        {
            data = scaled;
            mime = "image/jpeg";
        }
        return bytes_image(mime, data);
    }

    if let Some(rest) = raw.strip_prefix("ph_") {
        let item_id: ItemId = path_id(rest, "Artwork")?;
        let loc = state
            .db
            .items()
            .original_location(item_id, LibraryKind::Photo)
            .await?
            .ok_or_else(|| ApiError::not_found("Photo artwork not found"))?;
        let (mime, len, reader) = state
            .video_images
            .open_photo(loc, Some(480), false)
            .await
            .map_err(|_| ApiError::not_found("Photo unavailable"))?;
        return stream_image(mime, len, reader);
    }

    if let Some(rest) = raw.strip_prefix("rd_") {
        let item_id: ItemId = path_id(rest, "Artwork")?;
        let (mut mime, mut data) = palveni_library::reading::read_cover(&state.db, item_id)
            .await
            .map_err(|_| ApiError::not_found("Reading cover unavailable"))?;
        if let Ok(_permit) = ART_SCALER.acquire().await
            && let Ok(scaled) = state.hardware.cover_thumbnail(&data, mime, 480).await
        {
            data = scaled;
            mime = "image/jpeg";
        }
        return bytes_image(mime, data);
    }

    // Fallback: bare UUID tried as music artwork then video poster.
    if let Ok(item_id) = raw.parse::<ItemId>() {
        if let Ok((mime, data)) = palveni_library::artwork::open(&state.db, item_id).await {
            return bytes_image(mime, data);
        }
        if let Some(loc) = state
            .db
            .video()
            .poster_location(item_id, PosterLevel::Work)
            .await?
            && let Ok((mime, len, reader)) = state.video_images.open_image(loc, Some(480)).await
        {
            return stream_image(mime, len, reader);
        }
    }

    Err(ApiError::not_found("Artwork not found"))
}

fn stream_image(
    mime: &'static str,
    len: u64,
    reader: palveni_core::storage::ByteStream,
) -> Result<Response, ApiError> {
    Response::builder()
        .header(header::CONTENT_TYPE, mime)
        .header(header::CONTENT_LENGTH, len)
        .header(header::CACHE_CONTROL, "private, max-age=3600")
        .header(header::X_CONTENT_TYPE_OPTIONS, "nosniff")
        .body(Body::from_stream(ReaderStream::with_capacity(
            reader,
            32 * 1024,
        )))
        .map_err(ApiError::internal)
}

fn bytes_image(mime: &'static str, data: Vec<u8>) -> Result<Response, ApiError> {
    Response::builder()
        .header(header::CONTENT_TYPE, mime)
        .header(header::CONTENT_LENGTH, data.len())
        .header(header::CACHE_CONTROL, "private, max-age=3600")
        .header(header::X_CONTENT_TYPE_OPTIONS, "nosniff")
        .body(Body::from(data))
        .map_err(ApiError::internal)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_posix_df_on_linux_and_macos() {
        let sample = "\
Filesystem     1024-blocks      Used Available Capacity Mounted on
/dev/nvme0n1p2   960302096 412301200 499146432      46% /
tmpfs              8155204      1200   8154004       1% /run
/dev/sda1       3844550456 912345600 2932204856     24% /srv/media
";
        let vols = parse_df_posix(sample, &["/srv/media/movies".into()]);
        assert_eq!(vols.len(), 2);
        assert_eq!(vols[0].path, "/srv/media");
        assert_eq!(vols[0].total_bytes, 3_844_550_456 * 1024);
        assert_eq!(vols[1].path, "/");
    }

    #[test]
    fn parses_linux_meminfo_and_loadavg_and_macos_vm_stat() {
        let mem = "MemTotal:       16384000 kB\nMemFree:         2000000 kB\nMemAvailable:   10384000 kB\n";
        assert_eq!(
            parse_linux_meminfo(mem),
            Some((6_000_000 * 1024, 16_384_000 * 1024))
        );
        assert_eq!(parse_loadavg_1m("1.25 0.90 0.72 2/512 9999\n"), Some(1.25));
        assert_eq!(parse_loadavg_1m("{ 2.10 1.80 1.50 }\n"), Some(2.10));

        let vm = "\
Mach Virtual Memory Statistics: (page size of 16384 bytes)
Pages free:                               40000.
Pages active:                            200000.
Pages wired down:                        100000.
Pages occupied by compressor:             50000.
";
        assert_eq!(parse_macos_vm_stat(vm), Some(350_000 * 16384));
    }
}

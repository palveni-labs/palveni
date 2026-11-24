//! Observability endpoints (`module: observability`):
//! - `GET /api/v1/system/prometheus` (Admin): Prometheus 0.0.4 text exposition
//! - `GET /api/v1/system/doctor` (Admin): structured health diagnostics
//! - `GET /api/v1/system/logs` (Admin): live bounded ring buffer + audit + jobs log stream

use std::collections::VecDeque;
use std::fmt::Write as _;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

use axum::Json;
use axum::body::Body;
use axum::extract::rejection::QueryRejection;
use axum::extract::{Query, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::Response;
use palveni_core::observability::{
    DoctorCheck, DoctorStatus, ServerLogEntry, ServerLogLevel, ServerLogList, SystemDoctorReport,
};
use palveni_core::overview::MountState;
use palveni_core::{JobState, clock};
use palveni_media::EncodeMethod;
use serde::Deserialize;

use crate::auth::AdminUser;
use crate::error::ApiError;
use crate::routes::overview::{
    ACTIVE_SESSION_WINDOW_MS, collect_mounts, probe_df_volumes, read_cpu_load_ratio,
    read_memory_usage,
};
use crate::state::AppState;

const MAX_LOG_RING_ENTRIES: usize = 500;
const DEFAULT_LOG_LIMIT: usize = 100;
const MAX_LOG_LIMIT: usize = 500;

static STARTED_AT: OnceLock<Instant> = OnceLock::new();
static LOG_RING: OnceLock<Mutex<VecDeque<ServerLogEntry>>> = OnceLock::new();
static LOG_SEQ: AtomicU64 = AtomicU64::new(1);

fn uptime_seconds() -> f64 {
    STARTED_AT.get_or_init(Instant::now).elapsed().as_secs_f64()
}

fn ring_buffer() -> &'static Mutex<VecDeque<ServerLogEntry>> {
    let _ = STARTED_AT.get_or_init(Instant::now);
    LOG_RING.get_or_init(|| {
        let mut initial = VecDeque::with_capacity(MAX_LOG_RING_ENTRIES);
        initial.push_back(ServerLogEntry {
            id: "rt-0".to_owned(),
            timestamp: clock::now(),
            level: ServerLogLevel::Info,
            source: "runtime".to_owned(),
            target: "system.observability".to_owned(),
            message: "Server observability log ring buffer initialized".to_owned(),
        });
        Mutex::new(initial)
    })
}

/// Records a structured runtime log entry into the bounded in-process ring buffer.
pub(crate) fn record_log(
    level: ServerLogLevel,
    target: impl Into<String>,
    message: impl Into<String>,
) {
    let seq = LOG_SEQ.fetch_add(1, Ordering::Relaxed);
    let entry = ServerLogEntry {
        id: format!("rt-{seq}"),
        timestamp: clock::now(),
        level,
        source: "runtime".to_owned(),
        target: target.into(),
        message: message.into(),
    };
    if let Ok(mut guard) = ring_buffer().lock() {
        while guard.len() >= MAX_LOG_RING_ENTRIES {
            guard.pop_front();
        }
        guard.push_back(entry);
    }
}

fn snapshot_ring_logs() -> Vec<ServerLogEntry> {
    ring_buffer()
        .lock()
        .map(|guard| guard.iter().cloned().collect())
        .unwrap_or_default()
}

/// `GET /api/v1/system/prometheus` (Admin).
pub async fn prometheus(
    _admin: AdminUser,
    State(state): State<AppState>,
) -> Result<Response, ApiError> {
    let uptime = uptime_seconds();
    let libraries_total = state.db.libraries().list().await?.len();
    let items_total = state.db.items().count_live().await?;
    let users_total = state.db.users().count().await?;
    let active_sessions = state
        .db
        .overview()
        .active_sessions_count(ACTIVE_SESSION_WINDOW_MS)
        .await?;

    let cores = std::thread::available_parallelism()
        .map(std::num::NonZeroUsize::get)
        .unwrap_or(1);
    let cpu_load_ratio = read_cpu_load_ratio(cores).await;
    let fallback_total = state.hardware.cached().await.map_or(0, |r| r.memory_bytes);
    let (memory_used_bytes, memory_total_bytes) = read_memory_usage(fallback_total).await;
    let mounts_total = collect_mounts(&state).await.map_or(0, |m| m.len());

    let mut body = String::with_capacity(1024);
    let _ = writeln!(
        body,
        "# HELP palveni_uptime_seconds Process uptime in seconds."
    );
    let _ = writeln!(body, "# TYPE palveni_uptime_seconds gauge");
    let _ = writeln!(body, "palveni_uptime_seconds {uptime:.3}");

    let _ = writeln!(
        body,
        "# HELP palveni_libraries_total Total configured non-deleted media libraries."
    );
    let _ = writeln!(body, "# TYPE palveni_libraries_total gauge");
    let _ = writeln!(body, "palveni_libraries_total {libraries_total}");

    let _ = writeln!(
        body,
        "# HELP palveni_items_total Total live indexed leaf media items."
    );
    let _ = writeln!(body, "# TYPE palveni_items_total gauge");
    let _ = writeln!(body, "palveni_items_total {items_total}");

    let _ = writeln!(body, "# HELP palveni_users_total Total user accounts.");
    let _ = writeln!(body, "# TYPE palveni_users_total gauge");
    let _ = writeln!(body, "palveni_users_total {users_total}");

    let _ = writeln!(
        body,
        "# HELP palveni_active_sessions Active client sessions seen in the last 30 minutes."
    );
    let _ = writeln!(body, "# TYPE palveni_active_sessions gauge");
    let _ = writeln!(body, "palveni_active_sessions {active_sessions}");

    let _ = writeln!(
        body,
        "# HELP palveni_cpu_load_ratio Normalized 1-minute load average per logical CPU."
    );
    let _ = writeln!(body, "# TYPE palveni_cpu_load_ratio gauge");
    let _ = writeln!(body, "palveni_cpu_load_ratio {cpu_load_ratio:.4}");

    let _ = writeln!(
        body,
        "# HELP palveni_memory_used_bytes Host or container memory currently in use in bytes."
    );
    let _ = writeln!(body, "# TYPE palveni_memory_used_bytes gauge");
    let _ = writeln!(body, "palveni_memory_used_bytes {memory_used_bytes}");

    let _ = writeln!(
        body,
        "# HELP palveni_memory_total_bytes Host or container total memory capacity in bytes."
    );
    let _ = writeln!(body, "# TYPE palveni_memory_total_bytes gauge");
    let _ = writeln!(body, "palveni_memory_total_bytes {memory_total_bytes}");

    let _ = writeln!(
        body,
        "# HELP palveni_mounts_total Total tracked network/storage mounts."
    );
    let _ = writeln!(body, "# TYPE palveni_mounts_total gauge");
    let _ = writeln!(body, "palveni_mounts_total {mounts_total}");

    Response::builder()
        .status(StatusCode::OK)
        .header(
            header::CONTENT_TYPE,
            HeaderValue::from_static("text/plain; version=0.0.4; charset=utf-8"),
        )
        .header(
            header::CACHE_CONTROL,
            HeaderValue::from_static("private, no-store"),
        )
        .header(header::CONTENT_LENGTH, body.len())
        .body(Body::from(body))
        .map_err(ApiError::internal)
}

/// `GET /api/v1/system/doctor` (Admin).
pub async fn doctor(
    _admin: AdminUser,
    State(state): State<AppState>,
) -> Result<Json<SystemDoctorReport>, ApiError> {
    let mut checks = Vec::with_capacity(5);

    // 1. Database connectivity, backend & current revision
    let db_check = match state.db.current_revision().await {
        Ok(rev) => DoctorCheck {
            id: "database".to_owned(),
            name: "Database".to_owned(),
            status: DoctorStatus::Healthy,
            summary: format!(
                "{} connected (revision #{rev})",
                state.db.backend().as_str()
            ),
            detail: format!(
                "Backend: {} · Current catalog revision: {rev}",
                state.db.backend().as_str()
            ),
            remediation: None,
        },
        Err(err) => DoctorCheck {
            id: "database".to_owned(),
            name: "Database".to_owned(),
            status: DoctorStatus::Unhealthy,
            summary: format!("{} query failed", state.db.backend().as_str()),
            detail: err.to_string(),
            remediation: Some(
                "Verify database file permissions or network connectivity and inspect server logs."
                    .to_owned(),
            ),
        },
    };
    checks.push(db_check);

    // 2. FFmpeg / ffprobe & verified hardware encoders
    let ffmpeg_check = inspect_ffmpeg(&state).await;
    checks.push(ffmpeg_check);

    // 3. Storage mounts reachability & disk free space
    let libs = state.db.libraries().list().await.unwrap_or_default();
    let lib_paths: Vec<String> = libs.iter().flat_map(|l| l.paths.clone()).collect();
    let storage_check = inspect_storage(&state, &lib_paths).await;
    checks.push(storage_check);

    // 4. Library indexing health
    let live_items = state.db.items().count_live().await.unwrap_or(0);
    let unscanned_count = libs.iter().filter(|l| l.last_scan_at.is_none()).count();
    let libraries_check = if libs.is_empty() {
        DoctorCheck {
            id: "libraries".to_owned(),
            name: "Libraries & Index".to_owned(),
            status: DoctorStatus::Healthy,
            summary: "No libraries configured yet".to_owned(),
            detail: "0 libraries · 0 indexed items".to_owned(),
            remediation: Some(
                "Add a media library in System -> Libraries to begin indexing.".to_owned(),
            ),
        }
    } else if unscanned_count > 0 {
        DoctorCheck {
            id: "libraries".to_owned(),
            name: "Libraries & Index".to_owned(),
            status: DoctorStatus::Degraded,
            summary: format!(
                "{} libraries ({live_items} items), {unscanned_count} awaiting initial scan",
                libs.len()
            ),
            detail: format!(
                "Libraries: {} · Indexed items: {live_items} · Never scanned: {unscanned_count}",
                libs.len()
            ),
            remediation: Some(
                "Trigger a scan for unscanned libraries in System -> Libraries.".to_owned(),
            ),
        }
    } else {
        DoctorCheck {
            id: "libraries".to_owned(),
            name: "Libraries & Index".to_owned(),
            status: DoctorStatus::Healthy,
            summary: format!("{} libraries indexed ({live_items} items)", libs.len()),
            detail: format!(
                "Libraries: {} · Live leaf items: {live_items} · All libraries have completed at least one scan",
                libs.len()
            ),
            remediation: None,
        }
    };
    checks.push(libraries_check);

    // 5. Protocol surfaces
    let features = state.features.list();
    let has_mcp = features.iter().any(|f| f == "mcp");
    let mut surfaces = vec![
        "Native API (/api/v1)",
        "WebDAV (/webdav/)",
        "Subsonic (/rest)",
        "OPDS (/opds)",
    ];
    if has_mcp {
        surfaces.push("MCP (/api/v1/mcp)");
    }
    checks.push(DoctorCheck {
        id: "protocols".to_owned(),
        name: "Protocol Surfaces".to_owned(),
        status: DoctorStatus::Healthy,
        summary: format!("{} protocol surfaces active", surfaces.len()),
        detail: format!(
            "Active endpoints: {} · Registered capability modules: {}",
            surfaces.join(", "),
            features.len()
        ),
        remediation: None,
    });

    let overall_status = if checks.iter().any(|c| c.status == DoctorStatus::Unhealthy) {
        DoctorStatus::Unhealthy
    } else if checks.iter().any(|c| c.status == DoctorStatus::Degraded) {
        DoctorStatus::Degraded
    } else {
        DoctorStatus::Healthy
    };

    record_log(
        ServerLogLevel::Info,
        "system.doctor",
        format!(
            "Doctor check completed: status={:?}, checks={}",
            overall_status,
            checks.len()
        ),
    );

    Ok(Json(SystemDoctorReport {
        overall_status,
        generated_at: clock::now(),
        checks,
    }))
}

async fn inspect_ffmpeg(state: &AppState) -> DoctorCheck {
    if let Some(report) = state.hardware.cached().await {
        let verified_all: Vec<&str> = report
            .encoders
            .iter()
            .filter(|e| e.verified)
            .map(|e| e.id.as_str())
            .collect();
        let verified_hw: Vec<&str> = report
            .encoders
            .iter()
            .filter(|e| e.verified && e.method != EncodeMethod::Software)
            .map(|e| e.id.as_str())
            .collect();

        if !report.ffmpeg.found || !report.ffprobe.found {
            return DoctorCheck {
                id: "ffmpeg".to_owned(),
                name: "FFmpeg & Transcoding".to_owned(),
                status: DoctorStatus::Degraded,
                summary: format!(
                    "ffmpeg={}, ffprobe={}",
                    if report.ffmpeg.found { "found" } else { "missing" },
                    if report.ffprobe.found { "found" } else { "missing" }
                ),
                detail: "Direct play remains available, but transcoding and media probing require ffmpeg and ffprobe.".to_owned(),
                remediation: Some(
                    "Install ffmpeg and ffprobe on PATH or configure ffmpeg_path in palveni.toml."
                        .to_owned(),
                ),
            };
        }

        let summary = if verified_hw.is_empty() {
            format!(
                "FFmpeg ready (software encoding; {} verified encoders)",
                verified_all.len()
            )
        } else {
            format!(
                "FFmpeg ready ({} hardware encoders verified: {})",
                verified_hw.len(),
                verified_hw.join(", ")
            )
        };
        let detail = format!(
            "ffmpeg: {} ({}) · ffprobe: {} · Verified encoders: {}",
            report.ffmpeg.path.as_deref().unwrap_or("PATH"),
            report.ffmpeg.version.as_deref().unwrap_or("unknown"),
            report.ffprobe.path.as_deref().unwrap_or("PATH"),
            if verified_all.is_empty() {
                "none".to_owned()
            } else {
                verified_all.join(", ")
            }
        );
        return DoctorCheck {
            id: "ffmpeg".to_owned(),
            name: "FFmpeg & Transcoding".to_owned(),
            status: DoctorStatus::Healthy,
            summary,
            detail,
            remediation: None,
        };
    }

    // Fast path when hardware probe has not yet run (avoids blocking doctor on test encodes).
    let (ffmpeg_found, ffprobe_found, ffmpeg_path) = tokio::task::spawn_blocking(|| {
        let ff = palveni_media::discovery::locate("ffmpeg", None);
        let fp = palveni_media::discovery::locate("ffprobe", None);
        (
            ff.is_some(),
            fp.is_some(),
            ff.map(|t| t.path.display().to_string()),
        )
    })
    .await
    .unwrap_or((false, false, None));

    if ffmpeg_found && ffprobe_found {
        DoctorCheck {
            id: "ffmpeg".to_owned(),
            name: "FFmpeg & Transcoding".to_owned(),
            status: DoctorStatus::Healthy,
            summary: "ffmpeg and ffprobe located (hardware encoders pending probe)".to_owned(),
            detail: format!(
                "ffmpeg: {} · Open System -> Hardware to run full encoder verification.",
                ffmpeg_path.as_deref().unwrap_or("PATH")
            ),
            remediation: None,
        }
    } else {
        DoctorCheck {
            id: "ffmpeg".to_owned(),
            name: "FFmpeg & Transcoding".to_owned(),
            status: DoctorStatus::Degraded,
            summary: format!(
                "ffmpeg={}, ffprobe={}",
                if ffmpeg_found { "found" } else { "missing" },
                if ffprobe_found { "found" } else { "missing" }
            ),
            detail: "Transcoding, subtitle extraction and thumbnail generation require both ffmpeg and ffprobe.".to_owned(),
            remediation: Some(
                "Install ffmpeg/ffprobe or set ffmpeg_path in configuration.".to_owned(),
            ),
        }
    }
}

async fn inspect_storage(state: &AppState, lib_paths: &[String]) -> DoctorCheck {
    let mounts = collect_mounts(state).await.unwrap_or_default();
    let volumes = probe_df_volumes(lib_paths).await;

    let offline_mounts: Vec<&str> = mounts
        .iter()
        .filter(|m| m.state != MountState::Online)
        .map(|m| m.label.as_str())
        .collect();

    let mut missing_lib_paths = Vec::new();
    for path in lib_paths {
        let reachable = tokio::fs::metadata(path)
            .await
            .map(|m| m.is_dir())
            .unwrap_or(false);
        if !reachable {
            missing_lib_paths.push(path.as_str());
        }
    }

    let mut critical_vol = false;
    let mut warn_vol = false;
    let mut vol_details = Vec::new();
    for vol in &volumes {
        let free = vol.total_bytes.saturating_sub(vol.used_bytes);
        let pct_used = if vol.total_bytes > 0 {
            (vol.used_bytes as f64 / vol.total_bytes as f64) * 100.0
        } else {
            0.0
        };
        if pct_used >= 98.0 {
            critical_vol = true;
        } else if pct_used >= 90.0 {
            warn_vol = true;
        }
        let free_gib = free as f64 / (1024.0 * 1024.0 * 1024.0);
        vol_details.push(format!(
            "{} ({free_gib:.1} GiB free, {pct_used:.0}% used)",
            vol.path
        ));
    }

    let status = if critical_vol || (!mounts.is_empty() && offline_mounts.len() == mounts.len()) {
        DoctorStatus::Unhealthy
    } else if warn_vol || !offline_mounts.is_empty() || !missing_lib_paths.is_empty() {
        DoctorStatus::Degraded
    } else {
        DoctorStatus::Healthy
    };

    let summary = if !offline_mounts.is_empty() {
        format!(
            "{} mount(s) offline: {}",
            offline_mounts.len(),
            offline_mounts.join(", ")
        )
    } else if !missing_lib_paths.is_empty() {
        format!(
            "{} library path(s) currently unreachable",
            missing_lib_paths.len()
        )
    } else if critical_vol || warn_vol {
        format!("Low disk space detected across {} volume(s)", volumes.len())
    } else {
        format!(
            "{} volume(s) healthy · {} network mount(s) online",
            volumes.len(),
            mounts.len()
        )
    };

    let detail = if vol_details.is_empty() {
        format!("Mounts tracked: {}", mounts.len())
    } else {
        vol_details.join(" · ")
    };

    let remediation = match status {
        DoctorStatus::Healthy => None,
        DoctorStatus::Degraded | DoctorStatus::Unhealthy => Some(
            "Check mount connections, verify library directory permissions, or free disk space on near-full volumes."
                .to_owned(),
        ),
    };

    DoctorCheck {
        id: "storage".to_owned(),
        name: "Storage & Mounts".to_owned(),
        status,
        summary,
        detail,
        remediation,
    }
}

#[derive(Deserialize, Default)]
pub struct LogsQuery {
    pub level: Option<String>,
    pub limit: Option<String>,
}

/// `GET /api/v1/system/logs?level=info|warn|error&limit=100` (Admin).
pub async fn logs(
    _admin: AdminUser,
    State(state): State<AppState>,
    query: Result<Query<LogsQuery>, QueryRejection>,
) -> Result<Json<ServerLogList>, ApiError> {
    let Query(params) = query?;

    let level_filter = match params
        .level
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        None => None,
        Some(raw) if raw.eq_ignore_ascii_case("all") => None,
        Some(raw) => Some(ServerLogLevel::parse_filter(raw).ok_or_else(|| {
            ApiError::validation("level must be one of all, info, warn, error").field("level")
        })?),
    };

    let limit = match params
        .limit
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        None => DEFAULT_LOG_LIMIT,
        Some(raw) => {
            let parsed = raw.parse::<usize>().map_err(|_| {
                ApiError::validation("limit must be a positive integer").field("limit")
            })?;
            if parsed == 0 {
                return Err(ApiError::validation("limit must be at least 1").field("limit"));
            }
            parsed.min(MAX_LOG_LIMIT)
        }
    };

    let fetch_u64 = u64::try_from(limit).unwrap_or(100);
    let mut combined = snapshot_ring_logs();

    if let Ok(audits) = state.db.audit().recent(fetch_u64).await {
        for entry in audits {
            let level = if entry.action == "auth.login_failed" {
                ServerLogLevel::Warn
            } else {
                ServerLogLevel::Info
            };
            let mut msg = entry.action.clone();
            if let Some(ref target) = entry.target {
                let _ = write!(msg, " target={target}");
            }
            if let Some(ref detail) = entry.detail {
                let _ = write!(msg, " {detail}");
            }
            combined.push(ServerLogEntry {
                id: format!("audit-{}", entry.id),
                timestamp: entry.occurred_at,
                level,
                source: "audit".to_owned(),
                target: entry.action,
                message: msg,
            });
        }
    }

    if let Ok(jobs) = state.db.jobs().list_recent(fetch_u64).await {
        for job in jobs {
            let level = match job.state {
                JobState::Failed => ServerLogLevel::Error,
                JobState::Cancelled => ServerLogLevel::Warn,
                _ => ServerLogLevel::Info,
            };
            let subject = job
                .subject
                .as_ref()
                .and_then(|s| s.name.as_deref().or(Some(s.id.as_str())))
                .unwrap_or("server");
            let mut msg = format!("job {} [{}] ({subject})", job.kind, job.state.as_str());
            if let Some(ref err) = job.error {
                let _ = write!(msg, ": {err}");
            } else if let Some(ref note) = job.message {
                let _ = write!(msg, ": {note}");
            }
            combined.push(ServerLogEntry {
                id: format!("job-{}", job.id),
                timestamp: job.finished_at.or(job.started_at).unwrap_or(job.created_at),
                level,
                source: "job".to_owned(),
                target: job.kind,
                message: msg,
            });
        }
    }

    if let Some(want) = level_filter {
        combined.retain(|entry| entry.level == want);
    }

    combined.sort_by(|a, b| b.timestamp.cmp(&a.timestamp).then_with(|| b.id.cmp(&a.id)));
    let total = combined.len();
    combined.truncate(limit);

    Ok(Json(ServerLogList {
        entries: combined,
        total,
    }))
}

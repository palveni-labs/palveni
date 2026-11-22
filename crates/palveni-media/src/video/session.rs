//! Video media probing cache and live HLS remux/transcode session management.
use std::{
    collections::HashMap,
    fs::File,
    io::{ErrorKind, Seek, SeekFrom},
    path::{Path, PathBuf},
    process::Stdio,
    sync::{
        Arc, Mutex, MutexGuard,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::Duration,
};

use palveni_core::{ItemId, PerfTier, SessionId, UserId};
use tokio::{
    io::AsyncReadExt,
    process::Command,
    sync::{OnceCell, Semaphore, watch},
    time::{Instant, sleep, timeout},
};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::{
    devices::DeviceNodes,
    discovery::locate,
    process::{os_args, run, summarize_stderr},
    report::HardwareReport,
};

use super::{
    cache::{FileFingerprint, MediaInfoCache},
    decision::{Decision, DeliveryMode},
    encoder::{
        HlsPlan, PLAYLIST_FILE, SelectedEncoder, build_hls_args, is_valid_segment_name,
        select_h264_encoder,
    },
    info::{self, ProbeFailure, ProbedMedia},
};

const DEFAULT_IDLE_TIMEOUT: Duration = Duration::from_secs(60);
const REAPER_INTERVAL: Duration = Duration::from_secs(5);
const STARTUP_TIMEOUT: Duration = Duration::from_secs(15);
const WAIT_READY_TIMEOUT: Duration = Duration::from_secs(15);
const TERMINATE_WAIT_TIMEOUT: Duration = Duration::from_secs(3);
const MAX_SESSION_LIFETIME: Duration = Duration::from_secs(12 * 3600);
const PROBE_SLOT_WAIT: Duration = Duration::from_secs(5);
const MAX_RETAINED_SESSIONS: usize = 16;
const MAX_STDERR_BYTES: usize = 16 * 1024;

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum VideoError {
    #[error("Video transcoding is disabled for the {0} performance tier")]
    Disabled(PerfTier),
    #[error("Video {kind} limit reached ({active}/{limit} active for {tier} tier)")]
    Busy {
        kind: &'static str,
        active: usize,
        limit: usize,
        tier: PerfTier,
    },
    #[error("FFmpeg is not available")]
    Unavailable,
    #[error("Video session not found")]
    NotFound,
    #[error("Segment is still being prepared")]
    SegmentNotReady,
    #[error("The video could not be processed: {0}")]
    Invalid(String),
}

pub const fn transcode_limit(tier: PerfTier) -> usize {
    match tier {
        PerfTier::Tiny => 0,
        PerfTier::Lite => 1,
        PerfTier::Standard => 2,
        PerfTier::Performance => 4,
    }
}

pub const fn remux_limit(tier: PerfTier) -> usize {
    match tier {
        PerfTier::Tiny => 1,
        PerfTier::Lite => 2,
        PerfTier::Standard => 4,
        PerfTier::Performance => 8,
    }
}

#[derive(Clone, Debug)]
pub struct StartHlsRequest {
    pub owner_user: UserId,
    pub owner_session: SessionId,
    pub item: ItemId,
    pub tier: PerfTier,
    pub start_ms: u64,
    pub max_bitrate_kbps: Option<u32>,
    pub decision: Decision,
    pub media: ProbedMedia,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StartedHlsSession {
    pub session_id: Uuid,
    pub start_ms: u64,
    pub encoder: String,
}

#[derive(Default)]
struct ActiveCounters {
    transcodes: usize,
    remuxes: usize,
}

#[derive(Default)]
struct ActivePool {
    counters: Mutex<ActiveCounters>,
}

struct SlotGuard {
    pool: Arc<ActivePool>,
    is_transcode: bool,
}

impl ActivePool {
    fn lock(&self) -> MutexGuard<'_, ActiveCounters> {
        self.counters.lock().unwrap_or_else(|p| p.into_inner())
    }

    fn try_acquire(
        self: &Arc<Self>,
        mode: DeliveryMode,
        tier: PerfTier,
    ) -> Result<SlotGuard, VideoError> {
        let mut state = self.lock();
        match mode {
            DeliveryMode::Transcode => {
                let limit = transcode_limit(tier);
                if limit == 0 {
                    return Err(VideoError::Disabled(tier));
                }
                if state.transcodes >= limit {
                    return Err(VideoError::Busy {
                        kind: "transcoding",
                        active: state.transcodes,
                        limit,
                        tier,
                    });
                }
                state.transcodes += 1;
                Ok(SlotGuard {
                    pool: Arc::clone(self),
                    is_transcode: true,
                })
            }
            DeliveryMode::Remux | DeliveryMode::Direct => {
                let limit = remux_limit(tier);
                if state.remuxes >= limit {
                    return Err(VideoError::Busy {
                        kind: "remux",
                        active: state.remuxes,
                        limit,
                        tier,
                    });
                }
                state.remuxes += 1;
                Ok(SlotGuard {
                    pool: Arc::clone(self),
                    is_transcode: false,
                })
            }
        }
    }

    #[cfg(test)]
    fn counts(&self) -> (usize, usize) {
        let state = self.lock();
        (state.transcodes, state.remuxes)
    }
}

impl Drop for SlotGuard {
    fn drop(&mut self) {
        let mut state = self.pool.lock();
        if self.is_transcode {
            state.transcodes = state.transcodes.saturating_sub(1);
        } else {
            state.remuxes = state.remuxes.saturating_sub(1);
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ProcessState {
    Running,
    Finished,
    Failed(String),
}

struct SessionHandle {
    owner_user: UserId,
    owner_session: SessionId,
    item: ItemId,
    dir: PathBuf,
    stop: CancellationToken,
    state: watch::Receiver<ProcessState>,
    done: watch::Receiver<bool>,
    last_activity_ms: AtomicU64,
}

impl SessionHandle {
    fn touch(&self, epoch: Instant) {
        let ms = u64::try_from(epoch.elapsed().as_millis()).unwrap_or(u64::MAX);
        self.last_activity_ms.store(ms, Ordering::Relaxed);
    }

    fn idle_duration(&self, epoch: Instant) -> Duration {
        let now_ms = u64::try_from(epoch.elapsed().as_millis()).unwrap_or(u64::MAX);
        let last = self.last_activity_ms.load(Ordering::Relaxed);
        Duration::from_millis(now_ms.saturating_sub(last))
    }
}

pub struct VideoService {
    ffmpeg: Option<PathBuf>,
    ffprobe: Option<PathBuf>,
    cache: MediaInfoCache,
    sessions_root: PathBuf,
    probe_slots: Semaphore,
    supports_readrate_burst: OnceCell<bool>,
    active_pool: Arc<ActivePool>,
    sessions: Arc<Mutex<HashMap<Uuid, Arc<SessionHandle>>>>,
    shutdown: CancellationToken,
    epoch: Instant,
    idle_timeout: Duration,
    reaper_interval: Duration,
    reaper_started: AtomicBool,
}

impl VideoService {
    pub fn new(configured_ffmpeg: Option<&Path>, data_dir: PathBuf) -> Self {
        Self::with_timeouts(
            configured_ffmpeg,
            data_dir,
            DEFAULT_IDLE_TIMEOUT,
            REAPER_INTERVAL,
        )
    }

    fn with_timeouts(
        configured_ffmpeg: Option<&Path>,
        data_dir: PathBuf,
        idle_timeout: Duration,
        reaper_interval: Duration,
    ) -> Self {
        let sessions_root = data_dir.join("video-sessions");
        // Clean up any leftover session directories from a prior unclean exit.
        if sessions_root.is_dir()
            && let Ok(entries) = std::fs::read_dir(&sessions_root)
        {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let _ = std::fs::remove_dir_all(&path);
                } else {
                    let _ = std::fs::remove_file(&path);
                }
            }
        }
        Self {
            ffmpeg: locate("ffmpeg", configured_ffmpeg).map(|t| t.path),
            ffprobe: locate("ffprobe", configured_ffmpeg).map(|t| t.path),
            cache: MediaInfoCache::new(data_dir.join("media-info")),
            sessions_root,
            probe_slots: Semaphore::new(2),
            supports_readrate_burst: OnceCell::new(),
            active_pool: Arc::default(),
            sessions: Arc::new(Mutex::new(HashMap::new())),
            shutdown: CancellationToken::new(),
            epoch: Instant::now(),
            idle_timeout,
            reaper_interval,
            reaper_started: AtomicBool::new(false),
        }
    }

    fn lock_sessions(&self) -> MutexGuard<'_, HashMap<Uuid, Arc<SessionHandle>>> {
        self.sessions.lock().unwrap_or_else(|p| p.into_inner())
    }

    fn ensure_reaper(&self) {
        if self
            .reaper_started
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return;
        }
        let sessions = Arc::clone(&self.sessions);
        let shutdown = self.shutdown.clone();
        let epoch = self.epoch;
        let idle_timeout = self.idle_timeout;
        let interval = self.reaper_interval;
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    () = shutdown.cancelled() => break,
                    () = sleep(interval) => {}
                }
                let expired: Vec<Arc<SessionHandle>> = {
                    let mut map = sessions.lock().unwrap_or_else(|p| p.into_inner());
                    let ids: Vec<Uuid> = map
                        .iter()
                        .filter_map(|(id, h)| {
                            (h.idle_duration(epoch) >= idle_timeout).then_some(*id)
                        })
                        .collect();
                    ids.into_iter().filter_map(|id| map.remove(&id)).collect()
                };
                for handle in expired {
                    tracing::debug!(dir = %handle.dir.display(), "reaping idle video HLS session");
                    terminate_session(handle).await;
                }
            }
        });
    }

    /// Probes a video file, serving cached results when size and mtime match.
    pub async fn probe_media(
        &self,
        item: ItemId,
        file: &File,
        extension: Option<&str>,
    ) -> Result<ProbedMedia, ProbeFailure> {
        let fingerprint = FileFingerprint::from_file(file);
        if let Some(fp) = fingerprint
            && let Some(cached) = self.cache.load(item, fp).await
        {
            return Ok(cached);
        }
        let ffprobe = self.ffprobe.as_deref().ok_or(ProbeFailure::Unavailable)?;
        let _permit = timeout(PROBE_SLOT_WAIT, self.probe_slots.acquire())
            .await
            .map_err(|_| ProbeFailure::Busy)?
            .map_err(|_| ProbeFailure::Busy)?;
        let probe_file = file.try_clone().map_err(|_| ProbeFailure::Invalid)?;
        let mut probed = info::probe(ffprobe, probe_file, extension).await?;
        let _ = (&*file).seek(SeekFrom::Start(0));
        if probed.info.size_bytes.is_none()
            && let Some(fp) = fingerprint
        {
            probed.info.size_bytes = Some(fp.size);
        }
        if let Some(fp) = fingerprint {
            self.cache.store(item, fp, &probed).await;
        }
        Ok(probed)
    }

    /// Ends any active HLS session owned by the same device session for the same item.
    pub async fn close_for_owner_item(&self, owner_session: SessionId, item: ItemId) {
        let existing: Vec<Arc<SessionHandle>> = {
            let mut map = self.lock_sessions();
            let ids: Vec<Uuid> = map
                .iter()
                .filter_map(|(id, h)| {
                    (h.owner_session == owner_session && h.item == item).then_some(*id)
                })
                .collect();
            ids.into_iter().filter_map(|id| map.remove(&id)).collect()
        };
        for handle in existing {
            terminate_session(handle).await;
        }
    }

    /// Starts a live HLS remux or transcode session, replacing any prior session for this owner+item.
    pub async fn start_hls(
        &self,
        request: StartHlsRequest,
        file: File,
        report: Option<&HardwareReport>,
    ) -> Result<StartedHlsSession, VideoError> {
        if self.shutdown.is_cancelled() {
            return Err(VideoError::Unavailable);
        }
        let ffmpeg = self.ffmpeg.as_deref().ok_or(VideoError::Unavailable)?;
        let video_index = request
            .media
            .video_index
            .ok_or_else(|| VideoError::Invalid("File has no video stream".to_owned()))?;
        let video_info = request
            .media
            .info
            .video
            .as_ref()
            .ok_or_else(|| VideoError::Invalid("File has no video stream".to_owned()))?;

        // Close any previous session for the same device session & item first (e.g. seek/quality switch)
        // so its concurrency slot is freed immediately before acquiring a new slot.
        self.close_for_owner_item(request.owner_session, request.item)
            .await;

        let slot = self
            .active_pool
            .try_acquire(request.decision.mode, request.tier)?;

        // Snap remux seeks to the preceding keyframe so reported startMs matches actual video PTS.
        let actual_start_ms = if request.decision.mode == DeliveryMode::Remux
            && request.start_ms > 0
        {
            if let Some(ffprobe) = self.ffprobe.as_deref()
                && let Ok(probe_clone) = file.try_clone()
            {
                let kf =
                    info::keyframe_before(ffprobe, probe_clone, request.start_ms, &request.media)
                        .await;
                let _ = (&file).seek(SeekFrom::Start(0));
                kf.unwrap_or(request.start_ms)
            } else {
                request.start_ms
            }
        } else {
            request.start_ms
        };

        let use_readrate_burst = self.supports_readrate_burst(ffmpeg).await;
        let selected_encoder = if request.decision.mode == DeliveryMode::Transcode {
            select_h264_encoder(report)
        } else {
            SelectedEncoder::software()
        };
        let render_node = if request.decision.mode == DeliveryMode::Transcode
            && selected_encoder.method == crate::encoders::EncodeMethod::Vaapi
        {
            tokio::task::spawn_blocking(|| {
                DeviceNodes::scan()
                    .usable_render_node()
                    .map(Path::to_path_buf)
            })
            .await
            .ok()
            .flatten()
        } else {
            None
        };

        let session_id = Uuid::now_v7();
        let dir = self.sessions_root.join(session_id.to_string());
        tokio::fs::create_dir_all(&dir).await.map_err(|err| {
            VideoError::Invalid(format!("cannot create session directory: {err}"))
        })?;

        let mut plan = HlsPlan {
            mode: request.decision.mode,
            video_index,
            video_codec: Some(video_info.codec.clone()),
            video_width: video_info.width,
            video_height: video_info.height,
            frame_rate: video_info.frame_rate,
            container: request.media.info.container.clone(),
            audio_index: request.decision.audio,
            copy_audio: request.decision.copy_audio,
            scale_to: request.decision.scale_to,
            max_bitrate_kbps: request.max_bitrate_kbps,
            start_ms: actual_start_ms,
            tier: request.tier,
            encoder: selected_encoder.clone(),
            render_node,
            use_readrate_burst,
        };

        let mut used_encoder = if plan.mode == DeliveryMode::Transcode {
            plan.encoder.id.clone()
        } else {
            "copy".to_owned()
        };

        // Keep a cloned descriptor when trying a hardware encoder so we can fall back to libx264 cleanly.
        let fallback_file = if plan.mode == DeliveryMode::Transcode && plan.encoder.is_hardware() {
            file.try_clone().ok()
        } else {
            None
        };

        let handle = match self
            .spawn_and_wait_ready(
                ffmpeg,
                &plan,
                file,
                slot,
                request.owner_user,
                request.owner_session,
                request.item,
                dir.clone(),
            )
            .await
        {
            Ok(handle) => handle,
            Err((err, Some(fallback_slot))) if fallback_file.is_some() => {
                tracing::warn!(
                    encoder = %plan.encoder.id,
                    error = %err,
                    "hardware video encoder failed at startup; falling back to libx264"
                );
                let _ = clear_dir(&dir).await;
                let Some(mut retry_file) = fallback_file else {
                    let _ = tokio::fs::remove_dir_all(&dir).await;
                    return Err(err);
                };
                if retry_file.seek(SeekFrom::Start(0)).is_err() {
                    let _ = tokio::fs::remove_dir_all(&dir).await;
                    return Err(err);
                }
                plan.encoder = SelectedEncoder::software();
                plan.render_node = None;
                used_encoder = plan.encoder.id.clone();
                match self
                    .spawn_and_wait_ready(
                        ffmpeg,
                        &plan,
                        retry_file,
                        fallback_slot,
                        request.owner_user,
                        request.owner_session,
                        request.item,
                        dir.clone(),
                    )
                    .await
                {
                    Ok(handle) => handle,
                    Err((fallback_err, _)) => {
                        let _ = tokio::fs::remove_dir_all(&dir).await;
                        return Err(fallback_err);
                    }
                }
            }
            Err((err, _)) => {
                let _ = tokio::fs::remove_dir_all(&dir).await;
                return Err(err);
            }
        };

        if self.shutdown.is_cancelled() {
            terminate_session(handle).await;
            return Err(VideoError::Unavailable);
        }

        // Evict the oldest finished session if the retained map exceeds its soft cap.
        let evicted = {
            let mut map = self.lock_sessions();
            let evict_id = if map.len() >= MAX_RETAINED_SESSIONS {
                map.iter()
                    .filter(|(_, h)| *h.state.borrow() != ProcessState::Running)
                    .max_by_key(|(_, h)| h.idle_duration(self.epoch))
                    .map(|(id, _)| *id)
            } else {
                None
            };
            let evicted = evict_id.and_then(|id| map.remove(&id));
            map.insert(session_id, handle);
            evicted
        };
        if let Some(old) = evicted {
            terminate_session(old).await;
        }
        self.ensure_reaper();

        Ok(StartedHlsSession {
            session_id,
            start_ms: actual_start_ms,
            encoder: used_encoder,
        })
    }

    #[expect(clippy::too_many_arguments)]
    async fn spawn_and_wait_ready(
        &self,
        ffmpeg: &Path,
        plan: &HlsPlan,
        file: File,
        slot: SlotGuard,
        owner_user: UserId,
        owner_session: SessionId,
        item: ItemId,
        dir: PathBuf,
    ) -> Result<Arc<SessionHandle>, (VideoError, Option<SlotGuard>)> {
        let args = build_hls_args(plan, &dir);
        let mut cmd = Command::new(ffmpeg);
        cmd.args(&args)
            .stdin(Stdio::from(file))
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        let mut child = match cmd.spawn() {
            Ok(child) => child,
            Err(_) => return Err((VideoError::Unavailable, Some(slot))),
        };

        let stderr_pipe = child.stderr.take();
        let stop = self.shutdown.child_token();
        let (state_tx, mut state_rx) = watch::channel(ProcessState::Running);
        let (done_tx, done_rx) = watch::channel(false);

        let task_stop = stop.clone();
        let encoder_name = if plan.mode == DeliveryMode::Transcode {
            plan.encoder.id.clone()
        } else {
            "copy".to_owned()
        };
        let mode = plan.mode;
        let started = Instant::now();

        tokio::spawn(async move {
            let _slot = slot;
            let stderr_task = tokio::spawn(async move {
                let Some(mut pipe) = stderr_pipe else {
                    return String::new();
                };
                let mut collected = Vec::new();
                let mut buf = [0u8; 4096];
                loop {
                    match pipe.read(&mut buf).await {
                        Ok(0) | Err(_) => break,
                        Ok(n) => {
                            if collected.len() + n > MAX_STDERR_BYTES {
                                let keep = MAX_STDERR_BYTES / 2;
                                let drop_n = collected.len().saturating_sub(keep);
                                collected.drain(..drop_n);
                            }
                            collected.extend_from_slice(&buf[..n]);
                        }
                    }
                }
                String::from_utf8_lossy(&collected).into_owned()
            });

            let outcome = tokio::select! {
                () = task_stop.cancelled() => {
                    let _ = child.kill().await;
                    let _ = child.wait().await;
                    ProcessState::Failed("Session stopped".to_owned())
                }
                () = sleep(MAX_SESSION_LIFETIME) => {
                    let _ = child.kill().await;
                    let _ = child.wait().await;
                    ProcessState::Failed("Session exceeded maximum duration".to_owned())
                }
                status = child.wait() => {
                    let stderr = stderr_task.await.unwrap_or_default();
                    match status {
                        Ok(exit) if exit.success() => ProcessState::Finished,
                        Ok(exit) => {
                            let reason = summarize_stderr(&stderr).unwrap_or_else(|| {
                                format!("ffmpeg exited with status {}", exit.code().unwrap_or(-1))
                            });
                            ProcessState::Failed(reason)
                        }
                        Err(err) => ProcessState::Failed(err.to_string()),
                    }
                }
            };
            // Release encoding slot as soon as ffmpeg exits before signaling done.
            drop(_slot);
            let _ = state_tx.send(outcome.clone());
            let _ = done_tx.send(true);
            tracing::info!(
                ?mode,
                encoder = %encoder_name,
                elapsed_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
                outcome = ?outcome,
                "video HLS session process finished"
            );
        });

        let handle = Arc::new(SessionHandle {
            owner_user,
            owner_session,
            item,
            dir: dir.clone(),
            stop,
            state: state_rx.clone(),
            done: done_rx,
            last_activity_ms: AtomicU64::new(0),
        });
        handle.touch(self.epoch);

        let playlist_path = dir.join(PLAYLIST_FILE);
        let deadline = Instant::now() + STARTUP_TIMEOUT;
        loop {
            if let Ok(text) = tokio::fs::read_to_string(&playlist_path).await
                && text.contains("#EXTINF:")
            {
                return Ok(handle);
            }
            let current = state_rx.borrow().clone();
            match current {
                ProcessState::Finished => {
                    if let Ok(text) = tokio::fs::read_to_string(&playlist_path).await
                        && text.contains("#EXTINF:")
                    {
                        return Ok(handle);
                    }
                    let _ = wait_done(&handle.done).await;
                    return Err((
                        VideoError::Invalid(
                            "FFmpeg finished without producing HLS segments".into(),
                        ),
                        None,
                    ));
                }
                ProcessState::Failed(reason) => {
                    // Reclaim a fresh slot after child exit so HW->SW fallback can reuse it.
                    let _ = wait_done(&handle.done).await;
                    let reclaimed = self.active_pool.try_acquire(plan.mode, plan.tier).ok();
                    return Err((VideoError::Invalid(reason), reclaimed));
                }
                ProcessState::Running => {}
            }
            if Instant::now() >= deadline {
                handle.stop.cancel();
                let _ = wait_done(&handle.done).await;
                return Err((
                    VideoError::Invalid("Timed out waiting for first HLS segment".into()),
                    None,
                ));
            }
            tokio::select! {
                () = sleep(Duration::from_millis(50)) => {}
                _ = state_rx.changed() => {}
            }
        }
    }

    async fn supports_readrate_burst(&self, ffmpeg: &Path) -> bool {
        *self
            .supports_readrate_burst
            .get_or_init(|| async {
                run(
                    ffmpeg,
                    &os_args(["-readrate_initial_burst", "8", "-version"]),
                    Duration::from_secs(3),
                )
                .await
                .is_ok_and(|out| out.success)
            })
            .await
    }

    fn get_handle(&self, session_id: Uuid) -> Result<Arc<SessionHandle>, VideoError> {
        let handle = self
            .lock_sessions()
            .get(&session_id)
            .cloned()
            .ok_or(VideoError::NotFound)?;
        handle.touch(self.epoch);
        Ok(handle)
    }

    /// Reads the raw ffmpeg HLS playlist for a session, waiting briefly if it is still starting.
    pub async fn read_playlist(&self, session_id: Uuid) -> Result<String, VideoError> {
        let handle = self.get_handle(session_id)?;
        let path = handle.dir.join(PLAYLIST_FILE);
        let mut state_rx = handle.state.clone();
        let deadline = Instant::now() + WAIT_READY_TIMEOUT;
        loop {
            match tokio::fs::read_to_string(&path).await {
                Ok(text) if !text.is_empty() => return Ok(text),
                Ok(_) => {}
                Err(err) if err.kind() == ErrorKind::NotFound => {}
                Err(_) => return Err(VideoError::NotFound),
            }
            if handle.stop.is_cancelled() {
                return Err(VideoError::NotFound);
            }
            if *state_rx.borrow() != ProcessState::Running {
                return match tokio::fs::read_to_string(&path).await {
                    Ok(text) if !text.is_empty() => Ok(text),
                    _ => Err(VideoError::NotFound),
                };
            }
            if Instant::now() >= deadline {
                return Err(VideoError::SegmentNotReady);
            }
            tokio::select! {
                () = handle.stop.cancelled() => return Err(VideoError::NotFound),
                () = sleep(Duration::from_millis(50)) => {}
                _ = state_rx.changed() => {}
            }
        }
    }

    /// Opens a produced HLS segment (or init.mp4), waiting briefly if ffmpeg is still writing it.
    pub async fn open_segment(
        &self,
        session_id: Uuid,
        file: &str,
    ) -> Result<(tokio::fs::File, u64), VideoError> {
        if !is_valid_segment_name(file) {
            return Err(VideoError::Invalid("Invalid segment filename".to_owned()));
        }
        let handle = self.get_handle(session_id)?;
        let path = handle.dir.join(file);
        let mut state_rx = handle.state.clone();
        let deadline = Instant::now() + WAIT_READY_TIMEOUT;
        loop {
            match tokio::fs::File::open(&path).await {
                Ok(opened) => {
                    let size = opened
                        .metadata()
                        .await
                        .map_err(|_| VideoError::NotFound)?
                        .len();
                    return Ok((opened, size));
                }
                Err(err) if err.kind() == ErrorKind::NotFound => {}
                Err(_) => return Err(VideoError::NotFound),
            }
            if handle.stop.is_cancelled() {
                return Err(VideoError::NotFound);
            }
            if *state_rx.borrow() != ProcessState::Running {
                // One final check in case the rename landed right before ffmpeg exited.
                return match tokio::fs::File::open(&path).await {
                    Ok(opened) => {
                        let size = opened
                            .metadata()
                            .await
                            .map_err(|_| VideoError::NotFound)?
                            .len();
                        Ok((opened, size))
                    }
                    Err(_) => Err(VideoError::NotFound),
                };
            }
            if Instant::now() >= deadline {
                return Err(VideoError::SegmentNotReady);
            }
            tokio::select! {
                () = handle.stop.cancelled() => return Err(VideoError::NotFound),
                () = sleep(Duration::from_millis(50)) => {}
                _ = state_rx.changed() => {}
            }
        }
    }

    /// Stops a session owned by `requester` (or any session if `is_admin`), cleaning temp files.
    /// Returns `Ok(true)` if an active session was removed, `Ok(false)` if it was already gone,
    /// and `Err(VideoError::NotFound)` if the session belongs to another user.
    pub async fn delete_session(
        &self,
        session_id: Uuid,
        requester: UserId,
        is_admin: bool,
    ) -> Result<bool, VideoError> {
        let removed = {
            let mut map = self.lock_sessions();
            let Some(existing) = map.get(&session_id) else {
                return Ok(false);
            };
            if !is_admin && existing.owner_user != requester {
                return Err(VideoError::NotFound);
            }
            map.remove(&session_id)
        };
        if let Some(handle) = removed {
            terminate_session(handle).await;
            return Ok(true);
        }
        Ok(false)
    }

    /// Terminates every active HLS session and removes their temp directories.
    pub async fn shutdown(&self) {
        self.shutdown.cancel();
        let drained: Vec<Arc<SessionHandle>> = {
            let mut map = self.lock_sessions();
            map.drain().map(|(_, handle)| handle).collect()
        };
        for handle in drained {
            terminate_session(handle).await;
        }
        let _ = tokio::fs::remove_dir(&self.sessions_root).await;
    }

    #[cfg(test)]
    pub(crate) fn session_count(&self) -> usize {
        self.lock_sessions().len()
    }
}

impl Drop for VideoService {
    fn drop(&mut self) {
        self.shutdown.cancel();
    }
}

async fn wait_done(done: &watch::Receiver<bool>) {
    let mut rx = done.clone();
    if *rx.borrow() {
        return;
    }
    let _ = timeout(TERMINATE_WAIT_TIMEOUT, async {
        while !*rx.borrow_and_update() {
            if rx.changed().await.is_err() {
                break;
            }
        }
    })
    .await;
}

async fn terminate_session(handle: Arc<SessionHandle>) {
    handle.stop.cancel();
    wait_done(&handle.done).await;
    let _ = tokio::fs::remove_dir_all(&handle.dir).await;
}

async fn clear_dir(dir: &Path) -> std::io::Result<()> {
    let mut entries = tokio::fs::read_dir(dir).await?;
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.is_dir() {
            let _ = tokio::fs::remove_dir_all(&path).await;
        } else {
            let _ = tokio::fs::remove_file(&path).await;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::video::decision::{
        ClientProfile, DecisionRequest, HlsSupport, RequestedMode, decide,
    };

    #[test]
    fn concurrency_slots_enforce_tier_limits_and_release_on_drop() {
        let pool = Arc::new(ActivePool::default());
        assert_eq!(
            pool.try_acquire(DeliveryMode::Transcode, PerfTier::Tiny)
                .err(),
            Some(VideoError::Disabled(PerfTier::Tiny))
        );
        let r_tiny = pool
            .try_acquire(DeliveryMode::Remux, PerfTier::Tiny)
            .unwrap();
        assert!(matches!(
            pool.try_acquire(DeliveryMode::Remux, PerfTier::Tiny),
            Err(VideoError::Busy {
                kind: "remux",
                active: 1,
                limit: 1,
                tier: PerfTier::Tiny
            })
        ));
        drop(r_tiny);

        let t1 = pool
            .try_acquire(DeliveryMode::Transcode, PerfTier::Lite)
            .unwrap();
        assert!(matches!(
            pool.try_acquire(DeliveryMode::Transcode, PerfTier::Lite),
            Err(VideoError::Busy {
                kind: "transcoding",
                active: 1,
                limit: 1,
                tier: PerfTier::Lite
            })
        ));
        // Remux uses its own lightweight pool and is not blocked by transcoding.
        let r1 = pool
            .try_acquire(DeliveryMode::Remux, PerfTier::Lite)
            .unwrap();
        assert_eq!(pool.counts(), (1, 1));
        drop(t1);
        drop(r1);
        assert_eq!(pool.counts(), (0, 0));
    }

    #[tokio::test]
    async fn hls_session_lifecycle_seek_delete_and_idle_reap() {
        let Some(ffmpeg) = locate("ffmpeg", None) else {
            return;
        };
        let Some(_ffprobe) = locate("ffprobe", None) else {
            return;
        };
        let work = tempfile::tempdir().unwrap();
        let data_dir = work.path().join("data");
        let clip = work.path().join("clip.mkv");
        let status = tokio::process::Command::new(&ffmpeg.path)
            .args([
                "-y",
                "-v",
                "error",
                "-f",
                "lavfi",
                "-i",
                "testsrc2=size=320x180:rate=24:duration=10",
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=440:duration=10",
                "-c:v",
                "libx264",
                "-g",
                "48",
                "-keyint_min",
                "48",
                "-sc_threshold",
                "0",
                "-pix_fmt",
                "yuv420p",
                "-c:a",
                "ac3",
            ])
            .arg(&clip)
            .status()
            .await
            .unwrap();
        assert!(status.success());

        let service = VideoService::with_timeouts(
            Some(&ffmpeg.path),
            data_dir.clone(),
            Duration::from_millis(250),
            Duration::from_millis(50),
        );
        let item = ItemId::generate();
        let user = UserId::generate();
        let other_user = UserId::generate();
        let auth_session = SessionId::generate();

        let file = File::open(&clip).unwrap();
        let media = service.probe_media(item, &file, Some("mkv")).await.unwrap();
        assert_eq!(media.info.container.as_deref(), Some("mkv"));
        assert!(
            data_dir
                .join("media-info")
                .join(format!("{item}.json"))
                .is_file()
        );
        // Second call serves from cache without failing.
        let cached = service.probe_media(item, &file, Some("mkv")).await.unwrap();
        assert_eq!(cached, media);

        let decision = decide(
            &media,
            &DecisionRequest {
                mode: RequestedMode::Auto,
                client: ClientProfile {
                    containers: vec!["mp4".into()],
                    video_codecs: vec!["h264".into()],
                    audio_codecs: vec!["aac".into()],
                    hls: HlsSupport::Mse,
                },
                ..DecisionRequest::default()
            },
        )
        .unwrap();
        assert_eq!(decision.mode, DeliveryMode::Remux);

        let started = service
            .start_hls(
                StartHlsRequest {
                    owner_user: user,
                    owner_session: auth_session,
                    item,
                    tier: PerfTier::Standard,
                    start_ms: 0,
                    max_bitrate_kbps: None,
                    decision: decision.clone(),
                    media: media.clone(),
                },
                file,
                None,
            )
            .await
            .unwrap();
        assert_eq!(started.start_ms, 0);
        let session_dir = data_dir
            .join("video-sessions")
            .join(started.session_id.to_string());
        assert!(session_dir.is_dir());

        let playlist = service.read_playlist(started.session_id).await.unwrap();
        assert!(playlist.contains("#EXT-X-MAP:URI=\"init.mp4\""));
        assert!(playlist.contains("seg00000.m4s"));

        let (_init, init_len) = service
            .open_segment(started.session_id, "init.mp4")
            .await
            .unwrap();
        assert!(init_len > 0);
        let (_seg0, seg0_len) = service
            .open_segment(started.session_id, "seg00000.m4s")
            .await
            .unwrap();
        assert!(seg0_len > 0);

        // Another non-admin user cannot delete this session.
        assert_eq!(
            service
                .delete_session(started.session_id, other_user, false)
                .await,
            Err(VideoError::NotFound)
        );
        assert_eq!(service.session_count(), 1);

        // Starting a seek session for the same owner+item replaces the old session and cleans its dir.
        let seek_file = File::open(&clip).unwrap();
        let sought = service
            .start_hls(
                StartHlsRequest {
                    owner_user: user,
                    owner_session: auth_session,
                    item,
                    tier: PerfTier::Standard,
                    start_ms: 5_000,
                    max_bitrate_kbps: None,
                    decision,
                    media,
                },
                seek_file,
                None,
            )
            .await
            .unwrap();
        assert_ne!(sought.session_id, started.session_id);
        assert_eq!(sought.start_ms, 4_000);
        assert!(!session_dir.exists());
        assert_eq!(service.session_count(), 1);

        // Idle reaper cleans up the session and temp dir after 250ms of inactivity.
        let sought_dir = data_dir
            .join("video-sessions")
            .join(sought.session_id.to_string());
        for _ in 0..30 {
            if service.session_count() == 0 && !sought_dir.exists() {
                break;
            }
            sleep(Duration::from_millis(50)).await;
        }
        assert_eq!(service.session_count(), 0);
        assert!(!sought_dir.exists());

        service.shutdown().await;
    }

    #[tokio::test]
    async fn transcode_falls_back_to_software_when_hardware_encoder_fails() {
        let Some(ffmpeg) = locate("ffmpeg", None) else {
            return;
        };
        let work = tempfile::tempdir().unwrap();
        let data_dir = work.path().join("data");
        let clip = work.path().join("clip.mp4");
        let status = tokio::process::Command::new(&ffmpeg.path)
            .args([
                "-y",
                "-v",
                "error",
                "-f",
                "lavfi",
                "-i",
                "testsrc2=size=640x360:rate=24:duration=6",
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=440:duration=6",
                "-c:v",
                "libx264",
                "-g",
                "48",
                "-pix_fmt",
                "yuv420p",
                "-c:a",
                "aac",
            ])
            .arg(&clip)
            .status()
            .await
            .unwrap();
        assert!(status.success());

        let service = VideoService::new(Some(&ffmpeg.path), data_dir);
        let item = ItemId::generate();
        let file = File::open(&clip).unwrap();
        let media = service.probe_media(item, &file, Some("mp4")).await.unwrap();
        let decision = decide(
            &media,
            &DecisionRequest {
                max_height: Some(240),
                mode: RequestedMode::Auto,
                client: ClientProfile {
                    containers: vec!["mp4".into()],
                    video_codecs: vec!["h264".into()],
                    audio_codecs: vec!["aac".into()],
                    hls: HlsSupport::Mse,
                },
                ..DecisionRequest::default()
            },
        )
        .unwrap();
        assert_eq!(decision.mode, DeliveryMode::Transcode);

        // Claim an unusable hardware encoder as verified to exercise runtime fallback.
        let fake_hw = HardwareReport {
            perf_tier: PerfTier::Standard,
            cpu: crate::report::CpuInfo {
                model: "test".into(),
                cores: 4,
                arch: "aarch64".into(),
            },
            memory_bytes: 8 * 1024 * 1024 * 1024,
            ffmpeg: crate::report::ToolInfo {
                found: true,
                path: None,
                version: None,
                source: None,
            },
            ffprobe: crate::report::ToolInfo {
                found: true,
                path: None,
                version: None,
                source: None,
            },
            hwaccels: vec![],
            encoders: vec![crate::report::EncoderCapability {
                id: "h264_nonexistent_hw".into(),
                codec: "h264".into(),
                method: crate::encoders::EncodeMethod::Nvenc,
                listed: true,
                verified: true,
                error: None,
            }],
            probed_at: palveni_core::clock::now(),
        };

        let started = service
            .start_hls(
                StartHlsRequest {
                    owner_user: UserId::generate(),
                    owner_session: SessionId::generate(),
                    item,
                    tier: PerfTier::Standard,
                    start_ms: 0,
                    max_bitrate_kbps: Some(1_000),
                    decision,
                    media,
                },
                file,
                Some(&fake_hw),
            )
            .await
            .unwrap();
        assert_eq!(started.encoder, "libx264");
        let playlist = service.read_playlist(started.session_id).await.unwrap();
        assert!(playlist.contains("seg00000.m4s"));
        service.shutdown().await;
    }
}

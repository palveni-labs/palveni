use async_trait::async_trait;
use palveni_core::{
    ItemId, JobStep, LibraryId, PerfTier, clock, music::LOUDNESS_JOB_KIND,
    settings::RuntimeSettings,
};
use palveni_db::{Db, music::loudness::LoudnessCandidate};
use palveni_jobs::{JobContext, JobError, JobHandler, JobOutcome};
use palveni_media::HardwareService;
use std::{fs::File, io, path::PathBuf, sync::Arc, time::Instant};
use tokio::sync::RwLock;

pub struct LoudnessJob {
    pub db: Db,
    pub settings: Arc<RwLock<RuntimeSettings>>,
    pub hardware: Arc<HardwareService>,
    pub boot_tier: PerfTier,
    pub ffmpeg: Option<PathBuf>,
}

fn unchanged(
    file: &File,
    track: &LoudnessCandidate,
    expected_size: Option<i64>,
) -> io::Result<bool> {
    let meta = file.metadata()?;
    let modified = meta
        .modified()
        .ok()
        .map(|time| clock::from_system_time(time).timestamp_millis());
    Ok(expected_size == i64::try_from(meta.len()).ok() && track.modified_at == modified)
}

impl LoudnessJob {
    async fn analyze(
        &self,
        ctx: &JobContext,
        program: &std::path::Path,
        tier: PerfTier,
        track: &LoudnessCandidate,
    ) -> Result<(), JobError> {
        let id: ItemId = track
            .id
            .parse()
            .map_err(|_| JobError::Failed("Invalid track ID".into()))?;
        let source = self
            .db
            .music()
            .track_location(id)
            .await?
            .ok_or_else(|| JobError::Failed("Track unavailable".into()))?;
        let error = |_: io::Error| JobError::Failed("Cannot read audio file".into());
        let segment = source.cue_frames.clone();
        let file = palveni_library::local::open_audio_input(source)
            .await
            .map_err(|_| JobError::Failed("Cannot open audio file".into()))?;
        // A virtual track reports the size of its WAV representation.
        let expected_size = if segment.is_some() {
            i64::try_from(file.metadata().map_err(error)?.len()).ok()
        } else {
            track.size_bytes
        };
        if !unchanged(&file, track, expected_size).map_err(error)? {
            return Err(JobError::Failed("Track changed since its last scan".into()));
        }
        let values = palveni_media::loudness::measure(
            program,
            file.try_clone().map_err(error)?,
            segment,
            tier,
            ctx.cancellation_token().cancelled(),
        )
        .await
        .map_err(|err| {
            if ctx.is_cancelled() {
                JobError::Cancelled
            } else {
                JobError::Failed(err.to_string())
            }
        })?;
        if ctx.is_cancelled() {
            return Err(JobError::Cancelled);
        }
        if !unchanged(&file, track, expected_size).map_err(error)? {
            return Err(JobError::Failed("Track changed during analysis".into()));
        }
        self.db
            .music()
            .save_loudness(id, track.revision, values)
            .await?;
        Ok(())
    }
}

#[async_trait]
impl JobHandler for LoudnessJob {
    fn kind(&self) -> &'static str {
        LOUDNESS_JOB_KIND
    }
    async fn run(
        &self,
        ctx: JobContext,
        payload: serde_json::Value,
    ) -> Result<Option<JobOutcome>, JobError> {
        let library: LibraryId =
            serde_json::from_value(payload).map_err(|e| JobError::InvalidPayload(e.to_string()))?;
        if !self.db.music().has_library(library).await? {
            return Err(JobError::Failed("Music library unavailable".into()));
        }
        let program = palveni_media::discovery::locate("ffmpeg", self.ffmpeg.as_deref())
            .ok_or_else(|| JobError::Failed("FFmpeg is unavailable".into()))?;
        let override_tier = self.settings.read().await.perf_tier;
        let tier = override_tier.unwrap_or(
            self.hardware
                .cached()
                .await
                .map_or(self.boot_tier, |r| r.perf_tier),
        );
        let (mut measured, mut skipped, mut failed) = (0_u64, 0_u64, 0_u64);
        let mut after = String::new();
        let start = Instant::now();
        loop {
            let batch = self.db.music().loudness_page(library, &after).await?;
            if batch.is_empty() {
                break;
            }
            for track in batch {
                if ctx.is_cancelled() {
                    return Err(JobError::Cancelled);
                }
                if track.measured_revision == Some(track.revision) {
                    skipped += 1;
                } else {
                    let step = JobStep::new("loudness.analyzing")
                        .with("measured", measured)
                        .with("skipped", skipped)
                        .with("failed", failed);
                    let message = format!(
                        "Analyzing loudness: {measured} measured, {skipped} already current, {failed} failed"
                    );
                    ctx.report(None, step, message);
                    match self.analyze(&ctx, &program.path, tier, &track).await {
                        Ok(()) => measured += 1,
                        Err(JobError::Cancelled) => return Err(JobError::Cancelled),
                        Err(err) => {
                            failed += 1;
                            tracing::warn!(item = %track.id, error=%err,"cannot measure track loudness");
                        }
                    }
                }
                after = track.id;
            }
        }
        let summary = format!("{measured} measured, {skipped} already current, {failed} failed");
        tracing::info!(measured,skipped,failed,elapsed_ms=?start.elapsed().as_millis(),tier=%tier,"loudness analysis finished");
        if failed > 0 {
            return Err(JobError::Failed(summary));
        }
        Ok(Some(JobOutcome {
            step: JobStep::new("loudness.summary")
                .with("measured", measured)
                .with("skipped", skipped),
            message: summary,
        }))
    }
}

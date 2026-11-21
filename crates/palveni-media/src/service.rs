//! Cached hardware report.
//!
//! Probing runs real encodes and can take tens of seconds, so the last
//! report is kept in memory and in the settings table; it is refreshed at
//! startup in the background and on explicit request.

use palveni_db::Db;
use palveni_db::settings::KEY_HARDWARE_REPORT;
use tokio::sync::{Mutex, RwLock};

use crate::report::{HardwareReport, ProbeOptions, probe_hardware};

pub struct HardwareService {
    pub audio: crate::audio::AudioTranscoder,
    pub subtitles: crate::subtitles::EmbeddedSubtitles,
    db: Db,
    options: ProbeOptions,
    cache: RwLock<Option<HardwareReport>>,
    probing: Mutex<()>,
}

impl HardwareService {
    pub fn new(db: Db, options: ProbeOptions) -> Self {
        HardwareService {
            audio: crate::audio::AudioTranscoder::new(options.ffmpeg_path.as_deref()),
            subtitles: crate::subtitles::EmbeddedSubtitles::new(options.ffmpeg_path.as_deref()),
            db,
            options,
            cache: RwLock::new(None),
            probing: Mutex::new(()),
        }
    }

    pub(crate) fn configured_ffmpeg(&self) -> Option<&std::path::Path> {
        self.options.ffmpeg_path.as_deref()
    }

    /// Loads the report persisted by a previous run, if any.
    pub async fn load_persisted(&self) {
        let stored = match self.db.settings().get(KEY_HARDWARE_REPORT).await {
            Ok(stored) => stored,
            Err(err) => {
                tracing::warn!(error = %err, "cannot read stored hardware report");
                return;
            }
        };
        let Some(text) = stored else { return };
        match serde_json::from_str::<HardwareReport>(&text) {
            Ok(report) => *self.cache.write().await = Some(report),
            Err(err) => {
                tracing::debug!(error = %err, "ignoring stored hardware report from another version")
            }
        }
    }

    /// Cached report, probing first if there is none or `refresh` is set.
    pub async fn get(&self, refresh: bool) -> HardwareReport {
        if !refresh && let Some(report) = self.cached().await {
            return report;
        }
        let _probe = self.probing.lock().await;
        if !refresh && let Some(report) = self.cached().await {
            return report;
        }
        let report = probe_hardware(&self.options).await;
        match serde_json::to_string(&report) {
            Ok(text) => {
                if let Err(err) = self.db.settings().set(KEY_HARDWARE_REPORT, &text).await {
                    tracing::warn!(error = %err, "cannot persist hardware report");
                }
            }
            Err(err) => tracing::warn!(error = %err, "cannot encode hardware report"),
        }
        *self.cache.write().await = Some(report.clone());
        report
    }

    /// Cached report without probing.
    pub async fn cached(&self) -> Option<HardwareReport> {
        self.cache.read().await.clone()
    }
}

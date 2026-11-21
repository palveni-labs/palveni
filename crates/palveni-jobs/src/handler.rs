use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use palveni_core::{JobId, JobStep, ServerEvent};
use palveni_db::DbError;
use tokio_util::sync::CancellationToken;

use crate::service::{Shared, lock};

/// Progress events are published at most this often per job (the final
/// state is always published).
const PUBLISH_INTERVAL: Duration = Duration::from_millis(250);

#[derive(Debug, thiserror::Error)]
pub enum JobError {
    #[error("job cancelled")]
    Cancelled,
    #[error("invalid job payload: {0}")]
    InvalidPayload(String),
    #[error("{0}")]
    Failed(String),
    #[error(transparent)]
    Db(#[from] DbError),
}

/// Executes one kind of job.
#[async_trait]
pub trait JobHandler: Send + Sync + 'static {
    /// The `kind` string this handler runs, e.g. `"library.scan"`.
    fn kind(&self) -> &'static str;

    /// Runs the job. `Ok(Some(outcome))` becomes the job's final `step` and
    /// `message` (a result summary). Long-running handlers should check
    /// [`JobContext::is_cancelled`].
    async fn run(
        &self,
        ctx: JobContext,
        payload: serde_json::Value,
    ) -> Result<Option<JobOutcome>, JobError>;
}

/// Result summary of a successful job.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JobOutcome {
    pub step: JobStep,
    /// English rendering of `step`.
    pub message: String,
}

/// Handle given to a running job for progress reporting and cancellation.
#[derive(Clone)]
pub struct JobContext {
    job_id: JobId,
    shared: Arc<Shared>,
    cancel: CancellationToken,
    last_publish: Arc<std::sync::Mutex<Option<Instant>>>,
}

impl JobContext {
    pub(crate) fn new(job_id: JobId, shared: Arc<Shared>, cancel: CancellationToken) -> Self {
        JobContext {
            job_id,
            shared,
            cancel,
            last_publish: Arc::new(std::sync::Mutex::new(None)),
        }
    }

    pub fn job_id(&self) -> JobId {
        self.job_id
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancel.is_cancelled()
    }

    pub fn cancellation_token(&self) -> &CancellationToken {
        &self.cancel
    }

    /// Updates the live progress shown by `GET /jobs` and publishes a
    /// throttled `job.updated` event. `message` is the English rendering of
    /// `step`. Cheap and synchronous: progress is kept in memory; only
    /// state changes are written to the database.
    pub fn report(&self, progress: Option<f64>, step: JobStep, message: impl Into<String>) {
        let progress = progress
            .filter(|p| p.is_finite())
            .map(|p| p.clamp(0.0, 1.0));
        let snapshot = {
            let mut live = lock(&self.shared.live);
            let Some(job) = live.get_mut(&self.job_id) else {
                return;
            };
            job.progress = progress;
            job.step = Some(step);
            job.message = Some(message.into());
            job.clone()
        };
        let now = Instant::now();
        let due = {
            let mut last = lock(&self.last_publish);
            let due = match *last {
                None => true,
                Some(at) => now.duration_since(at) >= PUBLISH_INTERVAL,
            } || progress == Some(1.0);
            if due {
                *last = Some(now);
            }
            due
        };
        if due {
            self.shared.bus.publish(ServerEvent::JobUpdated(snapshot));
        }
    }
}

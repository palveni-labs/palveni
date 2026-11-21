use std::collections::{BTreeSet, HashMap};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, RwLock};
use std::time::Duration;

use palveni_core::{EventBus, Job, JobId, JobState, JobSubject, ServerEvent, clock};
use palveni_db::Db;
use palveni_db::jobs::{JobRecord, NewJobRecord};
use tokio::sync::{Notify, watch};
use tokio::task::{JoinHandle, JoinSet};
use tokio_util::sync::CancellationToken;

use crate::handler::{JobContext, JobError, JobHandler, JobOutcome};

/// How long the runner sleeps when idle before polling the queue anyway
/// (jobs inserted by another process are not signalled).
const IDLE_POLL: Duration = Duration::from_secs(30);
const ERROR_BACKOFF: Duration = Duration::from_secs(5);
/// Queued jobs considered per claim round.
const CLAIM_WINDOW: u64 = 64;

/// A job to enqueue.
#[derive(Clone, Debug)]
pub struct NewJob {
    pub kind: String,
    /// What the job is about (e.g. the library being scanned).
    pub subject: Option<JobSubject>,
    pub payload: serde_json::Value,
    /// Capability tags a worker needs (e.g. `"hw:hevc-encode"`); empty for
    /// jobs any worker can run.
    pub required_capabilities: Vec<String>,
    /// While a job with this key is queued or running, enqueueing another
    /// returns the existing one.
    pub dedupe_key: Option<String>,
}

pub(crate) struct Shared {
    pub(crate) db: Db,
    pub(crate) bus: EventBus,
    handlers: RwLock<HashMap<&'static str, Arc<dyn JobHandler>>>,
    /// Live snapshots of running jobs (progress is not persisted).
    pub(crate) live: Mutex<HashMap<JobId, Job>>,
    cancels: Mutex<HashMap<JobId, CancellationToken>>,
    capabilities: BTreeSet<String>,
    concurrency: watch::Sender<usize>,
    wake: Notify,
}

pub(crate) fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    // The protected maps stay consistent even if a holder panicked.
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Queue front-end and runner. Cheap to clone.
#[derive(Clone)]
pub struct JobService {
    shared: Arc<Shared>,
}

impl JobService {
    /// `concurrency` is the maximum number of jobs running at once
    /// (see `PerfTier::job_concurrency`).
    pub fn new(db: Db, bus: EventBus, concurrency: usize) -> Self {
        Self::with_capabilities(db, bus, concurrency, BTreeSet::new())
    }

    /// Like [`JobService::new`], also declaring the capability tags this
    /// process can satisfy.
    pub fn with_capabilities(
        db: Db,
        bus: EventBus,
        concurrency: usize,
        capabilities: BTreeSet<String>,
    ) -> Self {
        JobService {
            shared: Arc::new(Shared {
                db,
                bus,
                handlers: RwLock::new(HashMap::new()),
                live: Mutex::new(HashMap::new()),
                cancels: Mutex::new(HashMap::new()),
                capabilities,
                concurrency: watch::channel(concurrency.max(1)).0,
                wake: Notify::new(),
            }),
        }
    }

    /// Registers the handler for `handler.kind()`. Register everything
    /// before starting the runner.
    pub fn register(&self, handler: Arc<dyn JobHandler>) {
        let mut handlers = self
            .shared
            .handlers
            .write()
            .unwrap_or_else(PoisonError::into_inner);
        handlers.insert(handler.kind(), handler);
    }

    /// Persists a job and wakes the runner. Returns the existing job when
    /// an active job with the same dedupe key exists.
    pub async fn enqueue(&self, new: NewJob) -> Result<Job, JobError> {
        self.enqueue_status(new).await.map(|(job, _)| job)
    }

    /// Manual scans and filesystem notifications share one active job per library.
    pub async fn enqueue_library_scan(
        &self,
        id: palveni_core::LibraryId,
        name: String,
    ) -> Result<(Job, bool), JobError> {
        use palveni_core::model::{SCAN_JOB_KIND, ScanJobPayload};
        let payload = serde_json::to_value(ScanJobPayload { library_id: id })
            .map_err(|e| JobError::InvalidPayload(e.to_string()))?;
        self.enqueue_status(NewJob {
            kind: SCAN_JOB_KIND.to_owned(),
            subject: Some(palveni_core::JobSubject::library(id, name)),
            payload,
            required_capabilities: Vec::new(),
            dedupe_key: Some(format!("{SCAN_JOB_KIND}:{id}")),
        })
        .await
    }

    async fn enqueue_status(&self, new: NewJob) -> Result<(Job, bool), JobError> {
        let enqueued = self
            .shared
            .db
            .jobs()
            .enqueue(NewJobRecord {
                kind: new.kind,
                subject: new.subject,
                payload: new.payload,
                required_capabilities: new.required_capabilities,
                dedupe_key: new.dedupe_key,
            })
            .await?;
        let job = enqueued.record.job;
        if enqueued.created {
            self.shared
                .bus
                .publish(ServerEvent::JobUpdated(job.clone()));
            self.shared.wake.notify_one();
            Ok((job, true))
        } else {
            // Prefer the live snapshot so the caller sees current progress.
            Ok((
                lock(&self.shared.live).get(&job.id).cloned().unwrap_or(job),
                false,
            ))
        }
    }

    /// Most recent jobs, with live progress for running ones.
    pub async fn list_recent(&self, limit: u64) -> Result<Vec<Job>, JobError> {
        let mut jobs = self.shared.db.jobs().list_recent(limit).await?;
        let live = lock(&self.shared.live);
        for job in &mut jobs {
            if let Some(snapshot) = live.get(&job.id) {
                *job = snapshot.clone();
            }
        }
        Ok(jobs)
    }

    /// Running jobs finish naturally if the new limit is lower.
    pub fn set_concurrency(&self, concurrency: usize) {
        self.shared.concurrency.send_replace(concurrency.max(1));
    }

    /// Requests cancellation of a running job. Returns whether it was
    /// running in this process.
    pub fn cancel(&self, id: JobId) -> bool {
        match lock(&self.shared.cancels).get(&id) {
            Some(token) => {
                token.cancel();
                true
            }
            None => false,
        }
    }

    pub async fn request_cancel(&self, id: JobId) -> Result<(), palveni_db::DbError> {
        let jobs = self.shared.db.jobs();
        let cancelled_queued = jobs.cancel_queued(id).await?;
        let record = jobs
            .get(id)
            .await?
            .ok_or(palveni_db::DbError::NotFound("job"))?;
        if cancelled_queued {
            self.shared.bus.publish(ServerEvent::JobUpdated(record.job));
        } else if !record.job.state.is_terminal() && !self.cancel(id) {
            return Err(palveni_db::DbError::Conflict(
                "The job is running on another worker".into(),
            ));
        }
        Ok(())
    }

    /// Number of jobs currently executing in this process.
    pub fn running_count(&self) -> usize {
        lock(&self.shared.live).len()
    }

    /// Starts the runner loop. It stops claiming work when `shutdown` is
    /// cancelled; running jobs see the cancellation through their context.
    pub fn spawn_runner(&self, shutdown: CancellationToken) -> JoinHandle<()> {
        let shared = Arc::clone(&self.shared);
        tokio::spawn(run_loop(shared, shutdown))
    }

    /// Waits until no job is running, or `timeout` elapses.
    pub async fn wait_idle(&self, timeout: Duration) {
        let deadline = tokio::time::Instant::now() + timeout;
        while self.running_count() > 0 && tokio::time::Instant::now() < deadline {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
}

async fn run_loop(shared: Arc<Shared>, shutdown: CancellationToken) {
    match shared.db.jobs().requeue_interrupted().await {
        Ok(0) => {}
        Ok(count) => tracing::info!(count, "requeued jobs interrupted by the previous shutdown"),
        Err(err) => tracing::error!(error = %err, "cannot requeue interrupted jobs"),
    }
    let mut limit = shared.concurrency.subscribe();
    let mut tasks = JoinSet::new();
    while !shutdown.is_cancelled() {
        while tasks.try_join_next().is_some() {}
        if tasks.len() >= *limit.borrow_and_update() {
            tokio::select! {
                _ = shutdown.cancelled() => break,
                _ = limit.changed() => {},
                _ = tasks.join_next() => {},
            }
            continue;
        }
        match claim_next(&shared, &shutdown).await {
            Ok(Some((record, handler, job_token))) => {
                let job_shared = Arc::clone(&shared);
                let shutdown_token = shutdown.clone();
                tasks.spawn(execute(
                    job_shared,
                    record,
                    handler,
                    job_token,
                    shutdown_token,
                ));
            }
            Ok(None) => {
                tokio::select! {
                    _ = shutdown.cancelled() => break,
                    _ = shared.wake.notified() => {}
                    _ = tokio::time::sleep(IDLE_POLL) => {}
                }
            }
            Err(err) => {
                tracing::error!(error = %err, "job runner cannot read the queue");
                tokio::select! {
                    _ = shutdown.cancelled() => break,
                    _ = tokio::time::sleep(ERROR_BACKOFF) => {}
                }
            }
        }
    }
    tasks.detach_all();
    tracing::debug!("job runner stopped");
}

async fn claim_next(
    shared: &Shared,
    shutdown: &CancellationToken,
) -> Result<Option<(JobRecord, Arc<dyn JobHandler>, CancellationToken)>, JobError> {
    let queued = shared.db.jobs().list_queued(CLAIM_WINDOW).await?;
    for record in queued {
        let runnable_here = record
            .required_capabilities
            .iter()
            .all(|cap| shared.capabilities.contains(cap));
        if !runnable_here {
            continue;
        }
        let handler = shared
            .handlers
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .get(record.job.kind.as_str())
            .cloned();
        let now = clock::now();
        let id = record.job.id;
        let token = shutdown.child_token();
        // Expose the token before the database can report this job as running.
        lock(&shared.cancels).insert(id, token.clone());
        let claimed = shared.db.jobs().try_claim(id, now).await;
        if !matches!(claimed, Ok(true)) {
            lock(&shared.cancels).remove(&id);
            claimed?;
            continue;
        }
        let Some(handler) = handler else {
            lock(&shared.cancels).remove(&id);
            // Fail it so an unknown kind cannot block the queue forever.
            let mut job = record.job.clone();
            let error = format!("no handler for job kind `{}`", job.kind);
            apply_outcome(&mut job, Err(JobError::Failed(error)), clock::now());
            shared.db.jobs().finish(&job).await?;
            shared.bus.publish(ServerEvent::JobUpdated(job));
            continue;
        };
        let mut record = record;
        record.job.state = JobState::Running;
        record.job.started_at = Some(now);
        record.attempts += 1;
        return Ok(Some((record, handler, token)));
    }
    Ok(None)
}

async fn execute(
    shared: Arc<Shared>,
    record: JobRecord,
    handler: Arc<dyn JobHandler>,
    cancel: CancellationToken,
    shutdown: CancellationToken,
) {
    let job_id = record.job.id;
    lock(&shared.live).insert(job_id, record.job.clone());
    shared
        .bus
        .publish(ServerEvent::JobUpdated(record.job.clone()));
    tracing::info!(job = %job_id, kind = %record.job.kind, "job started");

    let ctx = JobContext::new(job_id, Arc::clone(&shared), cancel);
    let payload = record.payload.clone();
    let task_handler = Arc::clone(&handler);
    // A separate task turns a panicking handler into an ordinary failure.
    let outcome = match tokio::spawn(async move { task_handler.run(ctx, payload).await }).await {
        Ok(result) => result,
        Err(join_error) if join_error.is_panic() => {
            Err(JobError::Failed("job handler panicked".to_owned()))
        }
        Err(_) => Err(JobError::Cancelled),
    };

    let snapshot = lock(&shared.live).remove(&job_id);
    let mut job = snapshot.unwrap_or_else(|| record.job.clone());

    let interrupted_by_shutdown =
        matches!(outcome, Err(JobError::Cancelled)) && shutdown.is_cancelled();
    if interrupted_by_shutdown {
        // Not a user cancellation: run it again on the next start.
        if let Err(err) = shared.db.jobs().requeue(job_id).await {
            tracing::warn!(job = %job_id, error = %err, "cannot requeue interrupted job");
        }
        lock(&shared.cancels).remove(&job_id);
        tracing::info!(job = %job_id, "job interrupted by shutdown; requeued");
        return;
    }

    apply_outcome(&mut job, outcome, clock::now());
    tracing::info!(job = %job_id, state = %job.state, error = ?job.error, "job finished");

    if let Err(err) = shared.db.jobs().finish(&job).await {
        tracing::error!(job = %job_id, error = %err, "cannot record job result");
    }
    lock(&shared.cancels).remove(&job_id);
    shared.bus.publish(ServerEvent::JobUpdated(job));
}

/// Applies a handler result to the job (contract, "任务" rules):
/// - success: progress 1, `step`/`message` become the summary when the
///   handler returned one;
/// - failure: `step` and `message` are cleared, the reason goes to `error`;
/// - cancellation: `step`, `message` and `error` are all cleared.
fn apply_outcome(
    job: &mut Job,
    outcome: Result<Option<JobOutcome>, JobError>,
    finished_at: chrono::DateTime<chrono::Utc>,
) {
    match outcome {
        Ok(summary) => {
            job.state = JobState::Succeeded;
            job.progress = Some(1.0);
            if let Some(summary) = summary {
                job.step = Some(summary.step);
                job.message = Some(summary.message);
            }
            job.error = None;
        }
        Err(JobError::Cancelled) => {
            job.state = JobState::Cancelled;
            job.step = None;
            job.message = None;
            job.error = None;
        }
        Err(err) => {
            job.state = JobState::Failed;
            job.step = None;
            job.message = None;
            job.error = Some(err.to_string());
        }
    }
    job.finished_at = Some(finished_at);
}

#[cfg(test)]
mod tests {
    use palveni_core::JobStep;

    use super::*;

    fn running_job() -> Job {
        Job {
            id: JobId::generate(),
            kind: "library.scan".into(),
            state: JobState::Running,
            subject: None,
            progress: Some(0.4),
            step: Some(
                JobStep::new("scan.indexing")
                    .with("done", 4u64)
                    .with("total", 10u64),
            ),
            message: Some("Updating index: 4 of 10".into()),
            error: None,
            created_at: clock::now(),
            started_at: Some(clock::now()),
            finished_at: None,
        }
    }

    #[test]
    fn success_takes_the_summary() {
        let mut job = running_job();
        let summary = JobOutcome {
            step: JobStep::new("scan.summary").with("added", 1u64),
            message: "1 added".into(),
        };
        apply_outcome(&mut job, Ok(Some(summary)), clock::now());
        assert_eq!(job.state, JobState::Succeeded);
        assert_eq!(job.progress, Some(1.0));
        assert_eq!(
            job.step.as_ref().map(|s| s.code.as_str()),
            Some("scan.summary")
        );
        assert_eq!(job.message.as_deref(), Some("1 added"));
        assert!(job.error.is_none());
        assert!(job.finished_at.is_some());
    }

    #[test]
    fn failure_clears_step_and_message() {
        let mut job = running_job();
        apply_outcome(
            &mut job,
            Err(JobError::Failed("disk gone".into())),
            clock::now(),
        );
        assert_eq!(job.state, JobState::Failed);
        assert!(job.step.is_none());
        assert!(job.message.is_none());
        assert_eq!(job.error.as_deref(), Some("disk gone"));
    }

    #[test]
    fn cancellation_clears_everything() {
        let mut job = running_job();
        apply_outcome(&mut job, Err(JobError::Cancelled), clock::now());
        assert_eq!(job.state, JobState::Cancelled);
        assert!(job.step.is_none());
        assert!(job.message.is_none());
        assert!(job.error.is_none());
    }
}

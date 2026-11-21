//! Adapter that runs library scans as background jobs. It lives here
//! because `palveni-library` and `palveni-jobs` are siblings and must not
//! depend on each other.

use async_trait::async_trait;
use palveni_jobs::{JobContext, JobError, JobHandler, JobOutcome};
use palveni_library::{SCAN_JOB_KIND, ScanError, ScanJobPayload, ScanObserver, ScanStep, Scanner};

pub struct ScanJobHandler {
    scanner: Scanner,
}

impl ScanJobHandler {
    pub fn new(scanner: Scanner) -> Self {
        ScanJobHandler { scanner }
    }
}

struct JobObserver<'a> {
    ctx: &'a JobContext,
}

impl ScanObserver for JobObserver<'_> {
    fn progress(&self, fraction: Option<f64>, step: &ScanStep) {
        self.ctx
            .report(fraction, step.to_job_step(), step.message());
    }

    fn is_cancelled(&self) -> bool {
        self.ctx.is_cancelled()
    }
}

#[async_trait]
impl JobHandler for ScanJobHandler {
    fn kind(&self) -> &'static str {
        SCAN_JOB_KIND
    }

    async fn run(
        &self,
        ctx: JobContext,
        payload: serde_json::Value,
    ) -> Result<Option<JobOutcome>, JobError> {
        let payload: ScanJobPayload =
            serde_json::from_value(payload).map_err(|e| JobError::InvalidPayload(e.to_string()))?;
        let observer = JobObserver { ctx: &ctx };
        match self
            .scanner
            .scan_reviewed(
                payload.library_id,
                &observer,
                payload.review_revision.as_deref(),
            )
            .await
        {
            Ok(summary) => {
                let step = summary.step();
                Ok(Some(JobOutcome {
                    step: step.to_job_step(),
                    message: step.message(),
                }))
            }
            Err(ScanError::Cancelled) => Err(JobError::Cancelled),
            Err(err) => Err(JobError::Failed(err.to_string())),
        }
    }
}

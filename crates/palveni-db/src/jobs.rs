//! Persistent job queue (ops store).

use chrono::{DateTime, Utc};
use palveni_core::{Job, JobId, JobState, JobStep, JobSubject, clock};
use sea_orm::sea_query::Expr;
use sea_orm::{ActiveValue, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect};

use crate::Db;
use crate::convert::{opt_ts, parse_id, ts};
use crate::entities::jobs;
use crate::error::{DbError, is_unique_violation};

pub struct Jobs<'a> {
    pub(crate) db: &'a Db,
}

#[derive(Clone, Debug)]
pub struct NewJobRecord {
    pub kind: String,
    /// Fixed for the lifetime of the job.
    pub subject: Option<JobSubject>,
    pub payload: serde_json::Value,
    pub required_capabilities: Vec<String>,
    /// At most one queued-or-running job may exist per key.
    pub dedupe_key: Option<String>,
}

/// A job plus the fields that are not part of the public `Job` shape.
#[derive(Clone, Debug)]
pub struct JobRecord {
    pub job: Job,
    pub payload: serde_json::Value,
    pub required_capabilities: Vec<String>,
    pub attempts: i32,
    pub dedupe_key: Option<String>,
}

/// Outcome of [`Jobs::enqueue`].
#[derive(Clone, Debug)]
pub struct Enqueued {
    pub record: JobRecord,
    /// `false` when an active job with the same dedupe key was returned.
    pub created: bool,
}

fn encode_step(step: Option<&JobStep>) -> Result<Option<String>, DbError> {
    step.map(|s| {
        serde_json::to_string(s).map_err(|e| DbError::Internal(format!("cannot encode step: {e}")))
    })
    .transpose()
}

impl Jobs<'_> {
    pub async fn enqueue(&self, new: NewJobRecord) -> Result<Enqueued, DbError> {
        let now = clock::now();
        let now_ms = clock::to_millis(now);
        let id = JobId::generate();
        let payload_text = serde_json::to_string(&new.payload)
            .map_err(|e| DbError::Internal(format!("cannot encode job payload: {e}")))?;
        let caps_text = serde_json::to_string(&new.required_capabilities)
            .map_err(|e| DbError::Internal(format!("cannot encode capabilities: {e}")))?;
        let subject = new.subject.clone();
        let row = jobs::ActiveModel {
            id: ActiveValue::Set(id.to_string()),
            kind: ActiveValue::Set(new.kind.clone()),
            payload: ActiveValue::Set(payload_text),
            required_capabilities: ActiveValue::Set(caps_text),
            state: ActiveValue::Set(JobState::Queued.as_str().to_owned()),
            subject_type: ActiveValue::Set(subject.as_ref().map(|s| s.subject_type.clone())),
            subject_id: ActiveValue::Set(subject.as_ref().map(|s| s.id.clone())),
            subject_name: ActiveValue::Set(subject.as_ref().and_then(|s| s.name.clone())),
            progress: ActiveValue::Set(None),
            step: ActiveValue::Set(None),
            message: ActiveValue::Set(None),
            last_error: ActiveValue::Set(None),
            attempts: ActiveValue::Set(0),
            dedupe_key: ActiveValue::Set(new.dedupe_key.clone()),
            active_key: ActiveValue::Set(new.dedupe_key.clone()),
            created_at: ActiveValue::Set(now_ms),
            started_at: ActiveValue::Set(None),
            finished_at: ActiveValue::Set(None),
            updated_at: ActiveValue::Set(now_ms),
        };
        match jobs::Entity::insert(row)
            .exec_without_returning(self.db.ops())
            .await
        {
            Ok(_) => Ok(Enqueued {
                record: JobRecord {
                    job: Job {
                        id,
                        kind: new.kind,
                        state: JobState::Queued,
                        subject,
                        progress: None,
                        step: None,
                        message: None,
                        error: None,
                        created_at: now,
                        started_at: None,
                        finished_at: None,
                    },
                    payload: new.payload,
                    required_capabilities: new.required_capabilities,
                    attempts: 0,
                    dedupe_key: new.dedupe_key,
                },
                created: true,
            }),
            Err(err) if is_unique_violation(&err) && new.dedupe_key.is_some() => {
                let existing = jobs::Entity::find()
                    .filter(jobs::Column::ActiveKey.eq(new.dedupe_key.clone()))
                    .one(self.db.ops())
                    .await?;
                match existing {
                    Some(row) => Ok(Enqueued {
                        record: to_record(row)?,
                        created: false,
                    }),
                    // The active job finished between our insert and lookup.
                    None => Err(DbError::Conflict(
                        "a matching job just finished; try again".to_owned(),
                    )),
                }
            }
            Err(err) => Err(err.into()),
        }
    }

    pub async fn get(&self, id: JobId) -> Result<Option<JobRecord>, DbError> {
        jobs::Entity::find_by_id(id.to_string())
            .one(self.db.ops())
            .await?
            .map(to_record)
            .transpose()
    }

    /// Most recent jobs first.
    pub async fn list_recent(&self, limit: u64) -> Result<Vec<Job>, DbError> {
        jobs::Entity::find()
            .order_by_desc(jobs::Column::CreatedAt)
            .order_by_desc(jobs::Column::Id)
            .limit(limit)
            .all(self.db.ops())
            .await?
            .into_iter()
            .map(|row| to_record(row).map(|r| r.job))
            .collect()
    }

    /// Oldest queued jobs first.
    pub async fn list_queued(&self, limit: u64) -> Result<Vec<JobRecord>, DbError> {
        jobs::Entity::find()
            .filter(jobs::Column::State.eq(JobState::Queued.as_str()))
            .order_by_asc(jobs::Column::CreatedAt)
            .order_by_asc(jobs::Column::Id)
            .limit(limit)
            .all(self.db.ops())
            .await?
            .into_iter()
            .map(to_record)
            .collect()
    }

    /// Atomically moves a queued job to running. `false` if another runner
    /// (or a cancellation) got there first.
    pub async fn try_claim(&self, id: JobId, at: DateTime<Utc>) -> Result<bool, DbError> {
        let at_ms = clock::to_millis(at);
        let attempts = sea_orm::sea_query::ExprTrait::add(Expr::col(jobs::Column::Attempts), 1i32);
        let result = jobs::Entity::update_many()
            .col_expr(jobs::Column::State, Expr::val(JobState::Running.as_str()))
            .col_expr(jobs::Column::StartedAt, Expr::val(at_ms))
            .col_expr(jobs::Column::UpdatedAt, Expr::val(at_ms))
            .col_expr(jobs::Column::Attempts, attempts)
            .filter(jobs::Column::Id.eq(id.to_string()))
            .filter(jobs::Column::State.eq(JobState::Queued.as_str()))
            .exec(self.db.ops())
            .await?;
        Ok(result.rows_affected == 1)
    }

    /// Persists a job's terminal snapshot (state, progress, step, message,
    /// error, finish time) and releases its dedupe key.
    pub async fn finish(&self, job: &Job) -> Result<(), DbError> {
        let finished_ms = clock::to_millis(job.finished_at.unwrap_or_else(clock::now));
        jobs::Entity::update_many()
            .col_expr(jobs::Column::State, Expr::val(job.state.as_str()))
            .col_expr(jobs::Column::Progress, Expr::val(job.progress))
            .col_expr(
                jobs::Column::Step,
                Expr::val(encode_step(job.step.as_ref())?),
            )
            .col_expr(jobs::Column::Message, Expr::val(job.message.clone()))
            .col_expr(jobs::Column::LastError, Expr::val(job.error.clone()))
            .col_expr(jobs::Column::FinishedAt, Expr::val(finished_ms))
            .col_expr(jobs::Column::UpdatedAt, Expr::val(finished_ms))
            .col_expr(jobs::Column::ActiveKey, Expr::val(Option::<String>::None))
            .filter(jobs::Column::Id.eq(job.id.to_string()))
            .exec(self.db.ops())
            .await?;
        Ok(())
    }

    /// Puts a running job back in the queue (used when it was interrupted
    /// by shutdown rather than cancelled by a user).
    pub async fn requeue(&self, id: JobId) -> Result<(), DbError> {
        let now = clock::to_millis(clock::now());
        jobs::Entity::update_many()
            .col_expr(jobs::Column::State, Expr::val(JobState::Queued.as_str()))
            .col_expr(jobs::Column::StartedAt, Expr::val(Option::<i64>::None))
            .col_expr(jobs::Column::Progress, Expr::val(Option::<f64>::None))
            .col_expr(jobs::Column::Step, Expr::val(Option::<String>::None))
            .col_expr(jobs::Column::Message, Expr::val(Option::<String>::None))
            .col_expr(jobs::Column::UpdatedAt, Expr::val(now))
            .filter(jobs::Column::Id.eq(id.to_string()))
            .filter(jobs::Column::State.eq(JobState::Running.as_str()))
            .exec(self.db.ops())
            .await?;
        Ok(())
    }

    /// Requeues every job left `running` by a previous process.
    pub async fn requeue_interrupted(&self) -> Result<u64, DbError> {
        let now = clock::to_millis(clock::now());
        let result = jobs::Entity::update_many()
            .col_expr(jobs::Column::State, Expr::val(JobState::Queued.as_str()))
            .col_expr(jobs::Column::StartedAt, Expr::val(Option::<i64>::None))
            .col_expr(jobs::Column::Progress, Expr::val(Option::<f64>::None))
            .col_expr(jobs::Column::Step, Expr::val(Option::<String>::None))
            .col_expr(jobs::Column::Message, Expr::val(Option::<String>::None))
            .col_expr(jobs::Column::UpdatedAt, Expr::val(now))
            .filter(jobs::Column::State.eq(JobState::Running.as_str()))
            .exec(self.db.ops())
            .await?;
        Ok(result.rows_affected)
    }
}

fn to_record(row: jobs::Model) -> Result<JobRecord, DbError> {
    let state = row
        .state
        .parse::<JobState>()
        .map_err(|_| DbError::Corrupt(format!("unknown job state `{}`", row.state)))?;
    let payload = serde_json::from_str(&row.payload)
        .map_err(|e| DbError::Corrupt(format!("job payload is not JSON: {e}")))?;
    let required_capabilities = serde_json::from_str(&row.required_capabilities)
        .map_err(|e| DbError::Corrupt(format!("job capabilities are not a JSON list: {e}")))?;
    let step = row
        .step
        .as_deref()
        .map(serde_json::from_str::<JobStep>)
        .transpose()
        .map_err(|e| DbError::Corrupt(format!("job step is not valid JSON: {e}")))?;
    let subject = match (row.subject_type, row.subject_id) {
        (Some(subject_type), Some(id)) => Some(JobSubject {
            subject_type,
            id,
            name: row.subject_name,
        }),
        _ => None,
    };
    Ok(JobRecord {
        job: Job {
            id: parse_id(&row.id, "job id")?,
            kind: row.kind,
            state,
            subject,
            progress: row.progress,
            step,
            message: row.message,
            error: row.last_error,
            created_at: ts(row.created_at)?,
            started_at: opt_ts(row.started_at)?,
            finished_at: opt_ts(row.finished_at)?,
        },
        payload,
        required_capabilities,
        attempts: row.attempts,
        dedupe_key: row.dedupe_key,
    })
}

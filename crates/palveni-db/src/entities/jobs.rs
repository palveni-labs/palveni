//! `jobs` (ops store).

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "jobs")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub kind: String,
    /// JSON document.
    pub payload: String,
    /// JSON array of capability tags a worker must have.
    pub required_capabilities: String,
    pub state: String,
    /// `JobSubject` snapshot taken at creation; the name survives deletion
    /// of the subject.
    pub subject_type: Option<String>,
    pub subject_id: Option<String>,
    pub subject_name: Option<String>,
    pub progress: Option<f64>,
    /// JSON `JobStep` (code + params).
    pub step: Option<String>,
    pub message: Option<String>,
    pub last_error: Option<String>,
    pub attempts: i32,
    pub dedupe_key: Option<String>,
    /// Equal to `dedupe_key` while the job is queued or running, `NULL`
    /// afterwards. A unique index on it makes "at most one active job per
    /// key" hold on every backend (all three allow repeated NULLs).
    pub active_key: Option<String>,
    pub created_at: i64,
    pub started_at: Option<i64>,
    pub finished_at: Option<i64>,
    pub updated_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

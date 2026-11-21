//! Domain types whose JSON form is the HTTP contract.
//!
//! Field names are camelCase on the wire, timestamps are RFC 3339 UTC, and
//! nullable fields are always present (serialised as `null`, never skipped).

use std::collections::BTreeMap;
use std::fmt;
use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::ids::{ItemId, JobId, LibraryId, UserId};

// ---------------------------------------------------------------------------
// Enumerations
// ---------------------------------------------------------------------------

#[derive(utoipa::ToSchema, Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LibraryKind {
    Video,
    Music,
    Photo,
    Book,
    Comic,
}

impl LibraryKind {
    pub const ALL: [LibraryKind; 5] = [
        LibraryKind::Video,
        LibraryKind::Music,
        LibraryKind::Photo,
        LibraryKind::Book,
        LibraryKind::Comic,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            LibraryKind::Video => "video",
            LibraryKind::Music => "music",
            LibraryKind::Photo => "photo",
            LibraryKind::Book => "book",
            LibraryKind::Comic => "comic",
        }
    }

    /// Kind of the leaf items a file-level scan creates in this library.
    pub const fn leaf_item_kind(self) -> ItemKind {
        match self {
            LibraryKind::Video => ItemKind::Video,
            LibraryKind::Music => ItemKind::Track,
            LibraryKind::Photo => ItemKind::Photo,
            LibraryKind::Book => ItemKind::Book,
            LibraryKind::Comic => ItemKind::Comic,
        }
    }
}

impl fmt::Display for LibraryKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for LibraryKind {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "video" => Ok(LibraryKind::Video),
            "music" => Ok(LibraryKind::Music),
            "photo" => Ok(LibraryKind::Photo),
            "book" => Ok(LibraryKind::Book),
            "comic" => Ok(LibraryKind::Comic),
            other => Err(crate::Error::invalid(format!(
                "unknown library kind `{other}`"
            ))),
        }
    }
}

/// A folder or media item in a library.
#[derive(utoipa::ToSchema, Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ItemKind {
    Folder,
    Video,
    Track,
    Photo,
    Book,
    Comic,
}

impl ItemKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            ItemKind::Folder => "folder",
            ItemKind::Video => "video",
            ItemKind::Track => "track",
            ItemKind::Photo => "photo",
            ItemKind::Book => "book",
            ItemKind::Comic => "comic",
        }
    }

    pub const fn is_folder(self) -> bool {
        matches!(self, ItemKind::Folder)
    }
}

impl fmt::Display for ItemKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for ItemKind {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "folder" => Ok(ItemKind::Folder),
            "video" => Ok(ItemKind::Video),
            "track" => Ok(ItemKind::Track),
            "photo" => Ok(ItemKind::Photo),
            "book" => Ok(ItemKind::Book),
            "comic" => Ok(ItemKind::Comic),
            other => Err(crate::Error::invalid(format!(
                "unknown item kind `{other}`"
            ))),
        }
    }
}

#[derive(utoipa::ToSchema, Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Admin,
    User,
}

impl Role {
    pub const fn as_str(self) -> &'static str {
        match self {
            Role::Admin => "admin",
            Role::User => "user",
        }
    }
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Role {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "admin" => Ok(Role::Admin),
            "user" => Ok(Role::User),
            other => Err(crate::Error::invalid(format!("unknown role `{other}`"))),
        }
    }
}

#[derive(utoipa::ToSchema, Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum JobState {
    Queued,
    Running,
    Succeeded,
    Failed,
    Cancelled,
}

impl JobState {
    pub const fn as_str(self) -> &'static str {
        match self {
            JobState::Queued => "queued",
            JobState::Running => "running",
            JobState::Succeeded => "succeeded",
            JobState::Failed => "failed",
            JobState::Cancelled => "cancelled",
        }
    }

    pub const fn is_terminal(self) -> bool {
        matches!(
            self,
            JobState::Succeeded | JobState::Failed | JobState::Cancelled
        )
    }

    /// Allowed transitions of the job state machine.
    ///
    /// `Running -> Queued` exists so jobs interrupted by a shutdown can be
    /// picked up again on the next start.
    pub const fn can_transition_to(self, next: JobState) -> bool {
        matches!(
            (self, next),
            (JobState::Queued, JobState::Running)
                | (JobState::Queued, JobState::Cancelled)
                | (JobState::Running, JobState::Succeeded)
                | (JobState::Running, JobState::Failed)
                | (JobState::Running, JobState::Cancelled)
                | (JobState::Running, JobState::Queued)
        )
    }
}

impl fmt::Display for JobState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for JobState {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "queued" => Ok(JobState::Queued),
            "running" => Ok(JobState::Running),
            "succeeded" => Ok(JobState::Succeeded),
            "failed" => Ok(JobState::Failed),
            "cancelled" => Ok(JobState::Cancelled),
            other => Err(crate::Error::invalid(format!(
                "unknown job state `{other}`"
            ))),
        }
    }
}

// ---------------------------------------------------------------------------
// Records
// ---------------------------------------------------------------------------

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub id: UserId,
    pub username: String,
    pub role: Role,
    pub created_at: DateTime<Utc>,
}

impl User {
    pub fn is_admin(&self) -> bool {
        self.role == Role::Admin
    }
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Library {
    pub id: LibraryId,
    pub name: String,
    pub kind: LibraryKind,
    /// Absolute paths on the server's filesystem.
    pub paths: Vec<String>,
    /// Number of live leaf items (folders excluded).
    pub item_count: i64,
    #[schema(required = true)]
    pub last_scan_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    pub id: ItemId,
    pub library_id: LibraryId,
    #[schema(required = true)]
    pub parent_id: Option<ItemId>,
    pub kind: ItemKind,
    /// File name without extension, or the folder name.
    pub title: String,
    /// Absolute path; the API layer clears it for non-admin callers.
    #[schema(required = true)]
    pub path: Option<String>,
    /// `None` for folders.
    #[schema(required = true)]
    pub size_bytes: Option<i64>,
    #[schema(required = true)]
    pub modified_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Subject types a job can refer to. Clients treat unknown types as a
/// generic job, so new types can be added freely.
pub mod subject_types {
    pub const LIBRARY: &str = "library";
}

/// The object a job is about. Fixed when the job is created; `name` is a
/// snapshot, so it survives deletion of the object.
#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobSubject {
    #[serde(rename = "type")]
    pub subject_type: String,
    pub id: String,
    #[schema(required = true)]
    pub name: Option<String>,
}

impl JobSubject {
    pub fn library(id: LibraryId, name: impl Into<String>) -> Self {
        JobSubject {
            subject_type: subject_types::LIBRARY.to_owned(),
            id: id.to_string(),
            name: Some(name.into()),
        }
    }
}

/// A step parameter: the contract allows strings and numbers only.
#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum StepValue {
    Number(i64),
    Text(String),
}

impl From<i64> for StepValue {
    fn from(value: i64) -> Self {
        StepValue::Number(value)
    }
}

impl From<u64> for StepValue {
    fn from(value: u64) -> Self {
        StepValue::Number(i64::try_from(value).unwrap_or(i64::MAX))
    }
}

impl From<usize> for StepValue {
    fn from(value: usize) -> Self {
        StepValue::Number(i64::try_from(value).unwrap_or(i64::MAX))
    }
}

impl From<&str> for StepValue {
    fn from(value: &str) -> Self {
        StepValue::Text(value.to_owned())
    }
}

impl From<String> for StepValue {
    fn from(value: String) -> Self {
        StepValue::Text(value)
    }
}

/// Machine-readable description of what a job is doing, for clients to
/// localise; `Job::message` carries the English rendering of the same step.
#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobStep {
    pub code: String,
    pub params: BTreeMap<String, StepValue>,
}

impl JobStep {
    pub fn new(code: impl Into<String>) -> Self {
        JobStep {
            code: code.into(),
            params: BTreeMap::new(),
        }
    }

    #[must_use]
    pub fn with(mut self, key: &str, value: impl Into<StepValue>) -> Self {
        self.params.insert(key.to_owned(), value.into());
        self
    }
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Job {
    pub id: JobId,
    /// Handler key, e.g. `"library.scan"`.
    pub kind: String,
    pub state: JobState,
    /// What the job is about; `None` for jobs without a subject.
    #[schema(required = true)]
    pub subject: Option<JobSubject>,
    /// 0.0 to 1.0, or `None` when it cannot be estimated.
    #[schema(required = true)]
    pub progress: Option<f64>,
    /// Current step; on success the result summary. `None` when the job
    /// failed or was cancelled.
    #[schema(required = true)]
    pub step: Option<JobStep>,
    /// English rendering of `step`.
    #[schema(required = true)]
    pub message: Option<String>,
    /// Failure reason; only set when `state` is `failed`.
    #[schema(required = true)]
    pub error: Option<String>,
    pub created_at: DateTime<Utc>,
    #[schema(required = true)]
    pub started_at: Option<DateTime<Utc>>,
    #[schema(required = true)]
    pub finished_at: Option<DateTime<Utc>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock;

    #[test]
    fn enums_use_contract_spelling() {
        assert_eq!(
            serde_json::to_string(&LibraryKind::Music).unwrap(),
            "\"music\""
        );
        assert_eq!(
            serde_json::to_string(&ItemKind::Track).unwrap(),
            "\"track\""
        );
        assert_eq!(serde_json::to_string(&Role::Admin).unwrap(), "\"admin\"");
        assert_eq!(
            serde_json::to_string(&JobState::Cancelled).unwrap(),
            "\"cancelled\""
        );
        for kind in LibraryKind::ALL {
            assert_eq!(kind.as_str().parse::<LibraryKind>().unwrap(), kind);
        }
    }

    #[test]
    fn item_nullable_fields_are_present() {
        let now = clock::now();
        let item = Item {
            id: ItemId::generate(),
            library_id: LibraryId::generate(),
            parent_id: None,
            kind: ItemKind::Folder,
            title: "Albums".into(),
            path: None,
            size_bytes: None,
            modified_at: None,
            created_at: now,
            updated_at: now,
        };
        let value = serde_json::to_value(&item).unwrap();
        let obj = value.as_object().unwrap();
        for key in ["parentId", "path", "sizeBytes", "modifiedAt"] {
            assert!(
                obj.get(key).is_some_and(serde_json::Value::is_null),
                "{key}"
            );
        }
        assert!(obj.contains_key("libraryId"));
        assert!(obj.contains_key("createdAt"));
    }

    #[test]
    fn job_json_has_subject_and_step() {
        let library = LibraryId::generate();
        let job = Job {
            id: JobId::generate(),
            kind: "library.scan".into(),
            state: JobState::Running,
            subject: Some(JobSubject::library(library, "Music")),
            progress: Some(0.5),
            step: Some(
                JobStep::new("scan.indexing")
                    .with("done", 5u64)
                    .with("total", 10u64),
            ),
            message: Some("Updating index: 5 of 10".into()),
            error: None,
            created_at: clock::now(),
            started_at: None,
            finished_at: None,
        };
        let value = serde_json::to_value(&job).unwrap();
        assert_eq!(value["subject"]["type"], "library");
        assert_eq!(value["subject"]["id"], library.to_string());
        assert_eq!(value["subject"]["name"], "Music");
        assert_eq!(value["step"]["code"], "scan.indexing");
        assert_eq!(value["step"]["params"]["done"], 5);
        assert_eq!(value["step"]["params"]["total"], 10);
        assert!(value["error"].is_null());
        let back: Job = serde_json::from_value(value).unwrap();
        assert_eq!(back, job);

        let bare = Job {
            subject: None,
            step: None,
            message: None,
            ..job
        };
        let value = serde_json::to_value(&bare).unwrap();
        assert!(value.get("subject").is_some_and(serde_json::Value::is_null));
        assert!(value.get("step").is_some_and(serde_json::Value::is_null));
    }

    #[test]
    fn step_values_are_numbers_or_strings() {
        let step = JobStep::new("x").with("n", 3usize).with("s", "text");
        assert_eq!(
            serde_json::to_string(&step).unwrap(),
            r#"{"code":"x","params":{"n":3,"s":"text"}}"#
        );
    }

    #[test]
    fn job_state_machine() {
        assert!(JobState::Queued.can_transition_to(JobState::Running));
        assert!(JobState::Running.can_transition_to(JobState::Succeeded));
        assert!(!JobState::Succeeded.can_transition_to(JobState::Running));
        assert!(!JobState::Failed.can_transition_to(JobState::Queued));
        assert!(JobState::Cancelled.is_terminal());
        assert!(!JobState::Running.is_terminal());
    }
}

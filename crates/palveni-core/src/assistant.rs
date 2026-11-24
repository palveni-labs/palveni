//! Declarative UI blocks, AI assistant conversations/runs, and unified approval tasks
//!.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::home::DeviceInfo;
use crate::model::JobStep;

// ---------------------------------------------------------------------------
// Declarative UI Blocks (`web/src/blocks/types.ts`)
// ---------------------------------------------------------------------------

#[derive(utoipa::ToSchema, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockIcon {
    Library,
    Video,
    Music,
    Photo,
    Book,
    Comic,
    Folder,
    Storage,
    Duplicate,
    Sparkles,
    Clock,
    Download,
    Transcode,
    Light,
    Thermometer,
    Scene,
    Check,
    Warning,
    Move,
    Restore,
    Shield,
    Trash,
}

#[derive(utoipa::ToSchema, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockTone {
    Default,
    Muted,
    Success,
    Warning,
    Danger,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum StatValue {
    Int(i64),
    Float(f64),
    Text(String),
}

#[derive(utoipa::ToSchema, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CollectionItemKind {
    Video,
    Music,
    Photo,
    Book,
    Comic,
    Folder,
}

#[derive(utoipa::ToSchema, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CollectionLayout {
    Grid,
    Row,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectionItem {
    pub id: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subtitle: Option<String>,
    pub kind: CollectionItemKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artwork_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub badge: Option<String>,
}

#[derive(utoipa::ToSchema, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CellFormat {
    Text,
    Number,
    Bytes,
    Date,
    Duration,
}

#[derive(utoipa::ToSchema, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ColumnAlign {
    Start,
    End,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TableColumn {
    pub key: String,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub align: Option<ColumnAlign>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<CellFormat>,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CellValue {
    Int(i64),
    Float(f64),
    Text(String),
    Null,
}

#[derive(utoipa::ToSchema, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeRelation {
    Identical,
    Similar,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeRow {
    pub id: String,
    pub before: String,
    pub after: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    pub selected: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thumbs: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relation: Option<ChangeRelation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeGroup {
    pub id: String,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub collapse_after: Option<u32>,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeColumns {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub change: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalEffect {
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<BlockIcon>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ApprovalEffectItem {
    Plain(String),
    Structured(ApprovalEffect),
}

#[derive(utoipa::ToSchema, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalRisk {
    Low,
    Medium,
    High,
}

#[derive(utoipa::ToSchema, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalStatus {
    Pending,
    Approved,
    Cancelled,
}

#[derive(utoipa::ToSchema, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProgressBlockState {
    Running,
    Succeeded,
    Failed,
}

#[derive(utoipa::ToSchema, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChartKind {
    Bar,
    Donut,
}

#[derive(utoipa::ToSchema, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChartUnit {
    Bytes,
    Count,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChartPoint {
    pub label: String,
    pub value: f64,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum Block {
    #[serde(rename_all = "camelCase")]
    Text {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        title: Option<String>,
        text: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tone: Option<BlockTone>,
    },
    #[serde(rename_all = "camelCase")]
    Stat {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        title: Option<String>,
        label: String,
        value: StatValue,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        unit: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        hint: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        icon: Option<BlockIcon>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tone: Option<BlockTone>,
    },
    #[serde(rename_all = "camelCase")]
    Collection {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        title: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        layout: Option<CollectionLayout>,
        items: Vec<CollectionItem>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        more: Option<u32>,
    },
    #[serde(rename_all = "camelCase")]
    Table {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        title: Option<String>,
        columns: Vec<TableColumn>,
        rows: Vec<BTreeMap<String, CellValue>>,
    },
    #[serde(rename_all = "camelCase")]
    ChangePreview {
        id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        title: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        summary: Option<String>,
        rows: Vec<ChangeRow>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        groups: Option<Vec<ChangeGroup>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        columns: Option<ChangeColumns>,
    },
    #[serde(rename_all = "camelCase")]
    Approval {
        id: String,
        title: String,
        effects: Vec<ApprovalEffectItem>,
        risk: ApprovalRisk,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        preview_id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        confirm_label: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        confirm_icon: Option<BlockIcon>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        status: Option<ApprovalStatus>,
    },
    #[serde(rename_all = "camelCase")]
    Progress {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        title: Option<String>,
        label: String,
        #[schema(required = true)]
        value: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        detail: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        state: Option<ProgressBlockState>,
    },
    #[serde(rename_all = "camelCase")]
    Chart {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        title: Option<String>,
        kind: ChartKind,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        unit: Option<ChartUnit>,
        series: Vec<ChartPoint>,
    },
    #[serde(rename_all = "camelCase")]
    Device {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        title: Option<String>,
        device: DeviceInfo,
    },
}

// ---------------------------------------------------------------------------
// Assistant Conversations & Runs (`web/src/provisional/types.ts`)
// ---------------------------------------------------------------------------

#[derive(utoipa::ToSchema, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStepState {
    Running,
    Done,
    Failed,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunStep {
    pub id: String,
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    pub state: RunStepState,
}

#[derive(utoipa::ToSchema, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunState {
    Running,
    AwaitingApproval,
    Succeeded,
    Failed,
    Cancelled,
}

impl RunState {
    pub const fn as_str(self) -> &'static str {
        match self {
            RunState::Running => "running",
            RunState::AwaitingApproval => "awaiting_approval",
            RunState::Succeeded => "succeeded",
            RunState::Failed => "failed",
            RunState::Cancelled => "cancelled",
        }
    }

    pub fn from_db(raw: &str) -> Self {
        match raw {
            "running" => RunState::Running,
            "awaiting_approval" => RunState::AwaitingApproval,
            "failed" => RunState::Failed,
            "cancelled" => RunState::Cancelled,
            _ => RunState::Succeeded,
        }
    }
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunSubject {
    pub title: String,
    #[schema(required = true)]
    pub artwork_id: Option<String>,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssistantRun {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject: Option<RunSubject>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previews: Option<Vec<String>>,
    pub state: RunState,
    pub steps: Vec<RunStep>,
    pub blocks: Vec<Block>,
    #[schema(required = true)]
    pub task_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(utoipa::ToSchema, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageRole {
    User,
    Assistant,
}

impl MessageRole {
    pub const fn as_str(self) -> &'static str {
        match self {
            MessageRole::User => "user",
            MessageRole::Assistant => "assistant",
        }
    }

    pub fn from_db(raw: &str) -> Self {
        if raw == "assistant" {
            MessageRole::Assistant
        } else {
            MessageRole::User
        }
    }
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssistantMessage {
    pub id: String,
    pub role: MessageRole,
    pub text: String,
    pub created_at: DateTime<Utc>,
    #[schema(required = true)]
    pub run_id: Option<String>,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Conversation {
    pub id: String,
    pub messages: Vec<AssistantMessage>,
    pub runs: Vec<AssistantRun>,
}

#[derive(utoipa::ToSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssistantContext {
    #[serde(default)]
    pub page: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item_ids: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

#[derive(utoipa::ToSchema, Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SendMessageRequest {
    pub text: String,
    #[serde(default)]
    pub context: Option<AssistantContext>,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendMessageResponse {
    pub message: AssistantMessage,
    pub reply: AssistantMessage,
    pub run: AssistantRun,
}

#[derive(utoipa::ToSchema, Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ApproveRequest {
    #[serde(default)]
    pub selected_row_ids: Vec<String>,
}

#[derive(utoipa::ToSchema, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SuggestionIcon {
    Duplicate,
    Storage,
    Music,
    Subtitle,
    Scene,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Suggestion {
    pub id: String,
    pub text: String,
    pub prompt: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action_label: Option<String>,
    pub icon: SuggestionIcon,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SuggestionList {
    pub suggestions: Vec<Suggestion>,
}

// ---------------------------------------------------------------------------
// Unified Tasks (`web/src/provisional/types.ts`)
// ---------------------------------------------------------------------------

#[derive(utoipa::ToSchema, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskState {
    Queued,
    Running,
    AwaitingApproval,
    Succeeded,
    Failed,
    Cancelled,
}

#[derive(utoipa::ToSchema, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStepState {
    Pending,
    Running,
    Done,
    Failed,
    Skipped,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskStep {
    pub id: String,
    pub label: String,
    pub state: TaskStepState,
    #[schema(required = true)]
    pub detail: Option<String>,
    #[schema(required = true)]
    pub at: Option<DateTime<Utc>>,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskSubject {
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verb: Option<String>,
    #[schema(required = true)]
    pub detail: Option<String>,
    #[schema(required = true)]
    pub artwork_id: Option<String>,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskOrigin {
    pub module: String,
    pub label: String,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub state: TaskState,
    #[schema(required = true)]
    pub progress: Option<f64>,
    #[schema(required = true)]
    pub message: Option<String>,
    #[schema(required = true)]
    pub error: Option<String>,
    pub created_at: DateTime<Utc>,
    #[schema(required = true)]
    pub started_at: Option<DateTime<Utc>>,
    #[schema(required = true)]
    pub finished_at: Option<DateTime<Utc>>,
    pub steps: Vec<TaskStep>,
    pub result: Vec<Block>,
    #[schema(required = true)]
    pub origin: Option<TaskOrigin>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject: Option<TaskSubject>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rate: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub eta_seconds: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previews: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step: Option<JobStep>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub library_id: Option<String>,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskList {
    pub tasks: Vec<Task>,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackgroundRunResponse {
    pub run: AssistantRun,
    pub task: Task,
}

// ---------------------------------------------------------------------------
// Provider Configuration & Memories
// ---------------------------------------------------------------------------

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssistantPresetInfo {
    pub id: String,
    pub label: String,
    pub default_base_url: String,
    pub default_model: String,
    pub requires_api_key: bool,
}

fn default_redact_cloud_paths() -> bool {
    true
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssistantConfigResponse {
    pub preset: String,
    pub base_url: String,
    pub model: String,
    pub api_key_configured: bool,
    #[serde(default = "default_redact_cloud_paths")]
    pub redact_cloud_paths: bool,
    pub presets: Vec<AssistantPresetInfo>,
}

#[derive(utoipa::ToSchema, Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateAssistantConfigRequest {
    pub preset: String,
    #[serde(default)]
    pub base_url: String,
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default)]
    pub clear_api_key: bool,
    #[serde(default = "default_redact_cloud_paths")]
    pub redact_cloud_paths: bool,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssistantMemory {
    pub id: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssistantMemoryList {
    pub memories: Vec<AssistantMemory>,
}

#[derive(utoipa::ToSchema, Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateAssistantMemoryRequest {
    pub content: String,
}

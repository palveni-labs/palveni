//! Multi-user family accounts, per-library access policies, device sessions,
//! audit log records, portable database snapshots, and external history/playlist imports.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::ids::{BackupId, DeviceId, LibraryId, SessionId, UserId};
use crate::model::{LibraryKind, Role};

pub const DEFAULT_DAILY_AI_LIMIT: i32 = 100;
pub const MAX_DAILY_AI_LIMIT: i32 = 100_000;
pub const MAX_BACKUP_NAME_CHARS: usize = 128;
pub const MAX_IMPORT_BYTES: usize = 10 * 1024 * 1024;

const fn default_true() -> bool {
    true
}

const fn default_member_role() -> Role {
    Role::User
}

const fn default_auto_source() -> ExternalImportSource {
    ExternalImportSource::Auto
}

/// Per-account capability and quota policy.
#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserPolicy {
    pub all_libraries: bool,
    pub allow_download: bool,
    pub allow_transcode: bool,
    pub daily_ai_limit: i32,
}

impl Default for UserPolicy {
    fn default() -> Self {
        Self {
            all_libraries: true,
            allow_download: true,
            allow_transcode: true,
            daily_ai_limit: DEFAULT_DAILY_AI_LIMIT,
        }
    }
}

/// Partial update to a [`UserPolicy`].
#[derive(utoipa::ToSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UserPolicyPatch {
    #[serde(default)]
    pub all_libraries: Option<bool>,
    #[serde(default)]
    pub allow_download: Option<bool>,
    #[serde(default)]
    pub allow_transcode: Option<bool>,
    #[serde(default)]
    pub daily_ai_limit: Option<i32>,
}

impl UserPolicy {
    #[must_use]
    pub fn apply_patch(&self, patch: &UserPolicyPatch) -> Self {
        Self {
            all_libraries: patch.all_libraries.unwrap_or(self.all_libraries),
            allow_download: patch.allow_download.unwrap_or(self.allow_download),
            allow_transcode: patch.allow_transcode.unwrap_or(self.allow_transcode),
            daily_ai_limit: patch.daily_ai_limit.unwrap_or(self.daily_ai_limit),
        }
    }
}

/// Administrator view of an account including its policy, allowed libraries and active sessions.
#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedUser {
    pub id: UserId,
    pub username: String,
    pub role: Role,
    pub disabled: bool,
    pub policy: UserPolicy,
    pub allowed_library_ids: Vec<LibraryId>,
    pub active_sessions: u64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserList {
    pub users: Vec<ManagedUser>,
}

#[derive(utoipa::ToSchema, Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateUserRequest {
    pub username: String,
    pub password: String,
    #[serde(default = "default_member_role")]
    pub role: Role,
    #[serde(default)]
    pub policy: Option<UserPolicy>,
    #[serde(default)]
    pub allowed_library_ids: Vec<LibraryId>,
}

#[derive(utoipa::ToSchema, Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateUserRequest {
    #[serde(default)]
    pub role: Option<Role>,
    #[serde(default)]
    pub disabled: Option<bool>,
    #[serde(default)]
    pub password: Option<String>,
    #[serde(default)]
    pub policy: Option<UserPolicyPatch>,
    #[serde(default)]
    pub allowed_library_ids: Option<Vec<LibraryId>>,
}

#[derive(utoipa::ToSchema, Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChangePasswordRequest {
    pub current_password: String,
    pub new_password: String,
    #[serde(default = "default_true")]
    pub revoke_other_sessions: bool,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserSession {
    pub id: SessionId,
    #[schema(required = true)]
    pub device_id: Option<DeviceId>,
    pub device_name: String,
    pub current: bool,
    pub created_at: DateTime<Utc>,
    pub last_used_at: DateTime<Utc>,
    #[schema(required = true)]
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserSessionList {
    pub sessions: Vec<UserSession>,
}

// ---------------------------------------------------------------------------
// Security audit log
// ---------------------------------------------------------------------------

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditLogEntry {
    pub id: String,
    pub occurred_at: DateTime<Utc>,
    #[schema(required = true)]
    pub actor_id: Option<UserId>,
    #[schema(required = true)]
    pub actor_username: Option<String>,
    pub action: String,
    #[schema(required = true)]
    pub target: Option<String>,
    #[schema(required = true)]
    pub detail: Option<String>,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditLogPage {
    pub entries: Vec<AuditLogEntry>,
    #[schema(required = true)]
    pub next_cursor: Option<String>,
}

// ---------------------------------------------------------------------------
// Portable snapshot backups
// ---------------------------------------------------------------------------

#[derive(utoipa::ToSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupSummaryCounts {
    pub libraries: usize,
    pub settings: usize,
    pub playlists: usize,
    pub item_states: usize,
    pub photo_albums: usize,
    pub bookmarks: usize,
    pub home_devices: usize,
    pub home_scenes: usize,
    pub home_rules: usize,
    pub memories: usize,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupSummary {
    pub id: BackupId,
    pub name: String,
    #[schema(required = true)]
    pub creator_id: Option<UserId>,
    #[schema(required = true)]
    pub creator_username: Option<String>,
    pub size_bytes: i64,
    pub sha256: String,
    pub counts: BackupSummaryCounts,
    pub created_at: DateTime<Utc>,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupList {
    pub backups: Vec<BackupSummary>,
}

#[derive(utoipa::ToSchema, Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateBackupRequest {
    #[serde(default)]
    pub name: Option<String>,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupRestoreResult {
    pub backup_id: BackupId,
    pub sha256_verified: bool,
    pub libraries_restored: usize,
    pub settings_restored: usize,
    pub playlists_restored: usize,
    pub item_states_restored: usize,
    pub photo_albums_restored: usize,
    pub bookmarks_restored: usize,
    pub home_entities_restored: usize,
    pub memories_restored: usize,
}

/// Backend-agnostic portable snapshot payload serialized into `system_backups.payload_json`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PalveniSnapshot {
    pub format_version: u32,
    pub server_version: String,
    pub created_at: DateTime<Utc>,
    #[serde(default)]
    pub libraries: Vec<SnapshotLibrary>,
    #[serde(default)]
    pub settings: Vec<SnapshotSetting>,
    #[serde(default)]
    pub user_policies: Vec<SnapshotUserPolicy>,
    #[serde(default)]
    pub playlists: Vec<SnapshotPlaylist>,
    #[serde(default)]
    pub item_states: Vec<SnapshotItemState>,
    #[serde(default)]
    pub photo_albums: Vec<SnapshotPhotoAlbum>,
    #[serde(default)]
    pub bookmarks: Vec<SnapshotBookmark>,
    #[serde(default)]
    pub home_rooms: Vec<SnapshotHomeRoom>,
    #[serde(default)]
    pub home_devices: Vec<SnapshotHomeDevice>,
    #[serde(default)]
    pub home_scenes: Vec<SnapshotHomeScene>,
    #[serde(default)]
    pub home_rules: Vec<SnapshotHomeRule>,
    #[serde(default)]
    pub memories: Vec<SnapshotMemory>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotLibrary {
    pub id: String,
    pub name: String,
    pub kind: LibraryKind,
    pub paths: Vec<String>,
    pub last_scan_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotSetting {
    pub key: String,
    pub value: String,
    pub updated_at: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotUserPolicy {
    pub user_id: String,
    pub username: String,
    pub policy: UserPolicy,
    pub allowed_library_ids: Vec<String>,
    pub updated_at: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotPlaylistEntry {
    pub item_id: String,
    pub item_path: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotPlaylist {
    pub id: String,
    pub user_id: String,
    pub username: Option<String>,
    pub name: String,
    pub version: i64,
    pub comment: Option<String>,
    pub created_at: Option<i64>,
    pub changed_at: Option<i64>,
    pub entries: Vec<SnapshotPlaylistEntry>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotItemState {
    pub user_id: String,
    pub username: Option<String>,
    pub item_id: String,
    pub item_path: Option<String>,
    pub favorite_order: Option<i64>,
    pub stars: i32,
    pub position_ms: i64,
    pub completed: bool,
    pub play_count: i64,
    pub last_played_at: Option<i64>,
    pub playback_version: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotPhotoAlbumItem {
    pub item_id: String,
    pub item_path: Option<String>,
    pub position: i32,
    pub added_at: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotPhotoAlbum {
    pub id: String,
    pub user_id: String,
    pub username: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub cover_item_id: Option<String>,
    pub cover_item_path: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub items: Vec<SnapshotPhotoAlbumItem>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotBookmark {
    pub id: String,
    pub user_id: String,
    pub username: Option<String>,
    pub item_id: String,
    pub item_path: Option<String>,
    pub page: i32,
    pub locator: Option<String>,
    pub label: String,
    pub note: Option<String>,
    pub created_at: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotHomeRoom {
    pub id: String,
    pub name: String,
    pub sort_order: i32,
    pub created_at: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotHomeDevice {
    pub id: String,
    pub room_id: String,
    pub bridge_id: Option<String>,
    pub external_entity_id: Option<String>,
    pub name: String,
    pub kind: String,
    pub online: bool,
    pub state_json: String,
    pub sort_order: i32,
    pub updated_at: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotHomeScene {
    pub id: String,
    pub bridge_id: Option<String>,
    pub external_entity_id: Option<String>,
    pub name: String,
    pub icon: String,
    pub active: bool,
    pub actions_json: String,
    pub sort_order: i32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotHomeRule {
    pub id: String,
    pub event_kind: String,
    pub trigger_text: String,
    pub action_text: String,
    pub actions_json: String,
    pub enabled: bool,
    pub last_run_at: Option<i64>,
    pub sort_order: i32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotMemory {
    pub id: String,
    pub user_id: String,
    pub username: Option<String>,
    pub content: String,
    pub created_at: i64,
}

// ---------------------------------------------------------------------------
// External history & playlist importer
// ---------------------------------------------------------------------------

#[derive(utoipa::ToSchema, Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExternalImportSource {
    #[default]
    Auto,
    Jellyfin,
    Emby,
    Navidrome,
    M3u,
}

#[derive(utoipa::ToSchema, Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExternalImportRequest {
    #[serde(default = "default_auto_source")]
    pub source: ExternalImportSource,
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub playlist_name: Option<String>,
    #[serde(default = "default_true")]
    pub dry_run: bool,
    #[serde(default)]
    pub server_url: Option<String>,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub password: Option<String>,
    #[serde(default)]
    pub api_key: Option<String>,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalImportSample {
    pub title: String,
    pub matched: bool,
    #[schema(required = true)]
    pub matched_title: Option<String>,
    pub kind: String,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalImportResult {
    pub detected_source: ExternalImportSource,
    pub dry_run: bool,
    pub total_entries: usize,
    pub matched_items: usize,
    pub unmatched_entries: usize,
    pub favorites_imported: usize,
    pub history_imported: usize,
    pub playlists_imported: usize,
    pub playlist_tracks_imported: usize,
    pub samples: Vec<ExternalImportSample>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_policy_grants_full_access() {
        let policy = UserPolicy::default();
        assert!(policy.all_libraries);
        assert!(policy.allow_download);
        assert!(policy.allow_transcode);
        assert_eq!(policy.daily_ai_limit, DEFAULT_DAILY_AI_LIMIT);

        let patched = policy.apply_patch(&UserPolicyPatch {
            all_libraries: Some(false),
            daily_ai_limit: Some(25),
            ..Default::default()
        });
        assert!(!patched.all_libraries);
        assert!(patched.allow_download);
        assert_eq!(patched.daily_ai_limit, 25);
    }
}

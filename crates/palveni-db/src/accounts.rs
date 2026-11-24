//! Multi-user account management, library access policies, session self-service,
//! enriched audit queries, portable snapshot backups and external state imports.

use std::collections::{BTreeSet, HashMap, HashSet};

use chrono::DateTime;
use palveni_core::accounts::{
    AuditLogEntry, AuditLogPage, BackupRestoreResult, BackupSummary, BackupSummaryCounts,
    ExternalImportRequest, ExternalImportResult, ExternalImportSample, ExternalImportSource,
    MAX_BACKUP_NAME_CHARS, ManagedUser, PalveniSnapshot, SnapshotBookmark, SnapshotHomeDevice,
    SnapshotHomeRoom, SnapshotHomeRule, SnapshotHomeScene, SnapshotItemState, SnapshotLibrary,
    SnapshotMemory, SnapshotPhotoAlbum, SnapshotPhotoAlbumItem, SnapshotPlaylist,
    SnapshotPlaylistEntry, SnapshotSetting, SnapshotUserPolicy, UserPolicy, UserPolicyPatch,
    UserSession,
};
use palveni_core::ids::PlaylistId;
use palveni_core::{
    BackupId, Library, LibraryId, LibraryKind, LibraryPathId, Role, SessionId, UserId, VERSION,
    clock,
};
use sea_orm::sea_query::{Condition, Expr, OnConflict};
use sea_orm::{
    ActiveValue::Set, ColumnTrait, ConnectionTrait, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect, TransactionTrait,
};

use crate::Db;
use crate::convert::{now_ms, opt_ts, parse_id, parse_opt_id, sha256_hex, truncate_chars, ts};
use crate::entities::{
    assistant_memories, audit_log, devices, home_bridges, home_devices, home_rooms, home_rules,
    home_scenes, items, libraries, library_paths, photo_album_items, photo_albums,
    playlist_entries, playlists, reading_bookmarks, sessions, settings, system_backups,
    track_metadata, user_item_state, user_library_access, user_policies, users, video_metadata,
};
use crate::error::{DbError, is_unique_violation};
use crate::revision;
use crate::settings::KEY_SERVER_ID;
use crate::users::username_key;

const DEFAULT_DEVICE_LABEL: &str = "Unknown device";
const MAX_IMPORT_SAMPLES: usize = 25;

pub struct Accounts<'a> {
    pub(crate) db: &'a Db,
}

#[derive(Clone, Debug)]
pub struct NewManagedUser {
    pub username: String,
    pub password_hash: String,
    pub role: Role,
    pub policy: UserPolicy,
    pub allowed_library_ids: Vec<LibraryId>,
}

#[derive(Clone, Debug, Default)]
pub struct AccountUpdate {
    pub role: Option<Role>,
    pub disabled: Option<bool>,
    pub password_hash: Option<String>,
    pub policy: Option<UserPolicyPatch>,
    pub allowed_library_ids: Option<Vec<LibraryId>>,
    pub keep_session_id: Option<SessionId>,
}

const fn flag_to_i32(value: bool) -> i32 {
    if value { 1 } else { 0 }
}

fn policy_from_row(row: &user_policies::Model) -> UserPolicy {
    UserPolicy {
        all_libraries: row.all_libraries != 0,
        allow_download: row.allow_download != 0,
        allow_transcode: row.allow_transcode != 0,
        daily_ai_limit: row.daily_ai_limit,
    }
}

impl Accounts<'_> {
    // -----------------------------------------------------------------------
    // User accounts & policies
    // -----------------------------------------------------------------------

    pub async fn list_users(&self) -> Result<Vec<ManagedUser>, DbError> {
        assemble_users(self.db.main(), None).await
    }

    pub async fn get_user(&self, id: UserId) -> Result<Option<ManagedUser>, DbError> {
        let mut list = assemble_users(self.db.main(), Some(id)).await?;
        Ok(list.pop())
    }

    pub async fn get_policy(&self, user_id: UserId) -> Result<UserPolicy, DbError> {
        let row = user_policies::Entity::find_by_id(user_id.to_string())
            .one(self.db.main())
            .await?;
        Ok(row
            .as_ref()
            .map_or_else(UserPolicy::default, policy_from_row))
    }

    pub async fn create_user(&self, input: NewManagedUser) -> Result<ManagedUser, DbError> {
        let txn = self.db.main().begin().await?;
        revision::next_revision(&txn).await?;

        let allowed = normalize_and_verify_libraries(&txn, &input.allowed_library_ids).await?;
        let now = now_ms();
        let id = UserId::generate();
        let trimmed = input.username.trim().to_owned();

        users::Entity::insert(users::ActiveModel {
            id: Set(id.to_string()),
            username: Set(trimmed),
            username_key: Set(username_key(&input.username)),
            password_hash: Set(input.password_hash),
            subsonic_generation: Set(None),
            role: Set(input.role.as_str().to_owned()),
            created_at: Set(now),
            updated_at: Set(now),
            disabled_at: Set(None),
        })
        .exec_without_returning(&txn)
        .await
        .map_err(|err| {
            if is_unique_violation(&err) {
                DbError::Conflict("That username is already taken".to_owned())
            } else {
                DbError::Backend(err)
            }
        })?;

        upsert_policy(&txn, id, &input.policy, now).await?;
        replace_allowed_libraries(&txn, id, &allowed).await?;

        let created = assemble_users(&txn, Some(id))
            .await?
            .pop()
            .ok_or(DbError::NotFound("user"))?;
        txn.commit().await?;
        Ok(created)
    }

    pub async fn update_user(
        &self,
        id: UserId,
        patch: AccountUpdate,
    ) -> Result<ManagedUser, DbError> {
        let txn = self.db.main().begin().await?;
        revision::next_revision(&txn).await?;

        let existing = users::Entity::find_by_id(id.to_string())
            .one(&txn)
            .await?
            .ok_or(DbError::NotFound("user"))?;
        let current_role = existing
            .role
            .parse::<Role>()
            .map_err(|_| DbError::Corrupt(format!("unknown role `{}`", existing.role)))?;
        let current_disabled = existing.disabled_at.is_some();

        let next_role = patch.role.unwrap_or(current_role);
        let next_disabled = patch.disabled.unwrap_or(current_disabled);

        if current_role == Role::Admin
            && !current_disabled
            && (next_role != Role::Admin || next_disabled)
            && count_other_enabled_admins(&txn, id).await? == 0
        {
            txn.rollback().await?;
            return Err(DbError::Conflict(
                "Cannot disable or demote the last enabled administrator".to_owned(),
            ));
        }

        let allowed = match &patch.allowed_library_ids {
            Some(ids) => Some(normalize_and_verify_libraries(&txn, ids).await?),
            None => None,
        };

        let now = now_ms();
        let next_disabled_at = if next_disabled {
            Some(existing.disabled_at.unwrap_or(now))
        } else {
            None
        };

        let mut active: users::ActiveModel = existing.into();
        active.role = Set(next_role.as_str().to_owned());
        active.disabled_at = Set(next_disabled_at);
        active.updated_at = Set(now);
        if let Some(hash) = &patch.password_hash {
            active.password_hash = Set(hash.clone());
        }
        users::Entity::update(active).exec(&txn).await?;

        if next_disabled {
            revoke_sessions_for_user(&txn, id, None, now).await?;
        } else if patch.password_hash.is_some() {
            revoke_sessions_for_user(&txn, id, patch.keep_session_id, now).await?;
        }

        if let Some(policy_patch) = &patch.policy {
            let current_policy = user_policies::Entity::find_by_id(id.to_string())
                .one(&txn)
                .await?
                .as_ref()
                .map_or_else(UserPolicy::default, policy_from_row);
            let merged = current_policy.apply_patch(policy_patch);
            upsert_policy(&txn, id, &merged, now).await?;
        }

        if let Some(allowed) = &allowed {
            replace_allowed_libraries(&txn, id, allowed).await?;
        }

        let updated = assemble_users(&txn, Some(id))
            .await?
            .pop()
            .ok_or(DbError::NotFound("user"))?;
        txn.commit().await?;
        Ok(updated)
    }

    pub async fn delete_user(&self, id: UserId) -> Result<bool, DbError> {
        let txn = self.db.main().begin().await?;
        revision::next_revision(&txn).await?;

        let Some(existing) = users::Entity::find_by_id(id.to_string()).one(&txn).await? else {
            txn.rollback().await?;
            return Ok(false);
        };
        let current_role = existing
            .role
            .parse::<Role>()
            .map_err(|_| DbError::Corrupt(format!("unknown role `{}`", existing.role)))?;
        if current_role == Role::Admin
            && existing.disabled_at.is_none()
            && count_other_enabled_admins(&txn, id).await? == 0
        {
            txn.rollback().await?;
            return Err(DbError::Conflict(
                "Cannot delete the last enabled administrator".to_owned(),
            ));
        }

        let now = now_ms();
        revoke_sessions_for_user(&txn, id, None, now).await?;
        let deleted = users::Entity::delete_by_id(id.to_string())
            .exec(&txn)
            .await?;
        txn.commit().await?;
        Ok(deleted.rows_affected > 0)
    }

    // -----------------------------------------------------------------------
    // Self-service password & device session management
    // -----------------------------------------------------------------------

    pub async fn change_password(
        &self,
        user_id: UserId,
        new_hash: &str,
        keep_session_id: Option<SessionId>,
    ) -> Result<u64, DbError> {
        let txn = self.db.main().begin().await?;
        let now = now_ms();
        let updated = users::Entity::update_many()
            .col_expr(users::Column::PasswordHash, Expr::val(new_hash.to_owned()))
            .col_expr(users::Column::UpdatedAt, Expr::val(now))
            .filter(users::Column::Id.eq(user_id.to_string()))
            .filter(users::Column::DisabledAt.is_null())
            .exec(&txn)
            .await?;
        if updated.rows_affected == 0 {
            txn.rollback().await?;
            return Err(DbError::NotFound("user"));
        }
        let revoked = revoke_sessions_for_user(&txn, user_id, keep_session_id, now).await?;
        txn.commit().await?;
        Ok(revoked)
    }

    pub async fn update_password_only(
        &self,
        user_id: UserId,
        new_hash: &str,
    ) -> Result<(), DbError> {
        let now = now_ms();
        let updated = users::Entity::update_many()
            .col_expr(users::Column::PasswordHash, Expr::val(new_hash.to_owned()))
            .col_expr(users::Column::UpdatedAt, Expr::val(now))
            .filter(users::Column::Id.eq(user_id.to_string()))
            .filter(users::Column::DisabledAt.is_null())
            .exec(self.db.main())
            .await?;
        if updated.rows_affected == 0 {
            return Err(DbError::NotFound("user"));
        }
        Ok(())
    }

    pub async fn list_user_sessions(
        &self,
        user_id: UserId,
        current_session_id: SessionId,
    ) -> Result<Vec<UserSession>, DbError> {
        let now = now_ms();
        let rows = sessions::Entity::find()
            .filter(sessions::Column::UserId.eq(user_id.to_string()))
            .filter(sessions::Column::RevokedAt.is_null())
            .filter(
                Condition::any()
                    .add(sessions::Column::ExpiresAt.is_null())
                    .add(sessions::Column::ExpiresAt.gt(now)),
            )
            .order_by_desc(sessions::Column::LastUsedAt)
            .order_by_desc(sessions::Column::CreatedAt)
            .all(self.db.main())
            .await?;

        let device_ids: Vec<String> = rows
            .iter()
            .filter_map(|row| row.device_id.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();

        let device_names: HashMap<String, String> = if device_ids.is_empty() {
            HashMap::new()
        } else {
            devices::Entity::find()
                .filter(devices::Column::Id.is_in(device_ids))
                .all(self.db.main())
                .await?
                .into_iter()
                .map(|d| (d.id, d.name))
                .collect()
        };

        let mut list = Vec::with_capacity(rows.len());
        for row in rows {
            let id: SessionId = parse_id(&row.id, "session id")?;
            let device_id = parse_opt_id(row.device_id.as_deref(), "device id")?;
            let device_name = row
                .device_id
                .as_deref()
                .and_then(|did| device_names.get(did))
                .cloned()
                .filter(|name| !name.trim().is_empty())
                .unwrap_or_else(|| DEFAULT_DEVICE_LABEL.to_owned());
            list.push(UserSession {
                id,
                device_id,
                device_name,
                current: id == current_session_id,
                created_at: ts(row.created_at)?,
                last_used_at: ts(row.last_used_at)?,
                expires_at: opt_ts(row.expires_at)?,
            });
        }
        list.sort_by(|a, b| {
            b.current
                .cmp(&a.current)
                .then_with(|| b.last_used_at.cmp(&a.last_used_at))
                .then_with(|| b.created_at.cmp(&a.created_at))
        });
        Ok(list)
    }

    pub async fn revoke_user_session(
        &self,
        user_id: UserId,
        target_session_id: SessionId,
    ) -> Result<bool, DbError> {
        let existing = sessions::Entity::find_by_id(target_session_id.to_string())
            .filter(sessions::Column::UserId.eq(user_id.to_string()))
            .one(self.db.main())
            .await?;
        let Some(existing) = existing else {
            return Ok(false);
        };
        if existing.revoked_at.is_none() {
            sessions::Entity::update_many()
                .col_expr(sessions::Column::RevokedAt, Expr::val(now_ms()))
                .filter(sessions::Column::Id.eq(target_session_id.to_string()))
                .filter(sessions::Column::UserId.eq(user_id.to_string()))
                .filter(sessions::Column::RevokedAt.is_null())
                .exec(self.db.main())
                .await?;
        }
        Ok(true)
    }

    // -----------------------------------------------------------------------
    // Per-library access control
    // -----------------------------------------------------------------------

    pub async fn can_access_library(
        &self,
        user_id: UserId,
        role: Role,
        library_id: LibraryId,
    ) -> Result<bool, DbError> {
        if role == Role::Admin {
            return Ok(true);
        }
        let policy = self.get_policy(user_id).await?;
        if policy.all_libraries {
            return Ok(true);
        }
        let found =
            user_library_access::Entity::find_by_id((user_id.to_string(), library_id.to_string()))
                .one(self.db.main())
                .await?;
        Ok(found.is_some())
    }

    pub async fn filter_libraries(
        &self,
        user_id: UserId,
        role: Role,
        libraries: Vec<Library>,
    ) -> Result<Vec<Library>, DbError> {
        if role == Role::Admin || libraries.is_empty() {
            return Ok(libraries);
        }
        let policy = self.get_policy(user_id).await?;
        if policy.all_libraries {
            return Ok(libraries);
        }
        let allowed_ids: HashSet<String> = user_library_access::Entity::find()
            .filter(user_library_access::Column::UserId.eq(user_id.to_string()))
            .select_only()
            .column(user_library_access::Column::LibraryId)
            .into_tuple::<String>()
            .all(self.db.main())
            .await?
            .into_iter()
            .collect();
        Ok(libraries
            .into_iter()
            .filter(|lib| allowed_ids.contains(&lib.id.to_string()))
            .collect())
    }

    // -----------------------------------------------------------------------
    // Paginated & enriched security audit log
    // -----------------------------------------------------------------------

    pub async fn list_audit_logs(
        &self,
        action_prefix: Option<&str>,
        cursor: Option<&str>,
        limit: u32,
    ) -> Result<AuditLogPage, DbError> {
        let limit = limit.clamp(1, 200);
        let mut query = audit_log::Entity::find();
        if let Some(prefix) = action_prefix.map(str::trim).filter(|s| !s.is_empty()) {
            query = query.filter(audit_log::Column::Action.starts_with(prefix));
        }
        if let Some(raw_cursor) = cursor.map(str::trim).filter(|s| !s.is_empty()) {
            let (ts_raw, cursor_id) = raw_cursor
                .split_once(':')
                .ok_or(DbError::Invalid("Invalid audit cursor"))?;
            let cursor_ms: i64 = ts_raw
                .parse()
                .map_err(|_| DbError::Invalid("Invalid audit cursor"))?;
            if cursor_id.is_empty() {
                return Err(DbError::Invalid("Invalid audit cursor"));
            }
            query = query.filter(
                Condition::any()
                    .add(audit_log::Column::OccurredAt.lt(cursor_ms))
                    .add(
                        Condition::all()
                            .add(audit_log::Column::OccurredAt.eq(cursor_ms))
                            .add(audit_log::Column::Id.lt(cursor_id.to_owned())),
                    ),
            );
        }

        let fetch_limit = u64::from(limit) + 1;
        let mut rows = query
            .order_by_desc(audit_log::Column::OccurredAt)
            .order_by_desc(audit_log::Column::Id)
            .limit(fetch_limit)
            .all(self.db.ops())
            .await?;

        let next_cursor = if rows.len() > limit as usize {
            rows.truncate(limit as usize);
            rows.last()
                .map(|last| format!("{}:{}", last.occurred_at, last.id))
        } else {
            None
        };

        let actor_ids: Vec<String> = rows
            .iter()
            .filter_map(|row| row.actor_id.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();

        let usernames: HashMap<String, String> = if actor_ids.is_empty() {
            HashMap::new()
        } else {
            users::Entity::find()
                .filter(users::Column::Id.is_in(actor_ids))
                .select_only()
                .column(users::Column::Id)
                .column(users::Column::Username)
                .into_tuple::<(String, String)>()
                .all(self.db.main())
                .await?
                .into_iter()
                .collect()
        };

        let mut entries = Vec::with_capacity(rows.len());
        for row in rows {
            let actor_username = row
                .actor_id
                .as_deref()
                .and_then(|aid| usernames.get(aid))
                .cloned();
            let actor_id = row
                .actor_id
                .as_deref()
                .and_then(|aid| aid.parse::<UserId>().ok());
            entries.push(AuditLogEntry {
                id: row.id,
                occurred_at: ts(row.occurred_at)?,
                actor_id,
                actor_username,
                action: row.action,
                target: row.target,
                detail: row.detail,
            });
        }

        Ok(AuditLogPage {
            entries,
            next_cursor,
        })
    }

    // -----------------------------------------------------------------------
    // Portable database snapshot backups
    // -----------------------------------------------------------------------

    pub async fn create_backup(
        &self,
        creator_id: Option<UserId>,
        name: Option<&str>,
    ) -> Result<BackupSummary, DbError> {
        let now_dt = clock::now();
        let now = clock::to_millis(now_dt);
        let backup_id = BackupId::generate();

        let clean_name = name
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| truncate_chars(s, MAX_BACKUP_NAME_CHARS))
            .unwrap_or_else(|| format!("Snapshot {}", now_dt.format("%Y-%m-%d %H:%M UTC")));

        let snapshot = build_snapshot(self.db.main(), now_dt).await?;
        let counts = BackupSummaryCounts {
            libraries: snapshot.libraries.len(),
            settings: snapshot.settings.len(),
            playlists: snapshot.playlists.len(),
            item_states: snapshot.item_states.len(),
            photo_albums: snapshot.photo_albums.len(),
            bookmarks: snapshot.bookmarks.len(),
            home_devices: snapshot.home_devices.len(),
            home_scenes: snapshot.home_scenes.len(),
            home_rules: snapshot.home_rules.len(),
            memories: snapshot.memories.len(),
        };

        let payload_json = serde_json::to_string_pretty(&snapshot)
            .map_err(|e| DbError::Internal(format!("cannot serialize backup payload: {e}")))?;
        let summary_json = serde_json::to_string(&counts)
            .map_err(|e| DbError::Internal(format!("cannot serialize backup summary: {e}")))?;
        let sha256 = sha256_hex(&payload_json);
        let size_bytes = i64::try_from(payload_json.len()).unwrap_or(i64::MAX);

        system_backups::Entity::insert(system_backups::ActiveModel {
            id: Set(backup_id.to_string()),
            name: Set(clean_name.clone()),
            creator_id: Set(creator_id.map(|id| id.to_string())),
            size_bytes: Set(size_bytes),
            sha256: Set(sha256.clone()),
            summary_json: Set(summary_json),
            payload_json: Set(payload_json),
            created_at: Set(now),
        })
        .exec_without_returning(self.db.main())
        .await?;

        let creator_username = match creator_id {
            Some(uid) => {
                users::Entity::find_by_id(uid.to_string())
                    .select_only()
                    .column(users::Column::Username)
                    .into_tuple::<String>()
                    .one(self.db.main())
                    .await?
            }
            None => None,
        };

        Ok(BackupSummary {
            id: backup_id,
            name: clean_name,
            creator_id,
            creator_username,
            size_bytes,
            sha256,
            counts,
            created_at: now_dt,
        })
    }

    pub async fn list_backups(&self) -> Result<Vec<BackupSummary>, DbError> {
        let rows = system_backups::Entity::find()
            .select_only()
            .columns([
                system_backups::Column::Id,
                system_backups::Column::Name,
                system_backups::Column::CreatorId,
                system_backups::Column::SizeBytes,
                system_backups::Column::Sha256,
                system_backups::Column::SummaryJson,
                system_backups::Column::CreatedAt,
            ])
            .order_by_desc(system_backups::Column::CreatedAt)
            .order_by_desc(system_backups::Column::Id)
            .into_tuple::<(String, String, Option<String>, i64, String, String, i64)>()
            .all(self.db.main())
            .await?;

        let creator_ids: Vec<String> = rows
            .iter()
            .filter_map(|(_, _, cid, _, _, _, _)| cid.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();

        let usernames: HashMap<String, String> = if creator_ids.is_empty() {
            HashMap::new()
        } else {
            users::Entity::find()
                .filter(users::Column::Id.is_in(creator_ids))
                .select_only()
                .column(users::Column::Id)
                .column(users::Column::Username)
                .into_tuple::<(String, String)>()
                .all(self.db.main())
                .await?
                .into_iter()
                .collect()
        };

        let mut list = Vec::with_capacity(rows.len());
        for (id_raw, name, creator_raw, size_bytes, sha256, summary_json, created_at) in rows {
            let counts: BackupSummaryCounts =
                serde_json::from_str(&summary_json).unwrap_or_default();
            let creator_username = creator_raw
                .as_deref()
                .and_then(|cid| usernames.get(cid))
                .cloned();
            let creator_id = parse_opt_id(creator_raw.as_deref(), "creator id")?;
            list.push(BackupSummary {
                id: parse_id(&id_raw, "backup id")?,
                name,
                creator_id,
                creator_username,
                size_bytes,
                sha256,
                counts,
                created_at: ts(created_at)?,
            });
        }
        Ok(list)
    }

    pub async fn get_backup_payload(
        &self,
        id: BackupId,
    ) -> Result<Option<(BackupSummary, String)>, DbError> {
        let Some(row) = system_backups::Entity::find_by_id(id.to_string())
            .one(self.db.main())
            .await?
        else {
            return Ok(None);
        };
        let computed = sha256_hex(&row.payload_json);
        if computed != row.sha256 {
            return Err(DbError::Corrupt(
                "backup payload SHA-256 verification failed".to_owned(),
            ));
        }
        let counts: BackupSummaryCounts =
            serde_json::from_str(&row.summary_json).unwrap_or_default();
        let creator_username = match row.creator_id.as_deref() {
            Some(cid) => {
                users::Entity::find_by_id(cid.to_owned())
                    .select_only()
                    .column(users::Column::Username)
                    .into_tuple::<String>()
                    .one(self.db.main())
                    .await?
            }
            None => None,
        };
        let summary = BackupSummary {
            id: parse_id(&row.id, "backup id")?,
            name: row.name,
            creator_id: parse_opt_id(row.creator_id.as_deref(), "creator id")?,
            creator_username,
            size_bytes: row.size_bytes,
            sha256: row.sha256,
            counts,
            created_at: ts(row.created_at)?,
        };
        Ok(Some((summary, row.payload_json)))
    }

    pub async fn restore_backup(&self, id: BackupId) -> Result<BackupRestoreResult, DbError> {
        let row = system_backups::Entity::find_by_id(id.to_string())
            .one(self.db.main())
            .await?
            .ok_or(DbError::NotFound("backup"))?;

        let computed = sha256_hex(&row.payload_json);
        if computed != row.sha256 {
            return Err(DbError::Conflict(
                "Backup SHA-256 checksum mismatch; refusing to restore corrupted snapshot"
                    .to_owned(),
            ));
        }

        let snapshot: PalveniSnapshot = serde_json::from_str(&row.payload_json)
            .map_err(|e| DbError::Corrupt(format!("invalid snapshot JSON: {e}")))?;

        let txn = self.db.main().begin().await?;
        let result = apply_snapshot(&txn, id, &snapshot).await?;
        txn.commit().await?;
        Ok(result)
    }

    pub async fn delete_backup(&self, id: BackupId) -> Result<bool, DbError> {
        let deleted = system_backups::Entity::delete_by_id(id.to_string())
            .exec(self.db.main())
            .await?;
        Ok(deleted.rows_affected > 0)
    }

    // -----------------------------------------------------------------------
    // Jellyfin / Emby / Navidrome / M3U external history & playlist importer
    // -----------------------------------------------------------------------

    pub async fn import_external_state(
        &self,
        user_id: UserId,
        request: &ExternalImportRequest,
    ) -> Result<ExternalImportResult, DbError> {
        let content = request.content.trim();
        if content.is_empty() {
            return Err(DbError::Invalid("Import content must not be empty"));
        }

        let parsed =
            parse_external_payload(request.source, content, request.playlist_name.as_deref())?;

        let catalog = load_catalog_matcher(self.db.main()).await?;

        let mut matched_items = 0usize;
        let mut unmatched_entries = 0usize;
        let mut favorites_imported = 0usize;
        let mut history_imported = 0usize;
        let mut playlists_imported = 0usize;
        let mut playlist_tracks_imported = 0usize;
        let mut samples = Vec::new();

        // Deduplicated state updates keyed by matched item_id.
        let mut state_updates: HashMap<String, PendingItemImport> = HashMap::new();
        for entry in &parsed.state_entries {
            if let Some(hit) = catalog.match_entry(entry) {
                matched_items += 1;
                if entry.favorite {
                    favorites_imported += 1;
                }
                if entry.played || entry.play_count > 0 || entry.position_ms > 0 {
                    history_imported += 1;
                }
                if samples.len() < MAX_IMPORT_SAMPLES {
                    samples.push(ExternalImportSample {
                        title: entry.display_title(),
                        matched: true,
                        matched_title: Some(hit.title.clone()),
                        kind: if entry.favorite && !entry.played && entry.play_count == 0 {
                            "favorite".to_owned()
                        } else {
                            "history".to_owned()
                        },
                    });
                }
                let slot = state_updates.entry(hit.id.clone()).or_default();
                slot.favorite |= entry.favorite;
                slot.played |= entry.played;
                slot.play_count = slot.play_count.max(entry.play_count);
                if entry.position_ms > 0 {
                    slot.position_ms = slot.position_ms.max(entry.position_ms);
                }
                if entry.stars > 0 {
                    slot.stars = slot.stars.max(entry.stars.clamp(1, 5));
                }
                if let Some(ts_ms) = entry.last_played_at {
                    slot.last_played_at =
                        Some(slot.last_played_at.map_or(ts_ms, |prev| prev.max(ts_ms)));
                }
            } else {
                unmatched_entries += 1;
                if samples.len() < MAX_IMPORT_SAMPLES {
                    samples.push(ExternalImportSample {
                        title: entry.display_title(),
                        matched: false,
                        matched_title: None,
                        kind: if entry.favorite {
                            "favorite".to_owned()
                        } else {
                            "history".to_owned()
                        },
                    });
                }
            }
        }

        let mut resolved_playlists: Vec<(String, Vec<String>)> = Vec::new();
        for playlist in &parsed.playlists {
            let mut item_ids = Vec::new();
            for entry in &playlist.tracks {
                if let Some(hit) = catalog.match_entry(entry) {
                    matched_items += 1;
                    playlist_tracks_imported += 1;
                    item_ids.push(hit.id.clone());
                    if samples.len() < MAX_IMPORT_SAMPLES {
                        samples.push(ExternalImportSample {
                            title: entry.display_title(),
                            matched: true,
                            matched_title: Some(hit.title.clone()),
                            kind: "playlist_track".to_owned(),
                        });
                    }
                } else {
                    unmatched_entries += 1;
                    if samples.len() < MAX_IMPORT_SAMPLES {
                        samples.push(ExternalImportSample {
                            title: entry.display_title(),
                            matched: false,
                            matched_title: None,
                            kind: "playlist_track".to_owned(),
                        });
                    }
                }
            }
            if !item_ids.is_empty() {
                playlists_imported += 1;
                resolved_playlists.push((playlist.name.clone(), item_ids));
            }
        }

        let total_entries = parsed.state_entries.len()
            + parsed
                .playlists
                .iter()
                .map(|p| p.tracks.len())
                .sum::<usize>();

        if !request.dry_run && (!state_updates.is_empty() || !resolved_playlists.is_empty()) {
            let txn = self.db.main().begin().await?;
            revision::next_revision(&txn).await?;
            let now = now_ms();
            let uid = user_id.to_string();

            if !state_updates.is_empty() {
                let target_ids: Vec<String> = state_updates.keys().cloned().collect();
                let existing_rows: HashMap<String, user_item_state::Model> =
                    user_item_state::Entity::find()
                        .filter(user_item_state::Column::UserId.eq(uid.clone()))
                        .filter(user_item_state::Column::ItemId.is_in(target_ids))
                        .all(&txn)
                        .await?
                        .into_iter()
                        .map(|r| (r.item_id.clone(), r))
                        .collect();

                for (idx, (item_id, upd)) in state_updates.into_iter().enumerate() {
                    let prev = existing_rows.get(&item_id);
                    let favorite_order = if upd.favorite {
                        prev.and_then(|p| p.favorite_order)
                            .or(Some(now + i64::try_from(idx).unwrap_or(0)))
                    } else {
                        prev.and_then(|p| p.favorite_order)
                    };
                    let stars = if upd.stars > 0 {
                        upd.stars
                    } else {
                        prev.map_or(0, |p| p.stars)
                    };
                    let completed = prev.is_some_and(|p| p.completed) || upd.played;
                    let mut play_count = prev.map_or(0, |p| p.play_count).max(upd.play_count);
                    if completed && play_count == 0 {
                        play_count = 1;
                    }
                    let position_ms = if upd.position_ms > 0 {
                        upd.position_ms
                    } else {
                        prev.map_or(0, |p| p.position_ms)
                    };
                    let last_played_at =
                        match (prev.and_then(|p| p.last_played_at), upd.last_played_at) {
                            (Some(a), Some(b)) => Some(a.max(b)),
                            (Some(a), None) => Some(a),
                            (None, Some(b)) => Some(b),
                            (None, None) if completed || play_count > 0 || position_ms > 0 => {
                                Some(now)
                            }
                            (None, None) => None,
                        };
                    let next_ver = prev.map_or(1, |p| p.playback_version + 1);
                    let prev_ver = prev.map(|p| p.playback_version);

                    let model = user_item_state::ActiveModel {
                        user_id: Set(uid.clone()),
                        item_id: Set(item_id),
                        favorite_order: Set(favorite_order),
                        stars: Set(stars),
                        position_ms: Set(position_ms),
                        completed: Set(completed),
                        play_count: Set(play_count),
                        last_played_at: Set(last_played_at),
                        playback_version: Set(next_ver),
                        previous_version: Set(prev_ver),
                    };
                    user_item_state::Entity::insert(model)
                        .on_conflict(
                            OnConflict::columns([
                                user_item_state::Column::UserId,
                                user_item_state::Column::ItemId,
                            ])
                            .update_columns([
                                user_item_state::Column::FavoriteOrder,
                                user_item_state::Column::Stars,
                                user_item_state::Column::PositionMs,
                                user_item_state::Column::Completed,
                                user_item_state::Column::PlayCount,
                                user_item_state::Column::LastPlayedAt,
                                user_item_state::Column::PlaybackVersion,
                                user_item_state::Column::PreviousVersion,
                            ])
                            .to_owned(),
                        )
                        .exec_without_returning(&txn)
                        .await?;
                }
            }

            for (name, track_ids) in resolved_playlists {
                let clean_name = truncate_chars(name.trim(), 200);
                let clean_name = if clean_name.is_empty() {
                    "Imported Playlist".to_owned()
                } else {
                    clean_name
                };
                let existing_pl = playlists::Entity::find()
                    .filter(playlists::Column::UserId.eq(uid.clone()))
                    .filter(playlists::Column::Name.eq(clean_name.clone()))
                    .filter(playlists::Column::Deleted.eq(false))
                    .one(&txn)
                    .await?;
                let pl_id = match existing_pl {
                    Some(row) => {
                        let next_version = revision::next_revision(&txn).await?;
                        playlists::Entity::update_many()
                            .col_expr(playlists::Column::Version, Expr::val(next_version))
                            .col_expr(playlists::Column::ChangedAt, Expr::val(Some(now)))
                            .filter(playlists::Column::Id.eq(row.id.clone()))
                            .exec(&txn)
                            .await?;
                        playlist_entries::Entity::delete_many()
                            .filter(playlist_entries::Column::PlaylistId.eq(row.id.clone()))
                            .exec(&txn)
                            .await?;
                        row.id
                    }
                    None => {
                        let pid = PlaylistId::generate().to_string();
                        let version = revision::next_revision(&txn).await?;
                        playlists::Entity::insert(playlists::ActiveModel {
                            id: Set(pid.clone()),
                            user_id: Set(uid.clone()),
                            name: Set(clean_name),
                            version: Set(version),
                            comment: Set(None),
                            created_at: Set(Some(now)),
                            changed_at: Set(Some(now)),
                            deleted: Set(false),
                        })
                        .exec_without_returning(&txn)
                        .await?;
                        pid
                    }
                };

                let entry_models: Vec<playlist_entries::ActiveModel> = track_ids
                    .into_iter()
                    .take(crate::playlists::MAX_ENTRIES)
                    .enumerate()
                    .map(|(pos, item_id)| playlist_entries::ActiveModel {
                        playlist_id: Set(pl_id.clone()),
                        position: Set(i32::try_from(pos).unwrap_or(i32::MAX)),
                        item_id: Set(item_id),
                    })
                    .collect();
                for chunk in entry_models.chunks(200) {
                    playlist_entries::Entity::insert_many(chunk.to_vec())
                        .exec_without_returning(&txn)
                        .await?;
                }
            }

            txn.commit().await?;
        }

        Ok(ExternalImportResult {
            detected_source: parsed.source,
            dry_run: request.dry_run,
            total_entries,
            matched_items,
            unmatched_entries,
            favorites_imported,
            history_imported,
            playlists_imported,
            playlist_tracks_imported,
            samples,
        })
    }
}

// ---------------------------------------------------------------------------
// Internal helpers: users, policies, sessions
// ---------------------------------------------------------------------------

async fn count_other_enabled_admins<C: ConnectionTrait>(
    conn: &C,
    exclude_id: UserId,
) -> Result<u64, DbError> {
    Ok(users::Entity::find()
        .filter(users::Column::Role.eq(Role::Admin.as_str()))
        .filter(users::Column::DisabledAt.is_null())
        .filter(users::Column::Id.ne(exclude_id.to_string()))
        .count(conn)
        .await?)
}

async fn normalize_and_verify_libraries<C: ConnectionTrait>(
    conn: &C,
    ids: &[LibraryId],
) -> Result<Vec<LibraryId>, DbError> {
    let mut unique = Vec::new();
    let mut seen = BTreeSet::new();
    for &id in ids {
        if seen.insert(id) {
            unique.push(id);
        }
    }
    if unique.is_empty() {
        return Ok(unique);
    }
    let id_strings: Vec<String> = unique.iter().map(ToString::to_string).collect();
    let live_count = libraries::Entity::find()
        .filter(libraries::Column::Id.is_in(id_strings))
        .filter(libraries::Column::DeletedAt.is_null())
        .count(conn)
        .await?;
    if live_count != unique.len() as u64 {
        return Err(DbError::Invalid("One or more libraries do not exist"));
    }
    Ok(unique)
}

async fn upsert_policy<C: ConnectionTrait>(
    conn: &C,
    user_id: UserId,
    policy: &UserPolicy,
    now: i64,
) -> Result<(), DbError> {
    let model = user_policies::ActiveModel {
        user_id: Set(user_id.to_string()),
        all_libraries: Set(flag_to_i32(policy.all_libraries)),
        allow_download: Set(flag_to_i32(policy.allow_download)),
        allow_transcode: Set(flag_to_i32(policy.allow_transcode)),
        daily_ai_limit: Set(policy.daily_ai_limit.max(0)),
        updated_at: Set(now),
    };
    user_policies::Entity::insert(model)
        .on_conflict(
            OnConflict::column(user_policies::Column::UserId)
                .update_columns([
                    user_policies::Column::AllLibraries,
                    user_policies::Column::AllowDownload,
                    user_policies::Column::AllowTranscode,
                    user_policies::Column::DailyAiLimit,
                    user_policies::Column::UpdatedAt,
                ])
                .to_owned(),
        )
        .exec_without_returning(conn)
        .await?;
    Ok(())
}

async fn replace_allowed_libraries<C: ConnectionTrait>(
    conn: &C,
    user_id: UserId,
    allowed: &[LibraryId],
) -> Result<(), DbError> {
    user_library_access::Entity::delete_many()
        .filter(user_library_access::Column::UserId.eq(user_id.to_string()))
        .exec(conn)
        .await?;
    if !allowed.is_empty() {
        let models: Vec<user_library_access::ActiveModel> = allowed
            .iter()
            .map(|lib_id| user_library_access::ActiveModel {
                user_id: Set(user_id.to_string()),
                library_id: Set(lib_id.to_string()),
            })
            .collect();
        user_library_access::Entity::insert_many(models)
            .exec_without_returning(conn)
            .await?;
    }
    Ok(())
}

async fn revoke_sessions_for_user<C: ConnectionTrait>(
    conn: &C,
    user_id: UserId,
    keep_session_id: Option<SessionId>,
    now: i64,
) -> Result<u64, DbError> {
    let mut query = sessions::Entity::update_many()
        .col_expr(sessions::Column::RevokedAt, Expr::val(now))
        .filter(sessions::Column::UserId.eq(user_id.to_string()))
        .filter(sessions::Column::RevokedAt.is_null());
    if let Some(keep) = keep_session_id {
        query = query.filter(sessions::Column::Id.ne(keep.to_string()));
    }
    let res = query.exec(conn).await?;
    Ok(res.rows_affected)
}

async fn assemble_users<C: ConnectionTrait>(
    conn: &C,
    only_id: Option<UserId>,
) -> Result<Vec<ManagedUser>, DbError> {
    let mut user_query = users::Entity::find()
        .order_by_asc(users::Column::CreatedAt)
        .order_by_asc(users::Column::Id);
    if let Some(id) = only_id {
        user_query = user_query.filter(users::Column::Id.eq(id.to_string()));
    }
    let user_rows = user_query.all(conn).await?;
    if user_rows.is_empty() {
        return Ok(Vec::new());
    }

    let user_ids: Vec<String> = user_rows.iter().map(|u| u.id.clone()).collect();

    let policy_map: HashMap<String, UserPolicy> = user_policies::Entity::find()
        .filter(user_policies::Column::UserId.is_in(user_ids.clone()))
        .all(conn)
        .await?
        .into_iter()
        .map(|row| (row.user_id.clone(), policy_from_row(&row)))
        .collect();

    let live_libraries: HashSet<String> = libraries::Entity::find()
        .filter(libraries::Column::DeletedAt.is_null())
        .select_only()
        .column(libraries::Column::Id)
        .into_tuple::<String>()
        .all(conn)
        .await?
        .into_iter()
        .collect();

    let mut access_map: HashMap<String, Vec<LibraryId>> = HashMap::new();
    let access_rows = user_library_access::Entity::find()
        .filter(user_library_access::Column::UserId.is_in(user_ids.clone()))
        .order_by_asc(user_library_access::Column::LibraryId)
        .all(conn)
        .await?;
    for row in access_rows {
        if live_libraries.contains(&row.library_id) {
            let lib_id: LibraryId = parse_id(&row.library_id, "library id")?;
            access_map.entry(row.user_id).or_default().push(lib_id);
        }
    }

    let now = now_ms();
    let active_session_users = sessions::Entity::find()
        .filter(sessions::Column::UserId.is_in(user_ids))
        .filter(sessions::Column::RevokedAt.is_null())
        .filter(
            Condition::any()
                .add(sessions::Column::ExpiresAt.is_null())
                .add(sessions::Column::ExpiresAt.gt(now)),
        )
        .select_only()
        .column(sessions::Column::UserId)
        .into_tuple::<String>()
        .all(conn)
        .await?;
    let mut session_counts: HashMap<String, u64> = HashMap::new();
    for uid in active_session_users {
        *session_counts.entry(uid).or_insert(0) += 1;
    }

    let mut out = Vec::with_capacity(user_rows.len());
    for row in user_rows {
        let id: UserId = parse_id(&row.id, "user id")?;
        let role = row
            .role
            .parse::<Role>()
            .map_err(|_| DbError::Corrupt(format!("unknown role `{}`", row.role)))?;
        let disabled = row.disabled_at.is_some();
        let policy = policy_map.get(&row.id).cloned().unwrap_or_default();
        let allowed_library_ids = access_map.remove(&row.id).unwrap_or_default();
        let active_sessions = if disabled {
            0
        } else {
            session_counts.get(&row.id).copied().unwrap_or(0)
        };
        out.push(ManagedUser {
            id,
            username: row.username,
            role,
            disabled,
            policy,
            allowed_library_ids,
            active_sessions,
            created_at: ts(row.created_at)?,
            updated_at: ts(row.updated_at)?,
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// Internal helpers: snapshot build & idempotent restore
// ---------------------------------------------------------------------------

async fn build_snapshot<C: ConnectionTrait>(
    conn: &C,
    created_at: DateTime<chrono::Utc>,
) -> Result<PalveniSnapshot, DbError> {
    let users_map: HashMap<String, String> = users::Entity::find()
        .select_only()
        .column(users::Column::Id)
        .column(users::Column::Username)
        .into_tuple::<(String, String)>()
        .all(conn)
        .await?
        .into_iter()
        .collect();

    let item_paths: HashMap<String, String> = items::Entity::find()
        .filter(items::Column::DeletedAt.is_null())
        .filter(items::Column::Path.is_not_null())
        .select_only()
        .column(items::Column::Id)
        .column(items::Column::Path)
        .into_tuple::<(String, Option<String>)>()
        .all(conn)
        .await?
        .into_iter()
        .filter_map(|(id, p)| p.map(|path| (id, path)))
        .collect();

    // 1. Libraries + paths
    let lib_rows = libraries::Entity::find()
        .filter(libraries::Column::DeletedAt.is_null())
        .order_by_asc(libraries::Column::NameKey)
        .all(conn)
        .await?;
    let path_rows = library_paths::Entity::find()
        .order_by_asc(library_paths::Column::SortOrder)
        .all(conn)
        .await?;
    let mut paths_by_lib: HashMap<String, Vec<String>> = HashMap::new();
    for p in path_rows {
        paths_by_lib.entry(p.library_id).or_default().push(p.path);
    }
    let mut libraries_snap = Vec::with_capacity(lib_rows.len());
    for lib in lib_rows {
        let kind: LibraryKind = lib
            .kind
            .parse()
            .map_err(|_| DbError::Corrupt(format!("unknown library kind `{}`", lib.kind)))?;
        let paths = paths_by_lib.remove(&lib.id).unwrap_or_default();
        libraries_snap.push(SnapshotLibrary {
            id: lib.id,
            name: lib.name,
            kind,
            paths,
            last_scan_at: lib.last_scan_at,
            created_at: lib.created_at,
            updated_at: lib.updated_at,
        });
    }

    // 2. Runtime settings (excluding installation-specific server.id)
    let settings_snap: Vec<SnapshotSetting> = settings::Entity::find()
        .order_by_asc(settings::Column::SettingKey)
        .all(conn)
        .await?
        .into_iter()
        .filter(|s| s.setting_key != KEY_SERVER_ID)
        .map(|s| SnapshotSetting {
            key: s.setting_key,
            value: s.setting_value,
            updated_at: s.updated_at,
        })
        .collect();

    // 3. User policies + library access
    let policy_rows = user_policies::Entity::find()
        .order_by_asc(user_policies::Column::UserId)
        .all(conn)
        .await?;
    let access_rows = user_library_access::Entity::find()
        .order_by_asc(user_library_access::Column::LibraryId)
        .all(conn)
        .await?;
    let mut access_by_user: HashMap<String, Vec<String>> = HashMap::new();
    for acc in access_rows {
        access_by_user
            .entry(acc.user_id)
            .or_default()
            .push(acc.library_id);
    }
    let mut user_policies_snap = Vec::new();
    for p in policy_rows {
        if let Some(username) = users_map.get(&p.user_id) {
            user_policies_snap.push(SnapshotUserPolicy {
                policy: policy_from_row(&p),
                allowed_library_ids: access_by_user.remove(&p.user_id).unwrap_or_default(),
                updated_at: p.updated_at,
                user_id: p.user_id,
                username: username.clone(),
            });
        }
    }

    // 4. Playlists + entries
    let pl_rows = playlists::Entity::find()
        .filter(playlists::Column::Deleted.eq(false))
        .order_by_asc(playlists::Column::Id)
        .all(conn)
        .await?;
    let pl_entry_rows = playlist_entries::Entity::find()
        .order_by_asc(playlist_entries::Column::PlaylistId)
        .order_by_asc(playlist_entries::Column::Position)
        .all(conn)
        .await?;
    let mut entries_by_pl: HashMap<String, Vec<SnapshotPlaylistEntry>> = HashMap::new();
    for e in pl_entry_rows {
        let item_path = item_paths.get(&e.item_id).cloned();
        entries_by_pl
            .entry(e.playlist_id)
            .or_default()
            .push(SnapshotPlaylistEntry {
                item_id: e.item_id,
                item_path,
            });
    }
    let playlists_snap: Vec<SnapshotPlaylist> = pl_rows
        .into_iter()
        .map(|pl| {
            let entries = entries_by_pl.remove(&pl.id).unwrap_or_default();
            let username = users_map.get(&pl.user_id).cloned();
            SnapshotPlaylist {
                id: pl.id,
                user_id: pl.user_id,
                username,
                name: pl.name,
                version: pl.version,
                comment: pl.comment,
                created_at: pl.created_at,
                changed_at: pl.changed_at,
                entries,
            }
        })
        .collect();

    // 5. Personal media state
    let item_states_snap: Vec<SnapshotItemState> = user_item_state::Entity::find()
        .order_by_asc(user_item_state::Column::UserId)
        .order_by_asc(user_item_state::Column::ItemId)
        .all(conn)
        .await?
        .into_iter()
        .map(|st| SnapshotItemState {
            username: users_map.get(&st.user_id).cloned(),
            item_path: item_paths.get(&st.item_id).cloned(),
            user_id: st.user_id,
            item_id: st.item_id,
            favorite_order: st.favorite_order,
            stars: st.stars,
            position_ms: st.position_ms,
            completed: st.completed,
            play_count: st.play_count,
            last_played_at: st.last_played_at,
            playback_version: st.playback_version,
        })
        .collect();

    // 6. Photo albums + items
    let album_rows = photo_albums::Entity::find()
        .order_by_asc(photo_albums::Column::Id)
        .all(conn)
        .await?;
    let album_item_rows = photo_album_items::Entity::find()
        .order_by_asc(photo_album_items::Column::AlbumId)
        .order_by_asc(photo_album_items::Column::Position)
        .all(conn)
        .await?;
    let mut items_by_album: HashMap<String, Vec<SnapshotPhotoAlbumItem>> = HashMap::new();
    for ai in album_item_rows {
        items_by_album
            .entry(ai.album_id)
            .or_default()
            .push(SnapshotPhotoAlbumItem {
                item_path: item_paths.get(&ai.item_id).cloned(),
                item_id: ai.item_id,
                position: ai.position,
                added_at: ai.added_at,
            });
    }
    let photo_albums_snap: Vec<SnapshotPhotoAlbum> = album_rows
        .into_iter()
        .map(|a| {
            let cover_item_path = a
                .cover_item_id
                .as_deref()
                .and_then(|cid| item_paths.get(cid))
                .cloned();
            let items = items_by_album.remove(&a.id).unwrap_or_default();
            SnapshotPhotoAlbum {
                username: users_map.get(&a.user_id).cloned(),
                id: a.id,
                user_id: a.user_id,
                title: a.title,
                description: a.description,
                cover_item_id: a.cover_item_id,
                cover_item_path,
                created_at: a.created_at,
                updated_at: a.updated_at,
                items,
            }
        })
        .collect();

    // 7. Reading bookmarks
    let bookmarks_snap: Vec<SnapshotBookmark> = reading_bookmarks::Entity::find()
        .order_by_asc(reading_bookmarks::Column::Id)
        .all(conn)
        .await?
        .into_iter()
        .map(|b| SnapshotBookmark {
            username: users_map.get(&b.user_id).cloned(),
            item_path: item_paths.get(&b.item_id).cloned(),
            id: b.id,
            user_id: b.user_id,
            item_id: b.item_id,
            page: b.page,
            locator: b.locator,
            label: b.label,
            note: b.note,
            created_at: b.created_at,
        })
        .collect();

    // 8. Smart-home rooms, devices, scenes, rules
    let home_rooms_snap: Vec<SnapshotHomeRoom> = home_rooms::Entity::find()
        .order_by_asc(home_rooms::Column::SortOrder)
        .order_by_asc(home_rooms::Column::Id)
        .all(conn)
        .await?
        .into_iter()
        .map(|r| SnapshotHomeRoom {
            id: r.id,
            name: r.name,
            sort_order: r.sort_order,
            created_at: r.created_at,
        })
        .collect();

    let home_devices_snap: Vec<SnapshotHomeDevice> = home_devices::Entity::find()
        .order_by_asc(home_devices::Column::SortOrder)
        .order_by_asc(home_devices::Column::Id)
        .all(conn)
        .await?
        .into_iter()
        .map(|d| SnapshotHomeDevice {
            id: d.id,
            room_id: d.room_id,
            bridge_id: d.bridge_id,
            external_entity_id: d.external_entity_id,
            name: d.name,
            kind: d.kind,
            online: d.online,
            state_json: d.state_json,
            sort_order: d.sort_order,
            updated_at: d.updated_at,
        })
        .collect();

    let home_scenes_snap: Vec<SnapshotHomeScene> = home_scenes::Entity::find()
        .order_by_asc(home_scenes::Column::SortOrder)
        .order_by_asc(home_scenes::Column::Id)
        .all(conn)
        .await?
        .into_iter()
        .map(|s| SnapshotHomeScene {
            id: s.id,
            bridge_id: s.bridge_id,
            external_entity_id: s.external_entity_id,
            name: s.name,
            icon: s.icon,
            active: s.active,
            actions_json: s.actions_json,
            sort_order: s.sort_order,
        })
        .collect();

    let home_rules_snap: Vec<SnapshotHomeRule> = home_rules::Entity::find()
        .order_by_asc(home_rules::Column::SortOrder)
        .order_by_asc(home_rules::Column::Id)
        .all(conn)
        .await?
        .into_iter()
        .map(|r| SnapshotHomeRule {
            id: r.id,
            event_kind: r.event_kind,
            trigger_text: r.trigger_text,
            action_text: r.action_text,
            actions_json: r.actions_json,
            enabled: r.enabled,
            last_run_at: r.last_run_at,
            sort_order: r.sort_order,
        })
        .collect();

    // 9. Assistant memories
    let memories_snap: Vec<SnapshotMemory> = assistant_memories::Entity::find()
        .order_by_asc(assistant_memories::Column::CreatedAt)
        .order_by_asc(assistant_memories::Column::Id)
        .all(conn)
        .await?
        .into_iter()
        .map(|m| SnapshotMemory {
            username: users_map.get(&m.user_id).cloned(),
            id: m.id,
            user_id: m.user_id,
            content: m.content,
            created_at: m.created_at,
        })
        .collect();

    Ok(PalveniSnapshot {
        format_version: 1,
        server_version: VERSION.to_owned(),
        created_at,
        libraries: libraries_snap,
        settings: settings_snap,
        user_policies: user_policies_snap,
        playlists: playlists_snap,
        item_states: item_states_snap,
        photo_albums: photo_albums_snap,
        bookmarks: bookmarks_snap,
        home_rooms: home_rooms_snap,
        home_devices: home_devices_snap,
        home_scenes: home_scenes_snap,
        home_rules: home_rules_snap,
        memories: memories_snap,
    })
}

async fn apply_snapshot<C: ConnectionTrait>(
    conn: &C,
    backup_id: BackupId,
    snapshot: &PalveniSnapshot,
) -> Result<BackupRestoreResult, DbError> {
    let now = now_ms();

    // Build user lookup (by exact ID and by username_key fallback).
    let existing_users = users::Entity::find().all(conn).await?;
    let user_ids: HashSet<String> = existing_users.iter().map(|u| u.id.clone()).collect();
    let user_by_key: HashMap<String, String> = existing_users
        .into_iter()
        .map(|u| (u.username_key, u.id))
        .collect();
    let resolve_user = |uid: &str, uname: Option<&str>| -> Option<String> {
        if user_ids.contains(uid) {
            Some(uid.to_owned())
        } else {
            uname.and_then(|name| user_by_key.get(&username_key(name)).cloned())
        }
    };

    // 1. Restore libraries & paths idempotently
    let mut libraries_restored = 0usize;
    let mut lib_id_remap: HashMap<String, String> = HashMap::new();
    for snap_lib in &snapshot.libraries {
        let key = snap_lib.name.trim().to_lowercase();
        let by_id = libraries::Entity::find_by_id(snap_lib.id.clone())
            .one(conn)
            .await?;
        let target_lib_id = if let Some(existing) = by_id {
            let rev = revision::next_revision(conn).await?;
            let mut active: libraries::ActiveModel = existing.into();
            active.name = Set(snap_lib.name.clone());
            active.name_key = Set(key);
            active.kind = Set(snap_lib.kind.as_str().to_owned());
            active.deleted_at = Set(None);
            active.updated_at = Set(now);
            active.revision = Set(rev);
            libraries::Entity::update(active).exec(conn).await?;
            snap_lib.id.clone()
        } else if let Some(same_name) = libraries::Entity::find()
            .filter(libraries::Column::NameKey.eq(key.clone()))
            .filter(libraries::Column::DeletedAt.is_null())
            .one(conn)
            .await?
        {
            same_name.id
        } else {
            let rev = revision::next_revision(conn).await?;
            libraries::Entity::insert(libraries::ActiveModel {
                id: Set(snap_lib.id.clone()),
                name: Set(snap_lib.name.clone()),
                name_key: Set(key),
                kind: Set(snap_lib.kind.as_str().to_owned()),
                last_scan_at: Set(snap_lib.last_scan_at),
                created_at: Set(snap_lib.created_at),
                updated_at: Set(now),
                revision: Set(rev),
                deleted_at: Set(None),
            })
            .exec_without_returning(conn)
            .await?;
            snap_lib.id.clone()
        };

        lib_id_remap.insert(snap_lib.id.clone(), target_lib_id.clone());

        let existing_paths: Vec<library_paths::Model> = library_paths::Entity::find()
            .filter(library_paths::Column::LibraryId.eq(target_lib_id.clone()))
            .order_by_asc(library_paths::Column::SortOrder)
            .all(conn)
            .await?;
        let existing_path_set: HashSet<String> =
            existing_paths.iter().map(|p| p.path.clone()).collect();
        let mut next_order = existing_paths
            .last()
            .map_or(0, |p| p.sort_order.saturating_add(1));
        for path in &snap_lib.paths {
            if !existing_path_set.contains(path) {
                library_paths::Entity::insert(library_paths::ActiveModel {
                    id: Set(LibraryPathId::generate().to_string()),
                    library_id: Set(target_lib_id.clone()),
                    path: Set(path.clone()),
                    sort_order: Set(next_order),
                    created_at: Set(now),
                })
                .exec_without_returning(conn)
                .await?;
                next_order = next_order.saturating_add(1);
            }
        }
        libraries_restored += 1;
    }

    // 2. Restore runtime settings (never overwriting server.id)
    let mut settings_restored = 0usize;
    for s in &snapshot.settings {
        if s.key == KEY_SERVER_ID {
            continue;
        }
        settings::Entity::insert(settings::ActiveModel {
            setting_key: Set(s.key.clone()),
            setting_value: Set(s.value.clone()),
            updated_at: Set(now),
        })
        .on_conflict(
            OnConflict::column(settings::Column::SettingKey)
                .update_columns([settings::Column::SettingValue, settings::Column::UpdatedAt])
                .to_owned(),
        )
        .exec_without_returning(conn)
        .await?;
        settings_restored += 1;
    }

    // 3. Restore user policies & library ACLs
    let live_lib_ids: HashSet<String> = libraries::Entity::find()
        .filter(libraries::Column::DeletedAt.is_null())
        .select_only()
        .column(libraries::Column::Id)
        .into_tuple::<String>()
        .all(conn)
        .await?
        .into_iter()
        .collect();

    for up in &snapshot.user_policies {
        if let Some(uid_str) = resolve_user(&up.user_id, Some(&up.username))
            && let Ok(uid) = uid_str.parse::<UserId>()
        {
            upsert_policy(conn, uid, &up.policy, now).await?;
            let mapped_libs: Vec<LibraryId> = up
                .allowed_library_ids
                .iter()
                .map(|raw| {
                    lib_id_remap
                        .get(raw)
                        .cloned()
                        .unwrap_or_else(|| raw.clone())
                })
                .filter(|lid| live_lib_ids.contains(lid))
                .filter_map(|lid| lid.parse::<LibraryId>().ok())
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect();
            replace_allowed_libraries(conn, uid, &mapped_libs).await?;
        }
    }

    // Build item lookup (by exact ID and by path fallback).
    let item_rows = items::Entity::find()
        .select_only()
        .column(items::Column::Id)
        .column(items::Column::Path)
        .column(items::Column::DeletedAt)
        .into_tuple::<(String, Option<String>, Option<i64>)>()
        .all(conn)
        .await?;
    let mut all_item_ids = HashSet::new();
    let mut live_item_by_path = HashMap::new();
    for (id, path, deleted_at) in item_rows {
        all_item_ids.insert(id.clone());
        if deleted_at.is_none()
            && let Some(p) = path
        {
            live_item_by_path.insert(p, id);
        }
    }
    let resolve_item = |iid: &str, ipath: Option<&str>| -> Option<String> {
        if all_item_ids.contains(iid) {
            Some(iid.to_owned())
        } else {
            ipath.and_then(|p| live_item_by_path.get(p).cloned())
        }
    };

    // 4. Restore playlists & entries
    let mut playlists_restored = 0usize;
    for pl in &snapshot.playlists {
        let Some(uid) = resolve_user(&pl.user_id, pl.username.as_deref()) else {
            continue;
        };
        let rev = revision::next_revision(conn).await?;
        playlists::Entity::insert(playlists::ActiveModel {
            id: Set(pl.id.clone()),
            user_id: Set(uid),
            name: Set(pl.name.clone()),
            version: Set(rev),
            comment: Set(pl.comment.clone()),
            created_at: Set(pl.created_at.or(Some(now))),
            changed_at: Set(Some(now)),
            deleted: Set(false),
        })
        .on_conflict(
            OnConflict::column(playlists::Column::Id)
                .update_columns([
                    playlists::Column::UserId,
                    playlists::Column::Name,
                    playlists::Column::Version,
                    playlists::Column::Comment,
                    playlists::Column::ChangedAt,
                    playlists::Column::Deleted,
                ])
                .to_owned(),
        )
        .exec_without_returning(conn)
        .await?;

        playlist_entries::Entity::delete_many()
            .filter(playlist_entries::Column::PlaylistId.eq(pl.id.clone()))
            .exec(conn)
            .await?;

        let resolved_entries: Vec<playlist_entries::ActiveModel> = pl
            .entries
            .iter()
            .filter_map(|e| resolve_item(&e.item_id, e.item_path.as_deref()))
            .enumerate()
            .map(|(pos, item_id)| playlist_entries::ActiveModel {
                playlist_id: Set(pl.id.clone()),
                position: Set(i32::try_from(pos).unwrap_or(i32::MAX)),
                item_id: Set(item_id),
            })
            .collect();
        for chunk in resolved_entries.chunks(200) {
            playlist_entries::Entity::insert_many(chunk.to_vec())
                .exec_without_returning(conn)
                .await?;
        }
        playlists_restored += 1;
    }

    // 5. Restore user_item_state
    let mut item_states_restored = 0usize;
    for st in &snapshot.item_states {
        let Some(uid) = resolve_user(&st.user_id, st.username.as_deref()) else {
            continue;
        };
        let Some(iid) = resolve_item(&st.item_id, st.item_path.as_deref()) else {
            continue;
        };
        user_item_state::Entity::insert(user_item_state::ActiveModel {
            user_id: Set(uid),
            item_id: Set(iid),
            favorite_order: Set(st.favorite_order),
            stars: Set(st.stars),
            position_ms: Set(st.position_ms),
            completed: Set(st.completed),
            play_count: Set(st.play_count),
            last_played_at: Set(st.last_played_at),
            playback_version: Set(st.playback_version.max(1)),
            previous_version: Set(None),
        })
        .on_conflict(
            OnConflict::columns([
                user_item_state::Column::UserId,
                user_item_state::Column::ItemId,
            ])
            .update_columns([
                user_item_state::Column::FavoriteOrder,
                user_item_state::Column::Stars,
                user_item_state::Column::PositionMs,
                user_item_state::Column::Completed,
                user_item_state::Column::PlayCount,
                user_item_state::Column::LastPlayedAt,
                user_item_state::Column::PlaybackVersion,
            ])
            .to_owned(),
        )
        .exec_without_returning(conn)
        .await?;
        item_states_restored += 1;
    }

    // 6. Restore photo albums & items
    let mut photo_albums_restored = 0usize;
    for album in &snapshot.photo_albums {
        let Some(uid) = resolve_user(&album.user_id, album.username.as_deref()) else {
            continue;
        };
        let cover_item_id = album
            .cover_item_id
            .as_deref()
            .and_then(|cid| resolve_item(cid, album.cover_item_path.as_deref()));

        photo_albums::Entity::insert(photo_albums::ActiveModel {
            id: Set(album.id.clone()),
            user_id: Set(uid),
            title: Set(album.title.clone()),
            description: Set(album.description.clone()),
            cover_item_id: Set(cover_item_id),
            created_at: Set(album.created_at),
            updated_at: Set(now),
        })
        .on_conflict(
            OnConflict::column(photo_albums::Column::Id)
                .update_columns([
                    photo_albums::Column::UserId,
                    photo_albums::Column::Title,
                    photo_albums::Column::Description,
                    photo_albums::Column::CoverItemId,
                    photo_albums::Column::UpdatedAt,
                ])
                .to_owned(),
        )
        .exec_without_returning(conn)
        .await?;

        photo_album_items::Entity::delete_many()
            .filter(photo_album_items::Column::AlbumId.eq(album.id.clone()))
            .exec(conn)
            .await?;

        let mut seen_album_items = HashSet::new();
        let mut album_models = Vec::new();
        for ai in &album.items {
            if let Some(iid) = resolve_item(&ai.item_id, ai.item_path.as_deref())
                && seen_album_items.insert(iid.clone())
            {
                album_models.push(photo_album_items::ActiveModel {
                    album_id: Set(album.id.clone()),
                    item_id: Set(iid),
                    position: Set(ai.position),
                    added_at: Set(ai.added_at),
                });
            }
        }
        for chunk in album_models.chunks(200) {
            photo_album_items::Entity::insert_many(chunk.to_vec())
                .exec_without_returning(conn)
                .await?;
        }
        photo_albums_restored += 1;
    }

    // 7. Restore reading bookmarks
    let mut bookmarks_restored = 0usize;
    for bm in &snapshot.bookmarks {
        let Some(uid) = resolve_user(&bm.user_id, bm.username.as_deref()) else {
            continue;
        };
        let Some(iid) = resolve_item(&bm.item_id, bm.item_path.as_deref()) else {
            continue;
        };
        reading_bookmarks::Entity::insert(reading_bookmarks::ActiveModel {
            id: Set(bm.id.clone()),
            user_id: Set(uid),
            item_id: Set(iid),
            page: Set(bm.page),
            locator: Set(bm.locator.clone()),
            label: Set(bm.label.clone()),
            note: Set(bm.note.clone()),
            created_at: Set(bm.created_at),
        })
        .on_conflict(
            OnConflict::column(reading_bookmarks::Column::Id)
                .update_columns([
                    reading_bookmarks::Column::UserId,
                    reading_bookmarks::Column::ItemId,
                    reading_bookmarks::Column::Page,
                    reading_bookmarks::Column::Locator,
                    reading_bookmarks::Column::Label,
                    reading_bookmarks::Column::Note,
                ])
                .to_owned(),
        )
        .exec_without_returning(conn)
        .await?;
        bookmarks_restored += 1;
    }

    // 8. Restore smart-home rooms, devices, scenes, rules
    let existing_bridges: HashSet<String> = home_bridges::Entity::find()
        .select_only()
        .column(home_bridges::Column::Id)
        .into_tuple::<String>()
        .all(conn)
        .await?
        .into_iter()
        .collect();

    let mut home_entities_restored = 0usize;
    for room in &snapshot.home_rooms {
        home_rooms::Entity::insert(home_rooms::ActiveModel {
            id: Set(room.id.clone()),
            name: Set(room.name.clone()),
            sort_order: Set(room.sort_order),
            created_at: Set(room.created_at),
        })
        .on_conflict(
            OnConflict::column(home_rooms::Column::Id)
                .update_columns([home_rooms::Column::Name, home_rooms::Column::SortOrder])
                .to_owned(),
        )
        .exec_without_returning(conn)
        .await?;
        home_entities_restored += 1;
    }

    let existing_rooms: HashSet<String> = home_rooms::Entity::find()
        .select_only()
        .column(home_rooms::Column::Id)
        .into_tuple::<String>()
        .all(conn)
        .await?
        .into_iter()
        .collect();

    for dev in &snapshot.home_devices {
        if !existing_rooms.contains(&dev.room_id) {
            continue;
        }
        let bridge_id = dev
            .bridge_id
            .as_ref()
            .filter(|bid| existing_bridges.contains(*bid))
            .cloned();
        home_devices::Entity::insert(home_devices::ActiveModel {
            id: Set(dev.id.clone()),
            room_id: Set(dev.room_id.clone()),
            bridge_id: Set(bridge_id),
            external_entity_id: Set(dev.external_entity_id.clone()),
            name: Set(dev.name.clone()),
            kind: Set(dev.kind.clone()),
            online: Set(dev.online),
            state_json: Set(dev.state_json.clone()),
            sort_order: Set(dev.sort_order),
            updated_at: Set(now),
        })
        .on_conflict(
            OnConflict::column(home_devices::Column::Id)
                .update_columns([
                    home_devices::Column::RoomId,
                    home_devices::Column::BridgeId,
                    home_devices::Column::ExternalEntityId,
                    home_devices::Column::Name,
                    home_devices::Column::Kind,
                    home_devices::Column::Online,
                    home_devices::Column::StateJson,
                    home_devices::Column::SortOrder,
                    home_devices::Column::UpdatedAt,
                ])
                .to_owned(),
        )
        .exec_without_returning(conn)
        .await?;
        home_entities_restored += 1;
    }

    for scene in &snapshot.home_scenes {
        let bridge_id = scene
            .bridge_id
            .as_ref()
            .filter(|bid| existing_bridges.contains(*bid))
            .cloned();
        home_scenes::Entity::insert(home_scenes::ActiveModel {
            id: Set(scene.id.clone()),
            bridge_id: Set(bridge_id),
            external_entity_id: Set(scene.external_entity_id.clone()),
            name: Set(scene.name.clone()),
            icon: Set(scene.icon.clone()),
            active: Set(scene.active),
            actions_json: Set(scene.actions_json.clone()),
            sort_order: Set(scene.sort_order),
        })
        .on_conflict(
            OnConflict::column(home_scenes::Column::Id)
                .update_columns([
                    home_scenes::Column::BridgeId,
                    home_scenes::Column::ExternalEntityId,
                    home_scenes::Column::Name,
                    home_scenes::Column::Icon,
                    home_scenes::Column::Active,
                    home_scenes::Column::ActionsJson,
                    home_scenes::Column::SortOrder,
                ])
                .to_owned(),
        )
        .exec_without_returning(conn)
        .await?;
        home_entities_restored += 1;
    }

    for rule in &snapshot.home_rules {
        home_rules::Entity::insert(home_rules::ActiveModel {
            id: Set(rule.id.clone()),
            event_kind: Set(rule.event_kind.clone()),
            trigger_text: Set(rule.trigger_text.clone()),
            action_text: Set(rule.action_text.clone()),
            actions_json: Set(rule.actions_json.clone()),
            enabled: Set(rule.enabled),
            last_run_at: Set(rule.last_run_at),
            sort_order: Set(rule.sort_order),
        })
        .on_conflict(
            OnConflict::column(home_rules::Column::Id)
                .update_columns([
                    home_rules::Column::EventKind,
                    home_rules::Column::TriggerText,
                    home_rules::Column::ActionText,
                    home_rules::Column::ActionsJson,
                    home_rules::Column::Enabled,
                    home_rules::Column::LastRunAt,
                    home_rules::Column::SortOrder,
                ])
                .to_owned(),
        )
        .exec_without_returning(conn)
        .await?;
        home_entities_restored += 1;
    }

    // 9. Restore assistant memories
    let mut memories_restored = 0usize;
    for mem in &snapshot.memories {
        let Some(uid) = resolve_user(&mem.user_id, mem.username.as_deref()) else {
            continue;
        };
        assistant_memories::Entity::insert(assistant_memories::ActiveModel {
            id: Set(mem.id.clone()),
            user_id: Set(uid),
            content: Set(mem.content.clone()),
            created_at: Set(mem.created_at),
        })
        .on_conflict(
            OnConflict::column(assistant_memories::Column::Id)
                .update_columns([
                    assistant_memories::Column::UserId,
                    assistant_memories::Column::Content,
                ])
                .to_owned(),
        )
        .exec_without_returning(conn)
        .await?;
        memories_restored += 1;
    }

    Ok(BackupRestoreResult {
        backup_id,
        sha256_verified: true,
        libraries_restored,
        settings_restored,
        playlists_restored,
        item_states_restored,
        photo_albums_restored,
        bookmarks_restored,
        home_entities_restored,
        memories_restored,
    })
}

// ---------------------------------------------------------------------------
// Internal helpers: external importer parsing & catalog matching
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Default)]
struct PendingItemImport {
    favorite: bool,
    played: bool,
    play_count: i64,
    position_ms: i64,
    stars: i32,
    last_played_at: Option<i64>,
}

#[derive(Clone, Debug, Default)]
struct ParsedExternalEntry {
    path: Option<String>,
    title: Option<String>,
    artist: Option<String>,
    year: Option<i32>,
    favorite: bool,
    played: bool,
    play_count: i64,
    position_ms: i64,
    stars: i32,
    last_played_at: Option<i64>,
}

impl ParsedExternalEntry {
    fn display_title(&self) -> String {
        if let Some(title) = self.title.as_deref().filter(|s| !s.trim().is_empty()) {
            if let Some(artist) = self.artist.as_deref().filter(|s| !s.trim().is_empty()) {
                return format!("{artist} - {title}");
            }
            if let Some(year) = self.year {
                return format!("{title} ({year})");
            }
            return title.to_owned();
        }
        self.path.as_deref().unwrap_or("Unknown entry").to_owned()
    }
}

struct ParsedExternalPlaylist {
    name: String,
    tracks: Vec<ParsedExternalEntry>,
}

struct ParsedExternalPayload {
    source: ExternalImportSource,
    state_entries: Vec<ParsedExternalEntry>,
    playlists: Vec<ParsedExternalPlaylist>,
}

fn parse_external_payload(
    requested_source: ExternalImportSource,
    content: &str,
    playlist_name_override: Option<&str>,
) -> Result<ParsedExternalPayload, DbError> {
    let trimmed = content.trim().trim_start_matches('\u{feff}');
    if requested_source == ExternalImportSource::M3u
        || (requested_source == ExternalImportSource::Auto
            && !trimmed.starts_with('{')
            && !trimmed.starts_with('['))
    {
        return parse_m3u_payload(trimmed, playlist_name_override);
    }

    let json: serde_json::Value = serde_json::from_str(trimmed)
        .map_err(|_| DbError::Invalid("Invalid JSON or M3U import content"))?;

    let detected = match requested_source {
        ExternalImportSource::Jellyfin => ExternalImportSource::Jellyfin,
        ExternalImportSource::Emby => ExternalImportSource::Emby,
        ExternalImportSource::Navidrome => ExternalImportSource::Navidrome,
        ExternalImportSource::M3u => ExternalImportSource::M3u,
        ExternalImportSource::Auto => detect_json_source(&json),
    };

    let mut state_entries = Vec::new();
    let mut playlists = Vec::new();

    match detected {
        ExternalImportSource::Navidrome => {
            parse_navidrome_json(&json, &mut state_entries, &mut playlists);
        }
        _ => {
            parse_jellyfin_emby_json(&json, &mut state_entries, &mut playlists);
        }
    }

    if let Some(override_name) = playlist_name_override
        .map(str::trim)
        .filter(|s| !s.is_empty())
        && playlists.len() == 1
    {
        playlists[0].name = override_name.to_owned();
    }

    Ok(ParsedExternalPayload {
        source: detected,
        state_entries,
        playlists,
    })
}

fn detect_json_source(value: &serde_json::Value) -> ExternalImportSource {
    if let Some(obj) = value.as_object() {
        if obj.contains_key("subsonic-response")
            || obj.contains_key("starred")
            || obj.contains_key("starred2")
            || obj.contains_key("songs")
            || obj.contains_key("tracks")
        {
            return ExternalImportSource::Navidrome;
        }
        if obj
            .get("ServerType")
            .and_then(|v| v.as_str())
            .is_some_and(|s| s.eq_ignore_ascii_case("emby"))
        {
            return ExternalImportSource::Emby;
        }
        if obj.contains_key("Items") || obj.contains_key("UserData") {
            return ExternalImportSource::Jellyfin;
        }
    }
    if let Some(arr) = value.as_array()
        && let Some(first) = arr.first().and_then(|v| v.as_object())
        && (first.contains_key("starred")
            || first.contains_key("playCount") && first.contains_key("artist")
            || first.contains_key("userRating"))
        && !first.contains_key("UserData")
    {
        return ExternalImportSource::Navidrome;
    }
    ExternalImportSource::Jellyfin
}

fn parse_m3u_payload(
    text: &str,
    playlist_name_override: Option<&str>,
) -> Result<ParsedExternalPayload, DbError> {
    let mut playlist_name = playlist_name_override
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToOwned::to_owned);
    let mut pending_extinf: Option<(Option<String>, Option<String>)> = None;
    let mut tracks = Vec::new();

    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(rest) = line.strip_prefix("#PLAYLIST:") {
            if playlist_name.is_none() {
                let name = rest.trim();
                if !name.is_empty() {
                    playlist_name = Some(name.to_owned());
                }
            }
            continue;
        }
        if let Some(rest) = line.strip_prefix("#EXTINF:") {
            let display = rest.split_once(',').map_or(rest, |(_, info)| info).trim();
            if let Some((artist, title)) = display.split_once(" - ") {
                pending_extinf = Some((
                    Some(artist.trim().to_owned()).filter(|s| !s.is_empty()),
                    Some(title.trim().to_owned()).filter(|s| !s.is_empty()),
                ));
            } else if !display.is_empty() {
                pending_extinf = Some((None, Some(display.to_owned())));
            }
            continue;
        }
        if line.starts_with('#') {
            continue;
        }
        let (artist, title) = pending_extinf.take().unwrap_or((None, None));
        tracks.push(ParsedExternalEntry {
            path: Some(line.to_owned()),
            title,
            artist,
            ..Default::default()
        });
    }

    if tracks.is_empty() {
        return Err(DbError::Invalid("M3U playlist contains no track entries"));
    }

    Ok(ParsedExternalPayload {
        source: ExternalImportSource::M3u,
        state_entries: Vec::new(),
        playlists: vec![ParsedExternalPlaylist {
            name: playlist_name.unwrap_or_else(|| "Imported Playlist".to_owned()),
            tracks,
        }],
    })
}

fn str_field(obj: &serde_json::Map<String, serde_json::Value>, keys: &[&str]) -> Option<String> {
    for &key in keys {
        if let Some(s) = obj.get(key).and_then(|v| v.as_str()).map(str::trim)
            && !s.is_empty()
        {
            return Some(s.to_owned());
        }
    }
    None
}

fn bool_field(obj: &serde_json::Map<String, serde_json::Value>, keys: &[&str]) -> Option<bool> {
    for &key in keys {
        if let Some(v) = obj.get(key) {
            if let Some(b) = v.as_bool() {
                return Some(b);
            }
            if let Some(s) = v.as_str()
                && !s.trim().is_empty()
                && s != "false"
                && s != "0"
            {
                // Subsonic `starred` is often an ISO timestamp string when starred!
                return Some(true);
            }
        }
    }
    None
}

fn i64_field(obj: &serde_json::Map<String, serde_json::Value>, keys: &[&str]) -> Option<i64> {
    for &key in keys {
        if let Some(v) = obj.get(key) {
            if let Some(n) = v.as_i64() {
                return Some(n);
            }
            if let Some(u) = v.as_u64() {
                return i64::try_from(u).ok();
            }
        }
    }
    None
}

fn parse_rfc3339_ms(raw: &str) -> Option<i64> {
    DateTime::parse_from_rfc3339(raw.trim())
        .ok()
        .map(|dt| dt.timestamp_millis())
}

fn parse_jellyfin_item(
    obj: &serde_json::Map<String, serde_json::Value>,
) -> Option<ParsedExternalEntry> {
    let path = str_field(obj, &["Path", "path"]).or_else(|| {
        obj.get("MediaSources")
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
            .and_then(|v| v.as_object())
            .and_then(|ms| str_field(ms, &["Path", "path"]))
    });
    let title = str_field(obj, &["Name", "Title", "title", "OriginalTitle"]);
    let artist = str_field(obj, &["AlbumArtist", "Artist", "artist"]).or_else(|| {
        obj.get("Artists")
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
            .and_then(|v| v.as_str())
            .map(|s| s.trim().to_owned())
            .filter(|s| !s.is_empty())
    });
    let year =
        i64_field(obj, &["ProductionYear", "Year", "year"]).and_then(|y| i32::try_from(y).ok());

    let user_data = obj
        .get("UserData")
        .or_else(|| obj.get("userData"))
        .and_then(|v| v.as_object());
    let ud_or_self = |f: fn(&serde_json::Map<String, serde_json::Value>) -> Option<i64>| {
        user_data.and_then(f).or_else(|| f(obj))
    };
    let favorite = user_data
        .and_then(|ud| bool_field(ud, &["IsFavorite", "isFavorite", "Favorite", "favorite"]))
        .or_else(|| bool_field(obj, &["IsFavorite", "isFavorite", "Favorite", "favorite"]))
        .unwrap_or(false);
    let played = user_data
        .and_then(|ud| bool_field(ud, &["Played", "played", "Completed", "completed"]))
        .or_else(|| bool_field(obj, &["Played", "played", "Completed", "completed"]))
        .unwrap_or(false);
    let play_count = ud_or_self(|m| i64_field(m, &["PlayCount", "playCount"])).unwrap_or(0);
    let position_ms = ud_or_self(|m| {
        i64_field(m, &["PlaybackPositionTicks", "playbackPositionTicks"])
            .map(|ticks| ticks / 10_000)
    })
    .or_else(|| ud_or_self(|m| i64_field(m, &["PositionMs", "positionMs"])))
    .unwrap_or(0);
    let rating_10 = user_data
        .and_then(|ud| ud.get("Rating").or_else(|| ud.get("CustomRating")))
        .or_else(|| obj.get("UserRating"))
        .and_then(|v| v.as_f64());
    let stars = rating_10.map_or(0, |r| {
        if r >= 9.0 {
            5
        } else if r >= 7.0 {
            4
        } else if r >= 5.0 {
            3
        } else if r >= 3.0 {
            2
        } else if r > 0.0 {
            1
        } else {
            0
        }
    });
    let last_played_at = user_data
        .and_then(|ud| str_field(ud, &["LastPlayedDate", "lastPlayedDate"]))
        .or_else(|| str_field(obj, &["LastPlayedDate", "lastPlayedDate"]))
        .and_then(|s| parse_rfc3339_ms(&s));

    if path.is_none() && title.is_none() {
        return None;
    }
    Some(ParsedExternalEntry {
        path,
        title,
        artist,
        year,
        favorite,
        played,
        play_count,
        position_ms,
        stars,
        last_played_at,
    })
}

fn parse_jellyfin_emby_json(
    value: &serde_json::Value,
    state_entries: &mut Vec<ParsedExternalEntry>,
    playlists: &mut Vec<ParsedExternalPlaylist>,
) {
    if let Some(arr) = value.as_array() {
        for item in arr {
            if let Some(obj) = item.as_object() {
                if obj
                    .get("Type")
                    .and_then(|v| v.as_str())
                    .is_some_and(|t| t.eq_ignore_ascii_case("playlist"))
                {
                    if let Some(pl) = parse_json_playlist(obj) {
                        playlists.push(pl);
                    }
                } else if let Some(entry) = parse_jellyfin_item(obj) {
                    state_entries.push(entry);
                }
            }
        }
        return;
    }
    let Some(root) = value.as_object() else {
        return;
    };
    if let Some(items_arr) = root
        .get("Items")
        .or_else(|| root.get("items"))
        .and_then(|v| v.as_array())
    {
        for item in items_arr {
            if let Some(obj) = item.as_object() {
                if obj
                    .get("Type")
                    .and_then(|v| v.as_str())
                    .is_some_and(|t| t.eq_ignore_ascii_case("playlist"))
                {
                    if let Some(pl) = parse_json_playlist(obj) {
                        playlists.push(pl);
                    }
                } else if let Some(entry) = parse_jellyfin_item(obj) {
                    state_entries.push(entry);
                }
            }
        }
    } else if let Some(single) = parse_jellyfin_item(root) {
        state_entries.push(single);
    }

    if let Some(pl_arr) = root
        .get("Playlists")
        .or_else(|| root.get("playlists"))
        .and_then(|v| v.as_array())
    {
        for pl_val in pl_arr {
            if let Some(obj) = pl_val.as_object()
                && let Some(pl) = parse_json_playlist(obj)
            {
                playlists.push(pl);
            }
        }
    }
}

fn parse_navidrome_song(
    obj: &serde_json::Map<String, serde_json::Value>,
    force_favorite: bool,
) -> Option<ParsedExternalEntry> {
    let path = str_field(obj, &["path", "Path", "filePath"]);
    let title = str_field(obj, &["title", "Title", "name", "Name"]);
    let artist = str_field(obj, &["artist", "albumArtist", "Artist", "AlbumArtist"]);
    let year = i64_field(obj, &["year", "Year"]).and_then(|y| i32::try_from(y).ok());
    let favorite =
        force_favorite || bool_field(obj, &["starred", "isFavorite", "favorite"]).unwrap_or(false);
    let play_count = i64_field(obj, &["playCount", "PlayCount"]).unwrap_or(0);
    let played = play_count > 0 || bool_field(obj, &["played"]).unwrap_or(false);
    let stars = i64_field(obj, &["userRating", "rating"])
        .and_then(|r| i32::try_from(r).ok())
        .map(|s| s.clamp(0, 5))
        .unwrap_or(0);
    let last_played_at =
        str_field(obj, &["playDate", "played", "lastPlayedAt"]).and_then(|s| parse_rfc3339_ms(&s));

    if path.is_none() && title.is_none() {
        return None;
    }
    Some(ParsedExternalEntry {
        path,
        title,
        artist,
        year,
        favorite,
        played,
        play_count,
        position_ms: 0,
        stars,
        last_played_at,
    })
}

fn parse_json_playlist(
    obj: &serde_json::Map<String, serde_json::Value>,
) -> Option<ParsedExternalPlaylist> {
    let name = str_field(obj, &["Name", "name", "title", "Title"])
        .unwrap_or_else(|| "Imported Playlist".to_owned());
    let tracks_arr = obj
        .get("Items")
        .or_else(|| obj.get("items"))
        .or_else(|| obj.get("Children"))
        .or_else(|| obj.get("tracks"))
        .or_else(|| obj.get("entry"))
        .and_then(|v| v.as_array())?;
    let mut tracks = Vec::new();
    for t in tracks_arr {
        if let Some(tobj) = t.as_object()
            && let Some(entry) =
                parse_navidrome_song(tobj, false).or_else(|| parse_jellyfin_item(tobj))
        {
            tracks.push(entry);
        }
    }
    (!tracks.is_empty()).then_some(ParsedExternalPlaylist { name, tracks })
}

fn parse_navidrome_json(
    value: &serde_json::Value,
    state_entries: &mut Vec<ParsedExternalEntry>,
    playlists: &mut Vec<ParsedExternalPlaylist>,
) {
    if let Some(arr) = value.as_array() {
        for item in arr {
            if let Some(obj) = item.as_object()
                && let Some(song) = parse_navidrome_song(obj, false)
            {
                state_entries.push(song);
            }
        }
        return;
    }
    let Some(mut root) = value.as_object() else {
        return;
    };
    if let Some(sub) = root.get("subsonic-response").and_then(|v| v.as_object()) {
        root = sub;
    }

    for starred_key in ["starred", "starred2"] {
        if let Some(starred_val) = root.get(starred_key) {
            if let Some(arr) = starred_val.as_array() {
                for item in arr {
                    if let Some(obj) = item.as_object()
                        && let Some(song) = parse_navidrome_song(obj, true)
                    {
                        state_entries.push(song);
                    }
                }
            } else if let Some(sobj) = starred_val.as_object()
                && let Some(songs) = sobj.get("song").and_then(|v| v.as_array())
            {
                for item in songs {
                    if let Some(obj) = item.as_object()
                        && let Some(song) = parse_navidrome_song(obj, true)
                    {
                        state_entries.push(song);
                    }
                }
            }
        }
    }

    for list_key in ["songs", "tracks", "items"] {
        if let Some(arr) = root.get(list_key).and_then(|v| v.as_array()) {
            for item in arr {
                if let Some(obj) = item.as_object()
                    && let Some(song) = parse_navidrome_song(obj, false)
                {
                    state_entries.push(song);
                }
            }
        }
    }

    if let Some(pl_val) = root.get("playlists").or_else(|| root.get("Playlists")) {
        if let Some(arr) = pl_val.as_array() {
            for pl in arr {
                if let Some(obj) = pl.as_object()
                    && let Some(parsed) = parse_json_playlist(obj)
                {
                    playlists.push(parsed);
                }
            }
        } else if let Some(pobj) = pl_val.as_object()
            && let Some(arr) = pobj.get("playlist").and_then(|v| v.as_array())
        {
            for pl in arr {
                if let Some(obj) = pl.as_object()
                    && let Some(parsed) = parse_json_playlist(obj)
                {
                    playlists.push(parsed);
                }
            }
        }
    }
}

#[derive(Clone, Debug)]
struct CatalogHit {
    id: String,
    title: String,
}

struct CatalogMatcher {
    by_exact_path: HashMap<String, CatalogHit>,
    by_suffix3: HashMap<String, CatalogHit>,
    by_suffix2: HashMap<String, CatalogHit>,
    by_filename: HashMap<String, Vec<CatalogHit>>,
    by_title_artist: HashMap<(String, String), CatalogHit>,
    by_title_year: HashMap<(String, i32), CatalogHit>,
    by_title_only: HashMap<String, Vec<CatalogHit>>,
}

fn normalize_text_key(raw: &str) -> String {
    raw.trim()
        .to_lowercase()
        .chars()
        .filter(|c| !c.is_control())
        .collect::<String>()
}

fn normalize_path_segments(raw: &str) -> Vec<String> {
    let cleaned = raw
        .trim()
        .strip_prefix("file://")
        .unwrap_or(raw.trim())
        .replace('\\', "/")
        .replace("%20", " ");
    cleaned
        .split('/')
        .map(str::trim)
        .filter(|seg| !seg.is_empty() && *seg != "." && !seg.ends_with(':'))
        .map(str::to_lowercase)
        .collect()
}

impl CatalogMatcher {
    fn match_entry(&self, entry: &ParsedExternalEntry) -> Option<&CatalogHit> {
        if let Some(raw_path) = entry.path.as_deref() {
            let segs = normalize_path_segments(raw_path);
            if !segs.is_empty() {
                let full = segs.join("/");
                if let Some(hit) = self.by_exact_path.get(&full) {
                    return Some(hit);
                }
                if segs.len() >= 3 {
                    let s3 = segs[segs.len() - 3..].join("/");
                    if let Some(hit) = self.by_suffix3.get(&s3) {
                        return Some(hit);
                    }
                }
                if segs.len() >= 2 {
                    let s2 = segs[segs.len() - 2..].join("/");
                    if let Some(hit) = self.by_suffix2.get(&s2) {
                        return Some(hit);
                    }
                }
                if let Some(last) = segs.last()
                    && let Some(candidates) = self.by_filename.get(last)
                    && candidates.len() == 1
                {
                    return candidates.first();
                }
            }
        }

        if let Some(raw_title) = entry.title.as_deref() {
            let tkey = normalize_text_key(raw_title);
            if !tkey.is_empty() {
                if let Some(raw_artist) = entry.artist.as_deref() {
                    let akey = normalize_text_key(raw_artist);
                    if !akey.is_empty()
                        && let Some(hit) = self.by_title_artist.get(&(tkey.clone(), akey))
                    {
                        return Some(hit);
                    }
                }
                if let Some(year) = entry.year
                    && let Some(hit) = self.by_title_year.get(&(tkey.clone(), year))
                {
                    return Some(hit);
                }
                if let Some(candidates) = self.by_title_only.get(&tkey)
                    && candidates.len() == 1
                {
                    return candidates.first();
                }
            }
        }

        None
    }
}

async fn load_catalog_matcher<C: ConnectionTrait>(conn: &C) -> Result<CatalogMatcher, DbError> {
    let live_items = items::Entity::find()
        .filter(items::Column::DeletedAt.is_null())
        .filter(items::Column::Kind.ne("folder"))
        .select_only()
        .column(items::Column::Id)
        .column(items::Column::Title)
        .column(items::Column::Path)
        .into_tuple::<(String, String, Option<String>)>()
        .all(conn)
        .await?;

    let mut by_exact_path = HashMap::new();
    let mut by_suffix3 = HashMap::new();
    let mut by_suffix2 = HashMap::new();
    let mut by_filename: HashMap<String, Vec<CatalogHit>> = HashMap::new();
    let mut by_title_artist = HashMap::new();
    let mut by_title_year = HashMap::new();
    let mut by_title_only: HashMap<String, Vec<CatalogHit>> = HashMap::new();
    let mut live_ids = HashSet::new();

    for (id, title, path) in live_items {
        live_ids.insert(id.clone());
        let hit = CatalogHit {
            id: id.clone(),
            title: title.clone(),
        };
        let tkey = normalize_text_key(&title);
        if !tkey.is_empty() {
            by_title_only.entry(tkey).or_default().push(hit.clone());
        }
        if let Some(raw_path) = path {
            let segs = normalize_path_segments(&raw_path);
            if !segs.is_empty() {
                by_exact_path.insert(segs.join("/"), hit.clone());
                if segs.len() >= 3 {
                    by_suffix3
                        .entry(segs[segs.len() - 3..].join("/"))
                        .or_insert_with(|| hit.clone());
                }
                if segs.len() >= 2 {
                    by_suffix2
                        .entry(segs[segs.len() - 2..].join("/"))
                        .or_insert_with(|| hit.clone());
                }
                if let Some(last) = segs.last() {
                    by_filename.entry(last.clone()).or_default().push(hit);
                }
            }
        }
    }

    let tracks = track_metadata::Entity::find()
        .select_only()
        .column(track_metadata::Column::ItemId)
        .column(track_metadata::Column::Title)
        .column(track_metadata::Column::Artist)
        .column(track_metadata::Column::AlbumArtist)
        .column(track_metadata::Column::Year)
        .into_tuple::<(String, String, Option<String>, Option<String>, Option<i64>)>()
        .all(conn)
        .await?;

    for (item_id, title, artist, album_artist, year) in tracks {
        if !live_ids.contains(&item_id) {
            continue;
        }
        let hit = CatalogHit {
            id: item_id,
            title: title.clone(),
        };
        let tkey = normalize_text_key(&title);
        if tkey.is_empty() {
            continue;
        }
        let list = by_title_only.entry(tkey.clone()).or_default();
        if !list.iter().any(|existing| existing.id == hit.id) {
            list.push(hit.clone());
        }
        for candidate_artist in [artist.as_deref(), album_artist.as_deref()]
            .into_iter()
            .flatten()
        {
            let akey = normalize_text_key(candidate_artist);
            if !akey.is_empty() {
                by_title_artist
                    .entry((tkey.clone(), akey))
                    .or_insert_with(|| hit.clone());
            }
        }
        if let Some(y) = year.and_then(|v| i32::try_from(v).ok()) {
            by_title_year
                .entry((tkey, y))
                .or_insert_with(|| hit.clone());
        }
    }

    let videos = video_metadata::Entity::find()
        .select_only()
        .column(video_metadata::Column::ItemId)
        .column(video_metadata::Column::WorkTitle)
        .column(video_metadata::Column::Title)
        .column(video_metadata::Column::WorkYear)
        .into_tuple::<(String, String, String, Option<i32>)>()
        .all(conn)
        .await?;

    for (item_id, work_title, ep_title, work_year) in videos {
        if !live_ids.contains(&item_id) {
            continue;
        }
        let hit = CatalogHit {
            id: item_id,
            title: work_title.clone(),
        };
        for raw in [&work_title, &ep_title] {
            let tkey = normalize_text_key(raw);
            if tkey.is_empty() {
                continue;
            }
            let list = by_title_only.entry(tkey.clone()).or_default();
            if !list.iter().any(|existing| existing.id == hit.id) {
                list.push(hit.clone());
            }
            if let Some(year) = work_year {
                by_title_year
                    .entry((tkey, year))
                    .or_insert_with(|| hit.clone());
            }
        }
    }

    Ok(CatalogMatcher {
        by_exact_path,
        by_suffix3,
        by_suffix2,
        by_filename,
        by_title_artist,
        by_title_year,
        by_title_only,
    })
}

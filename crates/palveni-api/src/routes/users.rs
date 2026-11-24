//! Multi-user administration (`/api/v1/users*`) and self-service password &
//! active session endpoints (`/api/v1/me/password`, `/api/v1/me/sessions*`).

use axum::Json;
use axum::extract::rejection::PathRejection;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use palveni_core::accounts::{
    ChangePasswordRequest, CreateUserRequest, MAX_DAILY_AI_LIMIT, ManagedUser, UpdateUserRequest,
    UserList, UserSessionList,
};
use palveni_core::{SessionId, UserId};
use palveni_db::DbError;
use palveni_db::accounts::{AccountUpdate, NewManagedUser};
use palveni_db::audit::{AuditEntry, actions};
use serde_json::json;

use crate::auth::{
    AdminUser, CurrentUser, PASSWORD_MAX_CHARS, dummy_hash, hash_password, validate_password,
    validate_username, verify_password,
};
use crate::error::ApiError;
use crate::json::ApiJson;
use crate::routes::{audit, path_id};
use crate::state::AppState;

type IdPath = Result<Path<String>, PathRejection>;

fn validate_ai_limit(limit: i32) -> Result<(), ApiError> {
    if !(0..=MAX_DAILY_AI_LIMIT).contains(&limit) {
        return Err(ApiError::validation(format!(
            "dailyAiLimit must be between 0 and {MAX_DAILY_AI_LIMIT}"
        ))
        .field("policy"));
    }
    Ok(())
}

/// `GET /users` (Admin).
pub async fn list_users(
    _admin: AdminUser,
    State(state): State<AppState>,
) -> Result<Json<UserList>, ApiError> {
    let users = state.db.accounts().list_users().await?;
    Ok(Json(UserList { users }))
}

/// `POST /users` (Admin).
pub async fn create_user(
    AdminUser(admin): AdminUser,
    State(state): State<AppState>,
    body: Result<ApiJson<CreateUserRequest>, ApiError>,
) -> Result<(StatusCode, Json<ManagedUser>), ApiError> {
    let ApiJson(request) = body?;
    let username = request.username.trim();
    validate_username(username)?;
    validate_password(&request.password)?;

    let policy = request.policy.unwrap_or_default();
    validate_ai_limit(policy.daily_ai_limit)?;

    let password_hash = hash_password(request.password).await?;
    let created = state
        .db
        .accounts()
        .create_user(NewManagedUser {
            username: username.to_owned(),
            password_hash,
            role: request.role,
            policy,
            allowed_library_ids: request.allowed_library_ids,
        })
        .await
        .map_err(|err| match err {
            DbError::Conflict(msg) => ApiError::conflict(msg).field("username"),
            DbError::Invalid(msg) => ApiError::validation(msg).field("allowedLibraryIds"),
            other => other.into(),
        })?;

    audit(
        &state,
        AuditEntry {
            action: actions::USER_CREATED,
            actor: Some(admin.user.id),
            target: Some(created.username.clone()),
            detail: Some(json!({
                "userId": created.id,
                "role": created.role,
                "allLibraries": created.policy.all_libraries,
            })),
        },
    )
    .await;

    Ok((StatusCode::CREATED, Json(created)))
}

/// `PATCH /users/{id}` (Admin).
pub async fn update_user(
    AdminUser(admin): AdminUser,
    State(state): State<AppState>,
    path: IdPath,
    body: Result<ApiJson<UpdateUserRequest>, ApiError>,
) -> Result<Json<ManagedUser>, ApiError> {
    let id: UserId = path_id(&path?.0, "User")?;
    let ApiJson(request) = body?;

    if request.role.is_none()
        && request.disabled.is_none()
        && request.password.is_none()
        && request.policy.is_none()
        && request.allowed_library_ids.is_none()
    {
        return Err(ApiError::validation(
            "At least one field must be provided to update a user",
        ));
    }

    if let Some(policy_patch) = &request.policy
        && let Some(limit) = policy_patch.daily_ai_limit
    {
        validate_ai_limit(limit)?;
    }

    let password_hash = match request.password {
        Some(password) => {
            validate_password(&password)?;
            Some(hash_password(password).await?)
        }
        None => None,
    };
    let password_reset = password_hash.is_some();

    let updated = state
        .db
        .accounts()
        .update_user(
            id,
            AccountUpdate {
                role: request.role,
                disabled: request.disabled,
                password_hash,
                policy: request.policy,
                allowed_library_ids: request.allowed_library_ids,
                keep_session_id: (id == admin.user.id).then_some(admin.session_id),
            },
        )
        .await
        .map_err(|err| match err {
            DbError::Conflict(msg) => ApiError::conflict(msg),
            DbError::Invalid(msg) => ApiError::validation(msg).field("allowedLibraryIds"),
            other => other.into(),
        })?;

    audit(
        &state,
        AuditEntry {
            action: actions::USER_UPDATED,
            actor: Some(admin.user.id),
            target: Some(updated.username.clone()),
            detail: Some(json!({
                "userId": updated.id,
                "role": updated.role,
                "disabled": updated.disabled,
                "passwordReset": password_reset,
                "allLibraries": updated.policy.all_libraries,
            })),
        },
    )
    .await;

    Ok(Json(updated))
}

/// `DELETE /users/{id}` (Admin).
pub async fn delete_user(
    AdminUser(admin): AdminUser,
    State(state): State<AppState>,
    path: IdPath,
) -> Result<StatusCode, ApiError> {
    let id: UserId = path_id(&path?.0, "User")?;
    if id == admin.user.id {
        return Err(ApiError::conflict(
            "Cannot delete your currently signed-in administrator account",
        ));
    }
    let existing = state.db.accounts().get_user(id).await?;
    if !state.db.accounts().delete_user(id).await? {
        return Err(ApiError::not_found("User not found"));
    }

    audit(
        &state,
        AuditEntry {
            action: actions::USER_DELETED,
            actor: Some(admin.user.id),
            target: existing
                .as_ref()
                .map(|u| u.username.clone())
                .or_else(|| Some(id.to_string())),
            detail: Some(json!({ "userId": id })),
        },
    )
    .await;

    Ok(StatusCode::NO_CONTENT)
}

/// `PUT /me/password` (Authenticated).
pub async fn change_my_password(
    current: CurrentUser,
    State(state): State<AppState>,
    body: Result<ApiJson<ChangePasswordRequest>, ApiError>,
) -> Result<StatusCode, ApiError> {
    let ApiJson(request) = body?;
    validate_password(&request.new_password).map_err(|err| err.field("newPassword"))?;

    let valid_current = if request.current_password.chars().count() > PASSWORD_MAX_CHARS {
        false
    } else {
        match state
            .db
            .users()
            .find_credentials(&current.user.username)
            .await?
        {
            Some(found) if !found.disabled => {
                verify_password(request.current_password, found.password_hash).await?
            }
            _ => {
                if let Some(dummy) = dummy_hash().await {
                    let _ignored = verify_password(request.current_password, dummy).await;
                }
                false
            }
        }
    };

    if !valid_current {
        return Err(ApiError::invalid_credentials().field("currentPassword"));
    }

    let new_hash = hash_password(request.new_password).await?;
    let revoked_sessions = if request.revoke_other_sessions {
        state
            .db
            .accounts()
            .change_password(current.user.id, &new_hash, Some(current.session_id))
            .await?
    } else {
        state
            .db
            .accounts()
            .update_password_only(current.user.id, &new_hash)
            .await?;
        0
    };

    audit(
        &state,
        AuditEntry {
            action: actions::PASSWORD_CHANGED,
            actor: Some(current.user.id),
            target: Some(current.user.username),
            detail: Some(json!({ "revokedSessions": revoked_sessions })),
        },
    )
    .await;

    Ok(StatusCode::NO_CONTENT)
}

/// `GET /me/sessions` (Authenticated).
pub async fn list_my_sessions(
    current: CurrentUser,
    State(state): State<AppState>,
) -> Result<Json<UserSessionList>, ApiError> {
    let sessions = state
        .db
        .accounts()
        .list_user_sessions(current.user.id, current.session_id)
        .await?;
    Ok(Json(UserSessionList { sessions }))
}

/// `DELETE /me/sessions/{id}` (Authenticated).
pub async fn revoke_my_session(
    current: CurrentUser,
    State(state): State<AppState>,
    path: IdPath,
) -> Result<StatusCode, ApiError> {
    let target: SessionId = path_id(&path?.0, "Session")?;
    if !state
        .db
        .accounts()
        .revoke_user_session(current.user.id, target)
        .await?
    {
        return Err(ApiError::not_found("Session not found"));
    }

    audit(
        &state,
        AuditEntry {
            action: actions::SESSION_REVOKED,
            actor: Some(current.user.id),
            target: Some(target.to_string()),
            detail: Some(json!({ "self": target == current.session_id })),
        },
    )
    .await;

    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use axum::body::to_bytes;
    use axum::extract::{Path, Query, State};
    use axum::http::StatusCode;
    use palveni_core::accounts::{
        ChangePasswordRequest, CreateBackupRequest, CreateUserRequest, ExternalImportRequest,
        ExternalImportSource, UpdateUserRequest, UserPolicy,
    };
    use palveni_core::{FeatureRegistry, LibraryKind, PerfTier, Role, ServerId};
    use palveni_db::{Db, DbOptions};
    use palveni_jobs::JobService;
    use palveni_media::{HardwareService, ProbeOptions};
    use palveni_plugin_host::{PluginCatalog, skin_store::SkinStore};
    use tokio_util::sync::CancellationToken;

    use super::*;
    use crate::routes::libraries;
    use crate::routes::maintenance::{
        AuditQuery, create_backup, delete_backup, download_backup, import_external, list_audit,
        list_backups, restore_backup,
    };

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn users_library_acl_sessions_backups_import_and_audit_end_to_end() {
        let dir = std::env::temp_dir().join(format!("palveni-users-api-{}", ServerId::generate()));
        std::fs::create_dir_all(&dir).unwrap();
        let movies_dir = dir.join("movies");
        let kids_dir = dir.join("kids");
        std::fs::create_dir_all(&movies_dir).unwrap();
        std::fs::create_dir_all(&kids_dir).unwrap();

        let db = Db::open(&DbOptions {
            main_url: format!("sqlite://{}/main.db?mode=rwc", dir.display()),
            ops_url: format!("sqlite://{}/ops.db?mode=rwc", dir.display()),
            max_connections: Some(4),
            min_connections: Some(1),
        })
        .await
        .unwrap();

        let admin_hash = hash_password("adminpass123".into()).await.unwrap();
        let admin_user = db
            .users()
            .create_first_admin("admin", &admin_hash)
            .await
            .unwrap()
            .unwrap();
        let admin_sess = db
            .sessions()
            .create(admin_user.id, "Admin Mac", None)
            .await
            .unwrap();

        let bus = db.event_bus(16).await.unwrap();
        let jobs = JobService::new(db.clone(), bus.clone(), 1);
        let state = AppState {
            db: db.clone(),
            music_search: Arc::new(palveni_library::music_search::MusicSearch::new(
                dir.join("search"),
            )),
            video_images: Arc::new(palveni_library::video::VideoImages::new(
                dir.join("cache"),
                None,
                None,
            )),
            identity: Arc::new(palveni_link::NodeIdentity::generate().unwrap()),
            link: Arc::new(crate::LinkPairings::default()),
            login_limit: Arc::new(crate::LoginLimit::new(4)),
            subsonic_limit: Arc::new(crate::LoginLimit::new(4)),
            bus,
            jobs,
            hardware: Arc::new(HardwareService::new(
                db.clone(),
                dir.clone(),
                ProbeOptions::default(),
            )),
            plugins: Arc::new(PluginCatalog::new()),
            settings: Arc::new(tokio::sync::RwLock::new(Default::default())),
            boot_tier: PerfTier::detect(0, 1, false),
            skins: Arc::new(SkinStore::open(&dir.join("skins")).unwrap()),
            skin_writes: Arc::new(tokio::sync::Semaphore::new(1)),
            features: FeatureRegistry::new(),
            server: Arc::new(crate::ServerInfo {
                id: ServerId::generate(),
                name: "Test".into(),
                version: "0.1.0".into(),
            }),
            shutdown: CancellationToken::new(),
        };

        let admin_caller = CurrentUser {
            user: admin_user.clone(),
            session_id: admin_sess.session_id,
            device_id: Some(admin_sess.device_id),
        };

        let movies = db
            .libraries()
            .create(palveni_db::libraries::NewLibrary {
                name: "Movies".into(),
                kind: LibraryKind::Video,
                paths: vec![movies_dir.display().to_string()],
            })
            .await
            .unwrap();
        let kids = db
            .libraries()
            .create(palveni_db::libraries::NewLibrary {
                name: "Kids".into(),
                kind: LibraryKind::Video,
                paths: vec![kids_dir.display().to_string()],
            })
            .await
            .unwrap();

        // Create restricted member via POST /users
        let (status, Json(created_member)) = create_user(
            AdminUser(admin_caller.clone()),
            State(state.clone()),
            Ok(ApiJson(CreateUserRequest {
                username: "alice".into(),
                password: "alicepassword".into(),
                role: Role::User,
                policy: Some(UserPolicy {
                    all_libraries: false,
                    allow_download: false,
                    allow_transcode: true,
                    daily_ai_limit: 20,
                }),
                allowed_library_ids: vec![kids.id],
            })),
        )
        .await
        .unwrap();
        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(created_member.username, "alice");

        let alice_sess1 = db
            .sessions()
            .create(created_member.id, "Alice Phone", None)
            .await
            .unwrap();
        let alice_sess2 = db
            .sessions()
            .create(created_member.id, "Alice Tablet", None)
            .await
            .unwrap();
        let alice_caller = CurrentUser {
            user: palveni_core::User {
                id: created_member.id,
                username: created_member.username.clone(),
                role: Role::User,
                created_at: created_member.created_at,
            },
            session_id: alice_sess1.session_id,
            device_id: Some(alice_sess1.device_id),
        };

        // Library ACL enforced on GET /libraries and GET /libraries/{id}
        let Json(alice_libs) = libraries::list(alice_caller.clone(), State(state.clone()))
            .await
            .unwrap();
        let alice_libs_val = serde_json::to_value(&alice_libs).unwrap();
        assert_eq!(alice_libs_val["libraries"].as_array().unwrap().len(), 1);
        assert_eq!(alice_libs_val["libraries"][0]["id"], kids.id.to_string());
        assert!(
            libraries::get_one(
                alice_caller.clone(),
                State(state.clone()),
                Ok(Path(movies.id.to_string())),
            )
            .await
            .is_err()
        );
        assert!(
            libraries::get_one(
                alice_caller.clone(),
                State(state.clone()),
                Ok(Path(kids.id.to_string())),
            )
            .await
            .is_ok()
        );

        // Self-service sessions & password change
        let Json(my_sessions) = list_my_sessions(alice_caller.clone(), State(state.clone()))
            .await
            .unwrap();
        assert_eq!(my_sessions.sessions.len(), 2);
        assert!(my_sessions.sessions[0].current);

        // Wrong current password rejected without revoking session
        let wrong = change_my_password(
            alice_caller.clone(),
            State(state.clone()),
            Ok(ApiJson(ChangePasswordRequest {
                current_password: "wrongpassword".into(),
                new_password: "newalicepassword".into(),
                revoke_other_sessions: true,
            })),
        )
        .await
        .unwrap_err();
        assert_eq!(wrong.code(), "invalid_credentials");

        let pw_status = change_my_password(
            alice_caller.clone(),
            State(state.clone()),
            Ok(ApiJson(ChangePasswordRequest {
                current_password: "alicepassword".into(),
                new_password: "newalicepassword".into(),
                revoke_other_sessions: true,
            })),
        )
        .await
        .unwrap();
        assert_eq!(pw_status, StatusCode::NO_CONTENT);
        assert!(
            db.sessions()
                .authenticate(&alice_sess2.token)
                .await
                .unwrap()
                .is_none()
        );

        // Last enabled admin protection
        assert!(
            update_user(
                AdminUser(admin_caller.clone()),
                State(state.clone()),
                Ok(Path(admin_user.id.to_string())),
                Ok(ApiJson(UpdateUserRequest {
                    disabled: Some(true),
                    ..Default::default()
                })),
            )
            .await
            .is_err()
        );

        // Backup create, download, restore, delete
        let (b_status, Json(backup)) = create_backup(
            AdminUser(admin_caller.clone()),
            State(state.clone()),
            Ok(ApiJson(CreateBackupRequest {
                name: Some("Daily Snapshot".into()),
            })),
        )
        .await
        .unwrap();
        assert_eq!(b_status, StatusCode::CREATED);
        assert_eq!(
            list_backups(AdminUser(admin_caller.clone()), State(state.clone()))
                .await
                .unwrap()
                .0
                .backups
                .len(),
            1
        );

        let dl_resp = download_backup(
            AdminUser(admin_caller.clone()),
            State(state.clone()),
            Ok(Path(backup.id.to_string())),
        )
        .await
        .unwrap();
        assert_eq!(dl_resp.status(), StatusCode::OK);
        let dl_bytes = to_bytes(dl_resp.into_body(), usize::MAX).await.unwrap();
        assert!(!dl_bytes.is_empty());

        let Json(restored) = restore_backup(
            AdminUser(admin_caller.clone()),
            State(state.clone()),
            Ok(Path(backup.id.to_string())),
        )
        .await
        .unwrap();
        assert!(restored.sha256_verified);
        assert_eq!(restored.libraries_restored, 2);

        // External M3U dry-run import
        let Json(import_res) = import_external(
            alice_caller.clone(),
            State(state.clone()),
            Ok(ApiJson(ExternalImportRequest {
                source: ExternalImportSource::M3u,
                content: "#EXTM3U\n/nonexistent/song.flac\n".into(),
                playlist_name: Some("Test".into()),
                dry_run: true,
            })),
        )
        .await
        .unwrap();
        assert_eq!(import_res.total_entries, 1);

        // Audit log records user.created, auth.password_changed, system.backup_created, system.backup_restored
        let Json(audit_page) = list_audit(
            AdminUser(admin_caller.clone()),
            State(state.clone()),
            Ok(Query(AuditQuery::default())),
        )
        .await
        .unwrap();
        let actions_seen: Vec<&str> = audit_page
            .entries
            .iter()
            .map(|e| e.action.as_str())
            .collect();
        assert!(actions_seen.contains(&"user.created"));
        assert!(actions_seen.contains(&"auth.password_changed"));
        assert!(actions_seen.contains(&"system.backup_created"));
        assert!(actions_seen.contains(&"system.backup_restored"));

        // Cleanup
        assert_eq!(
            delete_backup(
                AdminUser(admin_caller.clone()),
                State(state.clone()),
                Ok(Path(backup.id.to_string())),
            )
            .await
            .unwrap(),
            StatusCode::NO_CONTENT
        );
        assert_eq!(
            delete_user(
                AdminUser(admin_caller.clone()),
                State(state.clone()),
                Ok(Path(created_member.id.to_string())),
            )
            .await
            .unwrap(),
            StatusCode::NO_CONTENT
        );
        assert_eq!(
            list_users(AdminUser(admin_caller), State(state))
                .await
                .unwrap()
                .0
                .users
                .len(),
            1
        );

        db.close().await.unwrap();
        let _ = std::fs::remove_dir_all(&dir);
    }
}

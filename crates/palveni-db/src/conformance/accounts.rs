use palveni_core::accounts::{
    ExternalImportRequest, ExternalImportSource, UserPolicy, UserPolicyPatch,
};
use palveni_core::{ItemId, ItemKind, LibraryKind, Role};
use serde_json::json;

use super::{Harness, track};
use crate::DbError;
use crate::accounts::{AccountUpdate, NewManagedUser};
use crate::audit::{AuditEntry, actions};
use crate::items::ItemUpsert;
use crate::libraries::NewLibrary;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn conformance_accounts_acl_sessions_audit_backups_and_import() {
    let h = Harness::new().await;

    // 1. Create first admin + libraries
    let admin =
        h.db.users()
            .create_first_admin("admin", "hash-admin-1")
            .await
            .expect("setup")
            .expect("first admin");

    let movies =
        h.db.libraries()
            .create(NewLibrary {
                name: "Movies".into(),
                kind: LibraryKind::Video,
                paths: vec!["/srv/movies".into()],
            })
            .await
            .expect("movies lib");
    let music =
        h.db.libraries()
            .create(NewLibrary {
                name: "Music".into(),
                kind: LibraryKind::Music,
                paths: vec!["/srv/music".into()],
            })
            .await
            .expect("music lib");
    let kids =
        h.db.libraries()
            .create(NewLibrary {
                name: "Kids".into(),
                kind: LibraryKind::Video,
                paths: vec!["/srv/kids".into()],
            })
            .await
            .expect("kids lib");

    // 2. Last enabled admin cannot be disabled, demoted, or deleted!
    assert!(matches!(
        h.db.accounts()
            .update_user(
                admin.id,
                AccountUpdate {
                    disabled: Some(true),
                    ..Default::default()
                },
            )
            .await,
        Err(DbError::Conflict(_))
    ));
    assert!(matches!(
        h.db.accounts()
            .update_user(
                admin.id,
                AccountUpdate {
                    role: Some(Role::User),
                    ..Default::default()
                },
            )
            .await,
        Err(DbError::Conflict(_))
    ));
    assert!(matches!(
        h.db.accounts().delete_user(admin.id).await,
        Err(DbError::Conflict(_))
    ));

    // 3. Create restricted family member with explicit library ACL
    let child =
        h.db.accounts()
            .create_user(NewManagedUser {
                username: "kid".into(),
                password_hash: "hash-kid-1".into(),
                role: Role::User,
                policy: UserPolicy {
                    all_libraries: false,
                    allow_download: false,
                    allow_transcode: true,
                    daily_ai_limit: 15,
                },
                allowed_library_ids: vec![kids.id, music.id],
            })
            .await
            .expect("create kid");
    assert_eq!(child.role, Role::User);
    assert!(!child.policy.all_libraries);
    assert!(!child.policy.allow_download);
    assert_eq!(child.policy.daily_ai_limit, 15);
    assert_eq!(child.allowed_library_ids.len(), 2);

    // Admin sees all 3 libraries; restricted member sees only Kids & Music.
    let all_libs = h.db.libraries().list().await.expect("list libs");
    assert_eq!(
        h.db.accounts()
            .filter_libraries(admin.id, Role::Admin, all_libs.clone())
            .await
            .unwrap()
            .len(),
        3
    );
    let kid_libs =
        h.db.accounts()
            .filter_libraries(child.id, Role::User, all_libs)
            .await
            .unwrap();
    assert_eq!(kid_libs.len(), 2);
    assert!(
        h.db.accounts()
            .can_access_library(child.id, Role::User, kids.id)
            .await
            .unwrap()
    );
    assert!(
        !h.db
            .accounts()
            .can_access_library(child.id, Role::User, movies.id)
            .await
            .unwrap()
    );

    // 4. Sessions, self-service password change, and revocation on disable
    let s1 =
        h.db.sessions()
            .create(child.id, "iPad", None)
            .await
            .unwrap();
    let s2 =
        h.db.sessions()
            .create(child.id, "Browser", None)
            .await
            .unwrap();

    let listed_sessions =
        h.db.accounts()
            .list_user_sessions(child.id, s2.session_id)
            .await
            .unwrap();
    assert_eq!(listed_sessions.len(), 2);
    assert!(listed_sessions[0].current);
    assert_eq!(listed_sessions[0].device_name, "Browser");

    // Changing password keeps s2 and revokes s1
    let revoked_count =
        h.db.accounts()
            .change_password(child.id, "hash-kid-2", Some(s2.session_id))
            .await
            .unwrap();
    assert_eq!(revoked_count, 1);
    assert!(
        h.db.sessions()
            .authenticate(&s1.token)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        h.db.sessions()
            .authenticate(&s2.token)
            .await
            .unwrap()
            .is_some()
    );

    // Disabling user revokes remaining session s2 immediately
    let s3 = h.db.sessions().create(child.id, "TV", None).await.unwrap();
    let disabled_user =
        h.db.accounts()
            .update_user(
                child.id,
                AccountUpdate {
                    disabled: Some(true),
                    policy: Some(UserPolicyPatch {
                        daily_ai_limit: Some(5),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
    assert!(disabled_user.disabled);
    assert_eq!(disabled_user.active_sessions, 0);
    assert_eq!(disabled_user.policy.daily_ai_limit, 5);
    assert!(
        h.db.sessions()
            .authenticate(&s2.token)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        h.db.sessions()
            .authenticate(&s3.token)
            .await
            .unwrap()
            .is_none()
    );

    // Re-enable child
    h.db.accounts()
        .update_user(
            child.id,
            AccountUpdate {
                disabled: Some(false),
                ..Default::default()
            },
        )
        .await
        .unwrap();

    // 5. Enriched & filterable audit log across ops + main stores
    h.db.audit()
        .record(AuditEntry {
            action: actions::USER_CREATED,
            actor: Some(admin.id),
            target: Some(child.username.clone()),
            detail: Some(json!({ "role": "user" })),
        })
        .await
        .unwrap();
    h.db.audit()
        .record(AuditEntry {
            action: actions::PASSWORD_CHANGED,
            actor: Some(child.id),
            target: None,
            detail: None,
        })
        .await
        .unwrap();

    let all_audit =
        h.db.accounts()
            .list_audit_logs(None, None, 1)
            .await
            .unwrap();
    assert_eq!(all_audit.entries.len(), 1);
    assert!(all_audit.next_cursor.is_some());
    assert_eq!(all_audit.entries[0].actor_username.as_deref(), Some("kid"));
    let page2 =
        h.db.accounts()
            .list_audit_logs(None, all_audit.next_cursor.as_deref(), 10)
            .await
            .unwrap();
    assert_eq!(page2.entries.len(), 1);
    assert_eq!(page2.entries[0].actor_username.as_deref(), Some("admin"));

    let filtered_audit =
        h.db.accounts()
            .list_audit_logs(Some("user."), None, 10)
            .await
            .unwrap();
    assert_eq!(filtered_audit.entries.len(), 1);
    assert_eq!(filtered_audit.entries[0].action, "user.created");

    // 6. Seed catalog items + external history & playlist importer (Jellyfin, Navidrome, M3U)
    let track1 = track(
        "Blue Train",
        "/srv/music/Coltrane/01 Blue Train.flac",
        None,
        1024,
    );
    let track2 = track(
        "Lazy Bird",
        "/srv/music/Coltrane/02 Lazy Bird.flac",
        None,
        2048,
    );
    let movie1 = ItemUpsert {
        music: None,
        id: ItemId::generate(),
        parent_id: None,
        kind: ItemKind::Video,
        title: "Spirited Away".into(),
        sort_title: "spirited away".into(),
        search_text: "spirited away".into(),
        path: "/srv/movies/Miyazaki/Spirited Away (2001).mkv".into(),
        size_bytes: Some(4096),
        modified_at: Some(1_700_000_000_000),
    };
    h.db.items()
        .apply_scan_batch(music.id, &[track1.clone(), track2.clone()], &[])
        .await
        .unwrap();
    h.db.items()
        .apply_scan_batch(movies.id, std::slice::from_ref(&movie1), &[])
        .await
        .unwrap();

    // Jellyfin dry-run then apply
    let jellyfin_payload = json!({
        "Items": [
            {
                "Name": "Spirited Away",
                "Path": "D:\\Media\\Miyazaki\\Spirited Away (2001).mkv",
                "ProductionYear": 2001,
                "UserData": {
                    "Played": false,
                    "PlayCount": 1,
                    "IsFavorite": true,
                    "PlaybackPositionTicks": 120_000_000
                }
            },
            {
                "Name": "Missing Film",
                "Path": "/other/Missing Film.mkv",
                "UserData": { "Played": true }
            }
        ]
    })
    .to_string();

    let preview =
        h.db.accounts()
            .import_external_state(
                admin.id,
                &ExternalImportRequest {
                    source: ExternalImportSource::Auto,
                    content: jellyfin_payload.clone(),
                    playlist_name: None,
                    dry_run: true,
                },
            )
            .await
            .unwrap();
    assert_eq!(preview.detected_source, ExternalImportSource::Jellyfin);
    assert!(preview.dry_run);
    assert_eq!(preview.matched_items, 1);
    assert_eq!(preview.unmatched_entries, 1);
    assert_eq!(preview.favorites_imported, 1);
    assert_eq!(preview.history_imported, 1);
    assert!(
        !h.db
            .personal()
            .get(admin.id, movie1.id)
            .await
            .unwrap()
            .favorite,
        "dry_run must not mutate DB"
    );

    let applied_jf =
        h.db.accounts()
            .import_external_state(
                admin.id,
                &ExternalImportRequest {
                    source: ExternalImportSource::Auto,
                    content: jellyfin_payload,
                    playlist_name: None,
                    dry_run: false,
                },
            )
            .await
            .unwrap();
    assert!(!applied_jf.dry_run);
    let movie_state = h.db.personal().get(admin.id, movie1.id).await.unwrap();
    assert!(movie_state.favorite);
    assert!(!movie_state.completed);
    assert_eq!(movie_state.play_count, 1);
    assert_eq!(movie_state.position_ms, 12_000);

    // Navidrome JSON import
    let navidrome_payload = json!({
        "starred": [
            { "title": "Blue Train", "path": "/music/Coltrane/01 Blue Train.flac", "playCount": 5, "userRating": 5 }
        ]
    })
    .to_string();
    let applied_nd =
        h.db.accounts()
            .import_external_state(
                admin.id,
                &ExternalImportRequest {
                    source: ExternalImportSource::Auto,
                    content: navidrome_payload,
                    playlist_name: None,
                    dry_run: false,
                },
            )
            .await
            .unwrap();
    assert_eq!(applied_nd.detected_source, ExternalImportSource::Navidrome);
    assert_eq!(applied_nd.matched_items, 1);
    let track1_state = h.db.personal().get(admin.id, track1.id).await.unwrap();
    assert!(track1_state.favorite);
    assert_eq!(track1_state.stars, 5);

    // M3U playlist import
    let m3u_text = "#EXTM3U\n#PLAYLIST:Late Night Jazz\n#EXTINF:640,John Coltrane - Blue Train\nColtrane/01 Blue Train.flac\n#EXTINF:420,John Coltrane - Lazy Bird\nColtrane/02 Lazy Bird.flac\n";
    let applied_m3u =
        h.db.accounts()
            .import_external_state(
                admin.id,
                &ExternalImportRequest {
                    source: ExternalImportSource::Auto,
                    content: m3u_text.into(),
                    playlist_name: None,
                    dry_run: false,
                },
            )
            .await
            .unwrap();
    assert_eq!(applied_m3u.detected_source, ExternalImportSource::M3u);
    assert_eq!(applied_m3u.playlists_imported, 1);
    assert_eq!(applied_m3u.playlist_tracks_imported, 2);

    // 7. Portable snapshot backup, SHA-256 verification, and idempotent restore!
    h.db.settings()
        .set(
            "server.runtime",
            r#"{"serverName":"Home Hub","perfTier":null}"#,
        )
        .await
        .unwrap();
    let _ = h.db.home().snapshot().await.unwrap();
    h.db.assistant()
        .add_memory(admin.id, "Prefers lossless jazz")
        .await
        .unwrap();

    let backup =
        h.db.accounts()
            .create_backup(Some(admin.id), Some("Before Cleanup"))
            .await
            .unwrap();
    assert_eq!(backup.name, "Before Cleanup");
    assert_eq!(backup.creator_username.as_deref(), Some("admin"));
    assert_eq!(backup.counts.libraries, 3);
    assert_eq!(backup.counts.playlists, 1);
    assert!(backup.counts.item_states >= 2);
    assert_eq!(backup.counts.memories, 1);

    let (summary, payload) =
        h.db.accounts()
            .get_backup_payload(backup.id)
            .await
            .unwrap()
            .expect("backup payload");
    assert_eq!(summary.sha256, backup.sha256);
    assert!(!payload.is_empty());

    // Mutate state, then restore twice to prove restoration & idempotency
    h.db.settings()
        .set(
            "server.runtime",
            r#"{"serverName":"Broken","perfTier":"tiny"}"#,
        )
        .await
        .unwrap();
    h.db.libraries().delete(kids.id).await.unwrap();
    assert_eq!(h.db.libraries().list().await.unwrap().len(), 2);

    let restore1 = h.db.accounts().restore_backup(backup.id).await.unwrap();
    assert!(restore1.sha256_verified);
    assert_eq!(restore1.libraries_restored, 3);
    assert_eq!(restore1.playlists_restored, 1);

    let restore2 = h.db.accounts().restore_backup(backup.id).await.unwrap();
    assert_eq!(restore1, restore2);
    assert_eq!(h.db.libraries().list().await.unwrap().len(), 3);
    assert_eq!(
        h.db.settings()
            .get("server.runtime")
            .await
            .unwrap()
            .as_deref(),
        Some(r#"{"serverName":"Home Hub","perfTier":null}"#)
    );

    // Deleting non-admin user succeeds; backup delete succeeds
    assert!(h.db.accounts().delete_user(child.id).await.unwrap());
    assert!(!h.db.accounts().delete_user(child.id).await.unwrap());
    assert!(h.db.accounts().delete_backup(backup.id).await.unwrap());
    assert!(!h.db.accounts().delete_backup(backup.id).await.unwrap());

    h.finish().await;
}

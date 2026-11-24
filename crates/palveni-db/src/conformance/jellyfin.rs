use palveni_core::jellyfin::{
    ClientApiKeyKind, VideoSegmentInput, VideoSegmentKind, VideoSegmentSource,
};
use palveni_core::{ItemKind, LibraryKind, Role};

use super::{Harness, NewLibrary, track};
use crate::DbError;
use crate::items::ItemUpsert;
use crate::jellyfin::{ChapterMarker, infer_segments_from_chapters};

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn conformance_client_api_keys_and_video_segments() {
    let h = Harness::new().await;
    let admin =
        h.db.users()
            .create("admin", "pw-hash", Role::Admin)
            .await
            .expect("create admin");
    let alice =
        h.db.users()
            .create("alice", "pw-hash", Role::User)
            .await
            .expect("create alice");
    let bob =
        h.db.users()
            .create("bob", "pw-hash", Role::User)
            .await
            .expect("create bob");

    let repo = h.db.jellyfin();

    // 1. Client API key creation, lookup, last_used_at update, and ACL revocation
    let created = repo
        .create_api_key(alice.id, "Infuse Apple TV", ClientApiKeyKind::Jellyfin)
        .await
        .expect("create api key");
    assert!(created.token.starts_with("pvk_"));
    assert_eq!(created.key.name, "Infuse Apple TV");
    assert_eq!(created.key.client_kind, ClientApiKeyKind::Jellyfin);
    assert!(created.token.starts_with(&created.key.key_prefix));
    assert!(created.key.last_used_at.is_none());
    assert!(created.key.revoked_at.is_none());

    let listed = repo.list_api_keys(alice.id).await.expect("list keys");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, created.key.id);

    // Authenticates active user and stamps last_used_at
    let (authed_user, key_id) = repo
        .authenticate_api_key(&created.token)
        .await
        .expect("auth lookup")
        .expect("valid key");
    assert_eq!(authed_user.id, alice.id);
    assert_eq!(key_id, created.key.id);

    let after_use = repo.list_api_keys(alice.id).await.expect("list after use");
    assert!(after_use[0].last_used_at.is_some());

    // Invalid key returns None
    assert!(
        repo.authenticate_api_key("pvk_does_not_exist")
            .await
            .expect("invalid lookup")
            .is_none()
    );

    // Another non-admin user cannot revoke alice's key
    assert!(
        !repo
            .revoke_api_key(&created.key.id, bob.id, false)
            .await
            .expect("bob revoke")
    );

    // Owner can revoke, after which lookup and active listing reject it
    assert!(
        repo.revoke_api_key(&created.key.id, alice.id, false)
            .await
            .expect("alice revoke")
    );
    assert!(
        repo.authenticate_api_key(&created.token)
            .await
            .expect("revoked lookup")
            .is_none()
    );
    assert!(
        repo.list_api_keys(alice.id)
            .await
            .expect("list empty")
            .is_empty()
    );

    // Admin can revoke another user's key
    let bob_key = repo
        .create_api_key(bob.id, "VidHub iPad", ClientApiKeyKind::Jellyfin)
        .await
        .expect("bob key");
    assert!(
        repo.revoke_api_key(&bob_key.key.id, admin.id, true)
            .await
            .expect("admin revoke")
    );

    // Cascade deletion when user is deleted
    let temp_user =
        h.db.users()
            .create("temp", "pw-hash", Role::User)
            .await
            .expect("temp user");
    let temp_key = repo
        .create_api_key(temp_user.id, "Temp Key", ClientApiKeyKind::General)
        .await
        .expect("temp key");
    assert!(
        h.db.accounts()
            .delete_user(temp_user.id)
            .await
            .expect("delete temp user")
    );
    assert!(
        repo.authenticate_api_key(&temp_key.token)
            .await
            .expect("cascaded lookup")
            .is_none()
    );

    // 2. Video segments CRUD + validation + cascade on library delete
    let lib =
        h.db.libraries()
            .create(NewLibrary {
                name: "Anime".into(),
                kind: LibraryKind::Video,
                paths: vec!["/media/anime".into()],
            })
            .await
            .expect("create library");
    let item = ItemUpsert {
        kind: ItemKind::Video,
        ..track(
            "Show S01E01",
            "/media/anime/Show S01E01.mkv",
            None,
            500_000_000,
        )
    };
    let ep_id = item.id;
    h.db.items()
        .apply_scan_batch(lib.id, &[item], &[])
        .await
        .expect("upsert ep");

    assert!(
        repo.list_video_segments(ep_id)
            .await
            .expect("list empty segments")
            .is_empty()
    );

    // Invalid bounds rejected
    let invalid = repo
        .replace_video_segments(
            ep_id,
            &[VideoSegmentInput {
                kind: VideoSegmentKind::Intro,
                start_ms: 90_000,
                end_ms: 90_000,
                source: Some(VideoSegmentSource::Manual),
            }],
        )
        .await;
    assert!(matches!(invalid, Err(DbError::Invalid(_))));

    let saved = repo
        .replace_video_segments(
            ep_id,
            &[
                VideoSegmentInput {
                    kind: VideoSegmentKind::Credits,
                    start_ms: 1_320_000,
                    end_ms: 1_410_000,
                    source: Some(VideoSegmentSource::Manual),
                },
                VideoSegmentInput {
                    kind: VideoSegmentKind::Intro,
                    start_ms: 30_000,
                    end_ms: 120_000,
                    source: None,
                },
            ],
        )
        .await
        .expect("replace segments");
    assert_eq!(saved.len(), 2);
    // Ordered by start_ms ascending
    assert_eq!(saved[0].kind, VideoSegmentKind::Intro);
    assert_eq!(saved[0].start_ms, 30_000);
    assert_eq!(saved[0].end_ms, 120_000);
    assert_eq!(saved[0].source, VideoSegmentSource::Manual);
    assert_eq!(saved[1].kind, VideoSegmentKind::Credits);

    let reloaded = repo.list_video_segments(ep_id).await.expect("reload");
    assert_eq!(reloaded.len(), 2);
    assert_eq!(reloaded[0].kind, VideoSegmentKind::Intro);

    // Replacing with a single segment overwrites the prior rows
    let replaced = repo
        .replace_video_segments(
            ep_id,
            &[VideoSegmentInput {
                kind: VideoSegmentKind::Recap,
                start_ms: 0,
                end_ms: 45_000,
                source: Some(VideoSegmentSource::Auto),
            }],
        )
        .await
        .expect("overwrite segments");
    assert_eq!(replaced.len(), 1);
    assert_eq!(replaced[0].kind, VideoSegmentKind::Recap);
    assert_eq!(replaced[0].source, VideoSegmentSource::Auto);

    // 3. Chapter auto-inference
    let inferred = infer_segments_from_chapters(
        ep_id,
        &[
            ChapterMarker {
                start_ms: 0,
                title: Some("Cold Open".into()),
            },
            ChapterMarker {
                start_ms: 45_000,
                title: Some("OP".into()),
            },
            ChapterMarker {
                start_ms: 135_000,
                title: Some("Part A".into()),
            },
            ChapterMarker {
                start_ms: 1_260_000,
                title: Some("片尾曲".into()),
            },
        ],
        Some(1_350_000),
    );
    assert_eq!(inferred.len(), 2);
    assert_eq!(inferred[0].kind, VideoSegmentKind::Intro);
    assert_eq!(inferred[0].start_ms, 45_000);
    assert_eq!(inferred[0].end_ms, 135_000);
    assert_eq!(inferred[0].source, VideoSegmentSource::Chapter);
    assert_eq!(inferred[1].kind, VideoSegmentKind::Credits);
    assert_eq!(inferred[1].start_ms, 1_260_000);
    assert_eq!(inferred[1].end_ms, 1_350_000);

    // 4. Playback reporting (Jellyfin ticks -> ms)
    let state = repo
        .report_playback(alice.id, ep_id, 65_000, false, false)
        .await
        .expect("report progress");
    assert_eq!(state.position_ms, 65_000);
    assert!(!state.completed);

    let done = repo
        .report_playback(alice.id, ep_id, 1_350_000, true, true)
        .await
        .expect("report completed");
    assert!(done.completed);
    assert_eq!(done.position_ms, 0);
    assert!(done.play_count >= 1);

    h.finish().await;
}

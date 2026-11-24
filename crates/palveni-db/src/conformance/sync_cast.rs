use palveni_core::ids::PlaylistId;
use palveni_core::sync_cast::{
    PlaybackClientKind, PlaybackMediaKind, PlaybackSessionState, SyncMutation,
};
use palveni_core::{BookmarkId, ItemId, ItemKind, LibraryKind, Role};

use super::{Harness, new_library, track};
use crate::items::ItemUpsert;
use crate::libraries::NewLibrary;
use crate::sync_cast::{
    ACTIVE_SESSION_WINDOW_MS, SaveMqttBridgeInput, SyncCursor, UpsertPlaybackSession,
    UpsertSessionOutcome,
};

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn conformance_sync_sessions_and_mqtt_bridges() {
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

    let repo = h.db.sync_cast();

    // 1. Playback sessions CRUD, ownership enforcement & user cascade
    let outcome = repo
        .upsert_playback_session(
            alice.id,
            UpsertPlaybackSession {
                session_id: "sess-alice-web".into(),
                device_name: "Alice MacBook".into(),
                client_kind: PlaybackClientKind::Web,
                media_kind: PlaybackMediaKind::Audio,
                item_id: Some(ItemId::generate().to_string()),
                library_id: None,
                title: Some("晴天".into()),
                subtitle: Some("周杰伦".into()),
                artwork_id: None,
                state: PlaybackSessionState::Playing,
                position_ms: 42_000,
                duration_ms: 269_000,
                volume: 75,
                ambient_hex: Some("#D97706".into()),
            },
        )
        .await
        .expect("upsert alice session");
    let session = match outcome {
        UpsertSessionOutcome::Saved(session) => *session,
        UpsertSessionOutcome::ForbiddenOwner => {
            unreachable!("alice owns her new playback session")
        }
    };
    assert_eq!(session.id, "sess-alice-web");
    assert_eq!(session.user_id, alice.id);
    assert_eq!(session.ambient_hex.as_deref(), Some("#D97706"));

    // Another user cannot hijack an existing session ID
    assert!(matches!(
        repo.upsert_playback_session(
            bob.id,
            UpsertPlaybackSession {
                session_id: "sess-alice-web".into(),
                device_name: "Bob Hijack".into(),
                client_kind: PlaybackClientKind::Ios,
                media_kind: PlaybackMediaKind::Audio,
                item_id: None,
                library_id: None,
                title: None,
                subtitle: None,
                artwork_id: None,
                state: PlaybackSessionState::Paused,
                position_ms: 0,
                duration_ms: 0,
                volume: 50,
                ambient_hex: None,
            },
        )
        .await
        .expect("hijack check"),
        UpsertSessionOutcome::ForbiddenOwner
    ));

    // Bob's own session
    assert!(matches!(
        repo.upsert_playback_session(
            bob.id,
            UpsertPlaybackSession {
                session_id: "sess-bob-tv".into(),
                device_name: "Living Room Apple TV".into(),
                client_kind: PlaybackClientKind::Tvos,
                media_kind: PlaybackMediaKind::Video,
                item_id: None,
                library_id: None,
                title: Some("Dune".into()),
                subtitle: None,
                artwork_id: None,
                state: PlaybackSessionState::Playing,
                position_ms: 100_000,
                duration_ms: 9_000_000,
                volume: 60,
                ambient_hex: None,
            },
        )
        .await
        .expect("upsert bob session"),
        UpsertSessionOutcome::Saved(_)
    ));

    let alice_sessions = repo
        .list_active_playback_sessions(Some(alice.id), ACTIVE_SESSION_WINDOW_MS)
        .await
        .expect("list alice sessions");
    assert_eq!(alice_sessions.len(), 1);
    assert_eq!(alice_sessions[0].id, "sess-alice-web");

    let all_sessions = repo
        .list_active_playback_sessions(None, ACTIVE_SESSION_WINDOW_MS)
        .await
        .expect("list all sessions");
    assert_eq!(all_sessions.len(), 2);

    assert!(
        repo.delete_playback_session("sess-bob-tv")
            .await
            .expect("delete bob session")
    );
    assert!(
        repo.get_playback_session("sess-bob-tv")
            .await
            .expect("get deleted")
            .is_none()
    );

    // 2. MQTT bridges CRUD, ciphertext preservation & summary redaction
    let bridge = repo
        .save_mqtt_bridge(SaveMqttBridgeInput {
            id: "mqtt-1".into(),
            name: "Zigbee2MQTT".into(),
            host: "192.168.1.50".into(),
            port: 1883,
            username: Some("home".into()),
            password_cipher: Some("nonce.ciphertext".into()),
            keep_existing_password: true,
            topic_prefix: "palveni".into(),
            ambient_light_device_id: Some("living-strip".into()),
            publish_playback: true,
            enabled: true,
        })
        .await
        .expect("save mqtt bridge");
    assert!(bridge.to_summary().password_configured);
    assert_eq!(
        bridge.ambient_light_device_id.as_deref(),
        Some("living-strip")
    );

    let updated_bridge = repo
        .save_mqtt_bridge(SaveMqttBridgeInput {
            id: "mqtt-1".into(),
            name: "Zigbee2MQTT Updated".into(),
            host: "192.168.1.50".into(),
            port: 1883,
            username: Some("home".into()),
            password_cipher: None,
            keep_existing_password: true,
            topic_prefix: "palveni/home".into(),
            ambient_light_device_id: Some("living-strip".into()),
            publish_playback: true,
            enabled: true,
        })
        .await
        .expect("update mqtt bridge");
    assert_eq!(
        updated_bridge.password_cipher.as_deref(),
        Some("nonce.ciphertext")
    );
    assert_eq!(updated_bridge.topic_prefix, "palveni/home");

    repo.touch_mqtt_bridge_published("mqtt-1", 1_700_000_999_000)
        .await
        .expect("touch published");
    let reloaded = repo
        .get_mqtt_bridge("mqtt-1")
        .await
        .expect("get bridge")
        .expect("exists");
    assert!(reloaded.last_published_at.is_some());
    assert_eq!(
        repo.list_enabled_mqtt_bridges()
            .await
            .expect("enabled bridges")
            .len(),
        1
    );
    assert!(
        repo.delete_mqtt_bridge("mqtt-1")
            .await
            .expect("delete bridge")
    );
    assert!(repo.list_mqtt_bridges().await.expect("empty").is_empty());

    // 3. Incremental sync delta pagination, tombstones, ACL & push mutations
    let music = new_library(&h.db, "SyncMusic").await;
    let books =
        h.db.libraries()
            .create(NewLibrary {
                name: "SyncBooks".into(),
                kind: LibraryKind::Book,
                paths: vec!["/media/SyncBooks".into()],
            })
            .await
            .expect("create books lib");

    let t1 = track("Alpha", "/media/SyncMusic/a.flac", None, 100);
    let t2 = track("Beta", "/media/SyncMusic/b.flac", None, 200);
    let t3 = track("Gamma", "/media/SyncMusic/c.flac", None, 300);
    h.db.items()
        .apply_scan_batch(music.id, &[t1.clone(), t2.clone(), t3.clone()], &[])
        .await
        .expect("seed music items");

    let book = ItemUpsert {
        music: None,
        id: ItemId::generate(),
        parent_id: None,
        kind: ItemKind::Book,
        title: "Three Body".into(),
        sort_title: "three body".into(),
        search_text: "three body".into(),
        path: "/media/SyncBooks/three-body.epub".into(),
        size_bytes: Some(500),
        modified_at: Some(1_700_000_000_000),
    };
    h.db.items()
        .apply_scan_batch(books.id, std::slice::from_ref(&book), &[])
        .await
        .expect("seed book item");

    // Tombstone t3
    h.db.items()
        .apply_scan_batch(music.id, &[], &[t3.id])
        .await
        .expect("tombstone t3");

    // Playlist owned by admin
    let pl_id = PlaylistId::generate();
    h.db.playlists()
        .save(admin.id, pl_id, 0, "Night Drive", vec![t1.id, t2.id])
        .await
        .expect("create playlist");

    // Push mutations (favorite, progress, reading bookmark)
    let bm_id = BookmarkId::generate();
    let push1 = repo
        .push_mutations(
            admin.id,
            &[music.id, books.id],
            &[
                SyncMutation::SetFavorite {
                    item_id: t1.id,
                    favorite: true,
                    client_updated_at: None,
                    base_revision: None,
                },
                SyncMutation::SetProgress {
                    item_id: t2.id,
                    position_ms: 12_345,
                    completed: false,
                    client_updated_at: None,
                    base_revision: None,
                },
                SyncMutation::AddReadingBookmark {
                    item_id: book.id,
                    bookmark_id: Some(bm_id),
                    page: 42,
                    locator: Some("ch4".into()),
                    label: "Dark Forest".into(),
                    note: Some("Key chapter".into()),
                    client_updated_at: None,
                    base_revision: None,
                },
            ],
        )
        .await
        .expect("push mutations");
    assert_eq!(push1.applied, 3);
    assert!(push1.conflicts.is_empty());

    // Idempotent replay + conflict on stale base_revision
    let push2 = repo
        .push_mutations(
            admin.id,
            &[music.id, books.id],
            &[
                SyncMutation::SetFavorite {
                    item_id: t1.id,
                    favorite: true,
                    client_updated_at: None,
                    base_revision: Some(0),
                },
                SyncMutation::SetProgress {
                    item_id: t2.id,
                    position_ms: 999,
                    completed: false,
                    client_updated_at: None,
                    base_revision: Some(0),
                },
                SyncMutation::AddReadingBookmark {
                    item_id: book.id,
                    bookmark_id: Some(bm_id),
                    page: 42,
                    locator: Some("ch4".into()),
                    label: "Dark Forest".into(),
                    note: Some("Key chapter".into()),
                    client_updated_at: None,
                    base_revision: None,
                },
            ],
        )
        .await
        .expect("push replay");
    assert_eq!(push2.applied, 2);
    assert_eq!(push2.conflicts.len(), 1);
    assert_eq!(push2.conflicts[0].reason, "server_newer");

    // Paginate items with limit = 2 using next_cursor
    let page1 = repo
        .changes(admin.id, &[music.id, books.id], true, 0, 2, None)
        .await
        .expect("sync page 1");
    assert!(page1.has_more);
    assert_eq!(page1.items.len(), 2);
    assert_eq!(page1.playlists.len(), 1);
    assert_eq!(page1.playlists[0].track_ids, vec![t1.id, t2.id]);
    assert_eq!(page1.reading_bookmarks.len(), 1);
    assert_eq!(page1.reading_bookmarks[0].id, bm_id);

    let cursor = SyncCursor::decode(page1.next_cursor.as_deref().expect("cursor")).expect("valid");
    let page2 = repo
        .changes(admin.id, &[music.id, books.id], true, 0, 2, Some(&cursor))
        .await
        .expect("sync page 2");
    assert_eq!(page2.items.len(), 2);
    assert!(page2.items.iter().any(|i| i.id == t3.id && i.deleted));

    // ACL isolation: user with only `music` cannot see `books` items or bookmarks
    let restricted = repo
        .changes(admin.id, &[music.id], false, 0, 50, None)
        .await
        .expect("restricted sync");
    assert!(restricted.items.iter().all(|i| i.library_id == music.id));
    assert!(restricted.reading_bookmarks.is_empty());

    // Remove bookmark idempotently
    let push3 = repo
        .push_mutations(
            admin.id,
            &[music.id, books.id],
            &[SyncMutation::RemoveReadingBookmark {
                bookmark_id: bm_id,
                client_updated_at: None,
                base_revision: None,
            }],
        )
        .await
        .expect("remove bookmark");
    assert_eq!(push3.applied, 1);

    let after_rm = repo
        .changes(admin.id, &[music.id, books.id], true, 0, 50, None)
        .await
        .expect("after remove");
    assert!(after_rm.reading_bookmarks.is_empty());

    // Ambient light update on seeded home light
    let lit =
        h.db.home()
            .apply_ambient_light("living-strip", 55, 3200)
            .await
            .expect("apply ambient light")
            .expect("living-strip exists");
    assert_eq!(
        lit.device.state,
        palveni_core::home::DeviceState::Light {
            on: true,
            brightness: 55,
            color_temp: Some(3200),
        }
    );

    h.finish().await;
}

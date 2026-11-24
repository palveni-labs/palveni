use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use futures_util::StreamExt;
use palveni_api::{AppState, LinkPairings, LoginLimit, ServerInfo, router};
use palveni_core::accounts::UserPolicy;
use palveni_core::ids::PlaylistId;
use palveni_core::{
    BookmarkId, FeatureRegistry, ItemId, ItemKind, LibraryKind, PerfTier, Role, ServerId,
};
use palveni_db::accounts::NewManagedUser;
use palveni_db::items::ItemUpsert;
use palveni_db::libraries::NewLibrary;
use palveni_db::{Db, DbOptions};
use palveni_jobs::JobService;
use palveni_library::music_search::MusicSearch;
use palveni_library::video::VideoImages;
use palveni_link::NodeIdentity;
use palveni_media::{HardwareService, ProbeOptions};
use palveni_plugin_host::PluginCatalog;
use palveni_plugin_host::skin_store::SkinStore;
use reqwest::{Client, Method, StatusCode};
use serde_json::{Value, json};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tokio::time::{sleep, timeout};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

struct TempDirGuard(PathBuf);

impl TempDirGuard {
    fn new(prefix: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("{prefix}-{}", Uuid::now_v7()));
        std::fs::create_dir_all(&dir).expect("create temp test dir");
        Self(dir)
    }
}

impl Drop for TempDirGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

struct TestServer {
    addr: SocketAddr,
    db: Db,
    client: Client,
    shutdown: CancellationToken,
    _dir: TempDirGuard,
}

impl TestServer {
    async fn start() -> Self {
        let dir = TempDirGuard::new("palveni-sync-cast-mqtt");
        let base = dir.0.display().to_string();
        let db = Db::open(&DbOptions {
            main_url: format!("sqlite://{base}/main.db?mode=rwc"),
            ops_url: format!("sqlite://{base}/ops.db?mode=rwc"),
            max_connections: Some(8),
            min_connections: Some(1),
        })
        .await
        .expect("open db");

        let bus = db.event_bus(64).await.expect("event bus");
        let jobs = JobService::new(db.clone(), bus.clone(), 1);
        let shutdown = CancellationToken::new();
        let state = AppState {
            db: db.clone(),
            music_search: Arc::new(MusicSearch::new(dir.0.join("search"))),
            video_images: Arc::new(VideoImages::new(dir.0.join("cache"), None, None)),
            identity: Arc::new(NodeIdentity::generate().expect("identity")),
            link: Arc::new(LinkPairings::default()),
            login_limit: Arc::new(LoginLimit::new(32)),
            subsonic_limit: Arc::new(LoginLimit::new(32)),
            bus,
            jobs,
            hardware: Arc::new(HardwareService::new(
                db.clone(),
                dir.0.clone(),
                ProbeOptions::default(),
            )),
            plugins: Arc::new(PluginCatalog::new()),
            settings: Arc::new(tokio::sync::RwLock::new(Default::default())),
            boot_tier: PerfTier::detect(0, 1, false),
            skins: Arc::new(SkinStore::open(&dir.0.join("skins")).expect("skins")),
            skin_writes: Arc::new(tokio::sync::Semaphore::new(1)),
            features: FeatureRegistry::new(),
            server: Arc::new(ServerInfo {
                id: ServerId::generate(),
                name: "SyncCastTest".into(),
                version: "0.1.0".into(),
            }),
            shutdown: shutdown.clone(),
        };

        let listener = TcpListener::bind(("127.0.0.1", 0)).await.expect("bind");
        let addr = listener.local_addr().expect("local addr");
        let app = router(state);
        let cancel = shutdown.clone();
        tokio::spawn(async move {
            let _ = axum::serve(listener, app)
                .with_graceful_shutdown(async move { cancel.cancelled().await })
                .await;
        });

        let client = Client::builder()
            .no_proxy()
            .timeout(Duration::from_secs(5))
            .build()
            .expect("client");

        Self {
            addr,
            db,
            client,
            shutdown,
            _dir: dir,
        }
    }

    fn url(&self, path: &str) -> String {
        format!("http://{}/api/v1{path}", self.addr)
    }

    fn req(&self, method: Method, path: &str, token: &str) -> reqwest::RequestBuilder {
        self.client
            .request(method, self.url(path))
            .bearer_auth(token)
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.shutdown.cancel();
    }
}

fn decode_varint(buf: &[u8], offset: &mut usize) -> Option<usize> {
    let mut multiplier = 1usize;
    let mut value = 0usize;
    for _ in 0..4 {
        let byte = *buf.get(*offset)?;
        *offset += 1;
        value += usize::from(byte & 0x7F) * multiplier;
        if (byte & 0x80) == 0 {
            return Some(value);
        }
        multiplier *= 128;
    }
    None
}

async fn spawn_mock_mqtt_broker() -> (u16, Arc<Mutex<Vec<(String, Value)>>>) {
    let listener = TcpListener::bind(("127.0.0.1", 0))
        .await
        .expect("bind mock mqtt");
    let port = listener.local_addr().expect("addr").port();
    let captured: Arc<Mutex<Vec<(String, Value)>>> = Arc::new(Mutex::new(Vec::new()));
    let store = Arc::clone(&captured);

    tokio::spawn(async move {
        while let Ok((mut sock, _)) = listener.accept().await {
            let store = Arc::clone(&store);
            tokio::spawn(async move {
                let mut buf = [0u8; 512];
                let Ok(n) = sock.read(&mut buf).await else {
                    return;
                };
                if n < 12 || buf[0] != 0x10 {
                    return;
                }
                // Reply CONNACK accepted
                if sock.write_all(&[0x20, 0x02, 0x00, 0x00]).await.is_err() {
                    return;
                }
                let mut rest = Vec::new();
                let _ = sock.read_to_end(&mut rest).await;

                // Parse all PUBLISH QoS 0 frames until DISCONNECT
                let mut pos = 0usize;
                while pos < rest.len() {
                    let pkt_type = rest[pos] & 0xF0;
                    pos += 1;
                    let Some(rem_len) = decode_varint(&rest, &mut pos) else {
                        break;
                    };
                    if pos + rem_len > rest.len() {
                        break;
                    }
                    let body = &rest[pos..pos + rem_len];
                    pos += rem_len;
                    if pkt_type == 0x30 && body.len() >= 2 {
                        let topic_len = usize::from(u16::from_be_bytes([body[0], body[1]]));
                        if 2 + topic_len <= body.len() {
                            let topic =
                                String::from_utf8_lossy(&body[2..2 + topic_len]).to_string();
                            let payload_bytes = &body[2 + topic_len..];
                            let payload: Value =
                                serde_json::from_slice(payload_bytes).unwrap_or(Value::Null);
                            store.lock().await.push((topic, payload));
                        }
                    } else if pkt_type == 0xE0 {
                        break;
                    }
                }
            });
        }
    });

    (port, captured)
}

#[tokio::test]
async fn sync_changes_pagination_acl_and_push_conflicts() {
    let srv = TestServer::start().await;

    let admin = srv
        .db
        .users()
        .create("admin", "hash", Role::Admin)
        .await
        .unwrap();
    let admin_tok = srv
        .db
        .sessions()
        .create(admin.id, "Admin Desktop", None)
        .await
        .unwrap()
        .token;

    let music = srv
        .db
        .libraries()
        .create(NewLibrary {
            name: "Music".into(),
            kind: LibraryKind::Music,
            paths: vec!["/media/music".into()],
        })
        .await
        .unwrap();
    let books = srv
        .db
        .libraries()
        .create(NewLibrary {
            name: "Books".into(),
            kind: LibraryKind::Book,
            paths: vec!["/media/books".into()],
        })
        .await
        .unwrap();

    // Restricted user with access ONLY to Music library
    let restricted = srv
        .db
        .accounts()
        .create_user(NewManagedUser {
            username: "kid".into(),
            password_hash: "hash".into(),
            role: Role::User,
            policy: UserPolicy {
                all_libraries: false,
                allow_download: true,
                allow_transcode: true,
                daily_ai_limit: 50,
            },
            allowed_library_ids: vec![music.id],
        })
        .await
        .unwrap();
    let kid_tok = srv
        .db
        .sessions()
        .create(restricted.id, "Kid iPad", None)
        .await
        .unwrap()
        .token;

    // Seed items across two batches + tombstone
    let t1 = ItemUpsert {
        music: None,
        id: ItemId::generate(),
        parent_id: None,
        kind: ItemKind::Track,
        title: "Track One".into(),
        sort_title: "track one".into(),
        search_text: "track one".into(),
        path: "/media/music/1.flac".into(),
        size_bytes: Some(1024),
        modified_at: Some(1_700_000_000_000),
    };
    let t2 = ItemUpsert {
        music: None,
        id: ItemId::generate(),
        parent_id: None,
        kind: ItemKind::Track,
        title: "Track Two".into(),
        sort_title: "track two".into(),
        search_text: "track two".into(),
        path: "/media/music/2.flac".into(),
        size_bytes: Some(2048),
        modified_at: Some(1_700_000_000_000),
    };
    let t3 = ItemUpsert {
        music: None,
        id: ItemId::generate(),
        parent_id: None,
        kind: ItemKind::Track,
        title: "Track Removed".into(),
        sort_title: "track removed".into(),
        search_text: "track removed".into(),
        path: "/media/music/3.flac".into(),
        size_bytes: Some(3072),
        modified_at: Some(1_700_000_000_000),
    };
    srv.db
        .items()
        .apply_scan_batch(music.id, &[t1.clone(), t2.clone(), t3.clone()], &[])
        .await
        .unwrap();
    srv.db
        .items()
        .apply_scan_batch(music.id, &[], &[t3.id])
        .await
        .unwrap();

    let book = ItemUpsert {
        music: None,
        id: ItemId::generate(),
        parent_id: None,
        kind: ItemKind::Book,
        title: "Foundation".into(),
        sort_title: "foundation".into(),
        search_text: "foundation".into(),
        path: "/media/books/foundation.epub".into(),
        size_bytes: Some(4096),
        modified_at: Some(1_700_000_000_000),
    };
    srv.db
        .items()
        .apply_scan_batch(books.id, std::slice::from_ref(&book), &[])
        .await
        .unwrap();

    srv.db
        .playlists()
        .save(
            admin.id,
            PlaylistId::generate(),
            0,
            "Favorites Mix",
            vec![t1.id],
        )
        .await
        .unwrap();

    // 1. Admin paginates GET /sync/changes?since=0&limit=2
    let p1: Value = srv
        .req(Method::GET, "/sync/changes?since=0&limit=2", &admin_tok)
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(p1["hasMore"], true);
    assert_eq!(p1["items"].as_array().unwrap().len(), 2);
    assert_eq!(p1["playlists"].as_array().unwrap().len(), 1);
    let cursor = p1["nextCursor"].as_str().expect("nextCursor");

    let p2: Value = srv
        .req(
            Method::GET,
            &format!("/sync/changes?since=0&limit=2&cursor={cursor}"),
            &admin_tok,
        )
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(p2["items"].as_array().unwrap().len(), 2);

    let mut all_ids: Vec<String> = p1["items"]
        .as_array()
        .unwrap()
        .iter()
        .chain(p2["items"].as_array().unwrap().iter())
        .map(|i| i["id"].as_str().unwrap().to_owned())
        .collect();
    all_ids.sort();
    assert_eq!(all_ids.len(), 4);
    assert!(
        p2["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["id"] == t3.id.to_string() && i["deleted"] == true),
        "tombstone item must have deleted=true"
    );

    // 2. Restricted user only receives Music library & items
    let kid_sync: Value = srv
        .req(Method::GET, "/sync/changes?since=0&limit=50", &kid_tok)
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(kid_sync["libraries"].as_array().unwrap().len(), 1);
    assert_eq!(kid_sync["libraries"][0]["id"], music.id.to_string());
    assert!(
        kid_sync["items"]
            .as_array()
            .unwrap()
            .iter()
            .all(|i| i["libraryId"] == music.id.to_string())
    );

    // 3. Push mutations + idempotent replay + conflict detection
    let bm_id = BookmarkId::generate();
    let push1: Value = srv
        .req(Method::POST, "/sync/push", &admin_tok)
        .json(&json!({
            "clientId": "primuse-macos",
            "mutations": [
                { "type": "set_favorite", "itemId": t1.id, "favorite": true },
                { "type": "set_progress", "itemId": t2.id, "positionMs": 45000, "completed": false },
                { "type": "add_reading_bookmark", "itemId": book.id, "bookmarkId": bm_id, "page": 15, "label": "Terminus" }
            ]
        }))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(push1["applied"], 3);
    assert_eq!(push1["conflicts"].as_array().unwrap().len(), 0);
    let rev_after_push1 = push1["latestRevision"].as_i64().unwrap();

    // Replay same favorite & bookmark + stale progress conflict
    let push2: Value = srv
        .req(Method::POST, "/sync/push", &admin_tok)
        .json(&json!({
            "clientId": "primuse-ios",
            "mutations": [
                { "type": "set_favorite", "itemId": t1.id, "favorite": true, "baseRevision": 0 },
                { "type": "set_progress", "itemId": t2.id, "positionMs": 1000, "completed": false, "baseRevision": 0 },
                { "type": "add_reading_bookmark", "itemId": book.id, "bookmarkId": bm_id, "page": 15, "label": "Terminus" }
            ]
        }))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(push2["applied"], 2);
    assert_eq!(push2["conflicts"].as_array().unwrap().len(), 1);
    assert_eq!(push2["conflicts"][0]["reason"], "server_newer");

    // Incremental changes since before push1 contain updated userStates & bookmark
    let delta: Value = srv
        .req(
            Method::GET,
            &format!("/sync/changes?since={}&limit=50", rev_after_push1 - 1),
            &admin_tok,
        )
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(delta["userStates"].as_array().unwrap().len(), 2);
    assert_eq!(delta["readingBookmarks"].as_array().unwrap().len(), 1);
    assert_eq!(delta["readingBookmarks"][0]["id"], bm_id.to_string());

    // Remove reading bookmark idempotently
    let rm: Value = srv
        .req(Method::POST, "/sync/push", &admin_tok)
        .json(&json!({
            "clientId": "elumi-ios",
            "mutations": [
                { "type": "remove_reading_bookmark", "bookmarkId": bm_id },
                { "type": "remove_reading_bookmark", "bookmarkId": bm_id }
            ]
        }))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(rm["applied"], 2);
    assert!(rm["conflicts"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn playback_sessions_owner_isolation_and_remote_command_sse() {
    let srv = TestServer::start().await;

    let admin = srv
        .db
        .users()
        .create("admin", "hash", Role::Admin)
        .await
        .unwrap();
    let alice = srv
        .db
        .users()
        .create("alice", "hash", Role::User)
        .await
        .unwrap();
    let bob = srv
        .db
        .users()
        .create("bob", "hash", Role::User)
        .await
        .unwrap();

    let admin_tok = srv
        .db
        .sessions()
        .create(admin.id, "Admin", None)
        .await
        .unwrap()
        .token;
    let alice_tok = srv
        .db
        .sessions()
        .create(alice.id, "Alice Web", None)
        .await
        .unwrap()
        .token;
    let bob_tok = srv
        .db
        .sessions()
        .create(bob.id, "Bob iPhone", None)
        .await
        .unwrap()
        .token;

    // Open Alice's live SSE stream BEFORE sending commands
    let sse_resp = srv
        .client
        .get(srv.url("/events"))
        .bearer_auth(&alice_tok)
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
    let mut sse_stream = sse_resp.bytes_stream();

    // Wait for initial ping frame so stream subscription is active
    let first_chunk = timeout(Duration::from_secs(3), sse_stream.next())
        .await
        .expect("initial ping timeout")
        .expect("stream item")
        .expect("chunk bytes");
    assert!(String::from_utf8_lossy(&first_chunk).contains("event: ping"));

    // Alice heartbeats her playback session
    let alice_hb: Value = srv
        .req(Method::POST, "/sessions/playback/heartbeat", &alice_tok)
        .json(&json!({
            "sessionId": "alice-tab-1",
            "deviceName": "Alice Chrome",
            "clientKind": "web",
            "mediaKind": "audio",
            "title": "Nocturne",
            "subtitle": "Chopin",
            "state": "playing",
            "positionMs": 12000,
            "durationMs": 240000,
            "volume": 75,
            "ambientHex": "#d97706"
        }))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(alice_hb["id"], "alice-tab-1");
    assert_eq!(alice_hb["ambientHex"], "#D97706");

    // Bob heartbeats his own session
    srv.req(Method::POST, "/sessions/playback/heartbeat", &bob_tok)
        .json(&json!({
            "sessionId": "bob-ios-1",
            "deviceName": "Bob iPhone",
            "clientKind": "ios",
            "mediaKind": "audio",
            "title": "Jazz FM",
            "state": "playing",
            "positionMs": 5000,
            "durationMs": 180000,
            "volume": 60
        }))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();

    // Owner isolation on GET /sessions/playback
    let alice_list: Value = srv
        .req(Method::GET, "/sessions/playback?all=true", &alice_tok)
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(alice_list["sessions"].as_array().unwrap().len(), 1);
    assert_eq!(alice_list["sessions"][0]["id"], "alice-tab-1");

    // Admin with ?all=true sees both Alice and Bob
    let admin_all: Value = srv
        .req(Method::GET, "/sessions/playback?all=true", &admin_tok)
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(admin_all["sessions"].as_array().unwrap().len(), 2);

    // Bob cannot hijack, control, or delete Alice's session
    let hijack = srv
        .req(Method::POST, "/sessions/playback/heartbeat", &bob_tok)
        .json(&json!({
            "sessionId": "alice-tab-1",
            "deviceName": "Hijack",
            "clientKind": "ios",
            "mediaKind": "audio",
            "state": "paused",
            "positionMs": 0,
            "durationMs": 0,
            "volume": 0
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(hijack.status(), StatusCode::FORBIDDEN);

    let bob_cmd = srv
        .req(
            Method::POST,
            "/sessions/playback/alice-tab-1/command",
            &bob_tok,
        )
        .json(&json!({ "command": "pause" }))
        .send()
        .await
        .unwrap();
    assert_eq!(bob_cmd.status(), StatusCode::FORBIDDEN);

    let bob_del = srv
        .req(Method::DELETE, "/sessions/playback/alice-tab-1", &bob_tok)
        .send()
        .await
        .unwrap();
    assert_eq!(bob_del.status(), StatusCode::FORBIDDEN);

    // Alice sends remote seek command to her own session
    let cmd_resp = srv
        .req(
            Method::POST,
            "/sessions/playback/alice-tab-1/command",
            &alice_tok,
        )
        .json(&json!({ "command": "seek", "positionMs": 65000 }))
        .send()
        .await
        .unwrap();
    assert_eq!(cmd_resp.status(), StatusCode::ACCEPTED);
    let ack: Value = cmd_resp.json().await.unwrap();
    assert_eq!(ack["accepted"], true);
    assert_eq!(ack["targetSessionId"], "alice-tab-1");
    assert_eq!(ack["command"], "seek");

    // Collect SSE frames on Alice's stream: must contain playback.session.updated and playback.command,
    // and MUST NOT leak Bob's session!
    let mut sse_text = String::new();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
    while tokio::time::Instant::now() < deadline && !sse_text.contains("event: playback.command") {
        if let Ok(Some(Ok(chunk))) = timeout(Duration::from_millis(500), sse_stream.next()).await {
            sse_text.push_str(&String::from_utf8_lossy(&chunk));
        }
    }
    assert!(
        sse_text.contains("event: playback.session.updated"),
        "expected playback.session.updated in SSE: {sse_text}"
    );
    assert!(
        sse_text.contains("event: playback.command"),
        "expected playback.command in SSE: {sse_text}"
    );
    assert!(sse_text.contains("\"positionMs\":65000"));
    assert!(
        !sse_text.contains("bob-ios-1"),
        "Bob's session leaked into Alice's SSE stream: {sse_text}"
    );

    // Alice deletes her session
    let del = srv
        .req(Method::DELETE, "/sessions/playback/alice-tab-1", &alice_tok)
        .send()
        .await
        .unwrap();
    assert_eq!(del.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn mqtt_bridge_live_publish_ssrf_guard_and_ambient_light_sync() {
    let srv = TestServer::start().await;
    let (broker_port, captured) = spawn_mock_mqtt_broker().await;

    let admin = srv
        .db
        .users()
        .create("admin", "hash", Role::Admin)
        .await
        .unwrap();
    let member = srv
        .db
        .users()
        .create("alice", "hash", Role::User)
        .await
        .unwrap();
    let admin_tok = srv
        .db
        .sessions()
        .create(admin.id, "Admin", None)
        .await
        .unwrap()
        .token;
    let member_tok = srv
        .db
        .sessions()
        .create(member.id, "Alice", None)
        .await
        .unwrap()
        .token;

    // Seed home devices so `living-strip` light exists
    let home_initial: Value = srv
        .req(Method::GET, "/home", &admin_tok)
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(
        home_initial["devices"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["id"] == "living-strip")
    );

    // Non-admin cannot access MQTT endpoints
    let forbidden = srv
        .req(Method::GET, "/home/mqtt", &member_tok)
        .send()
        .await
        .unwrap();
    assert_eq!(forbidden.status(), StatusCode::FORBIDDEN);

    // SSRF guard rejects cloud metadata hosts
    for blocked_host in [
        "169.254.169.254",
        "::ffff:169.254.169.254",
        "metadata.google.internal",
    ] {
        let resp = srv
            .req(Method::POST, "/home/mqtt", &admin_tok)
            .json(&json!({
                "name": "Bad Broker",
                "host": blocked_host,
                "port": 1883,
                "topicPrefix": "palveni"
            }))
            .send()
            .await
            .unwrap();
        assert_eq!(
            resp.status(),
            StatusCode::BAD_REQUEST,
            "expected {blocked_host} to be rejected"
        );
    }

    // Create MQTT bridge bound to local mock broker + living-strip ambient light
    let created: Value = srv
        .req(Method::POST, "/home/mqtt", &admin_tok)
        .json(&json!({
            "name": "Living Room Mosquitto",
            "host": "127.0.0.1",
            "port": broker_port,
            "username": "palveni",
            "password": "super-secret-password",
            "topicPrefix": "palveni/test",
            "ambientLightDeviceId": "living-strip",
            "publishPlayback": true,
            "enabled": true
        }))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    let bridge_id = created["id"].as_str().unwrap().to_owned();
    assert_eq!(created["passwordConfigured"], true);
    assert!(!created.to_string().contains("super-secret-password"));

    // Test publish endpoint
    let test_res: Value = srv
        .req(
            Method::POST,
            &format!("/home/mqtt/{bridge_id}/test"),
            &admin_tok,
        )
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(test_res["ok"], true);
    assert_eq!(test_res["topic"], "palveni/test/status");

    // Playing heartbeat with warm orange cover accent -> MQTT playback + ambient + living-strip Kelvin sync
    srv.req(Method::POST, "/sessions/playback/heartbeat", &member_tok)
        .json(&json!({
            "sessionId": "alice-living-room",
            "deviceName": "Living Room Web",
            "clientKind": "web",
            "mediaKind": "audio",
            "title": "Autumn Leaves",
            "subtitle": "Bill Evans",
            "state": "playing",
            "positionMs": 30000,
            "durationMs": 300000,
            "volume": 65,
            "ambientHex": "#FF5500"
        }))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();

    // Wait briefly for detached background MQTT & ambient light task
    for _ in 0..40 {
        let len = captured.lock().await.len();
        if len >= 3 {
            break;
        }
        sleep(Duration::from_millis(50)).await;
    }

    let msgs = captured.lock().await.clone();
    assert!(
        msgs.iter().any(|(t, p)| t == "palveni/test/status"
            && p["event"] == "test"
            && p["server"] == "palveni"),
        "missing test status MQTT publish: {msgs:?}"
    );
    assert!(
        msgs.iter()
            .any(|(t, p)| t == "palveni/test/playback" && p["title"] == "Autumn Leaves"),
        "missing playback MQTT publish: {msgs:?}"
    );
    assert!(
        msgs.iter().any(|(t, p)| t == "palveni/test/ambient"
            && p["colorTemp"] == 2700
            && p["deviceId"] == "living-strip"),
        "missing ambient MQTT publish: {msgs:?}"
    );

    // Confirm smart-home light state in DB updated to 2700K + cozy brightness
    let home_warm: Value = srv
        .req(Method::GET, "/home", &admin_tok)
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    let strip = home_warm["devices"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["id"] == "living-strip")
        .expect("living-strip");
    assert_eq!(strip["state"]["on"], true);
    assert_eq!(strip["state"]["colorTemp"], 2700);
    let warm_b = strip["state"]["brightness"].as_u64().unwrap();
    assert!((25..=75).contains(&warm_b), "brightness={warm_b}");

    // Cool blue accent -> 6500K
    srv.req(Method::POST, "/sessions/playback/heartbeat", &member_tok)
        .json(&json!({
            "sessionId": "alice-living-room",
            "deviceName": "Living Room Web",
            "clientKind": "web",
            "mediaKind": "audio",
            "title": "Kind of Blue",
            "state": "playing",
            "positionMs": 45000,
            "durationMs": 300000,
            "volume": 65,
            "ambientHex": "#0066FF"
        }))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();

    for _ in 0..40 {
        let home_cool: Value = srv
            .req(Method::GET, "/home", &admin_tok)
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        let temp = home_cool["devices"]
            .as_array()
            .unwrap()
            .iter()
            .find(|d| d["id"] == "living-strip")
            .and_then(|d| d["state"]["colorTemp"].as_u64());
        if temp == Some(6500) {
            break;
        }
        sleep(Duration::from_millis(50)).await;
    }

    let home_cool: Value = srv
        .req(Method::GET, "/home", &admin_tok)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let strip_cool = home_cool["devices"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["id"] == "living-strip")
        .expect("living-strip");
    assert_eq!(strip_cool["state"]["colorTemp"], 6500);

    // Delete MQTT bridge
    let del = srv
        .req(
            Method::DELETE,
            &format!("/home/mqtt/{bridge_id}"),
            &admin_tok,
        )
        .send()
        .await
        .unwrap();
    assert_eq!(del.status(), StatusCode::NO_CONTENT);
}

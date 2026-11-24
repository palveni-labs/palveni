use std::path::PathBuf;
use std::sync::Arc;

use axum::http::StatusCode;
use axum::serve::Listener;
use chrono::TimeZone;
use palveni_core::accounts::UserPolicy;
use palveni_core::{FeatureRegistry, ItemId, ItemKind, LibraryKind, PerfTier, Role, ServerId};
use palveni_db::accounts::NewManagedUser;
use palveni_db::items::ItemUpsert;
use palveni_db::libraries::NewLibrary;
use palveni_db::photos::PhotoMetadataUpsert;
use palveni_db::{Db, DbOptions};
use palveni_jobs::JobService;
use palveni_link::NodeIdentity;
use palveni_media::{HardwareService, ProbeOptions};
use palveni_plugin_host::{PluginCatalog, skin_store::SkinStore};
use serde_json::{Value, json};
use tokio::io::{AsyncReadExt, AsyncWriteExt, DuplexStream};
use tokio::sync::mpsc;
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

struct DuplexListener {
    rx: mpsc::UnboundedReceiver<DuplexStream>,
}

impl Listener for DuplexListener {
    type Io = DuplexStream;
    type Addr = std::net::SocketAddr;

    async fn accept(&mut self) -> (Self::Io, Self::Addr) {
        match self.rx.recv().await {
            Some(stream) => (stream, std::net::SocketAddr::from(([127, 0, 0, 1], 40000))),
            None => std::future::pending().await,
        }
    }

    fn local_addr(&self) -> std::io::Result<Self::Addr> {
        Ok(std::net::SocketAddr::from(([127, 0, 0, 1], 40000)))
    }
}

struct HttpReply {
    status: StatusCode,
    body: Vec<u8>,
}

impl HttpReply {
    fn json(&self) -> Value {
        serde_json::from_slice(&self.body).expect("valid json response")
    }
}

fn percent_encode_query(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for byte in s.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            out.push(char::from(byte));
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

fn decode_chunked(mut input: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    while let Some(pos) = input.windows(2).position(|w| w == b"\r\n") {
        let size_str = std::str::from_utf8(&input[..pos]).unwrap_or("0").trim();
        let size_hex = size_str.split(';').next().unwrap_or("0").trim();
        let len = usize::from_str_radix(size_hex, 16).unwrap_or(0);
        input = &input[pos + 2..];
        if len == 0 {
            break;
        }
        let take = len.min(input.len());
        out.extend_from_slice(&input[..take]);
        input = &input[take..];
        if input.starts_with(b"\r\n") {
            input = &input[2..];
        }
    }
    out
}

struct TestServer {
    tx: mpsc::UnboundedSender<DuplexStream>,
    db: Db,
    root: PathBuf,
    _guard: TempDirGuard,
    shutdown: CancellationToken,
}

impl TestServer {
    async fn start() -> Self {
        let guard = TempDirGuard::new("palveni-plugins-photo-ai");
        let root = guard.0.clone();
        let db_base = root.display().to_string();
        let db = Db::open(&DbOptions {
            main_url: format!("sqlite://{db_base}/main.db?mode=rwc"),
            ops_url: format!("sqlite://{db_base}/ops.db?mode=rwc"),
            max_connections: Some(6),
            min_connections: Some(1),
        })
        .await
        .expect("open test db");

        let bus = db.event_bus(32).await.expect("event bus");
        let jobs = JobService::new(db.clone(), bus.clone(), 1);
        let shutdown = CancellationToken::new();

        let state = palveni_api::AppState {
            db: db.clone(),
            music_search: Arc::new(palveni_library::music_search::MusicSearch::new(
                root.join("music-search"),
            )),
            video_images: Arc::new(palveni_library::video::VideoImages::new(
                root.join("video-images"),
                None,
                None,
            )),
            identity: Arc::new(NodeIdentity::generate().expect("node identity")),
            link: Arc::new(palveni_api::LinkPairings::default()),
            login_limit: Arc::new(palveni_api::LoginLimit::new(16)),
            subsonic_limit: Arc::new(palveni_api::LoginLimit::new(16)),
            bus,
            jobs,
            hardware: Arc::new(HardwareService::new(
                db.clone(),
                root.clone(),
                ProbeOptions::default(),
            )),
            plugins: Arc::new(PluginCatalog::new()),
            settings: Arc::new(tokio::sync::RwLock::new(Default::default())),
            boot_tier: PerfTier::detect(0, 1, false),
            skins: Arc::new(SkinStore::open(&root.join("skins")).expect("skin store")),
            skin_writes: Arc::new(tokio::sync::Semaphore::new(1)),
            features: FeatureRegistry::new(),
            server: Arc::new(palveni_api::ServerInfo {
                id: ServerId::generate(),
                name: "Test Server".into(),
                version: "0.1.0".into(),
            }),
            shutdown: shutdown.clone(),
        };

        let app = palveni_api::router(state).layer(axum::Extension(axum::extract::ConnectInfo(
            std::net::SocketAddr::from(([127, 0, 0, 1], 40000)),
        )));
        let (tx, rx) = mpsc::unbounded_channel();
        let listener = DuplexListener { rx };
        let stop = shutdown.clone();
        tokio::spawn(async move {
            let _ = axum::serve(listener, app)
                .with_graceful_shutdown(async move {
                    stop.cancelled().await;
                })
                .await;
        });

        Self {
            tx,
            db,
            root,
            _guard: guard,
            shutdown,
        }
    }

    async fn send(
        &self,
        method: &str,
        path: &str,
        token: Option<&str>,
        json_body: Option<Value>,
    ) -> HttpReply {
        let (mut client, server) = tokio::io::duplex(256 * 1024);
        self.tx.send(server).expect("send duplex stream");

        let body_bytes = match json_body {
            Some(v) => serde_json::to_vec(&v).expect("serialize body"),
            None => Vec::new(),
        };

        let mut req =
            format!("{method} /api/v1{path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n");
        if let Some(tok) = token {
            req.push_str(&format!("Authorization: Bearer {tok}\r\n"));
        }
        if !body_bytes.is_empty() {
            req.push_str("Content-Type: application/json\r\n");
            req.push_str(&format!("Content-Length: {}\r\n", body_bytes.len()));
        }
        req.push_str("\r\n");

        client.write_all(req.as_bytes()).await.expect("write head");
        if !body_bytes.is_empty() {
            client.write_all(&body_bytes).await.expect("write body");
        }
        client.flush().await.expect("flush request");

        let mut raw = Vec::new();
        client.read_to_end(&mut raw).await.expect("read response");

        let sep = raw
            .windows(4)
            .position(|w| w == b"\r\n\r\n")
            .expect("http response header separator");
        let head = std::str::from_utf8(&raw[..sep]).expect("utf8 response headers");
        let status_code: u16 = head
            .lines()
            .next()
            .expect("status line")
            .split_whitespace()
            .nth(1)
            .expect("status code")
            .parse()
            .expect("parse status code");
        let is_chunked = head.lines().any(|line| {
            line.to_ascii_lowercase()
                .contains("transfer-encoding: chunked")
        });
        let raw_body = &raw[sep + 4..];
        let body = if is_chunked {
            decode_chunked(raw_body)
        } else {
            raw_body.to_vec()
        };

        HttpReply {
            status: StatusCode::from_u16(status_code).expect("valid status"),
            body,
        }
    }

    async fn setup_admin(&self) -> String {
        let resp = self
            .send(
                "POST",
                "/setup",
                None,
                Some(json!({
                    "username": "admin",
                    "password": "password123"
                })),
            )
            .await;
        assert_eq!(resp.status, StatusCode::CREATED);
        let login = self
            .send(
                "POST",
                "/auth/login",
                None,
                Some(json!({
                    "username": "admin",
                    "password": "password123",
                    "deviceName": "test-admin"
                })),
            )
            .await;
        assert_eq!(login.status, StatusCode::OK);
        login.json()["accessToken"]
            .as_str()
            .expect("admin token")
            .to_owned()
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.shutdown.cancel();
    }
}

fn write_ppm(
    path: &PathBuf,
    width: usize,
    height: usize,
    rgb: impl Fn(usize, usize) -> (u8, u8, u8),
) -> i64 {
    let mut data = format!("P6\n{width} {height}\n255\n").into_bytes();
    for y in 0..height {
        for x in 0..width {
            let (r, g, b) = rgb(x, y);
            data.push(r);
            data.push(g);
            data.push(b);
        }
    }
    std::fs::write(path, &data).expect("write ppm");
    i64::try_from(data.len()).unwrap_or(0)
}

#[tokio::test]
async fn plugin_store_catalog_install_settings_rollback_uninstall_and_audit() {
    let srv = TestServer::start().await;
    let admin_token = srv.setup_admin().await;

    // Create a non-admin member to verify 403 guard
    let member_resp = srv
        .send(
            "POST",
            "/users",
            Some(&admin_token),
            Some(json!({
                "username": "member",
                "password": "password123",
                "role": "user"
            })),
        )
        .await;
    assert_eq!(member_resp.status, StatusCode::CREATED);

    let login_member = srv
        .send(
            "POST",
            "/auth/login",
            None,
            Some(json!({
                "username": "member",
                "password": "password123",
                "deviceName": "member-dev"
            })),
        )
        .await;
    assert_eq!(login_member.status, StatusCode::OK);
    let member_token = login_member.json()["accessToken"]
        .as_str()
        .unwrap()
        .to_owned();

    // Non-admin rejected with 403
    let forbidden = srv
        .send("GET", "/plugins/catalog", Some(&member_token), None)
        .await;
    assert_eq!(forbidden.status, StatusCode::FORBIDDEN);

    // 1. Official catalog lists 6 curated plugins with risk levels & zh-CN/en descriptions
    let catalog_resp = srv
        .send("GET", "/plugins/catalog", Some(&admin_token), None)
        .await;
    assert_eq!(catalog_resp.status, StatusCode::OK);
    let catalog = catalog_resp.json();
    let catalog_items = catalog["plugins"].as_array().unwrap();
    assert_eq!(catalog_items.len(), 6);
    let tmdb = catalog_items
        .iter()
        .find(|p| p["id"] == "palveni.recipe.tmdb")
        .unwrap();
    assert_eq!(tmdb["version"], "1.2.0");
    assert_eq!(tmdb["previousVersion"], "1.1.0");
    assert_eq!(tmdb["riskLevel"], "medium");
    assert_eq!(tmdb["installed"], false);
    assert!(tmdb["description"]["zh-CN"].is_string());
    assert!(tmdb["description"]["en"].is_string());

    // 2. Install rejected without granting all required permissions
    let missing_perm = srv
        .send(
            "POST",
            "/plugins/install",
            Some(&admin_token),
            Some(json!({
                "pluginId": "palveni.recipe.tmdb",
                "grantedPermissions": ["network:api.themoviedb.org"]
            })),
        )
        .await;
    assert_eq!(missing_perm.status, StatusCode::BAD_REQUEST);

    // 3. Install rejected with invalid settings (enum violation)
    let bad_settings = srv
        .send(
            "POST",
            "/plugins/install",
            Some(&admin_token),
            Some(json!({
                "pluginId": "palveni.recipe.tmdb",
                "grantedPermissions": ["network:api.themoviedb.org", "library:write_metadata"],
                "settings": { "language": "xx-BAD" }
            })),
        )
        .await;
    assert_eq!(bad_settings.status, StatusCode::BAD_REQUEST);

    // 4. Install succeeds with all required permissions & valid settings
    let install_resp = srv
        .send(
            "POST",
            "/plugins/install",
            Some(&admin_token),
            Some(json!({
                "pluginId": "palveni.recipe.tmdb",
                "grantedPermissions": ["network:api.themoviedb.org", "library:write_metadata"],
                "settings": { "language": "en-US", "includeAdult": false }
            })),
        )
        .await;
    assert_eq!(install_resp.status, StatusCode::CREATED);
    let installed = install_resp.json();
    assert_eq!(installed["id"], "palveni.recipe.tmdb");
    assert_eq!(installed["version"], "1.2.0");
    assert_eq!(installed["previousVersion"], "1.1.0");
    assert_eq!(installed["enabled"], true);
    assert_eq!(installed["settings"]["language"], "en-US");
    assert_eq!(installed["settings"]["autoDownloadPoster"], true);
    assert_eq!(installed["riskLevel"], "medium");

    // GET /api/v1/plugins reflects installed plugin with enriched fields
    let list_body = srv
        .send("GET", "/plugins", Some(&admin_token), None)
        .await
        .json();
    assert!(
        list_body["plugins"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["id"] == "palveni.recipe.tmdb" && p["previousVersion"] == "1.1.0")
    );

    // 5. PATCH rejects invalid settings & accepts valid toggle + settings update
    let bad_patch = srv
        .send(
            "PATCH",
            "/plugins/palveni.recipe.tmdb",
            Some(&admin_token),
            Some(json!({ "settings": { "unknownKey": 42 } })),
        )
        .await;
    assert_eq!(bad_patch.status, StatusCode::BAD_REQUEST);

    let ok_patch = srv
        .send(
            "PATCH",
            "/plugins/palveni.recipe.tmdb",
            Some(&admin_token),
            Some(json!({
                "enabled": false,
                "settings": { "language": "ja-JP", "autoDownloadPoster": false }
            })),
        )
        .await;
    assert_eq!(ok_patch.status, StatusCode::OK);
    let patched = ok_patch.json();
    assert_eq!(patched["enabled"], false);
    assert_eq!(patched["settings"]["language"], "ja-JP");
    assert_eq!(patched["settings"]["autoDownloadPoster"], false);

    // 6. Rollback swaps 1.2.0 -> 1.1.0; widget with no previousVersion returns 400
    let rollback_resp = srv
        .send(
            "POST",
            "/plugins/palveni.recipe.tmdb/rollback",
            Some(&admin_token),
            None,
        )
        .await;
    assert_eq!(rollback_resp.status, StatusCode::OK);
    let rolled = rollback_resp.json();
    assert_eq!(rolled["version"], "1.1.0");
    assert!(rolled.get("previousVersion").is_none() || rolled["previousVersion"].is_null());

    // Installing zero-permission widget works, and rollback without previous_version fails 400
    let widget_install = srv
        .send(
            "POST",
            "/plugins/install",
            Some(&admin_token),
            Some(json!({
                "pluginId": "palveni.widget.clock_weather",
                "grantedPermissions": []
            })),
        )
        .await;
    assert_eq!(widget_install.status, StatusCode::CREATED);

    let widget_rb = srv
        .send(
            "POST",
            "/plugins/palveni.widget.clock_weather/rollback",
            Some(&admin_token),
            None,
        )
        .await;
    assert_eq!(widget_rb.status, StatusCode::BAD_REQUEST);

    // 7. Uninstall removes DB plugin
    let del_resp = srv
        .send(
            "DELETE",
            "/plugins/palveni.recipe.tmdb",
            Some(&admin_token),
            None,
        )
        .await;
    assert_eq!(del_resp.status, StatusCode::NO_CONTENT);

    let list_after = srv
        .send("GET", "/plugins", Some(&admin_token), None)
        .await
        .json();
    assert!(
        !list_after["plugins"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["id"] == "palveni.recipe.tmdb")
    );

    // 8. Verify audit log entries recorded for full lifecycle
    let audits = srv.db.audit().recent(20).await.unwrap();
    let actions: Vec<&str> = audits.iter().map(|a| a.action.as_str()).collect();
    assert!(actions.contains(&"plugin.installed"));
    assert!(actions.contains(&"plugin.updated"));
    assert!(actions.contains(&"plugin.rolled_back"));
    assert!(actions.contains(&"plugin.uninstalled"));
}

#[tokio::test]
async fn photo_ai_analyze_bilingual_search_people_duplicates_and_acl() {
    let srv = TestServer::start().await;
    let admin_token = srv.setup_admin().await;

    let dir_public = srv.root.join("photos-public");
    let dir_private = srv.root.join("photos-private");
    std::fs::create_dir_all(&dir_public).unwrap();
    std::fs::create_dir_all(&dir_private).unwrap();

    // Blue landscape night shot (high-res)
    let f_blue_hi = dir_public.join("blue_night_hi.ppm");
    let size_blue_hi = write_ppm(&f_blue_hi, 36, 24, |x, _y| {
        let b = 240u8.saturating_sub(u8::try_from(x).unwrap_or(255).saturating_mul(4));
        (18, 72, b)
    });
    // Exact duplicate of blue shot (same content_hash, smaller size)
    let f_blue_exact = dir_public.join("blue_night_copy.ppm");
    let size_blue_exact = write_ppm(&f_blue_exact, 18, 12, |x, _y| {
        let b = 240u8.saturating_sub(u8::try_from(x).unwrap_or(255).saturating_mul(8));
        (18, 72, b)
    });
    // Two perceptually similar blue shots with distinct content hashes
    let f_blue_sim = dir_public.join("blue_night_sim.ppm");
    let size_blue_sim = write_ppm(&f_blue_sim, 27, 18, |x, _y| {
        let b = 236u8.saturating_sub(u8::try_from(x).unwrap_or(255).saturating_mul(5));
        (20, 74, b)
    });
    let f_blue_sim2 = dir_public.join("blue_night_sim2.ppm");
    let size_blue_sim2 = write_ppm(&f_blue_sim2, 27, 18, |x, _y| {
        let b = 234u8.saturating_sub(u8::try_from(x).unwrap_or(255).saturating_mul(5));
        (21, 75, b)
    });
    // Warm sunset portrait shot in private library (hidden from restricted user)
    let f_warm_priv = dir_private.join("sunset_portrait.ppm");
    let size_warm_priv = write_ppm(&f_warm_priv, 18, 32, |_x, y| {
        let r = 235u8.saturating_sub(u8::try_from(y).unwrap_or(255).saturating_mul(3));
        (r, 120, 35)
    });

    let lib_pub = srv
        .db
        .libraries()
        .create(NewLibrary {
            name: "Public Photos".into(),
            kind: LibraryKind::Photo,
            paths: vec![dir_public.to_string_lossy().into_owned()],
        })
        .await
        .unwrap();
    let lib_priv = srv
        .db
        .libraries()
        .create(NewLibrary {
            name: "Private Photos".into(),
            kind: LibraryKind::Photo,
            paths: vec![dir_private.to_string_lossy().into_owned()],
        })
        .await
        .unwrap();

    let id_blue_hi = ItemId::generate();
    let id_blue_exact = ItemId::generate();
    let id_blue_sim = ItemId::generate();
    let id_blue_sim2 = ItemId::generate();
    let id_warm_priv = ItemId::generate();

    let autumn_night = chrono::Utc
        .with_ymd_and_hms(2025, 10, 18, 22, 15, 0)
        .single()
        .unwrap()
        .timestamp_millis();
    let spring_sunset = chrono::Utc
        .with_ymd_and_hms(2026, 4, 5, 18, 10, 0)
        .single()
        .unwrap()
        .timestamp_millis();

    srv.db
        .items()
        .apply_scan_batch(
            lib_pub.id,
            &[
                ItemUpsert {
                    music: None,
                    id: id_blue_hi,
                    parent_id: None,
                    kind: ItemKind::Photo,
                    title: "blue_night_hi".into(),
                    sort_title: "blue_night_hi".into(),
                    search_text: "blue_night_hi".into(),
                    path: f_blue_hi.to_string_lossy().into_owned(),
                    size_bytes: Some(size_blue_hi + 4_000_000),
                    modified_at: Some(autumn_night),
                },
                ItemUpsert {
                    music: None,
                    id: id_blue_exact,
                    parent_id: None,
                    kind: ItemKind::Photo,
                    title: "blue_night_copy".into(),
                    sort_title: "blue_night_copy".into(),
                    search_text: "blue_night_copy".into(),
                    path: f_blue_exact.to_string_lossy().into_owned(),
                    size_bytes: Some(size_blue_exact + 1_000_000),
                    modified_at: Some(autumn_night),
                },
                ItemUpsert {
                    music: None,
                    id: id_blue_sim,
                    parent_id: None,
                    kind: ItemKind::Photo,
                    title: "blue_night_sim".into(),
                    sort_title: "blue_night_sim".into(),
                    search_text: "blue_night_sim".into(),
                    path: f_blue_sim.to_string_lossy().into_owned(),
                    size_bytes: Some(size_blue_sim + 2_000_000),
                    modified_at: Some(autumn_night + 10_000),
                },
                ItemUpsert {
                    music: None,
                    id: id_blue_sim2,
                    parent_id: None,
                    kind: ItemKind::Photo,
                    title: "blue_night_sim2".into(),
                    sort_title: "blue_night_sim2".into(),
                    search_text: "blue_night_sim2".into(),
                    path: f_blue_sim2.to_string_lossy().into_owned(),
                    size_bytes: Some(size_blue_sim2 + 1_500_000),
                    modified_at: Some(autumn_night + 20_000),
                },
            ],
            &[],
        )
        .await
        .unwrap();

    srv.db
        .items()
        .apply_scan_batch(
            lib_priv.id,
            &[ItemUpsert {
                music: None,
                id: id_warm_priv,
                parent_id: None,
                kind: ItemKind::Photo,
                title: "sunset_portrait".into(),
                sort_title: "sunset_portrait".into(),
                search_text: "sunset_portrait".into(),
                path: f_warm_priv.to_string_lossy().into_owned(),
                size_bytes: Some(size_warm_priv + 3_000_000),
                modified_at: Some(spring_sunset),
            }],
            &[],
        )
        .await
        .unwrap();

    srv.db
        .photos()
        .upsert_batch(
            lib_pub.id,
            &[
                PhotoMetadataUpsert {
                    item_id: id_blue_hi,
                    stamp: "s1".into(),
                    taken_at: autumn_night,
                    day_bucket: "2025-10-18".into(),
                    month_bucket: "2025-10".into(),
                    width: Some(6000),
                    height: Some(4000),
                    orientation: 1,
                    camera_make: Some("SONY".into()),
                    camera_model: Some("ILCE-7M4".into()),
                    lens_model: None,
                    iso: Some(800),
                    f_number: Some(2.0),
                    exposure_time: None,
                    focal_length: None,
                    latitude: Some(35.0),
                    longitude: Some(135.7),
                    content_hash: Some("exact_hash_blue".into()),
                },
                PhotoMetadataUpsert {
                    item_id: id_blue_exact,
                    stamp: "s2".into(),
                    taken_at: autumn_night,
                    day_bucket: "2025-10-18".into(),
                    month_bucket: "2025-10".into(),
                    width: Some(3000),
                    height: Some(2000),
                    orientation: 1,
                    camera_make: Some("SONY".into()),
                    camera_model: Some("ILCE-7M4".into()),
                    lens_model: None,
                    iso: Some(800),
                    f_number: Some(2.0),
                    exposure_time: None,
                    focal_length: None,
                    latitude: None,
                    longitude: None,
                    content_hash: Some("exact_hash_blue".into()),
                },
                PhotoMetadataUpsert {
                    item_id: id_blue_sim,
                    stamp: "s3".into(),
                    taken_at: autumn_night + 10_000,
                    day_bucket: "2025-10-18".into(),
                    month_bucket: "2025-10".into(),
                    width: Some(4500),
                    height: Some(3000),
                    orientation: 1,
                    camera_make: Some("SONY".into()),
                    camera_model: Some("ILCE-7M4".into()),
                    lens_model: None,
                    iso: Some(800),
                    f_number: Some(2.0),
                    exposure_time: None,
                    focal_length: None,
                    latitude: None,
                    longitude: None,
                    content_hash: Some("unique_sim_hash_1".into()),
                },
                PhotoMetadataUpsert {
                    item_id: id_blue_sim2,
                    stamp: "s5".into(),
                    taken_at: autumn_night + 20_000,
                    day_bucket: "2025-10-18".into(),
                    month_bucket: "2025-10".into(),
                    width: Some(3600),
                    height: Some(2400),
                    orientation: 1,
                    camera_make: Some("SONY".into()),
                    camera_model: Some("ILCE-7M4".into()),
                    lens_model: None,
                    iso: Some(800),
                    f_number: Some(2.0),
                    exposure_time: None,
                    focal_length: None,
                    latitude: None,
                    longitude: None,
                    content_hash: Some("unique_sim_hash_2".into()),
                },
            ],
        )
        .await
        .unwrap();

    srv.db
        .photos()
        .upsert_batch(
            lib_priv.id,
            &[PhotoMetadataUpsert {
                item_id: id_warm_priv,
                stamp: "s4".into(),
                taken_at: spring_sunset,
                day_bucket: "2026-04-05".into(),
                month_bucket: "2026-04".into(),
                width: Some(3000),
                height: Some(4000),
                orientation: 1,
                camera_make: Some("Apple".into()),
                camera_model: Some("iPhone 16 Pro".into()),
                lens_model: None,
                iso: Some(100),
                f_number: Some(1.8),
                exposure_time: None,
                focal_length: None,
                latitude: None,
                longitude: None,
                content_hash: Some("unique_priv_hash".into()),
            }],
        )
        .await
        .unwrap();

    // 1. Run pure-local AI analysis
    let analyze_resp = srv
        .send("POST", "/photos/analyze", Some(&admin_token), None)
        .await;
    assert_eq!(analyze_resp.status, StatusCode::OK);
    let analyzed = analyze_resp.json();
    assert_eq!(analyzed["analyzedCount"], 5);
    assert_eq!(analyzed["totalTagged"], 5);
    assert_eq!(analyzed["remaining"], 0);

    // 2. Bilingual natural-language search
    let night_search = srv
        .send(
            "GET",
            &format!("/photos/search?q={}", percent_encode_query("夜景")),
            Some(&admin_token),
            None,
        )
        .await;
    assert_eq!(night_search.status, StatusCode::OK);
    let night_json = night_search.json();
    assert_eq!(night_json["parsedFilters"]["timeOfDay"], "night");
    assert_eq!(night_json["items"].as_array().unwrap().len(), 4);

    let blue_search = srv
        .send("GET", "/photos/search?q=blue", Some(&admin_token), None)
        .await
        .json();
    assert_eq!(blue_search["parsedFilters"]["color"], "blue");
    assert_eq!(blue_search["items"].as_array().unwrap().len(), 4);

    let autumn_2025 = srv
        .send(
            "GET",
            &format!("/photos/search?q={}", percent_encode_query("2025年秋天")),
            Some(&admin_token),
            None,
        )
        .await
        .json();
    assert_eq!(autumn_2025["parsedFilters"]["year"], 2025);
    assert_eq!(autumn_2025["parsedFilters"]["season"], "autumn");
    assert_eq!(autumn_2025["items"].as_array().unwrap().len(), 4);

    // 3. People CRUD + tagging + search by person name
    let person_resp = srv
        .send(
            "POST",
            "/me/photo-people",
            Some(&admin_token),
            Some(json!({ "name": "小明" })),
        )
        .await;
    assert_eq!(person_resp.status, StatusCode::CREATED);
    let person = person_resp.json();
    let person_id = person["id"].as_str().unwrap().to_owned();
    assert!(person["coverItemId"].is_null());

    let tagged_resp = srv
        .send(
            "POST",
            &format!("/items/{id_blue_hi}/photo-people/{person_id}"),
            Some(&admin_token),
            None,
        )
        .await;
    assert_eq!(tagged_resp.status, StatusCode::OK);
    let tagged = tagged_resp.json();
    assert_eq!(tagged["photoCount"], 1);
    assert_eq!(tagged["coverItemId"], id_blue_hi.to_string());

    let by_person = srv
        .send(
            "GET",
            &format!("/photos/search?q={}", percent_encode_query("小明")),
            Some(&admin_token),
            None,
        )
        .await
        .json();
    assert_eq!(by_person["items"].as_array().unwrap().len(), 1);
    assert_eq!(by_person["items"][0]["id"], id_blue_hi.to_string());

    let renamed = srv
        .send(
            "PATCH",
            &format!("/me/photo-people/{person_id}"),
            Some(&admin_token),
            Some(json!({ "name": "陈小明" })),
        )
        .await
        .json();
    assert_eq!(renamed["name"], "陈小明");

    // 4. Exact & Similar duplicate clusters
    let dups_resp = srv
        .send(
            "GET",
            "/photos/duplicates?threshold=6",
            Some(&admin_token),
            None,
        )
        .await;
    assert_eq!(dups_resp.status, StatusCode::OK);
    let dups = dups_resp.json();
    let clusters = dups["clusters"].as_array().unwrap();
    assert_eq!(clusters.len(), 2);
    assert_eq!(clusters[0]["kind"], "exact");
    assert_eq!(clusters[0]["keepItemId"], id_blue_hi.to_string());
    assert!(clusters[0]["reclaimableBytes"].as_u64().unwrap() > 0);
    assert_eq!(clusters[1]["kind"], "similar");
    assert_eq!(clusters[1]["keepItemId"], id_blue_sim.to_string());
    assert!(clusters[1]["reclaimableBytes"].as_u64().unwrap() > 0);

    // 5. Library ACL enforcement for restricted user (only allowed lib_pub)
    let _db_user = srv
        .db
        .accounts()
        .create_user(NewManagedUser {
            username: "guest".into(),
            password_hash: "unused".into(),
            role: Role::User,
            policy: UserPolicy {
                all_libraries: false,
                ..UserPolicy::default()
            },
            allowed_library_ids: vec![lib_pub.id],
        })
        .await
        .unwrap();

    let api_kid = srv
        .send(
            "POST",
            "/users",
            Some(&admin_token),
            Some(json!({
                "username": "kid",
                "password": "password123",
                "role": "user",
                "policy": {
                    "allLibraries": false,
                    "allowDownload": true,
                    "allowTranscode": true,
                    "dailyAiLimit": 0
                },
                "allowedLibraryIds": [lib_pub.id]
            })),
        )
        .await;
    assert_eq!(api_kid.status, StatusCode::CREATED);

    let kid_login = srv
        .send(
            "POST",
            "/auth/login",
            None,
            Some(json!({
                "username": "kid",
                "password": "password123",
                "deviceName": "kid-ipad"
            })),
        )
        .await;
    assert_eq!(kid_login.status, StatusCode::OK);
    let kid_token = kid_login.json()["accessToken"].as_str().unwrap().to_owned();

    // Kid searching for "日落" (sunset portrait in lib_priv) must see 0 photos!
    let kid_sunset = srv
        .send(
            "GET",
            &format!("/photos/search?q={}", percent_encode_query("日落")),
            Some(&kid_token),
            None,
        )
        .await
        .json();
    assert_eq!(kid_sunset["items"].as_array().unwrap().len(), 0);

    // Kid searching for "夜景" sees only the 4 public photos
    let kid_night = srv
        .send(
            "GET",
            &format!("/photos/search?q={}", percent_encode_query("夜景")),
            Some(&kid_token),
            None,
        )
        .await
        .json();
    assert_eq!(kid_night["items"].as_array().unwrap().len(), 4);

    // Untag & delete person
    let untag = srv
        .send(
            "DELETE",
            &format!("/items/{id_blue_hi}/photo-people/{person_id}"),
            Some(&admin_token),
            None,
        )
        .await;
    assert_eq!(untag.status, StatusCode::NO_CONTENT);

    let del_person = srv
        .send(
            "DELETE",
            &format!("/me/photo-people/{person_id}"),
            Some(&admin_token),
            None,
        )
        .await;
    assert_eq!(del_person.status, StatusCode::NO_CONTENT);
}

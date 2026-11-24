//! End-to-end HTTP integration tests for Wave 3 Track 1:
//! - Jellyfin compatible protocol subset (auth, views, items, shows, images, playback, streaming, ticks, played/favorite, ACL, refresh)
//! - `.strm` scanning, SSRF rejection, playback `directUrl` & 302 redirect
//! - Native `/api/v1/me/api-keys`, `/api/v1/items/{id}/segments`, and `/api/v1/items/{id}/trickplay`

use std::collections::HashMap;
use std::error::Error;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::sync::Arc;

use axum::http::{StatusCode, header};
use palveni_api::{AppState, LinkPairings, LoginLimit, ServerInfo, router};
use palveni_core::{FeatureRegistry, LibraryId, LibraryKind, PerfTier, ServerId};
use palveni_db::{Db, DbOptions};
use palveni_jobs::JobService;
use palveni_library::{ScanObserver, ScanStep, Scanner};
use palveni_link::NodeIdentity;
use palveni_media::{HardwareService, ProbeOptions};
use palveni_plugin_host::{PluginCatalog, skin_store::SkinStore};
use serde_json::{Value, json};
use tokio::io::{AsyncReadExt, AsyncWriteExt, DuplexStream};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

struct NoopObserver;

impl ScanObserver for NoopObserver {
    fn progress(&self, _fraction: Option<f64>, _step: &ScanStep) {}
}

struct DuplexListener {
    rx: mpsc::UnboundedReceiver<(DuplexStream, SocketAddr)>,
    local: SocketAddr,
}

impl axum::serve::Listener for DuplexListener {
    type Io = DuplexStream;
    type Addr = SocketAddr;

    async fn accept(&mut self) -> (Self::Io, Self::Addr) {
        match self.rx.recv().await {
            Some(conn) => conn,
            None => std::future::pending().await,
        }
    }

    fn local_addr(&self) -> std::io::Result<Self::Addr> {
        Ok(self.local)
    }
}

struct HttpReply {
    status: StatusCode,
    headers: HashMap<String, String>,
    body: Vec<u8>,
}

impl HttpReply {
    fn json(&self) -> TestResult<Value> {
        Ok(serde_json::from_slice(&self.body)?)
    }

    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .get(&name.to_ascii_lowercase())
            .map(String::as_str)
    }
}

struct TestClient {
    tx: mpsc::UnboundedSender<(DuplexStream, SocketAddr)>,
    peer: SocketAddr,
}

impl TestClient {
    async fn send(
        &self,
        method: &str,
        path: &str,
        extra_headers: &[(&str, &str)],
        json_body: Option<&Value>,
    ) -> TestResult<HttpReply> {
        let (mut client, server) = tokio::io::duplex(256 * 1024);
        self.tx
            .send((server, self.peer))
            .map_err(|_| "in-memory server stopped")?;

        let body_bytes = match json_body {
            Some(v) => serde_json::to_vec(v)?,
            None => Vec::new(),
        };
        let mut req =
            format!("{method} {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n");
        if json_body.is_some() {
            req.push_str("Content-Type: application/json\r\n");
        }
        req.push_str(&format!("Content-Length: {}\r\n", body_bytes.len()));
        for (k, v) in extra_headers {
            req.push_str(&format!("{k}: {v}\r\n"));
        }
        req.push_str("\r\n");

        client.write_all(req.as_bytes()).await?;
        if !body_bytes.is_empty() {
            client.write_all(&body_bytes).await?;
        }
        client.flush().await?;

        let mut raw = Vec::new();
        client.read_to_end(&mut raw).await?;

        let split = raw
            .windows(4)
            .position(|w| w == b"\r\n\r\n")
            .ok_or("malformed HTTP response: missing header terminator")?;
        let head = std::str::from_utf8(&raw[..split])?;
        let body_raw = &raw[split + 4..];

        let mut lines = head.split("\r\n");
        let status_line = lines.next().ok_or("missing HTTP status line")?;
        let code: u16 = status_line
            .split_whitespace()
            .nth(1)
            .ok_or("missing HTTP status code")?
            .parse()?;
        let status = StatusCode::from_u16(code)?;

        let mut headers = HashMap::new();
        for line in lines {
            if let Some((k, v)) = line.split_once(':') {
                headers.insert(k.trim().to_ascii_lowercase(), v.trim().to_owned());
            }
        }

        let body = if headers
            .get("transfer-encoding")
            .is_some_and(|te| te.to_ascii_lowercase().contains("chunked"))
        {
            decode_chunked(body_raw)?
        } else {
            body_raw.to_vec()
        };

        Ok(HttpReply {
            status,
            headers,
            body,
        })
    }
}

fn decode_chunked(mut data: &[u8]) -> TestResult<Vec<u8>> {
    let mut out = Vec::new();
    loop {
        let pos = data
            .windows(2)
            .position(|w| w == b"\r\n")
            .ok_or("invalid chunked stream")?;
        let size_str = std::str::from_utf8(&data[..pos])?
            .split(';')
            .next()
            .unwrap_or("")
            .trim();
        let size = usize::from_str_radix(size_str, 16)?;
        data = &data[pos + 2..];
        if size == 0 {
            break;
        }
        if data.len() < size + 2 {
            return Err("truncated chunk".into());
        }
        out.extend_from_slice(&data[..size]);
        data = &data[size + 2..];
    }
    Ok(out)
}

struct TestServer {
    client: TestClient,
    db: Db,
    state: AppState,
    root: PathBuf,
    _shutdown_guard: CancellationToken,
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self._shutdown_guard.cancel();
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

async fn spawn_test_server() -> TestResult<TestServer> {
    let raw_root =
        std::env::temp_dir().join(format!("palveni-jellyfin-it-{}", ServerId::generate()));
    std::fs::create_dir_all(&raw_root)?;
    let root = std::fs::canonicalize(&raw_root)?;

    let db = Db::open(&DbOptions {
        main_url: format!("sqlite://{}/main.db?mode=rwc", root.display()),
        ops_url: format!("sqlite://{}/ops.db?mode=rwc", root.display()),
        max_connections: Some(4),
        min_connections: Some(1),
    })
    .await?;

    let bus = db.event_bus(64).await?;
    let jobs = JobService::new(db.clone(), bus.clone(), 1);
    let shutdown = CancellationToken::new();

    let state = AppState {
        db: db.clone(),
        music_search: Arc::new(palveni_library::music_search::MusicSearch::new(
            root.join("search"),
        )),
        video_images: Arc::new(palveni_library::video::VideoImages::new(
            root.join("cache"),
            None,
            None,
        )),
        identity: Arc::new(NodeIdentity::generate()?),
        link: Arc::new(LinkPairings::default()),
        login_limit: Arc::new(LoginLimit::new(8)),
        subsonic_limit: Arc::new(LoginLimit::new(8)),
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
        skins: Arc::new(SkinStore::open(&root.join("skins"))?),
        skin_writes: Arc::new(tokio::sync::Semaphore::new(1)),
        features: FeatureRegistry::new(),
        server: Arc::new(ServerInfo {
            id: ServerId::generate(),
            name: "Palveni Cinema".into(),
            version: "0.3.0".into(),
        }),
        shutdown: shutdown.clone(),
    };

    let local = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 8096);
    let app = router(state.clone()).layer(axum::Extension(axum::extract::ConnectInfo(local)));
    let (tx, rx) = mpsc::unbounded_channel();
    let listener = DuplexListener { rx, local };
    let cancel = shutdown.clone();
    tokio::spawn(async move {
        let _ = axum::serve(listener, app)
            .with_graceful_shutdown(async move {
                cancel.cancelled().await;
            })
            .await;
    });

    Ok(TestServer {
        client: TestClient { tx, peer: local },
        db,
        state,
        root,
        _shutdown_guard: shutdown,
    })
}

/// Minimal valid 1x1 JPEG bytes for poster artwork fixtures.
const TINY_JPEG: &[u8] = &[
    0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, b'J', b'F', b'I', b'F', 0x00, 0x01, 0x01, 0x00, 0x00, 0x01,
    0x00, 0x01, 0x00, 0x00, 0xFF, 0xDB, 0x00, 0x43, 0x00, 0x08, 0x06, 0x06, 0x07, 0x06, 0x05, 0x08,
    0x07, 0x07, 0x07, 0x09, 0x09, 0x08, 0x0A, 0x0C, 0x14, 0x0D, 0x0C, 0x0B, 0x0B, 0x0C, 0x19, 0x12,
    0x13, 0x0F, 0x14, 0x1D, 0x1A, 0x1F, 0x1E, 0x1D, 0x1A, 0x1C, 0x1C, 0x20, 0x24, 0x2E, 0x27, 0x20,
    0x22, 0x2C, 0x23, 0x1C, 0x1C, 0x28, 0x37, 0x29, 0x2C, 0x30, 0x31, 0x34, 0x34, 0x34, 0x1F, 0x27,
    0x39, 0x3D, 0x38, 0x32, 0x3C, 0x2E, 0x33, 0x34, 0x32, 0xFF, 0xC0, 0x00, 0x0B, 0x08, 0x00, 0x01,
    0x00, 0x01, 0x01, 0x01, 0x11, 0x00, 0xFF, 0xC4, 0x00, 0x14, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x09, 0xFF, 0xC4, 0x00, 0x14,
    0x10, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0xFF, 0xDA, 0x00, 0x08, 0x01, 0x01, 0x00, 0x00, 0x3F, 0x00, 0x7F, 0x00, 0xFF, 0xD9,
];

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn jellyfin_protocol_strm_api_keys_segments_and_acl() -> TestResult {
    let srv = spawn_test_server().await?;

    // 1. Setup admin & login via native API
    let setup_resp = srv
        .client
        .send(
            "POST",
            "/api/v1/setup",
            &[],
            Some(&json!({ "username": "admin", "password": "admin-password-1" })),
        )
        .await?;
    assert_eq!(setup_resp.status, StatusCode::CREATED);
    let admin_user = setup_resp.json()?;
    let admin_id = admin_user["user"]["id"]
        .as_str()
        .expect("admin id")
        .to_owned();

    let login_resp = srv
        .client
        .send(
            "POST",
            "/api/v1/auth/login",
            &[],
            Some(&json!({
                "username": "admin",
                "password": "admin-password-1",
                "deviceName": "Test Runner"
            })),
        )
        .await?;
    assert_eq!(login_resp.status, StatusCode::OK);
    let admin_token = login_resp.json()?["accessToken"]
        .as_str()
        .expect("admin token")
        .to_owned();
    let admin_bearer = format!("Bearer {admin_token}");

    // 2. Prepare two video libraries:
    //    - Movies (local mp4 + poster + valid .strm + SSRF .strm + series)
    //    - PrivateVault (restricted from family member)
    let movies_dir = srv.root.join("movies");
    let movie_folder = movies_dir.join("Dune (2021)");
    let show_dir = movies_dir.join("Arcane").join("Season 01");
    let private_dir = srv.root.join("private");
    std::fs::create_dir_all(&movie_folder)?;
    std::fs::create_dir_all(&show_dir)?;
    std::fs::create_dir_all(&private_dir)?;

    let local_video_bytes = b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
    std::fs::write(movie_folder.join("Dune (2021).mp4"), local_video_bytes)?;
    std::fs::write(movie_folder.join("poster.jpg"), TINY_JPEG)?;
    std::fs::write(
        movies_dir.join("Cloud Film (2024).strm"),
        "# CloudDrive link\nhttps://cdn.example.com/media/cloud-film.mp4?token=ok\n",
    )?;
    std::fs::write(
        movies_dir.join("Bad Metadata.strm"),
        "http://169.254.169.254/latest/meta-data/\n",
    )?;
    std::fs::write(movies_dir.join("Bad File.strm"), "file:///etc/passwd\n")?;
    std::fs::write(
        show_dir.join("Arcane S01E01.mp4"),
        b"episode-one-payload-bytes",
    )?;
    std::fs::write(
        show_dir.join("Arcane S01E02.mp4"),
        b"episode-two-payload-bytes",
    )?;
    std::fs::write(private_dir.join("Secret Film (2025).mp4"), b"secret-bytes")?;

    let movies_lib = srv
        .db
        .libraries()
        .create(palveni_db::libraries::NewLibrary {
            name: "Movies & Shows".into(),
            kind: LibraryKind::Video,
            paths: vec![movies_dir.display().to_string()],
        })
        .await?;
    let private_lib = srv
        .db
        .libraries()
        .create(palveni_db::libraries::NewLibrary {
            name: "Private Vault".into(),
            kind: LibraryKind::Video,
            paths: vec![private_dir.display().to_string()],
        })
        .await?;

    // Scan both libraries (verifies `.strm` files are indexed cleanly without ffprobe hang/failure)
    let scanner = Scanner::new(srv.db.clone(), srv.state.bus.clone());
    scanner.scan(movies_lib.id, &NoopObserver).await?;
    scanner.scan(private_lib.id, &NoopObserver).await?;

    // Create a family user who ONLY has access to `movies_lib` (NOT `private_lib`)
    let member_resp = srv
        .client
        .send(
            "POST",
            "/api/v1/users",
            &[("Authorization", &admin_bearer)],
            Some(&json!({
                "username": "kid",
                "password": "kid-password-123",
                "role": "user",
                "policy": {
                    "allLibraries": false,
                    "allowDownload": true,
                    "allowTranscode": true,
                    "dailyAiLimit": 0
                },
                "allowedLibraryIds": [movies_lib.id.to_string()]
            })),
        )
        .await?;
    assert_eq!(member_resp.status, StatusCode::CREATED);
    let kid_id = member_resp.json()?["id"]
        .as_str()
        .expect("kid id")
        .to_owned();

    // 3. Jellyfin Public & Ping endpoints (no auth)
    let pub_resp = srv
        .client
        .send("GET", "/System/Info/Public", &[], None)
        .await?;
    assert_eq!(pub_resp.status, StatusCode::OK);
    let pub_info = pub_resp.json()?;
    assert_eq!(pub_info["ProductName"], "Palveni");
    assert_eq!(pub_info["ServerName"], "Palveni Cinema");
    assert_eq!(pub_info["StartupWizardCompleted"], true);
    assert!(pub_info["Id"].as_str().is_some_and(|s| !s.is_empty()));

    let ping_get = srv.client.send("GET", "/System/Ping", &[], None).await?;
    assert_eq!(ping_get.status, StatusCode::OK);
    assert_eq!(ping_get.json()?, "Palveni");

    let ping_post = srv.client.send("POST", "/System/Ping", &[], None).await?;
    assert_eq!(ping_post.status, StatusCode::OK);
    assert_eq!(ping_post.json()?, "Palveni");

    // 4. Jellyfin AuthenticateByName for kid (validates Argon2 & returns AccessToken)
    let jf_auth_resp = srv
        .client
        .send(
            "POST",
            "/Users/AuthenticateByName",
            &[(
                "Authorization",
                r#"MediaBrowser Client="Infuse", Device="Apple TV", DeviceId="atv-1", Version="8.0""#,
            )],
            Some(&json!({ "Username": "kid", "Pw": "kid-password-123" })),
        )
        .await?;
    assert_eq!(jf_auth_resp.status, StatusCode::OK);
    let jf_auth = jf_auth_resp.json()?;
    let kid_jf_token = jf_auth["AccessToken"]
        .as_str()
        .expect("jellyfin access token")
        .to_owned();
    let kid_bearer = format!("Bearer {kid_jf_token}");
    assert_eq!(jf_auth["User"]["Name"], "kid");
    assert_eq!(jf_auth["User"]["Id"], kid_id);

    // /System/Info requires auth and accepts MediaBrowser Token header
    let unauth_info = srv.client.send("GET", "/System/Info", &[], None).await?;
    assert_eq!(unauth_info.status, StatusCode::UNAUTHORIZED);

    let mb_header = format!(
        r#"MediaBrowser Client="Infuse", Device="Apple TV", DeviceId="atv-1", Version="8.0", Token="{kid_jf_token}""#
    );
    let authed_info_resp = srv
        .client
        .send(
            "GET",
            "/System/Info",
            &[("Authorization", &mb_header)],
            None,
        )
        .await?;
    assert_eq!(authed_info_resp.status, StatusCode::OK);
    assert_eq!(authed_info_resp.json()?["ProductName"], "Palveni");

    // 5. Client API keys CRUD (`/api/v1/me/api-keys`) + Jellyfin auth using client API key
    let create_key_resp = srv
        .client
        .send(
            "POST",
            "/api/v1/me/api-keys",
            &[("Authorization", &kid_bearer)],
            Some(&json!({ "name": "VidHub iPad", "clientKind": "jellyfin" })),
        )
        .await?;
    assert_eq!(create_key_resp.status, StatusCode::CREATED);
    let created_key = create_key_resp.json()?;
    let api_key_id = created_key["key"]["id"]
        .as_str()
        .expect("api key id")
        .to_owned();
    let api_key_secret = created_key["token"]
        .as_str()
        .expect("api key secret")
        .to_owned();
    assert!(api_key_secret.starts_with("pvk_"));

    let keys_list = srv
        .client
        .send(
            "GET",
            "/api/v1/me/api-keys",
            &[("Authorization", &kid_bearer)],
            None,
        )
        .await?;
    assert_eq!(keys_list.status, StatusCode::OK);
    let keys_json = keys_list.json()?;
    assert_eq!(keys_json["keys"].as_array().map(Vec::len), Some(1));
    assert!(keys_json["keys"][0].get("keyHash").is_none());

    // 6. Jellyfin /Users/{id}/Views enforces per-library ACL via X-Emby-Token (client API key!)
    let kid_views_resp = srv
        .client
        .send(
            "GET",
            &format!("/Users/{kid_id}/Views"),
            &[("X-Emby-Token", &api_key_secret)],
            None,
        )
        .await?;
    assert_eq!(kid_views_resp.status, StatusCode::OK);
    let kid_views = kid_views_resp.json()?;
    let view_items = kid_views["Items"].as_array().expect("views array");
    assert_eq!(view_items.len(), 1);
    assert_eq!(view_items[0]["Id"], movies_lib.id.to_string());
    assert_eq!(view_items[0]["Type"], "CollectionFolder");

    // Admin sees both libraries
    let admin_views_resp = srv
        .client
        .send(
            "GET",
            &format!("/Users/{admin_id}/Views"),
            &[("X-Emby-Token", &admin_token)],
            None,
        )
        .await?;
    assert_eq!(admin_views_resp.status, StatusCode::OK);
    assert_eq!(
        admin_views_resp.json()?["Items"].as_array().map(Vec::len),
        Some(2)
    );

    // 7. Jellyfin /Users/{id}/Items, Series/Seasons/Episodes, SearchTerm, and ACL isolation
    let all_kid_items_resp = srv
        .client
        .send(
            "GET",
            &format!(
                "/Users/{kid_id}/Items?ParentId={}&SortBy=SortName&SortOrder=Ascending",
                movies_lib.id
            ),
            &[("X-MediaBrowser-Token", &api_key_secret)],
            None,
        )
        .await?;
    assert_eq!(all_kid_items_resp.status, StatusCode::OK);
    let all_kid_items = all_kid_items_resp.json()?;
    let items_arr = all_kid_items["Items"].as_array().expect("items array");
    assert!(items_arr.len() >= 5);

    let dune_id = items_arr
        .iter()
        .find(|i| i["Name"].as_str() == Some("Dune"))
        .and_then(|i| i["Id"].as_str())
        .expect("Dune movie found")
        .to_owned();
    let cloud_id = items_arr
        .iter()
        .find(|i| i["Name"].as_str() == Some("Cloud Film"))
        .and_then(|i| i["Id"].as_str())
        .expect("Cloud Film strm found")
        .to_owned();
    let bad_meta_id = items_arr
        .iter()
        .find(|i| i["Name"].as_str() == Some("Bad Metadata"))
        .and_then(|i| i["Id"].as_str())
        .expect("Bad Metadata id")
        .to_owned();
    let bad_file_id = items_arr
        .iter()
        .find(|i| i["Name"].as_str() == Some("Bad File"))
        .and_then(|i| i["Id"].as_str())
        .expect("Bad File id")
        .to_owned();
    let arcane_id = items_arr
        .iter()
        .find(|i| i["Type"].as_str() == Some("Series"))
        .and_then(|i| i["Id"].as_str())
        .expect("Arcane series found")
        .to_owned();

    // Seasons & Episodes endpoints
    let seasons = srv
        .client
        .send(
            "GET",
            &format!("/Shows/{arcane_id}/Seasons"),
            &[("X-Emby-Token", &api_key_secret)],
            None,
        )
        .await?;
    assert_eq!(seasons.status, StatusCode::OK);
    assert_eq!(seasons.json()?["TotalRecordCount"], 1);

    let episodes = srv
        .client
        .send(
            "GET",
            &format!("/Shows/{arcane_id}/Episodes?Season=1"),
            &[("X-Emby-Token", &api_key_secret)],
            None,
        )
        .await?;
    assert_eq!(episodes.status, StatusCode::OK);
    assert_eq!(episodes.json()?["TotalRecordCount"], 2);

    // SearchTerm filter
    let search_res = srv
        .client
        .send(
            "GET",
            "/Items?SearchTerm=Dune",
            &[("X-Emby-Token", &api_key_secret)],
            None,
        )
        .await?;
    assert_eq!(search_res.status, StatusCode::OK);
    let search_json = search_res.json()?;
    assert_eq!(search_json["TotalRecordCount"], 1);
    assert_eq!(search_json["Items"][0]["Id"], dune_id);

    // Locate Private Vault item as admin, confirm kid gets 403 on detail & stream!
    let private_items = srv
        .client
        .send(
            "GET",
            &format!("/Users/{admin_id}/Items?ParentId={}", private_lib.id),
            &[("X-Emby-Token", &admin_token)],
            None,
        )
        .await?;
    assert_eq!(private_items.status, StatusCode::OK);
    let secret_item_id = private_items.json()?["Items"][0]["Id"]
        .as_str()
        .expect("secret item id")
        .to_owned();

    let acl_detail = srv
        .client
        .send(
            "GET",
            &format!("/Users/{kid_id}/Items/{secret_item_id}"),
            &[("X-Emby-Token", &api_key_secret)],
            None,
        )
        .await?;
    assert_eq!(acl_detail.status, StatusCode::FORBIDDEN);

    let acl_stream = srv
        .client
        .send(
            "GET",
            &format!("/Videos/{secret_item_id}/stream?api_key={api_key_secret}"),
            &[],
            None,
        )
        .await?;
    assert_eq!(acl_stream.status, StatusCode::FORBIDDEN);

    // 8. Images, PlaybackInfo & Streaming (local Byte-Range + .strm 302 redirect)
    let poster_resp = srv
        .client
        .send(
            "GET",
            &format!("/Items/{dune_id}/Images/Primary?api_key={api_key_secret}"),
            &[],
            None,
        )
        .await?;
    assert_eq!(poster_resp.status, StatusCode::OK);
    assert_eq!(
        poster_resp.header(header::CONTENT_TYPE.as_str()),
        Some("image/jpeg")
    );

    let local_pb_resp = srv
        .client
        .send(
            "POST",
            &format!("/Items/{dune_id}/PlaybackInfo"),
            &[("X-Emby-Token", &api_key_secret)],
            None,
        )
        .await?;
    assert_eq!(local_pb_resp.status, StatusCode::OK);
    let local_pb = local_pb_resp.json()?;
    assert_eq!(local_pb["MediaSources"][0]["SupportsDirectPlay"], true);
    assert!(
        local_pb["MediaSources"][0]["DirectStreamUrl"]
            .as_str()
            .is_some_and(|u| u.starts_with("/Videos/"))
    );

    // Range request on local video stream
    let range_resp = srv
        .client
        .send(
            "GET",
            &format!("/Videos/{dune_id}/stream.mp4?api_key={api_key_secret}"),
            &[("Range", "bytes=0-9")],
            None,
        )
        .await?;
    assert_eq!(range_resp.status, StatusCode::PARTIAL_CONTENT);
    assert_eq!(range_resp.body.as_slice(), b"0123456789");

    // `.strm` Jellyfin PlaybackInfo returns direct external URL & /Videos/{id}/stream returns 302
    let strm_pb_resp = srv
        .client
        .send(
            "GET",
            &format!("/Items/{cloud_id}/PlaybackInfo"),
            &[("X-Emby-Token", &api_key_secret)],
            None,
        )
        .await?;
    assert_eq!(strm_pb_resp.status, StatusCode::OK);
    assert_eq!(
        strm_pb_resp.json()?["MediaSources"][0]["DirectStreamUrl"],
        "https://cdn.example.com/media/cloud-film.mp4?token=ok"
    );

    let strm_redirect = srv
        .client
        .send(
            "GET",
            &format!("/Videos/{cloud_id}/stream?api_key={api_key_secret}"),
            &[],
            None,
        )
        .await?;
    assert_eq!(strm_redirect.status, StatusCode::FOUND);
    assert_eq!(
        strm_redirect.header(header::LOCATION.as_str()),
        Some("https://cdn.example.com/media/cloud-film.mp4?token=ok")
    );

    // Native `/api/v1/items/{id}/video-playback` exposes `directUrl` for `.strm` & rejects SSRF targets
    let native_strm_resp = srv
        .client
        .send(
            "POST",
            &format!("/api/v1/items/{cloud_id}/video-playback"),
            &[("Authorization", &kid_bearer)],
            Some(&json!({
                "mode": "auto",
                "client": { "containers": ["mp4"], "videoCodecs": ["h264"], "audioCodecs": ["aac"], "hls": "none" }
            })),
        )
        .await?;
    assert_eq!(native_strm_resp.status, StatusCode::OK);
    let native_strm = native_strm_resp.json()?;
    assert_eq!(
        native_strm["directUrl"],
        "https://cdn.example.com/media/cloud-film.mp4?token=ok"
    );
    let stream_ticket_url = native_strm["url"].as_str().expect("stream ticket url");
    let native_redirect = srv.client.send("GET", stream_ticket_url, &[], None).await?;
    assert_eq!(native_redirect.status, StatusCode::FOUND);
    assert_eq!(
        native_redirect.header(header::LOCATION.as_str()),
        Some("https://cdn.example.com/media/cloud-film.mp4?token=ok")
    );

    for bad_id in [&bad_meta_id, &bad_file_id] {
        let ssrf_resp = srv
            .client
            .send(
                "POST",
                &format!("/api/v1/items/{bad_id}/video-playback"),
                &[("Authorization", &kid_bearer)],
                Some(&json!({
                    "mode": "auto",
                    "client": { "containers": ["mp4"], "videoCodecs": ["h264"], "audioCodecs": ["aac"], "hls": "none" }
                })),
            )
            .await?;
        assert_eq!(ssrf_resp.status, StatusCode::BAD_REQUEST);

        let jf_ssrf = srv
            .client
            .send(
                "GET",
                &format!("/Videos/{bad_id}/stream?api_key={api_key_secret}"),
                &[],
                None,
            )
            .await?;
        assert_eq!(jf_ssrf.status, StatusCode::BAD_REQUEST);
    }

    // 9. Jellyfin Playback Progress (ticks -> ms), Resume, Played & Favorite toggles
    let prog_resp = srv
        .client
        .send(
            "POST",
            "/Sessions/Playing/Progress",
            &[("X-Emby-Token", &api_key_secret)],
            Some(&json!({
                "ItemId": dune_id,
                "PositionTicks": 1_250_000_000_i64
            })),
        )
        .await?;
    assert_eq!(prog_resp.status, StatusCode::NO_CONTENT);

    let resume_resp = srv
        .client
        .send(
            "GET",
            &format!("/Users/{kid_id}/Items/Resume"),
            &[("X-Emby-Token", &api_key_secret)],
            None,
        )
        .await?;
    assert_eq!(resume_resp.status, StatusCode::OK);
    let resume = resume_resp.json()?;
    assert_eq!(resume["TotalRecordCount"], 1);
    assert_eq!(resume["Items"][0]["Id"], dune_id);
    assert_eq!(
        resume["Items"][0]["UserData"]["PlaybackPositionTicks"],
        1_250_000_000_i64
    );

    // Mark favorite & verify IsFavorite filter
    let fav_resp = srv
        .client
        .send(
            "POST",
            &format!("/Users/{kid_id}/FavoriteItems/{dune_id}"),
            &[("X-Emby-Token", &api_key_secret)],
            None,
        )
        .await?;
    assert_eq!(fav_resp.status, StatusCode::OK);
    assert_eq!(fav_resp.json()?["IsFavorite"], true);

    let fav_list_resp = srv
        .client
        .send(
            "GET",
            &format!("/Users/{kid_id}/Items?IsFavorite=true"),
            &[("X-Emby-Token", &api_key_secret)],
            None,
        )
        .await?;
    assert_eq!(fav_list_resp.status, StatusCode::OK);
    let fav_list = fav_list_resp.json()?;
    assert_eq!(fav_list["TotalRecordCount"], 1);
    assert_eq!(fav_list["Items"][0]["Id"], dune_id);

    // Mark played & unplayed
    let played_resp = srv
        .client
        .send(
            "POST",
            &format!("/Users/{kid_id}/PlayedItems/{dune_id}"),
            &[("X-Emby-Token", &api_key_secret)],
            None,
        )
        .await?;
    assert_eq!(played_resp.status, StatusCode::OK);
    assert_eq!(played_resp.json()?["Played"], true);

    let unplayed_resp = srv
        .client
        .send(
            "DELETE",
            &format!("/Users/{kid_id}/PlayedItems/{dune_id}"),
            &[("X-Emby-Token", &api_key_secret)],
            None,
        )
        .await?;
    assert_eq!(unplayed_resp.status, StatusCode::OK);
    assert_eq!(unplayed_resp.json()?["Played"], false);

    // 10. /Library/Refresh enforces Admin only
    let kid_refresh = srv
        .client
        .send(
            "POST",
            "/Library/Refresh",
            &[("X-Emby-Token", &api_key_secret)],
            None,
        )
        .await?;
    assert_eq!(kid_refresh.status, StatusCode::FORBIDDEN);

    let admin_refresh = srv
        .client
        .send(
            "POST",
            "/Library/Refresh",
            &[("X-Emby-Token", &admin_token)],
            None,
        )
        .await?;
    assert_eq!(admin_refresh.status, StatusCode::NO_CONTENT);

    // 11. Native `/api/v1/items/{id}/segments` CRUD & `/api/v1/items/{id}/trickplay` graceful fallback
    let empty_segs = srv
        .client
        .send(
            "GET",
            &format!("/api/v1/items/{dune_id}/segments"),
            &[("Authorization", &kid_bearer)],
            None,
        )
        .await?;
    assert_eq!(empty_segs.status, StatusCode::OK);
    assert_eq!(
        empty_segs.json()?["segments"].as_array().map(Vec::len),
        Some(0)
    );

    let saved_segs = srv
        .client
        .send(
            "PUT",
            &format!("/api/v1/items/{dune_id}/segments"),
            &[("Authorization", &kid_bearer)],
            Some(&json!({
                "segments": [
                    { "kind": "intro", "startMs": 10000, "endMs": 95000 },
                    { "kind": "credits", "startMs": 8800000, "endMs": 9100000, "source": "manual" }
                ]
            })),
        )
        .await?;
    assert_eq!(saved_segs.status, StatusCode::OK);
    let saved_segs_json = saved_segs.json()?;
    assert_eq!(
        saved_segs_json["segments"].as_array().map(Vec::len),
        Some(2)
    );
    assert_eq!(saved_segs_json["segments"][0]["kind"], "intro");
    assert_eq!(saved_segs_json["segments"][1]["kind"], "credits");

    // Trickplay gracefully returns 404 for `.strm` / missing ffmpeg without panicking
    let strm_trickplay = srv
        .client
        .send(
            "GET",
            &format!("/api/v1/items/{cloud_id}/trickplay?atMs=30000"),
            &[("Authorization", &kid_bearer)],
            None,
        )
        .await?;
    assert_eq!(strm_trickplay.status, StatusCode::NOT_FOUND);

    // 12. Revoking client API key immediately invalidates Jellyfin access
    let revoke_resp = srv
        .client
        .send(
            "DELETE",
            &format!("/api/v1/me/api-keys/{api_key_id}"),
            &[("Authorization", &kid_bearer)],
            None,
        )
        .await?;
    assert_eq!(revoke_resp.status, StatusCode::NO_CONTENT);

    let revoked_call = srv
        .client
        .send(
            "GET",
            "/Users/Me",
            &[("X-Emby-Token", &api_key_secret)],
            None,
        )
        .await?;
    assert_eq!(revoked_call.status, StatusCode::UNAUTHORIZED);

    let _: LibraryId = movies_lib.id;
    Ok(())
}

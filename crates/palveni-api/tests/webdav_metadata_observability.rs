use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use axum::http::StatusCode;
use palveni_api::{AppState, LinkPairings, LoginLimit, ServerInfo, router};
use palveni_core::accounts::UserPolicy;
use palveni_core::{FeatureRegistry, ItemId, ItemKind, LibraryKind, PerfTier, Role, ServerId};
use palveni_db::items::ItemUpsert;
use palveni_db::libraries::NewLibrary;
use palveni_db::video::VideoMetadata;
use palveni_db::{Db, DbOptions};
use palveni_jobs::JobService;
use palveni_link::NodeIdentity;
use palveni_media::{HardwareService, ProbeOptions};
use palveni_plugin_host::{PluginCatalog, skin_store::SkinStore};
use serde_json::{Value, json};
use tokio::io::{AsyncReadExt, AsyncWriteExt, DuplexStream};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

fn encode_base64(input: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let b0 = chunk[0];
        let b1 = *chunk.get(1).unwrap_or(&0);
        let b2 = *chunk.get(2).unwrap_or(&0);
        out.push(char::from(TABLE[usize::from(b0 >> 2)]));
        out.push(char::from(
            TABLE[usize::from(((b0 & 0x03) << 4) | (b1 >> 4))],
        ));
        if chunk.len() > 1 {
            out.push(char::from(
                TABLE[usize::from(((b1 & 0x0f) << 2) | (b2 >> 6))],
            ));
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(char::from(TABLE[usize::from(b2 & 0x3f)]));
        } else {
            out.push('=');
        }
    }
    out
}

fn basic_auth(user: &str, pass: &str) -> String {
    format!(
        "Basic {}",
        encode_base64(format!("{user}:{pass}").as_bytes())
    )
}

struct DuplexListener {
    rx: mpsc::Receiver<DuplexStream>,
    addr: SocketAddr,
}

impl axum::serve::Listener for DuplexListener {
    type Io = DuplexStream;
    type Addr = SocketAddr;

    async fn accept(&mut self) -> (Self::Io, Self::Addr) {
        match self.rx.recv().await {
            Some(io) => (io, self.addr),
            None => std::future::pending().await,
        }
    }

    fn local_addr(&self) -> std::io::Result<Self::Addr> {
        Ok(self.addr)
    }
}

struct HttpReply {
    status: StatusCode,
    headers: HashMap<String, String>,
    body: Vec<u8>,
}

impl HttpReply {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .get(&name.to_ascii_lowercase())
            .map(String::as_str)
    }

    fn text(&self) -> String {
        String::from_utf8(self.body.clone()).expect("utf8 body")
    }

    fn json(&self) -> Value {
        serde_json::from_slice(&self.body).expect("json body")
    }
}

fn decode_chunked(mut data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    while let Some(pos) = data.windows(2).position(|w| w == b"\r\n") {
        let size_hex = std::str::from_utf8(&data[..pos]).unwrap_or("0").trim();
        let size_hex = size_hex.split(';').next().unwrap_or("0").trim();
        let Ok(len) = usize::from_str_radix(size_hex, 16) else {
            break;
        };
        data = &data[pos + 2..];
        if len == 0 {
            break;
        }
        if data.len() < len {
            out.extend_from_slice(data);
            break;
        }
        out.extend_from_slice(&data[..len]);
        data = &data[len..];
        if data.starts_with(b"\r\n") {
            data = &data[2..];
        }
    }
    out
}

struct TestServer {
    tx: mpsc::Sender<DuplexStream>,
    db: Db,
    admin_token: String,
    member_token: String,
    movies_lib_id: palveni_core::LibraryId,
    secret_lib_id: palveni_core::LibraryId,
    movie_item_id: ItemId,
    movie_work_key: String,
    temp_dir: PathBuf,
    shutdown: CancellationToken,
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.shutdown.cancel();
        let _ = std::fs::remove_dir_all(&self.temp_dir);
    }
}

impl TestServer {
    async fn request(
        &self,
        method: &str,
        path: &str,
        extra_headers: &[(&str, &str)],
        body: Option<&[u8]>,
    ) -> HttpReply {
        let (mut client, server) = tokio::io::duplex(256 * 1024);
        self.tx.send(server).await.expect("send duplex stream");

        let mut wire =
            format!("{method} {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n");
        for (k, v) in extra_headers {
            wire.push_str(k);
            wire.push_str(": ");
            wire.push_str(v);
            wire.push_str("\r\n");
        }
        if let Some(bytes) = body {
            wire.push_str(&format!("Content-Length: {}\r\n", bytes.len()));
        }
        wire.push_str("\r\n");

        client.write_all(wire.as_bytes()).await.expect("write head");
        if let Some(bytes) = body {
            client.write_all(bytes).await.expect("write body");
        }
        client.flush().await.expect("flush write");

        let mut raw = Vec::new();
        client.read_to_end(&mut raw).await.expect("read response");

        let split = raw
            .windows(4)
            .position(|w| w == b"\r\n\r\n")
            .expect("http header separator");
        let head = std::str::from_utf8(&raw[..split]).expect("utf8 head");
        let mut lines = head.split("\r\n");
        let status_line = lines.next().expect("status line");
        let code: u16 = status_line
            .split_whitespace()
            .nth(1)
            .expect("status code")
            .parse()
            .expect("u16 status");
        let status = StatusCode::from_u16(code).expect("valid status");

        let mut headers = HashMap::new();
        for line in lines {
            if let Some((k, v)) = line.split_once(':') {
                headers.insert(k.trim().to_ascii_lowercase(), v.trim().to_owned());
            }
        }

        let raw_body = &raw[split + 4..];
        let body = if headers
            .get("transfer-encoding")
            .is_some_and(|v| v.to_ascii_lowercase().contains("chunked"))
        {
            decode_chunked(raw_body)
        } else {
            raw_body.to_vec()
        };

        HttpReply {
            status,
            headers,
            body,
        }
    }

    async fn json_request(
        &self,
        method: &str,
        path: &str,
        token: Option<&str>,
        payload: Option<Value>,
    ) -> HttpReply {
        let auth = token.map(|t| format!("Bearer {t}"));
        let bytes = payload.map(|v| serde_json::to_vec(&v).expect("encode json"));
        let mut headers: Vec<(&str, &str)> = Vec::new();
        if let Some(ref a) = auth {
            headers.push(("Authorization", a.as_str()));
        }
        if bytes.is_some() {
            headers.push(("Content-Type", "application/json"));
        }
        self.request(method, path, &headers, bytes.as_deref()).await
    }
}

async fn spawn_test_server() -> TestServer {
    let temp_dir = std::env::temp_dir().join(format!("palveni-wmo-test-{}", ServerId::generate()));
    std::fs::create_dir_all(&temp_dir).expect("create tempdir");
    let root = temp_dir.as_path();
    let movies_dir = root.join("movies");
    let secret_dir = root.join("secret");
    std::fs::create_dir_all(movies_dir.join("Sci & Fi")).expect("create movies dir");
    std::fs::create_dir_all(&secret_dir).expect("create secret dir");

    let sample_file = movies_dir.join("Sci & Fi").join("dune.mkv");
    std::fs::write(&sample_file, b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ").expect("write sample");
    let secret_file = secret_dir.join("private.txt");
    std::fs::write(&secret_file, b"TOP_SECRET_DATA").expect("write secret");

    let db = Db::open(&DbOptions {
        main_url: format!("sqlite://{}/main.db?mode=rwc", root.display()),
        ops_url: format!("sqlite://{}/ops.db?mode=rwc", root.display()),
        max_connections: Some(4),
        min_connections: Some(1),
    })
    .await
    .expect("open db");

    let bus = db.event_bus(32).await.expect("event bus");
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
        identity: Arc::new(NodeIdentity::generate().expect("identity")),
        link: Arc::new(LinkPairings::default()),
        login_limit: Arc::new(LoginLimit::new(32)),
        subsonic_limit: Arc::new(LoginLimit::new(32)),
        bus,
        jobs,
        hardware: Arc::new(HardwareService::new(
            db.clone(),
            root.to_path_buf(),
            ProbeOptions::default(),
        )),
        plugins: Arc::new(PluginCatalog::new()),
        settings: Arc::new(tokio::sync::RwLock::new(Default::default())),
        boot_tier: PerfTier::detect(0, 1, false),
        skins: Arc::new(SkinStore::open(&root.join("skins")).expect("skins")),
        skin_writes: Arc::new(tokio::sync::Semaphore::new(1)),
        features: FeatureRegistry::new(),
        server: Arc::new(ServerInfo {
            id: ServerId::generate(),
            name: "Palveni Test".into(),
            version: "0.1.0".into(),
        }),
        shutdown: shutdown.clone(),
    };

    let peer = SocketAddr::from(([127, 0, 0, 1], 8080));
    let app = router(state).layer(axum::Extension(axum::extract::ConnectInfo(peer)));
    let (tx, rx) = mpsc::channel(64);
    let listener = DuplexListener { rx, addr: peer };
    let shutdown_server = shutdown.clone();
    tokio::spawn(async move {
        let _ = axum::serve(listener, app)
            .with_graceful_shutdown(async move {
                shutdown_server.cancelled().await;
            })
            .await;
    });

    let mut srv = TestServer {
        tx,
        db: db.clone(),
        admin_token: String::new(),
        member_token: String::new(),
        movies_lib_id: palveni_core::LibraryId::generate(),
        secret_lib_id: palveni_core::LibraryId::generate(),
        movie_item_id: ItemId::generate(),
        movie_work_key: String::new(),
        temp_dir,
        shutdown,
    };

    // 1. Setup admin & login via real HTTP
    let setup_resp = srv
        .json_request(
            "POST",
            "/api/v1/setup",
            None,
            Some(json!({"username": "admin", "password": "adminpassword123"})),
        )
        .await;
    assert_eq!(setup_resp.status, StatusCode::CREATED);

    let login_resp = srv
        .json_request(
            "POST",
            "/api/v1/auth/login",
            None,
            Some(json!({
                "username": "admin",
                "password": "adminpassword123",
                "deviceName": "Test Runner"
            })),
        )
        .await;
    assert_eq!(login_resp.status, StatusCode::OK);
    srv.admin_token = login_resp.json()["accessToken"]
        .as_str()
        .expect("admin token")
        .to_owned();

    // 2. Create two libraries: Movies & Secret
    let canonical_movies = std::fs::canonicalize(&movies_dir).expect("canonical movies");
    let canonical_secret = std::fs::canonicalize(&secret_dir).expect("canonical secret");

    let movies = db
        .libraries()
        .create(NewLibrary {
            name: "Movies".into(),
            kind: LibraryKind::Video,
            paths: vec![canonical_movies.display().to_string()],
        })
        .await
        .expect("create movies lib");
    let secret = db
        .libraries()
        .create(NewLibrary {
            name: "Secret".into(),
            kind: LibraryKind::Book,
            paths: vec![canonical_secret.display().to_string()],
        })
        .await
        .expect("create secret lib");
    srv.movies_lib_id = movies.id;
    srv.secret_lib_id = secret.id;

    // 3. Index items + video metadata
    let movie_item_id = ItemId::generate();
    let canonical_sample = canonical_movies.join("Sci & Fi").join("dune.mkv");
    db.items()
        .apply_scan_batch(
            movies.id,
            &[ItemUpsert {
                music: None,
                id: movie_item_id,
                parent_id: None,
                kind: ItemKind::Video,
                title: "Raw Dune File".into(),
                sort_title: "raw dune file".into(),
                search_text: "raw dune file".into(),
                path: canonical_sample.display().to_string(),
                size_bytes: Some(36),
                modified_at: Some(1_730_000_000_000),
            }],
            &[],
        )
        .await
        .expect("scan movie");

    let work_group = "movies/dune";
    let work_key = {
        use ring::digest::{SHA256, digest};
        hex::encode(digest(&SHA256, work_group.as_bytes()))
    };
    db.video()
        .save_batch(
            movies.id,
            vec![VideoMetadata {
                item_id: movie_item_id,
                stamp: "s1".into(),
                work_group: work_group.into(),
                work_title: "Raw Dune File".into(),
                year: Some(1999),
                work_overview: Some("Initial text".into()),
                work_poster: None,
                work_original_title: None,
                work_tagline: None,
                work_rating: Some(50),
                work_official_rating: None,
                work_premiered: None,
                work_studios: Vec::new(),
                work_backdrop: None,
                season: None,
                season_poster: None,
                episode: None,
                title: "Raw Dune File".into(),
                overview: Some("Initial text".into()),
                episode_poster: None,
                runtime_ms: Some(9_300_000),
                rating: Some(50),
                premiered: None,
                thumb: None,
                genres: vec!["Drama".into()],
                people: Vec::new(),
            }],
        )
        .await
        .expect("save video metadata");
    srv.movie_item_id = movie_item_id;
    srv.movie_work_key = work_key;

    let secret_item_id = ItemId::generate();
    let canonical_secret_file = canonical_secret.join("private.txt");
    db.items()
        .apply_scan_batch(
            secret.id,
            &[ItemUpsert {
                music: None,
                id: secret_item_id,
                parent_id: None,
                kind: ItemKind::Book,
                title: "Private Note".into(),
                sort_title: "private note".into(),
                search_text: "private note".into(),
                path: canonical_secret_file.display().to_string(),
                size_bytes: Some(15),
                modified_at: Some(1_730_000_000_000),
            }],
            &[],
        )
        .await
        .expect("scan secret");

    // 4. Create restricted member `alice` with access ONLY to `Movies`
    let create_user_resp = srv
        .json_request(
            "POST",
            "/api/v1/users",
            Some(&srv.admin_token),
            Some(json!({
                "username": "alice",
                "password": "alicepassword123",
                "role": Role::User,
                "policy": UserPolicy {
                    all_libraries: false,
                    allow_download: true,
                    allow_transcode: true,
                    daily_ai_limit: 20,
                },
                "allowedLibraryIds": [movies.id],
            })),
        )
        .await;
    assert_eq!(create_user_resp.status, StatusCode::CREATED);

    let alice_login = srv
        .json_request(
            "POST",
            "/api/v1/auth/login",
            None,
            Some(json!({
                "username": "alice",
                "password": "alicepassword123",
                "deviceName": "Alice Browser"
            })),
        )
        .await;
    assert_eq!(alice_login.status, StatusCode::OK);
    srv.member_token = alice_login.json()["accessToken"]
        .as_str()
        .expect("alice token")
        .to_owned();

    srv
}

#[tokio::test]
async fn webdav_protocol_auth_streaming_security_and_acl() {
    let srv = spawn_test_server().await;
    let admin_basic = basic_auth("admin", "adminpassword123");
    let alice_basic = basic_auth("alice", "alicepassword123");

    // 1. OPTIONS returns DAV: 1, Allow, MS-Author-Via
    let opt = srv.request("OPTIONS", "/webdav/", &[], None).await;
    assert_eq!(opt.status, StatusCode::OK);
    assert_eq!(opt.header("dav"), Some("1"));
    assert_eq!(opt.header("ms-author-via"), Some("DAV"));
    let allow = opt.header("allow").unwrap_or("");
    assert!(allow.contains("PROPFIND") && allow.contains("GET") && allow.contains("HEAD"));

    // 2. Missing & invalid Basic Auth -> 401 + WWW-Authenticate
    let unauth = srv.request("PROPFIND", "/webdav/", &[], None).await;
    assert_eq!(unauth.status, StatusCode::UNAUTHORIZED);
    assert!(
        unauth
            .header("www-authenticate")
            .unwrap_or("")
            .contains("Basic realm=\"Palveni WebDAV\"")
    );

    let bad_basic = basic_auth("admin", "wrong-password");
    let bad_auth = srv
        .request(
            "PROPFIND",
            "/webdav/",
            &[("Authorization", &bad_basic)],
            None,
        )
        .await;
    assert_eq!(bad_auth.status, StatusCode::UNAUTHORIZED);

    // 3. PROPFIND Depth 0 vs Depth 1 for Admin
    let depth0 = srv
        .request(
            "PROPFIND",
            "/webdav/",
            &[("Authorization", &admin_basic), ("Depth", "0")],
            None,
        )
        .await;
    assert_eq!(depth0.status, StatusCode::MULTI_STATUS);
    assert!(
        depth0
            .header("content-type")
            .unwrap_or("")
            .starts_with("application/xml")
    );
    let xml0 = depth0.text();
    assert!(xml0.contains("<d:href>/webdav/</d:href>"));
    assert!(!xml0.contains("/webdav/Movies/"));

    let depth1 = srv
        .request(
            "PROPFIND",
            "/webdav/",
            &[("Authorization", &admin_basic), ("Depth", "1")],
            None,
        )
        .await;
    assert_eq!(depth1.status, StatusCode::MULTI_STATUS);
    let xml1 = depth1.text();
    assert!(xml1.contains("/webdav/Movies/"));
    assert!(xml1.contains("/webdav/Secret/"));

    // 4. PROPFIND by library UUID & by name, verifying XML escaping for `&`
    let by_id = srv
        .request(
            "PROPFIND",
            &format!("/webdav/{}/", srv.movies_lib_id),
            &[("Authorization", &admin_basic), ("Depth", "1")],
            None,
        )
        .await;
    assert_eq!(by_id.status, StatusCode::MULTI_STATUS);
    let xml_lib = by_id.text();
    assert!(xml_lib.contains("Sci &amp; Fi"));
    assert!(xml_lib.contains("Sci%20%26%20Fi/"));

    let sub_dir = srv
        .request(
            "PROPFIND",
            "/webdav/Movies/Sci%20%26%20Fi/",
            &[("Authorization", &admin_basic), ("Depth", "1")],
            None,
        )
        .await;
    assert_eq!(sub_dir.status, StatusCode::MULTI_STATUS);
    let xml_sub = sub_dir.text();
    assert!(xml_sub.contains("dune.mkv"));
    assert!(xml_sub.contains("<d:getcontentlength>36</d:getcontentlength>"));

    // 5. GET + HEAD + Range 206 streaming
    let file_path = "/webdav/Movies/Sci%20%26%20Fi/dune.mkv";
    let full = srv
        .request("GET", file_path, &[("Authorization", &admin_basic)], None)
        .await;
    assert_eq!(full.status, StatusCode::OK);
    assert_eq!(full.header("accept-ranges"), Some("bytes"));
    assert!(full.header("etag").is_some());
    assert_eq!(full.body, b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ");

    let head = srv
        .request("HEAD", file_path, &[("Authorization", &admin_basic)], None)
        .await;
    assert_eq!(head.status, StatusCode::OK);
    assert_eq!(head.header("content-length"), Some("36"));
    assert!(head.body.is_empty());

    let ranged = srv
        .request(
            "GET",
            file_path,
            &[("Authorization", &admin_basic), ("Range", "bytes=10-19")],
            None,
        )
        .await;
    assert_eq!(ranged.status, StatusCode::PARTIAL_CONTENT);
    assert_eq!(ranged.header("content-range"), Some("bytes 10-19/36"));
    assert_eq!(ranged.body, b"ABCDEFGHIJ");

    // 6. Mutating methods -> 405 Method Not Allowed
    for verb in [
        "PUT",
        "DELETE",
        "MKCOL",
        "MOVE",
        "COPY",
        "PROPPATCH",
        "LOCK",
        "UNLOCK",
    ] {
        let resp = srv
            .request(verb, file_path, &[("Authorization", &admin_basic)], None)
            .await;
        assert_eq!(resp.status, StatusCode::METHOD_NOT_ALLOWED, "verb={verb}");
    }

    // 7. Path traversal rejection (encoded and raw)
    let traversal = srv
        .request(
            "GET",
            "/webdav/Movies/%2e%2e/Secret/private.txt",
            &[("Authorization", &admin_basic)],
            None,
        )
        .await;
    assert_eq!(traversal.status, StatusCode::FORBIDDEN);

    let traversal_raw = srv
        .request(
            "GET",
            "/webdav/Movies/../Secret/private.txt",
            &[("Authorization", &admin_basic)],
            None,
        )
        .await;
    assert_eq!(traversal_raw.status, StatusCode::FORBIDDEN);

    // 8. Per-library ACL enforcement for restricted member `alice`
    let alice_root = srv
        .request(
            "PROPFIND",
            "/webdav/",
            &[("Authorization", &alice_basic), ("Depth", "1")],
            None,
        )
        .await;
    assert_eq!(alice_root.status, StatusCode::MULTI_STATUS);
    let alice_xml = alice_root.text();
    assert!(alice_xml.contains("/webdav/Movies/"));
    assert!(!alice_xml.contains("/webdav/Secret/"));

    let alice_secret = srv
        .request(
            "GET",
            "/webdav/Secret/private.txt",
            &[("Authorization", &alice_basic)],
            None,
        )
        .await;
    assert_eq!(alice_secret.status, StatusCode::NOT_FOUND);

    let alice_secret_by_id = srv
        .request(
            "PROPFIND",
            &format!("/webdav/{}/", srv.secret_lib_id),
            &[("Authorization", &alice_basic)],
            None,
        )
        .await;
    assert_eq!(alice_secret_by_id.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn metadata_config_masking_override_sync_and_tmdb_validation() {
    let srv = spawn_test_server().await;

    // 1. Non-admin forbidden from metadata config
    let forbidden = srv
        .json_request(
            "GET",
            "/api/v1/metadata/config",
            Some(&srv.member_token),
            None,
        )
        .await;
    assert_eq!(forbidden.status, StatusCode::FORBIDDEN);

    // 2. Default config shows unconfigured secrets
    let initial = srv
        .json_request(
            "GET",
            "/api/v1/metadata/config",
            Some(&srv.admin_token),
            None,
        )
        .await;
    assert_eq!(initial.status, StatusCode::OK);
    let initial_json = initial.json();
    assert_eq!(initial_json["tmdbConfigured"], false);
    assert_eq!(initial_json["bangumiConfigured"], false);
    assert_eq!(initial_json["preferredLanguage"], "zh-CN");
    assert_eq!(initial_json["autoFetchPosters"], true);

    // 3. Unconfigured TMDB search returns structured 422 error
    let unconfigured = srv
        .json_request(
            "POST",
            "/api/v1/metadata/search",
            Some(&srv.admin_token),
            Some(json!({
                "provider": "tmdb",
                "query": "Dune",
                "mediaType": "movie"
            })),
        )
        .await;
    assert_eq!(unconfigured.status, StatusCode::UNPROCESSABLE_ENTITY);
    let err_body = unconfigured.json();
    assert_eq!(err_body["error"]["code"], "validation_failed");
    assert!(
        err_body["error"]["message"]
            .as_str()
            .unwrap_or("")
            .contains("TMDB API Key")
    );

    // 4. Save secrets -> masked response, never echoes secret, empty preserves, clear removes
    let saved_cfg_resp = srv
        .json_request(
            "PUT",
            "/api/v1/metadata/config",
            Some(&srv.admin_token),
            Some(json!({
                "tmdbApiKey": "super-secret-tmdb-key-999",
                "bangumiToken": "super-secret-bgm-token-888",
                "preferredLanguage": "en-US",
                "autoFetchPosters": false
            })),
        )
        .await;
    assert_eq!(saved_cfg_resp.status, StatusCode::OK);
    let saved_cfg_text = saved_cfg_resp.text();
    assert!(!saved_cfg_text.contains("super-secret-tmdb-key-999"));
    assert!(!saved_cfg_text.contains("super-secret-bgm-token-888"));
    let saved_cfg = saved_cfg_resp.json();
    assert_eq!(saved_cfg["tmdbConfigured"], true);
    assert_eq!(saved_cfg["bangumiConfigured"], true);
    assert_eq!(saved_cfg["preferredLanguage"], "en-US");
    assert_eq!(saved_cfg["autoFetchPosters"], false);

    // Updating only language preserves existing secrets
    let preserved = srv
        .json_request(
            "PUT",
            "/api/v1/metadata/config",
            Some(&srv.admin_token),
            Some(json!({
                "tmdbApiKey": "",
                "preferredLanguage": "zh-CN"
            })),
        )
        .await
        .json();
    assert_eq!(preserved["tmdbConfigured"], true);
    assert_eq!(preserved["bangumiConfigured"], true);
    assert_eq!(preserved["preferredLanguage"], "zh-CN");

    // Clear TMDB key explicitly
    let cleared = srv
        .json_request(
            "PUT",
            "/api/v1/metadata/config",
            Some(&srv.admin_token),
            Some(json!({ "clearTmdbKey": true })),
        )
        .await
        .json();
    assert_eq!(cleared["tmdbConfigured"], false);
    assert_eq!(cleared["bangumiConfigured"], true);

    // 5. Override SSRF/URL validation
    let override_path = format!("/api/v1/items/{}/metadata-override", srv.movie_item_id);
    let bad_url = srv
        .json_request(
            "PUT",
            &override_path,
            Some(&srv.admin_token),
            Some(json!({
                "title": "Dune",
                "posterUrl": "https://127.0.0.1/evil.jpg"
            })),
        )
        .await;
    assert_eq!(bad_url.status, StatusCode::BAD_REQUEST);

    // 6. Save valid override -> syncs video_metadata + audit_log + GET returns it + DELETE removes it
    let put_override = srv
        .json_request(
            "PUT",
            &override_path,
            Some(&srv.admin_token),
            Some(json!({
                "provider": "tmdb",
                "externalId": "438631",
                "title": "沙丘",
                "originalTitle": "Dune",
                "year": 2021,
                "overview": "保罗·厄崔迪踏上厄拉科斯星球的命运旅程。",
                "rating": 8.4,
                "genres": ["科幻", "冒险"],
                "posterUrl": "https://image.tmdb.org/t/p/w500/dune.jpg",
                "backdropUrl": "https://image.tmdb.org/t/p/w1280/dune-bg.jpg"
            })),
        )
        .await;
    assert_eq!(put_override.status, StatusCode::OK);
    let put_json = put_override.json();
    assert_eq!(put_json["title"], "沙丘");
    assert_eq!(put_json["provider"], "tmdb");

    // Verify immediate sync into video work detail API
    let work_detail = srv
        .json_request(
            "GET",
            &format!(
                "/api/v1/libraries/{}/videos/works/{}",
                srv.movies_lib_id, srv.movie_work_key
            ),
            Some(&srv.member_token),
            None,
        )
        .await;
    assert_eq!(work_detail.status, StatusCode::OK);
    let work_json = work_detail.json();
    assert_eq!(work_json["work"]["title"], "沙丘");
    assert_eq!(work_json["work"]["originalTitle"], "Dune");
    assert_eq!(work_json["work"]["year"], 2021);
    assert_eq!(work_json["work"]["communityRating"], 8.4);

    // Authenticated member can read override
    let got_override = srv
        .json_request("GET", &override_path, Some(&srv.member_token), None)
        .await
        .json();
    assert_eq!(got_override["override"]["title"], "沙丘");

    // Audit log recorded metadata.override_saved
    let audits = srv.db.audit().recent(20).await.expect("recent audit");
    assert!(audits.iter().any(|a| a.action == "metadata.override_saved"));

    // Delete override -> 204, subsequent GET -> override: null
    let del_resp = srv
        .json_request("DELETE", &override_path, Some(&srv.admin_token), None)
        .await;
    assert_eq!(del_resp.status, StatusCode::NO_CONTENT);

    let after_del = srv
        .json_request("GET", &override_path, Some(&srv.admin_token), None)
        .await
        .json();
    assert!(after_del["override"].is_null());
}

#[tokio::test]
async fn observability_prometheus_doctor_and_logs_filtering() {
    let srv = spawn_test_server().await;

    // 1. Non-admin forbidden
    let forbidden = srv
        .json_request(
            "GET",
            "/api/v1/system/prometheus",
            Some(&srv.member_token),
            None,
        )
        .await;
    assert_eq!(forbidden.status, StatusCode::FORBIDDEN);

    // 2. Prometheus exposition format
    let prom_resp = srv
        .json_request(
            "GET",
            "/api/v1/system/prometheus",
            Some(&srv.admin_token),
            None,
        )
        .await;
    assert_eq!(prom_resp.status, StatusCode::OK);
    assert_eq!(
        prom_resp.header("content-type"),
        Some("text/plain; version=0.0.4; charset=utf-8")
    );
    let prom_text = prom_resp.text();
    for metric in [
        "palveni_uptime_seconds",
        "palveni_libraries_total 2",
        "palveni_items_total 2",
        "palveni_users_total 2",
        "palveni_active_sessions",
        "palveni_cpu_load_ratio",
        "palveni_memory_used_bytes",
        "palveni_memory_total_bytes",
        "palveni_mounts_total",
    ] {
        assert!(
            prom_text.contains(metric),
            "missing metric `{metric}` in:\n{prom_text}"
        );
    }

    // 3. System Doctor structured checks
    let doctor_resp = srv
        .json_request("GET", "/api/v1/system/doctor", Some(&srv.admin_token), None)
        .await;
    assert_eq!(doctor_resp.status, StatusCode::OK);
    let doctor_json = doctor_resp.json();
    let overall = doctor_json["overallStatus"]
        .as_str()
        .expect("overallStatus");
    assert!(matches!(overall, "healthy" | "degraded" | "unhealthy"));
    let checks = doctor_json["checks"].as_array().expect("checks array");
    let ids: Vec<&str> = checks.iter().filter_map(|c| c["id"].as_str()).collect();
    for expected in ["database", "ffmpeg", "storage", "libraries", "protocols"] {
        assert!(ids.contains(&expected), "missing doctor check `{expected}`");
    }

    // 4. Trigger a warn log via invalid WebDAV auth, then query /system/logs with filtering
    let _ = srv.request("GET", "/webdav/", &[], None).await;

    let all_logs = srv
        .json_request(
            "GET",
            "/api/v1/system/logs?limit=50",
            Some(&srv.admin_token),
            None,
        )
        .await
        .json();
    let entries = all_logs["entries"].as_array().expect("entries");
    assert!(!entries.is_empty());
    assert!(entries.iter().any(|e| e["source"] == "runtime"));
    assert!(entries.iter().any(|e| e["source"] == "audit"));

    let warn_logs = srv
        .json_request(
            "GET",
            "/api/v1/system/logs?level=warn&limit=20",
            Some(&srv.admin_token),
            None,
        )
        .await
        .json();
    let warn_entries = warn_logs["entries"].as_array().expect("warn entries");
    assert!(!warn_entries.is_empty());
    assert!(warn_entries.iter().all(|e| e["level"] == "warn"));

    let bad_level = srv
        .json_request(
            "GET",
            "/api/v1/system/logs?level=trace",
            Some(&srv.admin_token),
            None,
        )
        .await;
    assert_eq!(bad_level.status, StatusCode::BAD_REQUEST);
}

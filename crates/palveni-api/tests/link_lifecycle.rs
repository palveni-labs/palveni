use std::{
    error::Error,
    fs,
    net::{SocketAddr, TcpListener, UdpSocket},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

use palveni_core::DeviceId;
use palveni_link::{NodeId, NodeIdentity, Transport, quic::QuicTransport};
use reqwest::{Client, Method, RequestBuilder, StatusCode};
use serde_json::{Value, json};
use tokio::time::{sleep, timeout};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

struct Instance {
    data: PathBuf,
    binary: PathBuf,
    http: SocketAddr,
    udp: SocketAddr,
    client: Client,
    child: Option<Child>,
}

impl Instance {
    fn new(binary: PathBuf) -> Result<Self> {
        let http = TcpListener::bind(("127.0.0.1", 0))?.local_addr()?;
        let udp = UdpSocket::bind(("127.0.0.1", 0))?.local_addr()?;
        let data = std::env::temp_dir().join(format!("palveni-link-test-{}", DeviceId::generate()));
        fs::create_dir(&data)?;
        Ok(Self {
            data,
            binary,
            http,
            udp,
            client: Client::builder()
                .no_proxy()
                .timeout(Duration::from_secs(8))
                .build()?,
            child: None,
        })
    }

    fn stop(&mut self) -> Result<()> {
        if let Some(mut child) = self.child.take() {
            if child.try_wait()?.is_none() {
                child.kill()?;
            }
            child.wait()?;
        }
        Ok(())
    }

    async fn start(&mut self, link: bool) -> Result<()> {
        let config = self.data.join("palveni.toml");
        fs::write(&config, "")?;
        let mut command = Command::new(&self.binary);
        if link {
            command.args(["--link-bind", &self.udp.to_string()]);
        }
        for (key, _) in std::env::vars_os() {
            if key.to_string_lossy().starts_with("PALVENI_") {
                command.env_remove(key);
            }
        }
        self.child = Some(
            command
                .env("PATH", &self.data)
                .arg("--config")
                .arg(config)
                .arg("--data-dir")
                .arg(self.data.join("server"))
                .args([
                    "--listen",
                    "127.0.0.1",
                    "--port",
                    &self.http.port().to_string(),
                    "serve",
                ])
                .stdout(Stdio::null())
                .spawn()?,
        );
        let deadline = Instant::now() + Duration::from_secs(20);
        loop {
            if let Some(status) = self.child.as_mut().expect("server child").try_wait()? {
                return Err(format!("server exited with {status}").into());
            }
            if self
                .api(Method::GET, "/system/health")
                .send()
                .await
                .is_ok_and(|r| r.status().is_success())
            {
                return Ok(());
            }
            if Instant::now() >= deadline {
                return Err("server startup timed out".into());
            }
            sleep(Duration::from_millis(40)).await;
        }
    }

    fn api(&self, method: Method, path: &str) -> RequestBuilder {
        self.client
            .request(method, format!("http://{}/api/v1{path}", self.http))
    }

    async fn login(&self, password: &str, name: &str) -> Result<String> {
        let login = self
            .api(Method::POST, "/auth/login")
            .json(&json!({"username":"reader", "password":password, "deviceName":name}))
            .send()
            .await?
            .error_for_status()?
            .json::<Value>()
            .await?;
        Ok(login["accessToken"]
            .as_str()
            .ok_or("missing token")?
            .to_owned())
    }

    async fn enroll(&self, identity: &NodeIdentity, token: &str) -> Result<(String, NodeId)> {
        let challenge = self
            .api(Method::POST, "/link/pairing/challenge")
            .bearer_auth(token)
            .send()
            .await?
            .error_for_status()?
            .json::<Value>()
            .await?;
        let message = challenge["challenge"].as_str().ok_or("missing challenge")?;
        let proof = json!({
            "nodeId": identity.node_id().to_string(),
            "signature": hex::encode(identity.sign(message.as_bytes())),
        });
        let pair = || {
            self.api(Method::PUT, "/link/pairing")
                .bearer_auth(token)
                .json(&proof)
        };
        let paired = pair()
            .send()
            .await?
            .error_for_status()?
            .json::<Value>()
            .await?;
        assert_eq!(paired["serverNodeId"], challenge["serverNodeId"]);
        assert_eq!(pair().send().await?.status(), StatusCode::FORBIDDEN);
        Ok((
            paired["deviceId"].as_str().ok_or("missing device")?.into(),
            paired["serverNodeId"]
                .as_str()
                .ok_or("missing server key")?
                .parse()?,
        ))
    }

    async fn wait_online(&self, observer: &str, id: &str) -> Result<()> {
        timeout(Duration::from_secs(4), async {
            loop {
                let list = self
                    .api(Method::GET, "/link/devices")
                    .bearer_auth(observer)
                    .send()
                    .await?
                    .error_for_status()?
                    .json::<Value>()
                    .await?;
                if list["devices"]
                    .as_array()
                    .ok_or("missing devices")?
                    .iter()
                    .any(|d| d["id"] == id && d["connected"] == true)
                {
                    return Ok::<_, Box<dyn Error>>(());
                }
                sleep(Duration::from_millis(20)).await;
            }
        })
        .await?
    }
}

impl Drop for Instance {
    fn drop(&mut self) {
        let _ = self.stop();
        let _ = fs::remove_dir_all(&self.data);
    }
}

async fn peer(identity: &NodeIdentity, server: NodeId, udp: SocketAddr) -> Result<QuicTransport> {
    let client = QuicTransport::bind(identity, Some(([127, 0, 0, 1], 0).into()), vec![]).await?;
    client.register_direct(server, udp)?;
    Ok(client)
}

async fn assert_refused(client: &QuicTransport, server: NodeId) -> Result<()> {
    // iroh can complete its TLS handshake before application admission rejects the peer.
    let reason = match timeout(Duration::from_secs(4), client.connect(server)).await? {
        Ok(connection) => timeout(Duration::from_secs(4), connection.closed())
            .await?
            .to_string(),
        Err(error) => error.to_string(),
    };
    assert!(reason.contains("not paired"), "{reason}");
    Ok(())
}

#[tokio::test]
#[ignore = "requires a palveni-server binary built with --features link and loopback TCP/UDP"]
async fn pairing_survives_server_restart_and_revocation_disconnects_only_its_device() -> Result<()>
{
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let binary = std::env::var_os("PALVENI_BIN")
        .map_or_else(|| root.join("target/debug/palveni"), PathBuf::from);
    let mut server = Instance::new(binary)?;
    server.start(false).await?;
    let info = server
        .api(Method::GET, "/system/info")
        .send()
        .await?
        .json::<Value>()
        .await?;
    assert!(
        !info["features"]
            .as_array()
            .ok_or("missing features")?
            .contains(&json!("link"))
    );
    assert_eq!(
        server
            .api(Method::GET, "/link/devices")
            .send()
            .await?
            .status(),
        StatusCode::NOT_FOUND
    );
    server.stop()?;
    server.start(true).await?;
    let password = DeviceId::generate().to_string();
    server
        .api(Method::POST, "/setup")
        .json(&json!({"username":"reader","password":password}))
        .send()
        .await?
        .error_for_status()?;
    let primary = server.login(&password, "Reader").await?;
    let observer = server.login(&password, "Desk").await?;
    let book_key = server.data.join("reader").join("identity.key");
    let server_pin = server.data.join("reader").join("server.pub");
    let desk = NodeIdentity::generate()?;
    let (desk_id, server_node) = server.enroll(&desk, &observer).await?;
    let (reader_id, pinned_server) = server
        .enroll(&NodeIdentity::load_or_generate(&book_key)?.0, &primary)
        .await?;
    assert_eq!(pinned_server, server_node);
    pinned_server.pin(&server_pin)?;
    let renewed = server.login(&password, "Reader").await?;
    let (known_reader, _) = server
        .enroll(&NodeIdentity::load(&book_key)?, &renewed)
        .await?;
    assert_eq!(known_reader, reader_id);

    let reader = peer(&NodeIdentity::load(&book_key)?, pinned_server, server.udp).await?;
    let first = reader.connect(pinned_server).await?;
    server.wait_online(&observer, &reader_id).await?;
    server.stop()?;
    drop(first);
    reader.close().await;

    server.start(true).await?;
    let pinned_server = NodeId::load_pin(&server_pin)?;
    let reader = peer(&NodeIdentity::load(&book_key)?, pinned_server, server.udp).await?;
    let desk = peer(&desk, server_node, server.udp).await?;
    let connection = reader.connect(pinned_server).await?;
    let second = reader.connect(pinned_server).await?;
    assert_ne!(connection.stable_id(), second.stable_id());
    // A client handshake may finish before admission. Wait for each server-side stream
    // handler to answer or reset the stream while keeping the connection open.
    for active in [&connection, &second] {
        let (mut send, mut receive) = active.open_bi().await?;
        send.write_all(
            b"GET /api/v1/system/health HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
        )
        .await?;
        let _ = send.finish();
        let _ = timeout(Duration::from_secs(4), receive.read_to_end(4096)).await?;
        assert!(active.close_reason().is_none());
    }
    let unaffected = desk.connect(server_node).await?;
    server.wait_online(&observer, &reader_id).await?;
    server.wait_online(&observer, &desk_id).await?;
    let started = Instant::now();
    let status = server
        .api(Method::DELETE, &format!("/link/devices/{reader_id}"))
        .bearer_auth(&observer)
        .send()
        .await?
        .status();
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (one, two) = timeout(Duration::from_secs(2), async {
        tokio::join!(connection.closed(), second.closed())
    })
    .await?;
    let elapsed = started.elapsed();
    assert!(one.to_string().contains("revoked") && two.to_string().contains("revoked"));
    assert_refused(&reader, pinned_server).await?;
    for session in [&primary, &renewed] {
        assert_eq!(
            server
                .api(Method::GET, "/auth/me")
                .bearer_auth(session)
                .send()
                .await?
                .status(),
            StatusCode::UNAUTHORIZED
        );
    }
    server.wait_online(&observer, &desk_id).await?;
    assert!(unaffected.close_reason().is_none());
    server.stop()?;
    server.start(true).await?;
    assert_refused(&reader, pinned_server).await?;
    let reconnected = desk.connect(server_node).await?;
    server.wait_online(&observer, &desk_id).await?;
    assert!(reconnected.close_reason().is_none());
    eprintln!(
        "Two live connections closed in {} ms from the revoke request",
        elapsed.as_millis()
    );
    tokio::join!(reader.close(), desk.close());
    Ok(())
}

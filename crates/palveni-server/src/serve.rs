//! `palveni serve`: wires all services together and runs the HTTP server.

use std::future::IntoFuture;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use axum::Router;
use palveni_api::{AppState, ServerInfo};
use palveni_core::config::Config;
use palveni_core::settings::RuntimeSettings;
use palveni_core::{FeatureRegistry, HostInfo, PerfTier};
use palveni_db::{Db, DbOptions};
use palveni_jobs::JobService;
use palveni_library::Scanner;
use palveni_link::NodeIdentity;
use palveni_media::{HardwareService, ProbeOptions};
use palveni_plugin_host::{PluginCatalog, skin_store::SkinStore};
use tokio_util::sync::CancellationToken;

use crate::cli::Cli;
use crate::logging;
use crate::scan_job::ScanJobHandler;

/// How long open connections and running jobs get to finish after a
/// shutdown signal before the process exits anyway.
const SHUTDOWN_GRACE: Duration = Duration::from_secs(10);
const EVENT_BUS_CAPACITY: usize = 256;

pub async fn run(cli: &Cli) -> anyhow::Result<()> {
    let (config, config_file) = cli.load_config()?;
    logging::init(&config.log_level)?;
    tracing::info!(version = palveni_core::VERSION, "starting Palveni Server");
    if let Some(file) = &config_file {
        tracing::info!(file = %file.display(), "loaded configuration file");
    }

    std::fs::create_dir_all(&config.data_dir)
        .with_context(|| format!("cannot create data directory {}", config.data_dir.display()))?;

    let host = tokio::task::spawn_blocking(HostInfo::detect)
        .await
        .context("host detection failed")?;
    // Hardware encoders are not known yet; the background probe refines
    // the tier shown in the hardware report.
    let tier = config
        .perf_tier
        .unwrap_or_else(|| PerfTier::detect(host.memory_bytes, host.cores, false));
    tracing::info!(
        tier = %tier,
        cores = host.cores,
        memory_mib = host.memory_bytes / (1024 * 1024),
        arch = %host.arch,
        "performance tier"
    );

    let db_options = DbOptions::from_config(&config.database);
    let db = Db::open(&db_options).await.with_context(|| {
        format!(
            "cannot open database {}",
            palveni_db::redact_url(&db_options.main_url)
        )
    })?;
    tracing::info!(backend = %db.backend(), "database ready");

    let server_id = db.settings().server_id().await?;
    let identity = load_identity(&config).await?;
    tracing::info!(server_id = %server_id, node_id = %identity.node_id(), "server identity");

    let shutdown = CancellationToken::new();
    let bus = db.event_bus(EVENT_BUS_CAPACITY).await?;

    let overrides: RuntimeSettings = db
        .settings()
        .get(palveni_db::settings::KEY_RUNTIME)
        .await?
        .map(|value| serde_json::from_str(&value))
        .transpose()?
        .unwrap_or_default();
    let jobs = JobService::new(
        db.clone(),
        bus.clone(),
        overrides.perf_tier.unwrap_or(tier).job_concurrency(),
    );
    let settings = Arc::new(tokio::sync::RwLock::new(overrides));
    jobs.register(Arc::new(ScanJobHandler::new(Scanner::new(
        db.clone(),
        bus.clone(),
    ))));
    let runner = jobs.spawn_runner(shutdown.clone());

    let hardware = Arc::new(HardwareService::new(
        db.clone(),
        ProbeOptions {
            ffmpeg_path: config.ffmpeg_path.clone(),
            perf_tier_override: config.perf_tier,
            ..ProbeOptions::default()
        },
    ));
    hardware.load_persisted().await;
    {
        let hardware = Arc::clone(&hardware);
        let settings = Arc::clone(&settings);
        let jobs = jobs.clone();
        tokio::spawn(async move {
            let report = hardware.get(true).await;
            let overrides = settings.read().await;
            jobs.set_concurrency(
                overrides
                    .perf_tier
                    .unwrap_or(report.perf_tier)
                    .job_concurrency(),
            );
            drop(overrides);
            let verified = report.encoders.iter().filter(|e| e.verified).count();
            tracing::info!(
                ffmpeg = report.ffmpeg.found,
                verified_encoders = verified,
                tier = %report.perf_tier,
                "hardware probe finished"
            );
        });
    }

    let features = FeatureRegistry::new();
    let skin_data = config.data_dir.clone();
    let skins = tokio::task::spawn_blocking(move || SkinStore::open(&skin_data)).await??;
    let state = AppState {
        db: db.clone(),
        login_limit: Arc::new(palveni_api::LoginLimit::new(tier.job_concurrency())),
        bus,
        jobs: jobs.clone(),
        hardware,
        boot_tier: tier,
        settings,
        plugins: Arc::new(PluginCatalog::new()),
        skins: Arc::new(skins),
        skin_writes: Arc::new(tokio::sync::Semaphore::new(2)),
        features,
        server: Arc::new(ServerInfo {
            id: server_id,
            name: server_name(&config),
            version: palveni_core::VERSION.to_owned(),
        }),
        shutdown: shutdown.clone(),
    };
    let app = palveni_api::with_http_layers(with_fallback(palveni_api::router(state)));

    let addr = config.listen_addr();
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .with_context(|| format!("cannot listen on {addr}"))?;
    tracing::info!(%addr, "listening");

    tokio::spawn(cancel_on_signal(shutdown.clone()));
    let graceful = {
        let token = shutdown.clone();
        async move { token.cancelled().await }
    };
    let server = axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(graceful);
    let deadline = {
        let token = shutdown.clone();
        async move {
            token.cancelled().await;
            tokio::time::sleep(SHUTDOWN_GRACE).await;
        }
    };
    tokio::select! {
        result = server.into_future() => result.context("HTTP server failed")?,
        () = deadline => tracing::warn!("connections did not close in time; shutting down anyway"),
    }

    shutdown.cancel();
    jobs.wait_idle(SHUTDOWN_GRACE).await;
    if let Err(err) = runner.await {
        tracing::warn!(error = %err, "job runner ended abnormally");
    }
    if let Err(err) = db.close().await {
        tracing::warn!(error = %err, "closing the database failed");
    }
    tracing::info!("stopped");
    Ok(())
}

/// Everything outside `/api/v1`: the embedded web UI when built with
/// `embed-web`, otherwise a JSON 404.
#[cfg(feature = "embed-web")]
fn with_fallback(router: Router) -> Router {
    router.fallback(crate::web::serve)
}

#[cfg(not(feature = "embed-web"))]
fn with_fallback(router: Router) -> Router {
    router.fallback(palveni_api::not_found)
}

async fn load_identity(config: &Config) -> anyhow::Result<NodeIdentity> {
    let path = config.node_key_path();
    let (identity, created) =
        tokio::task::spawn_blocking(move || NodeIdentity::load_or_generate(&path))
            .await
            .context("identity task failed")??;
    if created {
        tracing::info!(path = %config.node_key_path().display(), "created node identity");
    }
    Ok(identity)
}

/// Configured name, else the host name, else a generic name.
fn server_name(config: &Config) -> String {
    if let Some(name) = &config.server_name {
        return name.trim().to_owned();
    }
    let host = std::fs::read_to_string("/proc/sys/kernel/hostname")
        .or_else(|_| std::fs::read_to_string("/etc/hostname"))
        .map(|s| s.trim().to_owned())
        .unwrap_or_default();
    if host.is_empty() {
        "Palveni Server".to_owned()
    } else {
        host
    }
}

async fn cancel_on_signal(shutdown: CancellationToken) {
    wait_for_signal().await;
    tracing::info!("shutdown requested");
    shutdown.cancel();
}

async fn wait_for_signal() {
    let ctrl_c = async {
        if let Err(err) = tokio::signal::ctrl_c().await {
            tracing::error!(error = %err, "cannot listen for Ctrl-C");
            std::future::pending::<()>().await;
        }
    };
    #[cfg(unix)]
    let terminate = async {
        use tokio::signal::unix::{SignalKind, signal};
        match signal(SignalKind::terminate()) {
            Ok(mut stream) => {
                stream.recv().await;
            }
            Err(err) => {
                tracing::error!(error = %err, "cannot listen for SIGTERM");
                std::future::pending::<()>().await;
            }
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {}
        () = terminate => {}
    }
}

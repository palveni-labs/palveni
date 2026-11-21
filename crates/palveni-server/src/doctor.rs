//! `palveni doctor`: a human-readable health check to paste into support
//! requests. It never modifies anything except creating
//! the migration bookkeeping table of an existing database.

use std::fmt::Write as _;
use std::path::Path;

use palveni_core::config::Config;
use palveni_core::{HostInfo, PerfTier};
use palveni_db::DbOptions;
use palveni_db::health::{DbHealth, StoreHealth};
use palveni_link::NodeIdentity;
use palveni_media::ProbeOptions;
use palveni_media::devices::DeviceNode;
use palveni_media::report::{ProbeDetails, probe_hardware_detailed};

use crate::cli::Cli;

pub async fn run(cli: &Cli) -> anyhow::Result<()> {
    let (config, config_file) = cli.load_config()?;
    let mut out = String::new();

    section(&mut out, "Palveni");
    line(
        &mut out,
        "version",
        format!(
            "{} ({}/{})",
            palveni_core::VERSION,
            std::env::consts::OS,
            std::env::consts::ARCH
        ),
    );
    line(
        &mut out,
        "config file",
        config_file.map_or_else(
            || "none (defaults and environment)".to_owned(),
            |p| p.display().to_string(),
        ),
    );
    line(&mut out, "data dir", config.data_dir.display().to_string());
    line(&mut out, "listen", config.listen_addr().to_string());
    print!("{out}");
    out.clear();

    let host = tokio::task::spawn_blocking(HostInfo::detect).await?;
    section(&mut out, "Host");
    line(
        &mut out,
        "cpu",
        format!("{} ({} cores, {})", host.cpu_model, host.cores, host.arch),
    );
    line(&mut out, "memory", format_bytes(host.memory_bytes));
    print!("{out}");
    out.clear();

    println!("\nProbing ffmpeg and hardware encoders (this runs short test encodes)...");
    let details = probe_hardware_detailed(&ProbeOptions {
        ffmpeg_path: config.ffmpeg_path.clone(),
        perf_tier_override: config.perf_tier,
        ..ProbeOptions::default()
    })
    .await;
    write_hardware(&mut out, &config, &details);
    print!("{out}");
    out.clear();

    section(&mut out, "Storage");
    write_storage(&mut out, &config);
    write_port(&mut out, &config);
    print!("{out}");
    out.clear();

    section(&mut out, "Database");
    let health = palveni_db::health::inspect(&DbOptions::from_config(&config.database)).await;
    write_database(&mut out, &health);
    print!("{out}");
    out.clear();

    section(&mut out, "Link");
    write_identity(&mut out, &config);
    print!("{out}");
    Ok(())
}

fn section(out: &mut String, title: &str) {
    let _ = writeln!(out, "\n{title}");
}

fn line(out: &mut String, label: &str, value: impl AsRef<str>) {
    let _ = writeln!(out, "  {label:<22} {}", value.as_ref());
}

fn format_bytes(bytes: u64) -> String {
    if bytes == 0 {
        return "unknown".to_owned();
    }
    const GIB: u64 = 1024 * 1024 * 1024;
    const MIB: u64 = 1024 * 1024;
    if bytes >= GIB {
        format!("{}.{} GiB", bytes / GIB, (bytes % GIB) * 10 / GIB)
    } else {
        format!("{} MiB", bytes / MIB)
    }
}

fn write_hardware(out: &mut String, config: &Config, details: &ProbeDetails) {
    let report = &details.report;
    section(out, "FFmpeg");
    for (name, tool) in [("ffmpeg", &report.ffmpeg), ("ffprobe", &report.ffprobe)] {
        let text = if tool.found {
            format!(
                "{} [{}] version {}",
                tool.path.as_deref().unwrap_or("?"),
                tool.source.map_or("?", |s| match s {
                    palveni_media::ToolSource::Config => "config",
                    palveni_media::ToolSource::Bundled => "bundled",
                    palveni_media::ToolSource::Path => "PATH",
                }),
                tool.version.as_deref().unwrap_or("unknown")
            )
        } else {
            "NOT FOUND".to_owned()
        };
        line(out, name, text);
    }
    if let Some(configured) = &config.ffmpeg_path {
        line(out, "configured path", configured.display().to_string());
    }
    line(
        out,
        "hwaccels",
        if report.hwaccels.is_empty() {
            "none".to_owned()
        } else {
            report.hwaccels.join(", ")
        },
    );

    section(out, "Devices");
    let nodes: Vec<&DeviceNode> = details.devices.all().collect();
    if nodes.is_empty() {
        line(out, "nodes", "none found");
    }
    for node in nodes {
        let state = match node.accessible {
            Some(true) => "accessible",
            Some(false) => "NOT ACCESSIBLE (permissions?)",
            None => "present",
        };
        line(out, &node.path.display().to_string(), state);
    }

    section(out, "Encoders");
    for encoder in &report.encoders {
        let status = if encoder.verified {
            "ok".to_owned()
        } else if !encoder.listed {
            "not in this build".to_owned()
        } else {
            format!(
                "FAILED: {}",
                encoder.error.as_deref().unwrap_or("unknown error")
            )
        };
        line(
            out,
            &format!(
                "{} ({}, {})",
                encoder.id,
                encoder.method.as_str(),
                encoder.codec
            ),
            status,
        );
    }

    section(out, "Performance");
    let detected_note = if config.perf_tier.is_some() {
        " (set in configuration)"
    } else {
        " (detected)"
    };
    line(out, "tier", format!("{}{detected_note}", report.perf_tier));
    line(
        out,
        "background jobs",
        PerfTier::job_concurrency(report.perf_tier).to_string(),
    );
}

fn write_storage(out: &mut String, config: &Config) {
    let dir = &config.data_dir;
    if !dir.exists() {
        line(
            out,
            "data dir",
            "does not exist yet (created on first start)",
        );
        return;
    }
    let probe = dir.join(".palveni-doctor-write-test");
    let writable = std::fs::write(&probe, b"ok").is_ok();
    let _ = std::fs::remove_file(&probe);
    line(
        out,
        "data dir writable",
        if writable {
            "yes"
        } else {
            "NO (check ownership and permissions)"
        },
    );
}

fn write_port(out: &mut String, config: &Config) {
    let addr = config.listen_addr();
    let status = match std::net::TcpListener::bind(addr) {
        Ok(_) => "free".to_owned(),
        Err(err) if err.kind() == std::io::ErrorKind::AddrInUse => {
            "in use (is palveni already running?)".to_owned()
        }
        Err(err) => format!("cannot bind: {err}"),
    };
    line(out, &format!("port {}", addr.port()), status);
}

fn write_store(out: &mut String, label: &str, store: &StoreHealth) {
    let backend = store.backend.map_or("unknown", |b| b.as_str());
    let state = match (store.reachable, &store.error) {
        (true, None) => format!(
            "{backend}, reachable, {} pending migration(s)",
            store.pending_migrations.unwrap_or(0)
        ),
        (true, Some(err)) => format!("{backend}, reachable, {err}"),
        (false, Some(err)) => format!("{backend}, {err}"),
        (false, None) => format!("{backend}, not reachable"),
    };
    line(out, label, format!("{} ({state})", store.url));
}

fn write_database(out: &mut String, health: &DbHealth) {
    write_store(out, "main store", &health.main);
    write_store(out, "ops store", &health.ops);
    if let Some(users) = health.users {
        line(out, "users", users.to_string());
    }
    if let Some(items) = health.live_items {
        line(out, "items", items.to_string());
    }
    for library in &health.libraries {
        for path in &library.paths {
            let readable = std::fs::read_dir(Path::new(path)).is_ok();
            line(
                out,
                &format!("library \"{}\"", library.name),
                format!(
                    "{path} ({})",
                    if readable { "readable" } else { "NOT READABLE" }
                ),
            );
        }
    }
}

fn write_identity(out: &mut String, config: &Config) {
    let path = config.node_key_path();
    let text = if path.exists() {
        match NodeIdentity::load(&path) {
            Ok(identity) => identity.node_id().to_string(),
            Err(err) => format!("ERROR: {err}"),
        }
    } else {
        "not created yet (created on first start)".to_owned()
    };
    line(out, "node id", text);
}

use std::path::PathBuf;

use clap::{Parser, Subcommand};
use palveni_core::config::{self, Config, ConfigLayer, ENV_CONFIG_PATH};

#[derive(Debug, Parser)]
#[command(name = "palveni", about = "Palveni media server")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

    /// Configuration file [env: PALVENI_CONFIG]
    /// (default: ./palveni.toml, then <data-dir>/palveni.toml)
    #[arg(long, global = true, value_name = "FILE")]
    pub config: Option<PathBuf>,

    /// Data directory for the database, keys and caches [env: PALVENI_DATA_DIR]
    #[arg(long, global = true, value_name = "DIR")]
    pub data_dir: Option<PathBuf>,

    /// Address to listen on, e.g. 0.0.0.0 or :: [env: PALVENI_LISTEN]
    #[arg(long, global = true, value_name = "ADDR")]
    pub listen: Option<String>,

    /// HTTP port (default 7258) [env: PALVENI_PORT]
    #[arg(long, global = true)]
    pub port: Option<u16>,

    /// Enable direct Link connections on this UDP address [env: PALVENI_LINK_BIND]
    #[arg(long, global = true, value_name = "IP:PORT")]
    pub link_bind: Option<std::net::SocketAddr>,

    /// Log filter, e.g. info or info,palveni_db=debug [env: PALVENI_LOG_LEVEL]
    #[arg(long, global = true, value_name = "FILTER")]
    pub log_level: Option<String>,

    /// Database URL (sqlite://…, postgres://…, mysql://…) [env: PALVENI_DATABASE_URL]
    #[arg(long, global = true, value_name = "URL")]
    pub database_url: Option<String>,
}

#[derive(Clone, Copy, Debug, Subcommand)]
pub enum Command {
    /// Run the server (default)
    Serve,
    /// Check ffmpeg, hardware acceleration, storage, port and database
    Doctor,
    /// Print the OpenAPI 3.1 document without starting the server
    Openapi,
    /// Print version information
    Version,
}

impl Cli {
    /// Loads the configuration: CLI > environment > file > defaults.
    pub fn load_config(&self) -> anyhow::Result<(Config, Option<PathBuf>)> {
        let cli_layer = ConfigLayer {
            data_dir: self.data_dir.clone(),
            listen: self.listen.clone(),
            port: self.port,
            link_bind: self.link_bind,
            log_level: self.log_level.clone(),
            database_url: self.database_url.clone(),
            ..ConfigLayer::default()
        };
        let explicit = self.config.clone().or_else(|| {
            std::env::var_os(ENV_CONFIG_PATH)
                .filter(|v| !v.is_empty())
                .map(PathBuf::from)
        });
        let loaded = config::load(explicit.as_deref(), cli_layer, |key| {
            std::env::var(key).ok()
        })?;
        Ok(loaded)
    }
}

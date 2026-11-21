//! Server configuration.
//!
//! Precedence: command line > environment > config file >
//! defaults. Each source produces a [`ConfigLayer`] of optional values; layers
//! are merged and then resolved into a complete [`Config`].
//!
//! Config file keys (TOML, all optional):
//!
//! | key                         | env var                              |
//! |-----------------------------|--------------------------------------|
//! | `data_dir`                  | `PALVENI_DATA_DIR`                   |
//! | `listen`                    | `PALVENI_LISTEN`                     |
//! | `port`                      | `PALVENI_PORT`                       |
//! | `link_bind`                 | `PALVENI_LINK_BIND`                  |
//! | `ffmpeg_path`               | `PALVENI_FFMPEG_PATH`                |
//! | `log_level`                 | `PALVENI_LOG_LEVEL`                  |
//! | `perf_tier`                 | `PALVENI_PERF_TIER`                  |
//! | `server_name`               | `PALVENI_SERVER_NAME`                |
//! | `database_url`              | `PALVENI_DATABASE_URL`               |
//! | `ops_database_url`          | `PALVENI_OPS_DATABASE_URL`           |
//! | `database_max_connections`  | `PALVENI_DATABASE_MAX_CONNECTIONS`   |
//! | `database_min_connections`  | `PALVENI_DATABASE_MIN_CONNECTIONS`   |
//!
//! The config file itself is chosen by `--config` / `PALVENI_CONFIG`, else
//! `./palveni.toml`, else `<data_dir>/palveni.toml`, whichever exists first.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::perf::PerfTier;

/// Default HTTP port: P-A-L-V on a phone keypad.
pub const DEFAULT_PORT: u16 = 7258;
pub const DEFAULT_DATA_DIR: &str = "data";
pub const DEFAULT_LOG_LEVEL: &str = "info";
pub const CONFIG_FILE_NAME: &str = "palveni.toml";
pub const MAIN_DB_FILE: &str = "palveni.db";
pub const ENV_CONFIG_PATH: &str = "PALVENI_CONFIG";

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("cannot read config file {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("invalid config file {path}: {message}")]
    Parse { path: PathBuf, message: String },
    #[error("invalid value for {key}: {message}")]
    Invalid { key: &'static str, message: String },
}

/// Fully resolved configuration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Config {
    pub data_dir: PathBuf,
    pub listen: IpAddr,
    pub port: u16,
    /// Direct UDP listen address. Link stays disabled when unset.
    pub link_bind: Option<SocketAddr>,
    /// Explicit ffmpeg binary (or directory containing ffmpeg and ffprobe).
    pub ffmpeg_path: Option<PathBuf>,
    /// `tracing` filter directive, e.g. `info` or `info,palveni_db=debug`.
    pub log_level: String,
    /// Administrator override of the detected performance tier.
    pub perf_tier: Option<PerfTier>,
    /// Display name; the host name is used when unset.
    pub server_name: Option<String>,
    pub database: DatabaseConfig,
}

/// Connection settings for the two logical stores (see `palveni-db`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DatabaseConfig {
    /// Main (catalog) store, e.g. `sqlite://data/palveni.db?mode=rwc` or
    /// `postgres://user:pass@host/palveni`.
    pub url: String,
    /// Operational store; `None` derives it from `url` (see
    /// [`DatabaseConfig::ops_url`]).
    pub ops_url: Option<String>,
    /// Per-store pool size; `None` lets the database layer choose.
    pub max_connections: Option<u32>,
    pub min_connections: Option<u32>,
}

impl DatabaseConfig {
    /// URL of the operational store.
    ///
    /// For SQLite this is a sibling file with `-ops` appended to the stem
    /// (`palveni.db` → `palveni-ops.db`) so that job and log churn never
    /// contends for the catalog's write lock. Network databases use the same
    /// database for both stores.
    pub fn ops_url(&self) -> String {
        match &self.ops_url {
            Some(url) => url.clone(),
            None => derive_ops_url(&self.url),
        }
    }
}

/// See [`DatabaseConfig::ops_url`].
pub fn derive_ops_url(main_url: &str) -> String {
    if !main_url.starts_with("sqlite:") {
        return main_url.to_owned();
    }
    let (path_part, query) = match main_url.split_once('?') {
        Some((p, q)) => (p, Some(q)),
        None => (main_url, None),
    };
    let ops_path = match path_part.rsplit_once('.') {
        // Only treat it as an extension if the dot is in the last path segment.
        Some((stem, ext)) if !ext.contains('/') && !stem.ends_with('/') => {
            format!("{stem}-ops.{ext}")
        }
        _ => format!("{path_part}-ops"),
    };
    match query {
        Some(q) => format!("{ops_path}?{q}"),
        None => ops_path,
    }
}

/// Builds the default SQLite URL for a data directory.
///
/// sqlx percent-decodes the path part, so the characters that would end or
/// alter it are escaped.
pub fn default_database_url(data_dir: &Path) -> String {
    let path = data_dir.join(MAIN_DB_FILE);
    let raw = path.to_string_lossy();
    let mut escaped = String::with_capacity(raw.len());
    for ch in raw.chars() {
        match ch {
            '%' => escaped.push_str("%25"),
            '?' => escaped.push_str("%3F"),
            '#' => escaped.push_str("%23"),
            other => escaped.push(other),
        }
    }
    format!("sqlite://{escaped}?mode=rwc")
}

impl Config {
    pub fn listen_addr(&self) -> SocketAddr {
        SocketAddr::new(self.listen, self.port)
    }

    /// Location of this server's Ed25519 identity (palveni-link).
    pub fn node_key_path(&self) -> PathBuf {
        self.data_dir.join("node.key")
    }

    /// Resolves a merged layer into a complete configuration.
    pub fn resolve(layer: ConfigLayer) -> Result<Config, ConfigError> {
        let data_dir = layer
            .data_dir
            .unwrap_or_else(|| PathBuf::from(DEFAULT_DATA_DIR));
        let listen = match layer.listen.as_deref() {
            None => IpAddr::V4(Ipv4Addr::UNSPECIFIED),
            Some(text) => parse_listen(text)?,
        };
        let perf_tier = match layer.perf_tier.as_deref() {
            None => None,
            Some(text) if text.trim().eq_ignore_ascii_case("auto") => None,
            Some(text) => Some(text.parse::<PerfTier>().map_err(|e| ConfigError::Invalid {
                key: "perf_tier",
                message: e.to_string(),
            })?),
        };
        let log_level = layer
            .log_level
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_LOG_LEVEL.to_owned());
        let url = layer
            .database_url
            .unwrap_or_else(|| default_database_url(&data_dir));
        if let (Some(min), Some(max)) = (
            layer.database_min_connections,
            layer.database_max_connections,
        ) && min > max
        {
            return Err(ConfigError::Invalid {
                key: "database_min_connections",
                message: format!("{min} is larger than database_max_connections ({max})"),
            });
        }
        if layer.database_max_connections == Some(0) {
            return Err(ConfigError::Invalid {
                key: "database_max_connections",
                message: "must be at least 1".to_owned(),
            });
        }
        Ok(Config {
            data_dir,
            listen,
            port: layer.port.unwrap_or(DEFAULT_PORT),
            link_bind: layer.link_bind,
            ffmpeg_path: layer.ffmpeg_path,
            log_level,
            perf_tier,
            server_name: layer.server_name.filter(|s| !s.trim().is_empty()),
            database: DatabaseConfig {
                url,
                ops_url: layer.ops_database_url,
                max_connections: layer.database_max_connections,
                min_connections: layer.database_min_connections,
            },
        })
    }
}

fn parse_listen(text: &str) -> Result<IpAddr, ConfigError> {
    let trimmed = text.trim().trim_start_matches('[').trim_end_matches(']');
    trimmed.parse::<IpAddr>().map_err(|_| ConfigError::Invalid {
        key: "listen",
        message: format!("`{text}` is not an IP address (use e.g. 0.0.0.0 or ::)"),
    })
}

/// One source of configuration values; every field is optional.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigLayer {
    pub data_dir: Option<PathBuf>,
    pub listen: Option<String>,
    pub port: Option<u16>,
    pub link_bind: Option<SocketAddr>,
    pub ffmpeg_path: Option<PathBuf>,
    pub log_level: Option<String>,
    pub perf_tier: Option<String>,
    pub server_name: Option<String>,
    pub database_url: Option<String>,
    pub ops_database_url: Option<String>,
    pub database_max_connections: Option<u32>,
    pub database_min_connections: Option<u32>,
}

impl ConfigLayer {
    pub fn from_toml_str(text: &str, path: &Path) -> Result<Self, ConfigError> {
        toml::from_str(text).map_err(|e| ConfigError::Parse {
            path: path.to_path_buf(),
            message: e.to_string(),
        })
    }

    pub fn from_file(path: &Path) -> Result<Self, ConfigError> {
        let text = std::fs::read_to_string(path).map_err(|source| ConfigError::Read {
            path: path.to_path_buf(),
            source,
        })?;
        Self::from_toml_str(&text, path)
    }

    /// Reads `PALVENI_*` variables through `lookup` (injected so tests do not
    /// have to mutate the process environment). Empty values count as unset.
    pub fn from_env<F>(lookup: F) -> Result<Self, ConfigError>
    where
        F: Fn(&str) -> Option<String>,
    {
        let get = |key: &str| lookup(key).filter(|v| !v.trim().is_empty());
        let parse_u16 = |key: &'static str| -> Result<Option<u16>, ConfigError> {
            get(key)
                .map(|v| {
                    v.trim().parse::<u16>().map_err(|_| ConfigError::Invalid {
                        key,
                        message: format!("`{v}` is not a port number"),
                    })
                })
                .transpose()
        };
        let parse_u32 = |key: &'static str| -> Result<Option<u32>, ConfigError> {
            get(key)
                .map(|v| {
                    v.trim().parse::<u32>().map_err(|_| ConfigError::Invalid {
                        key,
                        message: format!("`{v}` is not a non-negative integer"),
                    })
                })
                .transpose()
        };
        Ok(ConfigLayer {
            data_dir: get("PALVENI_DATA_DIR").map(PathBuf::from),
            listen: get("PALVENI_LISTEN"),
            port: parse_u16("PALVENI_PORT")?,
            link_bind: get("PALVENI_LINK_BIND")
                .map(|v| {
                    v.trim().parse().map_err(|_| ConfigError::Invalid {
                        key: "PALVENI_LINK_BIND",
                        message: "expected an IP address with UDP port".to_owned(),
                    })
                })
                .transpose()?,
            ffmpeg_path: get("PALVENI_FFMPEG_PATH").map(PathBuf::from),
            log_level: get("PALVENI_LOG_LEVEL"),
            perf_tier: get("PALVENI_PERF_TIER"),
            server_name: get("PALVENI_SERVER_NAME"),
            database_url: get("PALVENI_DATABASE_URL"),
            ops_database_url: get("PALVENI_OPS_DATABASE_URL"),
            database_max_connections: parse_u32("PALVENI_DATABASE_MAX_CONNECTIONS")?,
            database_min_connections: parse_u32("PALVENI_DATABASE_MIN_CONNECTIONS")?,
        })
    }

    /// Returns `self` with any unset field filled from `lower`.
    pub fn or(self, lower: ConfigLayer) -> ConfigLayer {
        ConfigLayer {
            data_dir: self.data_dir.or(lower.data_dir),
            listen: self.listen.or(lower.listen),
            port: self.port.or(lower.port),
            link_bind: self.link_bind.or(lower.link_bind),
            ffmpeg_path: self.ffmpeg_path.or(lower.ffmpeg_path),
            log_level: self.log_level.or(lower.log_level),
            perf_tier: self.perf_tier.or(lower.perf_tier),
            server_name: self.server_name.or(lower.server_name),
            database_url: self.database_url.or(lower.database_url),
            ops_database_url: self.ops_database_url.or(lower.ops_database_url),
            database_max_connections: self
                .database_max_connections
                .or(lower.database_max_connections),
            database_min_connections: self
                .database_min_connections
                .or(lower.database_min_connections),
        }
    }
}

/// Loads configuration from all sources.
///
/// `explicit_file` comes from `--config` or `PALVENI_CONFIG` and must exist.
/// Without it, `./palveni.toml` and then `<data_dir>/palveni.toml` are tried,
/// where `data_dir` is taken from the CLI/env layers or the default.
///
/// Returns the config and the file that was read, if any.
pub fn load<F>(
    explicit_file: Option<&Path>,
    cli: ConfigLayer,
    env_lookup: F,
) -> Result<(Config, Option<PathBuf>), ConfigError>
where
    F: Fn(&str) -> Option<String>,
{
    let env = ConfigLayer::from_env(env_lookup)?;
    let upper = cli.or(env);

    let file_path = match explicit_file {
        Some(path) => Some(path.to_path_buf()),
        None => {
            let data_dir = upper
                .data_dir
                .clone()
                .unwrap_or_else(|| PathBuf::from(DEFAULT_DATA_DIR));
            [
                PathBuf::from(CONFIG_FILE_NAME),
                data_dir.join(CONFIG_FILE_NAME),
            ]
            .into_iter()
            .find(|candidate| candidate.is_file())
        }
    };
    let file_layer = match &file_path {
        Some(path) => ConfigLayer::from_file(path)?,
        None => ConfigLayer::default(),
    };
    let config = Config::resolve(upper.or(file_layer))?;
    Ok((config, file_path))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn env(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let map: HashMap<String, String> = pairs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect();
        move |key: &str| map.get(key).cloned()
    }

    #[test]
    fn defaults() {
        let config = Config::resolve(ConfigLayer::default()).unwrap();
        assert_eq!(config.port, 7258);
        assert_eq!(config.data_dir, PathBuf::from("data"));
        assert_eq!(config.listen, IpAddr::V4(Ipv4Addr::UNSPECIFIED));
        assert_eq!(config.log_level, "info");
        assert_eq!(config.perf_tier, None);
        assert_eq!(config.database.url, "sqlite://data/palveni.db?mode=rwc");
        assert_eq!(
            config.database.ops_url(),
            "sqlite://data/palveni-ops.db?mode=rwc"
        );
    }

    #[test]
    fn absolute_data_dir_url() {
        let url = default_database_url(Path::new("/data"));
        assert_eq!(url, "sqlite:///data/palveni.db?mode=rwc");
        assert_eq!(
            derive_ops_url(&url),
            "sqlite:///data/palveni-ops.db?mode=rwc"
        );
        assert_eq!(
            default_database_url(Path::new("/srv/a?b#c%d")),
            "sqlite:///srv/a%3Fb%23c%25d/palveni.db?mode=rwc"
        );
    }

    #[test]
    fn ops_url_for_network_backends_is_shared() {
        let url = "postgres://palveni:secret@db.local/palveni";
        assert_eq!(derive_ops_url(url), url);
        assert_eq!(
            derive_ops_url("sqlite:///x/catalog"),
            "sqlite:///x/catalog-ops"
        );
        assert_eq!(
            derive_ops_url("sqlite:///x.y/catalog"),
            "sqlite:///x.y/catalog-ops"
        );
    }

    #[test]
    fn env_overrides_file_and_cli_overrides_env() {
        let file = ConfigLayer::from_toml_str(
            "port = 8000\nlog_level = \"debug\"\nperf_tier = \"lite\"\n",
            Path::new("palveni.toml"),
        )
        .unwrap();
        let env_layer = ConfigLayer::from_env(env(&[
            ("PALVENI_PORT", "9000"),
            ("PALVENI_DATA_DIR", "/data"),
            ("PALVENI_LISTEN", "::"),
            ("PALVENI_LOG_LEVEL", ""),
        ]))
        .unwrap();
        let cli = ConfigLayer {
            port: Some(9100),
            ..ConfigLayer::default()
        };
        let config = Config::resolve(cli.or(env_layer).or(file)).unwrap();
        assert_eq!(config.port, 9100);
        assert_eq!(config.data_dir, PathBuf::from("/data"));
        assert_eq!(config.listen.to_string(), "::");
        // Empty env value falls through to the file.
        assert_eq!(config.log_level, "debug");
        assert_eq!(config.perf_tier, Some(PerfTier::Lite));
        assert_eq!(config.database.url, "sqlite:///data/palveni.db?mode=rwc");
    }

    #[test]
    fn rejects_unknown_keys_and_bad_values() {
        assert!(ConfigLayer::from_toml_str("prot = 1", Path::new("x.toml")).is_err());
        assert!(ConfigLayer::from_env(env(&[("PALVENI_PORT", "http")])).is_err());
        let bad_listen = ConfigLayer {
            listen: Some("localhost".into()),
            ..ConfigLayer::default()
        };
        assert!(Config::resolve(bad_listen).is_err());
        let bad_pool = ConfigLayer {
            database_min_connections: Some(5),
            database_max_connections: Some(2),
            ..ConfigLayer::default()
        };
        assert!(Config::resolve(bad_pool).is_err());
    }

    #[test]
    fn perf_tier_auto_means_detect() {
        let layer = ConfigLayer {
            perf_tier: Some("auto".into()),
            ..ConfigLayer::default()
        };
        assert_eq!(Config::resolve(layer).unwrap().perf_tier, None);
    }
}

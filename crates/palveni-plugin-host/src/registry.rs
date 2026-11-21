//! Plugin registry index.
//!
//! A registry is a static, signed JSON document listing every version of
//! every plugin. Signature verification (Ed25519, key pinned when the
//! registry is added) belongs to the installer and is not part of this
//! model.

use chrono::{DateTime, Utc};
use semver::Version;
use serde::{Deserialize, Serialize};

use crate::compat;
use crate::manifest::{RuntimeKind, is_valid_plugin_id};
use crate::permissions::Permissions;

/// Index format version understood by this host.
pub const REGISTRY_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, thiserror::Error)]
pub enum RegistryError {
    #[error("registry index is not valid JSON: {0}")]
    Parse(String),
    #[error("unsupported registry schema version {0}")]
    UnsupportedSchema(u32),
    #[error("invalid entry {plugin}: {reason}")]
    InvalidEntry { plugin: String, reason: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistryIndex {
    pub schema: u32,
    pub name: String,
    pub generated_at: DateTime<Utc>,
    pub plugins: Vec<RegistryPlugin>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistryPlugin {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub releases: Vec<RegistryRelease>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistryRelease {
    pub version: String,
    /// Download URL of the `.pvp` package.
    pub url: String,
    /// Lowercase hex SHA-256 of the package.
    pub sha256: String,
    pub size: u64,
    /// Contract range, same syntax as `[compat] api`.
    pub api: String,
    pub runtime: RuntimeKind,
    /// Permission summary, same keys as the manifest's `[permissions]`.
    #[serde(default)]
    pub permissions: Permissions,
    pub published_at: DateTime<Utc>,
    #[serde(default)]
    pub yanked: Option<Yank>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Yank {
    pub reason: String,
    /// Malicious releases are also disabled where already installed.
    #[serde(default)]
    pub malicious: bool,
}

impl RegistryIndex {
    pub fn parse(text: &str) -> Result<RegistryIndex, RegistryError> {
        let index: RegistryIndex =
            serde_json::from_str(text).map_err(|e| RegistryError::Parse(e.to_string()))?;
        index.validate()?;
        Ok(index)
    }

    pub fn validate(&self) -> Result<(), RegistryError> {
        if self.schema != REGISTRY_SCHEMA_VERSION {
            return Err(RegistryError::UnsupportedSchema(self.schema));
        }
        for plugin in &self.plugins {
            let invalid = |reason: String| RegistryError::InvalidEntry {
                plugin: plugin.id.clone(),
                reason,
            };
            if !is_valid_plugin_id(&plugin.id) {
                return Err(invalid("invalid plugin id".to_owned()));
            }
            for release in &plugin.releases {
                if Version::parse(&release.version).is_err() {
                    return Err(invalid(format!("invalid version `{}`", release.version)));
                }
                let sha_ok = release.sha256.len() == 64
                    && release
                        .sha256
                        .chars()
                        .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c));
                if !sha_ok {
                    return Err(invalid(format!("{}: invalid sha256", release.version)));
                }
                if !release.url.starts_with("https://") {
                    return Err(invalid(format!(
                        "{}: download URL must be https",
                        release.version
                    )));
                }
                if compat::parse_range(&release.api).is_err() {
                    return Err(invalid(format!("{}: invalid api range", release.version)));
                }
            }
        }
        Ok(())
    }

    pub fn find(&self, id: &str) -> Option<&RegistryPlugin> {
        self.plugins.iter().find(|p| p.id == id)
    }
}

impl RegistryPlugin {
    /// Newest release that is not yanked and works with `supported`
    /// contract versions.
    pub fn best_release(&self, supported: &[Version]) -> Option<&RegistryRelease> {
        self.releases
            .iter()
            .filter(|r| r.yanked.is_none())
            .filter(|r| compat::check_compat_with(&r.api, supported).is_ok())
            .filter_map(|r| Version::parse(&r.version).ok().map(|v| (v, r)))
            .max_by(|(a, _), (b, _)| a.cmp(b))
            .map(|(_, r)| r)
    }

    /// Whether `version` was yanked as malicious.
    pub fn is_malicious(&self, version: &str) -> bool {
        self.releases
            .iter()
            .any(|r| r.version == version && r.yanked.as_ref().is_some_and(|y| y.malicious))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const INDEX: &str = r#"{
      "schema": 1,
      "name": "Example registry",
      "generatedAt": "2026-11-20T08:30:00Z",
      "plugins": [{
        "id": "org.example.tmdb",
        "name": "TMDB",
        "description": null,
        "releases": [
          {
            "version": "1.1.0",
            "url": "https://plugins.example.org/tmdb-1.1.0.pvp",
            "sha256": "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
            "size": 120000,
            "api": "^0.1",
            "runtime": "wasm",
            "permissions": { "network": ["api.themoviedb.org"] },
            "publishedAt": "2026-10-01T00:00:00Z"
          },
          {
            "version": "1.2.0",
            "url": "https://plugins.example.org/tmdb-1.2.0.pvp",
            "sha256": "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
            "size": 121000,
            "api": "^0.1",
            "runtime": "wasm",
            "publishedAt": "2026-11-01T00:00:00Z",
            "yanked": { "reason": "credential leak", "malicious": true }
          },
          {
            "version": "2.0.0",
            "url": "https://plugins.example.org/tmdb-2.0.0.pvp",
            "sha256": "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
            "size": 130000,
            "api": "^1.0",
            "runtime": "wasm",
            "publishedAt": "2026-11-10T00:00:00Z"
          }
        ]
      }]
    }"#;

    #[test]
    fn parses_and_picks_best_release() {
        let index = RegistryIndex::parse(INDEX).unwrap();
        let plugin = index.find("org.example.tmdb").unwrap();
        let best = plugin
            .best_release(&compat::supported_contract_versions())
            .unwrap();
        // 1.2.0 is yanked and 2.0.0 needs a newer contract.
        assert_eq!(best.version, "1.1.0");
        assert!(plugin.is_malicious("1.2.0"));
        assert!(!plugin.is_malicious("1.1.0"));
    }

    #[test]
    fn rejects_unknown_schema_and_bad_hashes() {
        let future = INDEX.replacen("\"schema\": 1", "\"schema\": 2", 1);
        assert!(matches!(
            RegistryIndex::parse(&future),
            Err(RegistryError::UnsupportedSchema(2))
        ));
        let bad_hash = INDEX.replacen("ba7816bf", "XYZ", 1);
        assert!(RegistryIndex::parse(&bad_hash).is_err());
        let http = INDEX.replacen("https://plugins", "http://plugins", 1);
        assert!(RegistryIndex::parse(&http).is_err());
    }
}

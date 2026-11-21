//! `plugin.toml`.
//!
//! Unknown keys are tolerated in the informational sections so that newer
//! plugins with extra metadata still load; only `[permissions]` is strict.

use std::collections::BTreeMap;
use std::fmt;
use std::str::FromStr;

use semver::Version;
use serde::{Deserialize, Serialize};

use crate::compat;
use crate::error::ManifestError;
use crate::permissions::Permissions;

#[derive(utoipa::ToSchema, Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RuntimeKind {
    /// Pure presentation data, validated by the skin package loader.
    None,
    /// Declarative JSON recipe, interpreted by the host.
    Recipe,
    /// WebAssembly component running in the host sandbox.
    Wasm,
    /// Separate process or remote service speaking the contract over HTTP.
    Sidecar,
}

/// Extension points declared in a plugin manifest.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ExtensionPoint {
    Storage,
    Source,
    Metadata,
    Lyrics,
    Subtitles,
    Danmaku,
    Sync,
    Notification,
    Auth,
    Ai,
    EventHook,
    Schedule,
    Importer,
    HttpEndpoint,
}

impl ExtensionPoint {
    pub const ALL: [ExtensionPoint; 14] = [
        ExtensionPoint::Storage,
        ExtensionPoint::Source,
        ExtensionPoint::Metadata,
        ExtensionPoint::Lyrics,
        ExtensionPoint::Subtitles,
        ExtensionPoint::Danmaku,
        ExtensionPoint::Sync,
        ExtensionPoint::Notification,
        ExtensionPoint::Auth,
        ExtensionPoint::Ai,
        ExtensionPoint::EventHook,
        ExtensionPoint::Schedule,
        ExtensionPoint::Importer,
        ExtensionPoint::HttpEndpoint,
    ];

    /// Key used in `[provides]`.
    pub const fn key(self) -> &'static str {
        match self {
            ExtensionPoint::Storage => "storage",
            ExtensionPoint::Source => "source",
            ExtensionPoint::Metadata => "metadata",
            ExtensionPoint::Lyrics => "lyrics",
            ExtensionPoint::Subtitles => "subtitles",
            ExtensionPoint::Danmaku => "danmaku",
            ExtensionPoint::Sync => "sync",
            ExtensionPoint::Notification => "notification",
            ExtensionPoint::Auth => "auth",
            ExtensionPoint::Ai => "ai",
            ExtensionPoint::EventHook => "event-hook",
            ExtensionPoint::Schedule => "schedule",
            ExtensionPoint::Importer => "importer",
            ExtensionPoint::HttpEndpoint => "http-endpoint",
        }
    }
}

impl fmt::Display for ExtensionPoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.key())
    }
}

impl FromStr for ExtensionPoint {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        ExtensionPoint::ALL
            .into_iter()
            .find(|point| point.key() == s)
            .ok_or(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginInfo {
    /// Reverse-DNS identifier, globally unique and immutable.
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub authors: Vec<String>,
    #[serde(default)]
    pub license: Option<String>,
    #[serde(default)]
    pub homepage: Option<String>,
    pub runtime: RuntimeKind,
    /// `plugin.wasm`, `recipe.json`, or for sidecars a URL / command.
    pub entry: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Compat {
    /// Semver range of the contract, e.g. `^0.1`.
    pub api: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProvideSpec {
    /// Media kinds handled (metadata/lyrics providers), e.g. `movie`.
    #[serde(default)]
    pub kinds: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigSpec {
    /// JSON Schema file for the settings page.
    #[serde(default)]
    pub schema: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginManifest {
    pub plugin: PluginInfo,
    pub compat: Compat,
    /// Keyed by extension point name; validated into [`ExtensionPoint`]s.
    #[serde(default)]
    pub provides: BTreeMap<String, ProvideSpec>,
    #[serde(default)]
    pub permissions: Permissions,
    #[serde(default)]
    pub config: Option<ConfigSpec>,
}

impl PluginManifest {
    /// Parses and validates `plugin.toml` (without the host compatibility
    /// check, which callers run separately to report it distinctly).
    pub fn parse(text: &str) -> Result<PluginManifest, ManifestError> {
        let manifest: PluginManifest =
            toml::from_str(text).map_err(|e| ManifestError::Parse(e.to_string()))?;
        manifest.validate()?;
        Ok(manifest)
    }

    pub fn validate(&self) -> Result<(), ManifestError> {
        let info = &self.plugin;
        if !is_valid_plugin_id(&info.id) {
            return Err(ManifestError::invalid(
                "plugin.id",
                format!("`{}` must be reverse-DNS, e.g. org.example.tmdb", info.id),
            ));
        }
        let name_len = info.name.trim().chars().count();
        if name_len == 0 || name_len > 64 {
            return Err(ManifestError::invalid(
                "plugin.name",
                "must be 1-64 characters",
            ));
        }
        Version::parse(&info.version).map_err(|e| {
            ManifestError::invalid("plugin.version", format!("`{}`: {e}", info.version))
        })?;
        validate_entry(info.runtime, &info.entry)?;
        if let Some(homepage) = &info.homepage
            && !(homepage.starts_with("https://") || homepage.starts_with("http://"))
        {
            return Err(ManifestError::invalid(
                "plugin.homepage",
                "must be an http(s) URL",
            ));
        }
        compat::parse_range(&self.compat.api)?;

        if self.provides.is_empty() {
            return Err(ManifestError::invalid(
                "provides",
                "a plugin must implement at least one extension point",
            ));
        }
        for (key, spec) in &self.provides {
            if key.parse::<ExtensionPoint>().is_err() {
                return Err(ManifestError::invalid(
                    format!("provides.{key}"),
                    "unknown extension point",
                ));
            }
            for kind in &spec.kinds {
                let ok = !kind.is_empty()
                    && kind
                        .chars()
                        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
                if !ok {
                    return Err(ManifestError::invalid(
                        format!("provides.{key}.kinds"),
                        format!("`{kind}` is not a media kind"),
                    ));
                }
            }
        }

        self.permissions.validate()?;
        // Extension points that need a matching permission.
        let provides = self.extension_points();
        if provides.contains(&ExtensionPoint::HttpEndpoint) && !self.permissions.http_endpoint {
            return Err(ManifestError::invalid(
                "permissions.http-endpoint",
                "required by provides.http-endpoint",
            ));
        }
        if provides.contains(&ExtensionPoint::Schedule) && !self.permissions.schedule {
            return Err(ManifestError::invalid(
                "permissions.schedule",
                "required by provides.schedule",
            ));
        }
        Ok(())
    }

    /// Declared extension points (unknown keys are skipped; `validate`
    /// rejects them).
    pub fn extension_points(&self) -> Vec<ExtensionPoint> {
        self.provides
            .keys()
            .filter_map(|key| key.parse().ok())
            .collect()
    }

    pub fn version(&self) -> Option<Version> {
        Version::parse(&self.plugin.version).ok()
    }
}

/// Reverse-DNS: at least two dot-separated labels of `[a-z0-9-]`.
pub fn is_valid_plugin_id(id: &str) -> bool {
    if id.is_empty() || id.len() > 128 {
        return false;
    }
    let labels: Vec<&str> = id.split('.').collect();
    labels.len() >= 2
        && labels.iter().all(|label| {
            !label.is_empty()
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        })
}

fn validate_entry(runtime: RuntimeKind, entry: &str) -> Result<(), ManifestError> {
    let entry = entry.trim();
    if entry.is_empty() {
        return Err(ManifestError::invalid("plugin.entry", "must not be empty"));
    }
    let required_suffix = match runtime {
        RuntimeKind::None => {
            return Err(ManifestError::invalid(
                "plugin.entry",
                "resource plugins cannot declare an entry point",
            ));
        }
        RuntimeKind::Wasm => Some(".wasm"),
        RuntimeKind::Recipe => Some(".json"),
        RuntimeKind::Sidecar => None,
    };
    if let Some(suffix) = required_suffix {
        let inside_package = !entry.starts_with('/')
            && !entry.contains('\\')
            && !entry.contains(':')
            && entry
                .split('/')
                .all(|c| !c.is_empty() && c != "." && c != "..");
        if !inside_package {
            return Err(ManifestError::invalid(
                "plugin.entry",
                "must be a relative path inside the package",
            ));
        }
        if !entry.ends_with(suffix) {
            return Err(ManifestError::invalid(
                "plugin.entry",
                format!("a {runtime:?} plugin entry must end with {suffix}"),
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::permissions::UserStateAccess;

    const EXAMPLE: &str = r#"
[plugin]
id = "org.example.tmdb"
name = "TMDB"
version = "1.2.0"
description = "从 The Movie Database 获取电影和剧集元数据"
authors = ["Example Author"]
license = "MIT"
homepage = "https://example.org/palveni-tmdb"
runtime = "wasm"
entry = "plugin.wasm"

[compat]
api = "^0.1"

[provides]
metadata = { kinds = ["movie", "series"] }

[permissions]
network = ["api.themoviedb.org", "image.tmdb.org"]
storage = "10MB"
secrets = ["api_key"]

[config]
schema = "config.schema.json"
"#;

    fn with(replace: &str, by: &str) -> String {
        assert!(EXAMPLE.contains(replace), "{replace}");
        EXAMPLE.replacen(replace, by, 1)
    }

    #[test]
    fn parses_the_documented_example() {
        let manifest = PluginManifest::parse(EXAMPLE).unwrap();
        assert_eq!(manifest.plugin.id, "org.example.tmdb");
        assert_eq!(manifest.plugin.runtime, RuntimeKind::Wasm);
        assert_eq!(manifest.extension_points(), vec![ExtensionPoint::Metadata]);
        assert_eq!(manifest.provides["metadata"].kinds, vec!["movie", "series"]);
        assert_eq!(
            manifest.permissions.storage_quota().unwrap(),
            Some(10 * 1024 * 1024)
        );
        assert_eq!(
            manifest.config.unwrap().schema.as_deref(),
            Some("config.schema.json")
        );
        assert_eq!(
            compat::check_compat(&manifest.compat.api)
                .unwrap()
                .to_string(),
            "0.1.0"
        );
    }

    #[test]
    fn rejects_bad_fields() {
        let cases = [
            ("id = \"org.example.tmdb\"", "id = \"tmdb\""),
            ("id = \"org.example.tmdb\"", "id = \"Org.Example\""),
            ("version = \"1.2.0\"", "version = \"1.2\""),
            ("entry = \"plugin.wasm\"", "entry = \"../plugin.wasm\""),
            ("entry = \"plugin.wasm\"", "entry = \"plugin.js\""),
            ("runtime = \"wasm\"", "runtime = \"native\""),
            ("api = \"^0.1\"", "api = \"latest\""),
            ("metadata = {", "teleport = {"),
            ("\"movie\", \"series\"", "\"Movie\""),
            ("storage = \"10MB\"", "storage = \"lots\""),
            (
                "network = [\"api.themoviedb.org\"",
                "network = [\"192.168.1.10\"",
            ),
            (
                "secrets = [\"api_key\"]",
                "secrets = [\"api_key\"]\nshell = true",
            ),
            ("homepage = \"https://", "homepage = \"ftp://"),
        ];
        for (from, to) in cases {
            let text = with(from, to);
            assert!(PluginManifest::parse(&text).is_err(), "{to}");
        }
    }

    #[test]
    fn missing_provides_is_rejected() {
        let text = with(
            "[provides]\nmetadata = { kinds = [\"movie\", \"series\"] }\n",
            "",
        );
        assert!(PluginManifest::parse(&text).is_err());
    }

    #[test]
    fn http_endpoint_needs_permission() {
        let text = with(
            "metadata = { kinds = [\"movie\", \"series\"] }",
            "metadata = { kinds = [\"movie\"] }\nhttp-endpoint = {}",
        );
        assert!(PluginManifest::parse(&text).is_err());
        let granted = text.replace(
            "secrets = [\"api_key\"]",
            "secrets = [\"api_key\"]\nhttp-endpoint = true",
        );
        let manifest = PluginManifest::parse(&granted).unwrap();
        assert!(
            manifest
                .extension_points()
                .contains(&ExtensionPoint::HttpEndpoint)
        );
    }

    #[test]
    fn unknown_informational_keys_are_tolerated() {
        let text = with(
            "license = \"MIT\"",
            "license = \"MIT\"\nfunding = \"https://example.org\"",
        );
        assert!(PluginManifest::parse(&text).is_ok());
    }

    #[test]
    fn sidecar_entries_are_free_form() {
        let text = with("runtime = \"wasm\"", "runtime = \"sidecar\"")
            .replace(
                "entry = \"plugin.wasm\"",
                "entry = \"http://127.0.0.1:9000\"",
            )
            .replace(
                "secrets = [\"api_key\"]",
                "secrets = [\"api_key\"]\nuser-state = \"read\"",
            );
        let manifest = PluginManifest::parse(&text).unwrap();
        assert_eq!(manifest.plugin.runtime, RuntimeKind::Sidecar);
        assert_eq!(manifest.permissions.user_state, Some(UserStateAccess::Read));
    }
}

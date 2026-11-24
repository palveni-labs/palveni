//! Domain types and wire contracts for the Recipe & Widget Plugin Store.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum PluginRuntimeKind {
    None,
    Recipe,
    Widget,
    Wasm,
    Sidecar,
}

impl PluginRuntimeKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Recipe => "recipe",
            Self::Widget => "widget",
            Self::Wasm => "wasm",
            Self::Sidecar => "sidecar",
        }
    }

    pub fn parse(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "recipe" => Self::Recipe,
            "widget" => Self::Widget,
            "wasm" => Self::Wasm,
            "sidecar" => Self::Sidecar,
            _ => Self::None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum PluginRiskLevel {
    Low,
    Medium,
    High,
}

/// Computes an objective risk tier (`low`, `medium`, `high`) from requested permission scopes.
pub fn compute_risk_level(permissions: &[String]) -> PluginRiskLevel {
    let mut has_write = false;
    let mut has_network = false;

    for raw in permissions {
        let p = raw.trim().to_ascii_lowercase();
        if p.starts_with("system:")
            || p.starts_with("fs:")
            || p.starts_with("exec:")
            || p.starts_with("shell:")
            || p == "network:*"
            || p.contains(":delete")
            || p.contains(":admin")
        {
            return PluginRiskLevel::High;
        }
        if p.contains(":write") {
            has_write = true;
        }
        if p.starts_with("network:") {
            has_network = true;
        }
    }

    if permissions.len() >= 4 && has_write && has_network {
        return PluginRiskLevel::High;
    }
    if has_write {
        return PluginRiskLevel::Medium;
    }
    PluginRiskLevel::Low
}

/// Enriched installed plugin summary returned by `GET /api/v1/plugins`,
/// `POST /api/v1/plugins/install`, `PATCH /api/v1/plugins/{id}` and
/// `POST /api/v1/plugins/{id}/rollback`.
///
/// The first six fields match the original M0 contract (`id`, `name`, `version`,
/// `runtime`, `enabled`, `provides`); the enrichment fields remain optional in
/// the schema for wire backwards compatibility.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PluginSummary {
    pub id: String,
    pub name: String,
    pub version: String,
    pub runtime: PluginRuntimeKind,
    pub enabled: bool,
    pub provides: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub permissions: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub granted_permissions: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settings_schema: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settings: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub risk_level: Option<PluginRiskLevel>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<BTreeMap<String, String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub managed: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub installed_at: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PluginList {
    pub plugins: Vec<PluginSummary>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PluginCatalogEntry {
    pub id: String,
    pub name: String,
    pub version: String,
    #[schema(required = true)]
    pub previous_version: Option<String>,
    pub runtime: PluginRuntimeKind,
    pub provides: Vec<String>,
    pub permissions: Vec<String>,
    pub risk_level: PluginRiskLevel,
    pub source: String,
    pub description: BTreeMap<String, String>,
    pub settings_schema: Value,
    pub default_settings: Value,
    pub installed: bool,
    #[schema(required = true)]
    pub installed_version: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PluginCatalogResponse {
    pub plugins: Vec<PluginCatalogEntry>,
}

#[derive(Clone, Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InstallPluginRequest {
    #[schema(min_length = 1, max_length = 128)]
    pub plugin_id: String,
    #[serde(default)]
    pub granted_permissions: Vec<String>,
    pub settings: Option<Value>,
}

#[derive(Clone, Debug, Default, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdatePluginRequest {
    pub enabled: Option<bool>,
    pub settings: Option<Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn risk_level_tiers_are_computed_accurately() {
        assert_eq!(compute_risk_level(&[]), PluginRiskLevel::Low);
        assert_eq!(
            compute_risk_level(&["library:read".into()]),
            PluginRiskLevel::Low
        );
        assert_eq!(
            compute_risk_level(&["network:lrclib.net".into()]),
            PluginRiskLevel::Low
        );
        assert_eq!(
            compute_risk_level(&[
                "network:api.themoviedb.org".into(),
                "library:write_metadata".into()
            ]),
            PluginRiskLevel::Medium
        );
        assert_eq!(
            compute_risk_level(&["fs:write".into()]),
            PluginRiskLevel::High
        );
        assert_eq!(
            compute_risk_level(&["network:*".into()]),
            PluginRiskLevel::High
        );
    }
}

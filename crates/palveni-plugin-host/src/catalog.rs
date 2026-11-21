//! Installed plugins as listed by `GET /plugins`.

use std::sync::{PoisonError, RwLock};

use serde::{Deserialize, Serialize};

use crate::manifest::{PluginManifest, RuntimeKind};

/// The contract's `Plugin` shape.
#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginSummary {
    pub id: String,
    pub name: String,
    pub version: String,
    pub runtime: RuntimeKind,
    pub enabled: bool,
    /// Implemented extension points.
    pub provides: Vec<String>,
}

impl PluginSummary {
    pub fn from_manifest(manifest: &PluginManifest, enabled: bool) -> Self {
        PluginSummary {
            id: manifest.plugin.id.clone(),
            name: manifest.plugin.name.clone(),
            version: manifest.plugin.version.clone(),
            runtime: manifest.plugin.runtime,
            enabled,
            provides: manifest
                .extension_points()
                .into_iter()
                .map(|p| p.key().to_owned())
                .collect(),
        }
    }
}

#[derive(Debug, Default)]
pub struct PluginCatalog {
    installed: RwLock<Vec<PluginSummary>>,
}

impl PluginCatalog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn list(&self) -> Vec<PluginSummary> {
        self.installed
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

//! Executable capabilities registered by the server's enabled HTTP modules.

use std::sync::{Arc, PoisonError, RwLock};

use serde::Serialize;

/// Access level required by the operation's HTTP handler.
#[derive(utoipa::ToSchema, Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
    Public,
    Authenticated,
    Admin,
}

#[derive(utoipa::ToSchema, Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Risk {
    Read,
    Write,
    Destructive,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Capability {
    pub id: &'static str,
    pub module: &'static str,
    pub method: &'static str,
    pub path: &'static str,
    pub permission: Permission,
    pub risk: Risk,
    pub reversible: bool,
    pub description: &'static str,
}

#[derive(Clone, Debug, Default)]
pub struct FeatureRegistry {
    operations: Arc<RwLock<Vec<Capability>>>,
}

impl FeatureRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&self, capability: Capability) {
        let mut operations = self
            .operations
            .write()
            .unwrap_or_else(PoisonError::into_inner);
        if !operations.iter().any(|item| item.id == capability.id) {
            operations.push(capability);
        }
    }

    /// Business modules appear only when they have a mounted endpoint.
    pub fn list(&self) -> Vec<String> {
        let operations = self
            .operations
            .read()
            .unwrap_or_else(PoisonError::into_inner);
        let mut modules = Vec::new();
        for op in operations.iter() {
            if !matches!(op.module, "system" | "auth" | "events") && !modules.contains(&op.module) {
                modules.push(op.module);
            }
        }
        modules.into_iter().map(str::to_owned).collect()
    }

    pub fn capabilities(&self, is_admin: bool) -> Vec<Capability> {
        self.operations
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .iter()
            .filter(|op| is_admin || op.permission != Permission::Admin)
            .cloned()
            .collect()
    }
}

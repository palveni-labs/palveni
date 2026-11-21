//! Runtime interfaces for loading, testing and shutting down plugins.

use std::path::Path;

use async_trait::async_trait;

use crate::manifest::{PluginManifest, RuntimeKind};

#[derive(Debug, thiserror::Error)]
pub enum PluginError {
    #[error("plugin runtime `{0:?}` is not available in this build")]
    RuntimeUnavailable(RuntimeKind),
    #[error("plugin failed to load: {0}")]
    Load(String),
    #[error("plugin self-test failed: {0}")]
    SelfTest(String),
    #[error("plugin call timed out")]
    Timeout,
    #[error("plugin call failed: {0}")]
    Call(String),
}

/// Loads plugins of one runtime kind.
#[async_trait]
pub trait PluginRuntime: Send + Sync {
    fn kind(&self) -> RuntimeKind;

    /// Instantiates an installed, already validated and authorised plugin
    /// from its unpacked package directory.
    async fn load(
        &self,
        manifest: &PluginManifest,
        package_dir: &Path,
    ) -> Result<Box<dyn PluginInstance>, PluginError>;
}

/// A loaded plugin.
#[async_trait]
pub trait PluginInstance: Send + Sync {
    fn id(&self) -> &str;

    /// Run after install/upgrade before switching over.
    async fn self_test(&self) -> Result<(), PluginError>;

    /// Releases resources; the instance is dropped afterwards.
    async fn shutdown(&self);
}

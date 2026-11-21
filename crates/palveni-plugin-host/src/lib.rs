//! Plugin host: manifest model and validation, permission model, contract
//! version compatibility, registry index model and the runtime trait.
//!
//! The server installs data-only skins.

pub mod catalog;
pub mod compat;
pub mod error;
pub mod manifest;
pub mod permissions;
pub mod registry;
pub mod runtime;
pub mod skin_store;
pub mod skin_validation;

pub use catalog::{PluginCatalog, PluginSummary};
pub use compat::{HOST_CONTRACT_VERSION, check_compat, supported_contract_versions};
pub use error::ManifestError;
pub use manifest::{ExtensionPoint, PluginManifest, RuntimeKind};
pub use permissions::{PermissionChange, Permissions};
pub use runtime::{PluginError, PluginInstance, PluginRuntime};

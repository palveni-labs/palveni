//! Domain types, configuration, errors, the in-process event bus and the
//! traits that the higher layers code against.
//!
//! This crate sits at the bottom of the dependency graph: it must not depend
//! on any other workspace crate.

pub mod broadcast;
pub mod cache;
pub mod clock;
pub mod config;
pub mod connections;
pub mod error;
pub mod events;
pub mod features;
pub mod ids;
pub mod model;
pub mod music;
pub mod perf;
pub mod personal;
pub mod photos;
pub mod settings;
pub mod spoken;
pub mod storage;
pub mod video;

pub use config::Config;
pub use error::{Error, Result};
pub use events::{EventBus, ServerEvent};
pub use features::FeatureRegistry;
pub use ids::{
    AuditId, DeviceId, ItemId, JobId, LibraryId, LibraryPathId, PhotoAlbumId, ServerId, SessionId,
    UserId,
};
pub use model::{
    Item, ItemKind, Job, JobState, JobStep, JobSubject, Library, LibraryKind, Role, StepValue, User,
};
pub use perf::{HostInfo, PerfTier};

/// Version of this server build, taken from the workspace manifest.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

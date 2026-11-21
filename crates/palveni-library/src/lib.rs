//! Storage providers, media discovery and incremental audio tag indexing.

pub mod broadcast;
pub mod classify;
pub mod local;
pub mod lyrics;
pub mod remote;
pub mod scanner;
mod tags;
pub mod video;
pub mod watch;

pub use local::LocalStorage;
pub use scanner::{
    SCAN_JOB_KIND, ScanError, ScanJobPayload, ScanObserver, ScanStep, ScanSummary, Scanner,
};

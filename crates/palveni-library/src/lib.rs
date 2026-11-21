//! Storage providers, media discovery and incremental audio tag indexing.

pub mod classify;
pub mod local;
pub mod scanner;
mod tags;

pub use local::LocalStorage;
pub use scanner::{
    SCAN_JOB_KIND, ScanError, ScanJobPayload, ScanObserver, ScanStep, ScanSummary, Scanner,
};

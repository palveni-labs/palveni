//! Storage providers and library scanning.
//!
//! Each recognised local media file becomes a leaf item. Its containing
//! directories are indexed as folders.

pub mod classify;
pub mod local;
pub mod scanner;

pub use local::LocalStorage;
pub use scanner::{
    SCAN_JOB_KIND, ScanError, ScanJobPayload, ScanObserver, ScanStep, ScanSummary, Scanner,
};

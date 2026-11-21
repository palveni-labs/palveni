//! Background jobs.
//!
//! Jobs are persisted in the ops store, so they survive restarts, and run
//! in-process on a tokio runner whose concurrency follows the performance
//! tier. The local runner only claims jobs whose required capabilities it has.
//!
//! State machine: `queued → running → succeeded | failed | cancelled`,
//! plus `running → queued` when a job is interrupted by shutdown.

mod handler;
mod service;

pub use handler::{JobContext, JobError, JobHandler, JobOutcome};
pub use service::{JobService, NewJob};

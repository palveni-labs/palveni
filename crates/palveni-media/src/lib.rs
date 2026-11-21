//! FFmpeg integration.
//!
//! ffmpeg and ffprobe are always external processes started with argument
//! vectors (never through a shell) and a timeout. Nothing about their
//! capabilities is assumed: encoders are only reported as verified after a
//! real short test encode succeeded.

pub mod audio;
pub mod chapters;
mod cover;
pub mod devices;
pub mod discovery;
pub mod encoders;
pub mod loudness;
pub mod parse;
pub mod probe;
pub mod process;
pub mod report;
pub mod service;
pub mod subtitles;

pub use discovery::{LocatedTool, ToolSource};
pub use encoders::EncodeMethod;
pub use report::{
    CpuInfo, EncoderCapability, HardwareReport, ProbeOptions, ToolInfo, probe_hardware,
};
pub use service::HardwareService;

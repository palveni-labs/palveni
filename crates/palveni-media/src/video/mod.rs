//! Video media probing, direct/remux/transcode decision engine, and live HLS sessions.
mod cache;
pub mod decision;
pub mod encoder;
pub mod info;
pub mod session;

pub use decision::{
    ClientProfile, Decision, DecisionError, DecisionRequest, DeliveryMode, HlsSupport,
    RequestedMode, decide,
};
pub use encoder::{
    INIT_SEGMENT_FILE, PLAYLIST_FILE, SEGMENT_SECONDS, SelectedEncoder, is_valid_segment_name,
    rewrite_playlist, segment_content_type, select_h264_encoder,
};
pub use info::{
    AudioStreamInfo, ChapterInfo, HdrFormat, MediaInfo, ProbeFailure, ProbedMedia,
    SubtitleStreamInfo, VideoStreamInfo,
};
pub use session::{
    StartHlsRequest, StartedHlsSession, VideoError, VideoService, remux_limit, transcode_limit,
};

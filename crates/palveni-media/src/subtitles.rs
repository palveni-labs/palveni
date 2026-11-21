//! On-demand extraction of a single text stream from an authorized, seekable file.
use std::{
    fs::File,
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};

use serde::Deserialize;
use tokio::{
    io::AsyncSeekExt,
    process::Command,
    sync::{OnceCell, Semaphore},
};

use crate::{
    discovery::locate,
    process::{FILE_INPUT as INPUT, bounded_output, configure_file_input},
};

const MAX_PROBE_BYTES: usize = 128 * 1024;
const MAX_VTT_BYTES: usize = 2 * 1024 * 1024;
const TEXT_CODECS: [&str; 5] = ["subrip", "ass", "ssa", "mov_text", "webvtt"];

#[derive(Debug)]
pub struct EmbeddedTrack {
    pub index: u32,
    pub language: Option<String>,
    pub forced: bool,
    pub hearing_impaired: bool,
}

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum SubtitleError {
    #[error("Subtitle processing is busy")]
    Busy,
    #[error("Subtitle processing timed out")]
    Timeout,
    #[error("Subtitle stream unavailable")]
    Missing,
    #[error("Invalid or oversized subtitle content")]
    Invalid,
}

pub struct EmbeddedSubtitles {
    tools: Option<Tools>,
    slots: Semaphore,
}

struct Tools {
    ffmpeg: PathBuf,
    ffprobe: PathBuf,
    capabilities: OnceCell<Capabilities>,
}

#[derive(Default)]
struct Capabilities {
    demuxers: String,
    decoders: Vec<String>,
}

impl EmbeddedSubtitles {
    pub(crate) fn new(configured: Option<&Path>) -> Self {
        let tools = locate("ffmpeg", configured)
            .zip(locate("ffprobe", configured))
            .map(|(ffmpeg, ffprobe)| Tools {
                ffmpeg: ffmpeg.path,
                ffprobe: ffprobe.path,
                capabilities: OnceCell::new(),
            });
        Self {
            tools,
            slots: Semaphore::new(2),
        }
    }

    pub async fn list(&self, file: File) -> Result<Vec<EmbeddedTrack>, SubtitleError> {
        let Some(tools) = &self.tools else {
            return Ok(Vec::new());
        };
        let _slot = self.slots.try_acquire().map_err(|_| SubtitleError::Busy)?;
        let capabilities = tools.capabilities().await?;
        tools.tracks(file, capabilities).await
    }

    pub async fn read(&self, file: File, index: u32) -> Result<Vec<u8>, SubtitleError> {
        let tools = self.tools.as_ref().ok_or(SubtitleError::Missing)?;
        let _slot = self.slots.try_acquire().map_err(|_| SubtitleError::Busy)?;
        let capabilities = tools.capabilities().await?;
        let probe_input = file.try_clone().map_err(|_| SubtitleError::Missing)?;
        if !tools
            .tracks(probe_input, capabilities)
            .await?
            .iter()
            .any(|track| track.index == index)
        {
            return Err(SubtitleError::Missing);
        }

        // A cloned descriptor shares the offset advanced by ffprobe.
        let mut file = tokio::fs::File::from_std(file);
        file.rewind().await.map_err(|_| SubtitleError::Missing)?;
        let mut cmd = Command::new(&tools.ffmpeg);
        cmd.args(["-nostdin", "-xerror"]);
        configure_file_input(&mut cmd, &capabilities.demuxers);
        cmd.args([
            "-i",
            &format!("{INPUT}:"),
            "-map",
            &format!("0:{index}"),
            "-map_metadata",
            "-1",
            "-map_chapters",
            "-1",
            "-c:s",
            "webvtt",
            "-f",
            "webvtt",
            "pipe:1",
        ])
        .stdin(Stdio::from(file.into_std().await));
        bounded_output(cmd, MAX_VTT_BYTES, Duration::from_secs(60))
            .await
            .map_err(Into::into)
    }
}

impl Tools {
    async fn capabilities(&self) -> Result<&Capabilities, SubtitleError> {
        self.capabilities
            .get_or_try_init(|| async {
                let protocols = self.listing("-protocols").await?;
                let can_pipe = protocols
                    .split_once("Output:")
                    .is_some_and(|(input, output)| {
                        input.lines().any(|line| line.trim() == INPUT)
                            && output.lines().any(|line| line.trim() == "pipe")
                    });
                if !can_pipe {
                    return Ok(Capabilities::default());
                }
                Ok(capabilities(
                    &self.listing("-formats").await?,
                    &self.listing("-codecs").await?,
                ))
            })
            .await
    }

    async fn listing(&self, flag: &str) -> Result<String, SubtitleError> {
        let mut cmd = Command::new(&self.ffmpeg);
        cmd.args(["-hide_banner", flag]).stdin(Stdio::null());
        match bounded_output(cmd, 256 * 1024, Duration::from_secs(3)).await {
            Ok(output) => Ok(String::from_utf8_lossy(&output).into_owned()),
            Err(err) if err.kind() == std::io::ErrorKind::TimedOut => Err(SubtitleError::Timeout),
            Err(_) => Ok(String::new()),
        }
    }

    async fn tracks(
        &self,
        file: File,
        capabilities: &Capabilities,
    ) -> Result<Vec<EmbeddedTrack>, SubtitleError> {
        if capabilities.decoders.is_empty() || capabilities.demuxers.is_empty() {
            return Ok(Vec::new());
        }
        let mut cmd = Command::new(&self.ffprobe);
        configure_file_input(&mut cmd, &capabilities.demuxers);
        cmd.args(["-nofind_stream_info", "-select_streams", "s", "-show_entries",
            "stream=index,codec_type,codec_name:stream_disposition=forced,hearing_impaired:stream_tags=language",
            "-of", "json", "-i", &format!("{INPUT}:")])
            .stdin(Stdio::from(file));
        match bounded_output(cmd, MAX_PROBE_BYTES, Duration::from_secs(5)).await {
            Ok(output) => Ok(parse_tracks(&output, capabilities)),
            Err(err) if err.kind() == std::io::ErrorKind::TimedOut => Err(SubtitleError::Timeout),
            // Unsupported containers and tool builds leave standalone sidecars usable.
            Err(_) => Ok(Vec::new()),
        }
    }
}

fn capabilities(formats: &str, codecs: &str) -> Capabilities {
    let has_format = |name: &str, flag: char| {
        formats.lines().any(|line| {
            let mut words = line.split_whitespace();
            words.next().is_some_and(|flags| flags.contains(flag))
                && words
                    .next()
                    .is_some_and(|names| names.split(',').any(|s| s == name))
        })
    };
    let has_codec = |name: &str, flag: u8, position: usize| {
        codecs.lines().any(|line| {
            let mut words = line.split_whitespace();
            let flags = words.next().unwrap_or_default().as_bytes();
            flags.get(2) == Some(&b'S')
                && flags.get(position) == Some(&flag)
                && words.next() == Some(name)
        })
    };
    if !has_format("webvtt", 'E') || !has_codec("webvtt", b'E', 1) {
        return Capabilities::default();
    }
    Capabilities {
        demuxers: ["mov", "matroska"]
            .into_iter()
            .filter(|name| has_format(name, 'D'))
            .collect::<Vec<_>>()
            .join(","),
        decoders: TEXT_CODECS
            .into_iter()
            .filter(|name| has_codec(name, b'D', 0))
            .map(str::to_owned)
            .collect(),
    }
}

#[derive(Deserialize)]
struct StreamList {
    streams: Vec<Stream>,
}

#[derive(Deserialize)]
struct Stream {
    index: u32,
    codec_type: Option<String>,
    codec_name: Option<String>,
    #[serde(default)]
    disposition: Disposition,
    #[serde(default)]
    tags: Tags,
}

#[derive(Default, Deserialize)]
struct Tags {
    language: Option<String>,
}

#[derive(Default, Deserialize)]
#[serde(default)]
struct Disposition {
    forced: u8,
    hearing_impaired: u8,
}

fn parse_tracks(output: &[u8], capabilities: &Capabilities) -> Vec<EmbeddedTrack> {
    let Ok(list) = serde_json::from_slice::<StreamList>(output) else {
        return Vec::new();
    };
    list.streams
        .into_iter()
        .filter_map(|stream| {
            if stream.codec_type.as_deref() != Some("subtitle")
                || !capabilities.decoders.contains(&stream.codec_name?)
            {
                return None;
            }
            Some(EmbeddedTrack {
                index: stream.index,
                language: stream.tags.language,
                forced: stream.disposition.forced != 0,
                hearing_impaired: stream.disposition.hearing_impaired != 0,
            })
        })
        .collect()
}

impl From<std::io::Error> for SubtitleError {
    fn from(error: std::io::Error) -> Self {
        match error.kind() {
            std::io::ErrorKind::TimedOut => Self::Timeout,
            std::io::ErrorKind::NotFound => Self::Missing,
            _ => Self::Invalid,
        }
    }
}

#[cfg(test)]
mod tests;

//! Live audio encoding uses a confined, already-open file as FFmpeg's seekable stdin.
mod slots;
use palveni_core::PerfTier;
use slots::Slots;
pub use slots::{CueRequest, PlaybackKey};
use std::{
    fs::File,
    io,
    path::{Path, PathBuf},
    process::Stdio,
    sync::Arc,
    time::Duration,
};
use tokio::{
    io::AsyncReadExt,
    process::Command,
    sync::{OnceCell, mpsc, watch},
    time::{Instant, timeout},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioFormat {
    Pcm,
    Mp3,
    Aac,
    Opus,
}

impl AudioFormat {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Pcm => "pcm",
            Self::Mp3 => "mp3",
            Self::Aac => "aac",
            Self::Opus => "opus",
        }
    }

    pub const fn content_type(self) -> &'static str {
        match self {
            Self::Pcm => "audio/wav",
            Self::Mp3 => "audio/mpeg",
            Self::Aac => "audio/mp4",
            Self::Opus => "audio/ogg",
        }
    }
}

#[derive(Clone, Copy)]
pub struct Profile {
    pub format: AudioFormat,
    pub bitrate_kbps: u16,
    pub start_ms: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum AudioError {
    #[error("Audio transcoding is disabled for this tier")]
    Disabled,
    #[error("All audio encoding slots are in use")]
    Busy,
    #[error("Audio encoder unavailable")]
    Unavailable,
    #[error("This audio format is not available in the configured FFmpeg")]
    UnsupportedFormat,
    #[error("The audio source could not be encoded")]
    Invalid,
}

pub struct AudioTranscoder {
    executable: Option<PathBuf>,
    formats: OnceCell<Vec<AudioFormat>>,
    supports_initial_burst: OnceCell<bool>,
    active: Arc<Slots>,
    shutdown: watch::Sender<bool>,
}

pub type Chunks = mpsc::Receiver<io::Result<Vec<u8>>>;

impl AudioTranscoder {
    pub fn new(configured: Option<&Path>) -> Self {
        Self {
            executable: crate::discovery::locate("ffmpeg", configured).map(|tool| tool.path),
            formats: OnceCell::new(),
            supports_initial_burst: OnceCell::new(),
            active: Arc::default(),
            shutdown: watch::channel(false).0,
        }
    }

    pub fn available(&self, tier: PerfTier) -> Result<(), AudioError> {
        if tier == PerfTier::Tiny {
            return Err(AudioError::Disabled);
        }
        if self.executable.is_none() || *self.shutdown.borrow() {
            return Err(AudioError::Unavailable);
        }
        Ok(())
    }

    /// Cache the capabilities of this server's configured FFmpeg. NAS builds can
    /// ship a smaller set of encoders and protocols than desktop distributions.
    pub async fn check_format(
        &self,
        tier: PerfTier,
        format: AudioFormat,
    ) -> Result<(), AudioError> {
        self.available(tier)?;
        let ffmpeg = self.executable.as_deref().ok_or(AudioError::Unavailable)?;
        let formats = self
            .formats
            .get_or_try_init(|| supported_formats(ffmpeg))
            .await?;
        if formats.contains(&format) {
            Ok(())
        } else {
            Err(AudioError::UnsupportedFormat)
        }
    }

    pub fn shutdown(&self) {
        self.shutdown.send_replace(true);
    }

    /// When set, the input window is measured in samples at 44.1 kHz, independent of the source rate.
    pub async fn start(
        &self,
        tier: PerfTier,
        profile: Profile,
        file: File,
        window: Option<std::ops::Range<u64>>,
        cue: Option<CueRequest>,
    ) -> Result<(Vec<u8>, Chunks), AudioError> {
        self.check_format(tier, profile.format).await?;
        let mut shutdown = self.shutdown.subscribe();
        if *shutdown.borrow() {
            return Err(AudioError::Unavailable);
        }
        let slot = self.active.acquire(tier.job_concurrency(), cue).await?;
        if *shutdown.borrow() {
            return Err(AudioError::Unavailable);
        }
        let count = slot.count;
        let executable = self.executable.as_ref().ok_or(AudioError::Unavailable)?;
        let mut cmd = Command::new(executable);
        let (read_rate, initial_burst) = match (profile.format, window.is_some()) {
            // Some players seek by reading a virtual WAV from byte zero again.
            (AudioFormat::Pcm, true) => ("4", Some("24")),
            (AudioFormat::Aac, _) => ("1.25", Some("8")),
            (_, true) => ("1.25", Some("2")),
            _ => ("1.25", None),
        };
        cmd.args([
            "-nostdin",
            "-hide_banner",
            "-v",
            "error",
            "-threads",
            "1",
            "-filter_threads",
            "1",
            "-protocol_whitelist",
            "fd",
            "-readrate",
            read_rate,
        ]);
        // Warm the decoder/resampler from a whole second before a CUE seek. This
        // keeps arbitrary byte ranges on the same sample grid as a full-track read.
        let anchor = window
            .as_ref()
            .map_or(0, |r| (r.start / 44_100).saturating_sub(1) * 44_100);
        let start_ms = if window.is_some() {
            anchor / 44_100 * 1000
        } else {
            profile.start_ms
        };
        if start_ms > 0 {
            cmd.args(["-ss", &format!("{start_ms}ms")]);
        }
        if let Some(seconds) = initial_burst {
            // Older FFmpeg versions can pace input without supporting a separate initial burst.
            let option = ["-readrate_initial_burst", "8"];
            let supported = self
                .supports_initial_burst
                .get_or_init(|| async {
                    crate::process::run(
                        executable,
                        &crate::process::os_args(option.into_iter().chain(["-version"])),
                        Duration::from_secs(3),
                    )
                    .await
                    .is_ok_and(|output| output.success)
                })
                .await;
            if *supported {
                cmd.args(["-readrate_initial_burst", seconds]);
            }
        }
        cmd.args([
            "-i",
            "fd:",
            "-map",
            "0:a:0",
            "-vn",
            "-sn",
            "-dn",
            "-map_metadata",
            "-1",
            "-map_chapters",
            "-1",
            "-threads",
            "1",
            "-ac",
            "2",
        ]);
        if let Some(window) = window {
            if window.is_empty() {
                return Err(AudioError::Invalid);
            }
            let offset = window.start - anchor;
            let end = window.end - anchor;
            cmd.args([
                "-af",
                &format!("aresample=44100,apad,atrim=start_sample={offset}:end_sample={end},asetpts=PTS-STARTPTS"),
            ]);
        }
        cmd.args([
            "-ar",
            if profile.format == AudioFormat::Pcm {
                "44100"
            } else {
                "48000"
            },
        ]);
        match profile.format {
            AudioFormat::Pcm => {
                cmd.args(["-c:a", "pcm_s16le", "-f", "s16le"]);
            }
            AudioFormat::Mp3 => {
                cmd.args(["-c:a", "libmp3lame", "-f", "mp3", "-write_xing", "0"]);
            }
            AudioFormat::Aac => {
                cmd.args([
                    "-c:a",
                    "aac",
                    "-f",
                    "mp4",
                    "-movflags",
                    "+empty_moov+default_base_moof+frag_keyframe",
                    "-frag_duration",
                    "250000",
                ]);
            }
            AudioFormat::Opus => {
                cmd.args(["-c:a", "libopus", "-f", "ogg", "-page_duration", "200000"]);
            }
        }
        if profile.format != AudioFormat::Pcm {
            cmd.args(["-b:a", &format!("{}k", profile.bitrate_kbps)]);
        }
        cmd.args(["-flush_packets", "1", "pipe:1"]);
        let started = Instant::now();
        let mut child = cmd
            .stdin(Stdio::from(file))
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .map_err(|_| AudioError::Unavailable)?;
        let mut stdout = child.stdout.take().ok_or(AudioError::Unavailable)?;
        let (sender, mut chunks) = mpsc::channel(2);
        tokio::spawn(async move {
            let mut bytes = 0u64;
            let mut first_ms = None;
            let outcome = tokio::select! {
                _ = shutdown.changed() => "shutdown",
                _ = sender.closed() => "disconnected",
                _ = tokio::time::sleep(Duration::from_secs(12 * 3600)) => "timeout",
                result = async {
                    let mut buffer = vec![0; 16 * 1024];
                    loop {
                        let read = stdout.read(&mut buffer).await?;
                        if read == 0 {
                            break;
                        }
                        first_ms.get_or_insert_with(|| {
                            u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX)
                        });
                        bytes += read as u64;
                        if sender.send(Ok(buffer[..read].to_vec())).await.is_err() {
                            return Ok("disconnected");
                        }
                    }
                    let status = child.wait().await?;
                    let outcome = if status.success() && bytes > 0 { "completed" } else { "failed" };
                    Ok::<_, io::Error>(outcome)
                } => result.unwrap_or("failed"),
            };
            if outcome != "completed" {
                let _ = child.kill().await;
                let _ = child.wait().await;
            }
            drop(slot);
            tracing::info!(
                codec = profile.format.name(),
                bitrate_kbps = profile.bitrate_kbps,
                active_at_start = count,
                bytes,
                first_ms,
                elapsed_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
                outcome,
                "audio transcode"
            );
            if !matches!(outcome, "completed" | "disconnected") {
                let _ = sender
                    .send(Err(io::Error::other("Audio encoder stopped")))
                    .await;
            }
        });
        match timeout(Duration::from_secs(15), chunks.recv()).await {
            Ok(Some(Ok(first))) => Ok((first, chunks)),
            _ => Err(AudioError::Invalid),
        }
    }
}

async fn supported_formats(ffmpeg: &Path) -> Result<Vec<AudioFormat>, AudioError> {
    async fn listing(ffmpeg: &Path, option: &str) -> Result<String, AudioError> {
        let result = crate::process::run(
            ffmpeg,
            &crate::process::os_args(["-hide_banner", option]),
            Duration::from_secs(3),
        )
        .await
        .map_err(|_| AudioError::Unavailable)?;
        if result.success {
            Ok(result.stdout)
        } else {
            Err(AudioError::Unavailable)
        }
    }

    let protocols = listing(ffmpeg, "-protocols").await?;
    let (inputs, outputs) = protocols
        .split_once("Output:")
        .ok_or(AudioError::Unavailable)?;
    if !inputs.lines().any(|l| l.trim() == "fd") || !outputs.lines().any(|l| l.trim() == "pipe") {
        return Err(AudioError::Unavailable);
    }
    let encoders = crate::parse::parse_encoders(&listing(ffmpeg, "-encoders").await?);
    let muxers = listing(ffmpeg, "-muxers").await?;
    let formats = [
        (AudioFormat::Mp3, "libmp3lame", "mp3"),
        (AudioFormat::Pcm, "pcm_s16le", "s16le"),
        (AudioFormat::Aac, "aac", "mp4"),
        (AudioFormat::Opus, "libopus", "ogg"),
    ];
    Ok(formats
        .into_iter()
        .filter_map(|(format, encoder, muxer)| {
            let encoded = encoders
                .iter()
                .any(|entry| entry.media_type == 'A' && entry.name == encoder);
            let muxed = muxers.lines().any(|line| {
                let mut fields = line.split_whitespace();
                fields.next() == Some("E")
                    && fields
                        .next()
                        .is_some_and(|names| names.split(',').any(|name| name == muxer))
            });
            (encoded && muxed).then_some(format)
        })
        .collect())
}

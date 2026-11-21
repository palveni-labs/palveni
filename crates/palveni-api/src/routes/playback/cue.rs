//! A CUE entry is exposed as its own seekable PCM WAVE file. Byte offsets, including
//! unaligned ranges issued by browsers, are relative to this track.
use std::{fs::File, io, ops::Range};

use super::{ApiError, AppState, Body, transcode};
use futures_util::{StreamExt, stream};
use palveni_media::audio::{AudioFormat, CueRequest, Profile};

const HEADER: usize = 44;
const SAMPLES_PER_FRAME: u64 = 588;
const BYTES_PER_SAMPLE_PAIR: u64 = 4;

pub(in crate::routes) struct Audio {
    file: File,
    first_sample: u64,
    payload_bytes: u32,
}

impl Audio {
    pub(super) fn new(file: File, frames: Range<u64>) -> palveni_core::Result<Self> {
        let first_sample = frames.start.checked_mul(SAMPLES_PER_FRAME);
        let size = frames
            .end
            .checked_sub(frames.start)
            .and_then(|n| n.checked_mul(SAMPLES_PER_FRAME * BYTES_PER_SAMPLE_PAIR))
            .and_then(|n| u32::try_from(n).ok())
            .filter(|n| *n > 0 && *n <= u32::MAX - 36);
        match (first_sample, size) {
            (Some(first_sample), Some(payload_bytes)) => Ok(Self {
                file,
                first_sample,
                payload_bytes,
            }),
            _ => Err(palveni_core::Error::invalid("Invalid CUE track bounds")),
        }
    }

    pub(super) fn len(&self) -> u64 {
        HEADER as u64 + u64::from(self.payload_bytes)
    }

    pub(super) async fn body(
        self,
        state: &AppState,
        range: Range<u64>,
        cue: CueRequest,
    ) -> Result<Body, ApiError> {
        let header = wav_header(self.payload_bytes);
        let header_end = usize::try_from(range.end).unwrap_or(HEADER).min(HEADER);
        let prefix =
            header[usize::try_from(range.start).unwrap_or(HEADER).min(HEADER)..header_end].to_vec();
        if range.end <= HEADER as u64 {
            return Ok(Body::from(prefix));
        }
        let offset = range.start.saturating_sub(HEADER as u64);
        let payload_len = range.end - HEADER as u64 - offset;
        let skip = offset % BYTES_PER_SAMPLE_PAIR;
        let start = self.first_sample + offset / BYTES_PER_SAMPLE_PAIR;
        let count = (skip + payload_len).div_ceil(BYTES_PER_SAMPLE_PAIR);
        let profile = Profile {
            format: AudioFormat::Pcm,
            bitrate_kbps: 0,
            start_ms: 0,
        };
        let (first, rest) = state
            .hardware
            .audio
            .start(
                transcode::tier(state).await,
                profile,
                self.file,
                Some(start..start + count),
                Some(cue),
            )
            .await
            .map_err(transcode::problem)?;
        let input = stream::iter([Ok(first)]).chain(stream::unfold(rest, |mut rx| async move {
            rx.recv().await.map(|chunk| (chunk, rx))
        }));
        let body = stream::try_unfold(
            (Box::pin(input), skip, payload_len),
            |(mut input, mut skip, remaining)| async move {
                if remaining == 0 {
                    return Ok(None);
                }
                loop {
                    let chunk = input.next().await.ok_or_else(|| {
                        io::Error::new(io::ErrorKind::UnexpectedEof, "CUE audio ended early")
                    })??;
                    let discard = usize::try_from(skip).unwrap_or(usize::MAX).min(chunk.len());
                    skip -= discard as u64;
                    let mut data = chunk;
                    data.drain(..discard);
                    if data.is_empty() {
                        continue;
                    }
                    let n = data
                        .len()
                        .min(usize::try_from(remaining).unwrap_or(usize::MAX));
                    data.truncate(n);
                    let remaining = remaining - n as u64;
                    if remaining == 0 {
                        // Complete the encoder before acknowledging the last byte. A player can
                        // issue its next range immediately when Content-Length has been met.
                        while let Some(tail) = input.next().await {
                            tail?;
                        }
                    }
                    return Ok::<_, io::Error>(Some((data, (input, skip, remaining))));
                }
            },
        );
        Ok(Body::from_stream(stream::iter([Ok(prefix)]).chain(body)))
    }
}

fn wav_header(bytes: u32) -> [u8; HEADER] {
    let mut value=*b"RIFF\x00\x00\x00\x00WAVEfmt \x10\x00\x00\x00\x01\x00\x02\x00\x44\xac\x00\x00\x10\xb1\x02\x00\x04\x00\x10\x00data\x00\x00\x00\x00";
    value[4..8].copy_from_slice(&(36 + bytes).to_le_bytes());
    value[40..44].copy_from_slice(&bytes.to_le_bytes());
    value
}

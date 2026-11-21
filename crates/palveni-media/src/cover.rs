//! Bounded, single-frame album thumbnail rendering for compatible music clients.
use crate::{HardwareService, discovery};
use std::{io, process::Stdio, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    process::Command,
};

const MAX_OUTPUT: usize = 4 * 1024 * 1024;

impl HardwareService {
    pub async fn cover_thumbnail(&self, data: &[u8], mime: &str, size: u16) -> io::Result<Vec<u8>> {
        let codec = match mime {
            "image/png" => "png",
            "image/jpeg" => "mjpeg",
            "image/webp" => "webp",
            _ => return Err(io::Error::other("unsupported cover format")),
        };
        if !(1..=1024).contains(&size) {
            return Err(io::Error::other("invalid cover size"));
        }
        let configured = self.configured_ffmpeg().map(ToOwned::to_owned);
        let tool =
            tokio::task::spawn_blocking(move || discovery::locate("ffmpeg", configured.as_deref()))
                .await?
                .ok_or_else(|| io::Error::other("ffmpeg is unavailable"))?;
        let scale = format!("scale={size}:{size}:force_original_aspect_ratio=decrease");
        let mut child = Command::new(&tool.path)
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-nostdin",
                "-max_alloc",
                "33554432",
                "-threads",
                "1",
                "-max_pixels",
                "25000000",
                "-protocol_whitelist",
                "pipe",
                "-f",
                "image2pipe",
                "-c:v",
                codec,
                "-i",
                "pipe:0",
                "-an",
                "-frames:v",
                "1",
                "-filter_threads",
                "1",
                "-vf",
                &scale,
                "-threads",
                "1",
                "-c:v",
                "mjpeg",
                "-q:v",
                "4",
                "-f",
                "image2pipe",
                "pipe:1",
            ])
            .kill_on_drop(true)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;
        let mut input = child
            .stdin
            .take()
            .ok_or_else(|| io::Error::other("missing ffmpeg input"))?;
        let output = child
            .stdout
            .take()
            .ok_or_else(|| io::Error::other("missing ffmpeg output"))?;
        let write = async move {
            input.write_all(data).await?;
            input.shutdown().await
        };
        let read = async move {
            let mut bytes = Vec::new();
            output
                .take((MAX_OUTPUT + 1) as u64)
                .read_to_end(&mut bytes)
                .await?;
            if bytes.len() > MAX_OUTPUT || !bytes.starts_with(&[0xff, 0xd8, 0xff]) {
                return Err(io::Error::other("invalid thumbnail output"));
            }
            Ok(bytes)
        };
        tokio::time::timeout(Duration::from_secs(5), async {
            let ((), bytes, status) = tokio::try_join!(write, read, child.wait())?;
            if !status.success() {
                return Err(io::Error::other("image conversion failed"));
            }
            Ok(bytes)
        })
        .await
        .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "cover conversion timed out"))?
    }
}

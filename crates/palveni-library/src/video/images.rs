//! Video artwork resizing, disk caching, and FFmpeg frame-grabbed episode thumbnails.
use std::{
    fs::File,
    io::{self, Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    process::Stdio,
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};

use palveni_core::{Error, Result, clock, storage::ByteStream};
use palveni_db::{
    music::TrackLocation,
    video::{VideoThumbLocation, sidecar_fingerprint},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    process::Command,
    sync::Semaphore,
};

use super::MAX_POSTER;

const MAX_RESIZED_BYTES: usize = 4 * 1024 * 1024;
const BASE_THUMB_WIDTH: u16 = 960;
const FILE_INPUT: &str = if cfg!(windows) { "handle" } else { "fd" };
static TMP_SEQ: AtomicU64 = AtomicU64::new(0);

/// Snaps a requested pixel width to one of the contract's fixed buckets.
pub fn snap_size(requested: u16) -> u16 {
    const BUCKETS: [u16; 5] = [96, 240, 480, 960, 1920];
    BUCKETS
        .into_iter()
        .find(|&bucket| bucket >= requested)
        .unwrap_or(1920)
}

pub struct VideoImages {
    cache_dir: PathBuf,
    ffmpeg: Option<PathBuf>,
    ffprobe: Option<PathBuf>,
    workers: Semaphore,
}

impl VideoImages {
    pub fn new(cache_dir: PathBuf, ffmpeg: Option<PathBuf>, ffprobe: Option<PathBuf>) -> Self {
        Self {
            cache_dir,
            ffmpeg,
            ffprobe,
            workers: Semaphore::new(2),
        }
    }

    pub async fn open_image(
        &self,
        location: TrackLocation,
        size: Option<u16>,
    ) -> Result<(&'static str, u64, ByteStream)> {
        let Some(requested) = size else {
            return super::open_poster(location).await;
        };
        let bucket = snap_size(requested);
        let key_prefix = format!(
            "img-v1\0{}\0{}",
            location.root.display(),
            location.relative_path.display()
        );
        let (root, path) = crate::local::track_path(location).await?;
        let opened = tokio::task::spawn_blocking(move || open_local_image(&root, &path))
            .await
            .map_err(|e| Error::internal(e.to_string()))??;

        let cache_key = sidecar_fingerprint(&format!(
            "{key_prefix}\0{}\0{}\0{bucket}",
            opened.size, opened.modified_ms
        ));
        let cache_path = self.cache_path("resized", &cache_key);
        if let Ok(cached) = open_cached_jpeg(&cache_path).await {
            return Ok(cached);
        }

        let Some(ffmpeg) = &self.ffmpeg else {
            return stream_opened_file(opened.file, opened.mime, opened.size);
        };

        let _permit = self
            .workers
            .acquire()
            .await
            .map_err(|e| Error::internal(e.to_string()))?;
        if let Ok(cached) = open_cached_jpeg(&cache_path).await {
            return Ok(cached);
        }

        let mime = opened.mime;
        let orig_size = opened.size;
        let (raw, mut file) = tokio::task::spawn_blocking(move || {
            let mut buf = Vec::with_capacity(usize::try_from(orig_size).unwrap_or(0));
            let mut f = opened.file;
            f.read_to_end(&mut buf)?;
            f.rewind()?;
            Ok::<_, io::Error>((buf, f))
        })
        .await
        .map_err(|e| Error::internal(e.to_string()))??;

        match resize_image(ffmpeg, &raw, mime, bucket).await {
            Ok(resized) => {
                let len = resized.len() as u64;
                if write_atomic(&cache_path, &resized).await.is_ok()
                    && let Ok(cached) = open_cached_jpeg(&cache_path).await
                {
                    return Ok(cached);
                }
                let cursor = io::Cursor::new(resized);
                Ok(("image/jpeg", len, Box::pin(cursor) as ByteStream))
            }
            Err(_) => {
                let _ = file.rewind();
                stream_opened_file(file, mime, orig_size)
            }
        }
    }

    pub async fn open_or_grab_thumb(
        &self,
        loc: VideoThumbLocation,
        size: Option<u16>,
    ) -> Result<(&'static str, u64, ByteStream)> {
        if let Some(local) = loc.local_thumb
            && let Ok(img) = self.open_image(local, size).await
        {
            return Ok(img);
        }
        if super::strm::is_strm_path(&loc.video.relative_path) {
            return Err(Error::not_found("thumb"));
        }

        let Some(ffmpeg) = &self.ffmpeg else {
            return Err(Error::not_found("thumb"));
        };
        let key_prefix = format!(
            "thumb-v1\0{}\0{}",
            loc.video.root.display(),
            loc.video.relative_path.display()
        );
        let (root, path) = crate::local::track_path(loc.video).await?;
        let (file, video_size, modified_ms) = tokio::task::spawn_blocking(move || {
            let f = root.open_file_blocking(&path)?;
            let meta = f.metadata()?;
            let modified_ms = meta
                .modified()
                .ok()
                .map(|t| clock::to_millis(clock::from_system_time(t)))
                .unwrap_or(0);
            Ok::<_, Error>((f, meta.len(), modified_ms))
        })
        .await
        .map_err(|e| Error::internal(e.to_string()))??;

        let base_key = sidecar_fingerprint(&format!("{key_prefix}\0{video_size}\0{modified_ms}"));
        let base_path = self.cache_path("thumbs", &base_key);

        if tokio::fs::metadata(&base_path).await.is_err() {
            let _permit = self
                .workers
                .acquire()
                .await
                .map_err(|e| Error::internal(e.to_string()))?;
            if tokio::fs::metadata(&base_path).await.is_err() {
                let duration_ms = match loc.runtime_ms.filter(|&ms| ms > 0) {
                    Some(ms) => Some(ms),
                    None => match &self.ffprobe {
                        Some(ffprobe) => match file.try_clone() {
                            Ok(cloned) => probe_duration_ms(ffprobe, cloned).await,
                            Err(_) => None,
                        },
                        None => None,
                    },
                };
                let seek_secs = duration_ms
                    .filter(|&ms| ms > 0)
                    .map(|ms| (ms as f64 / 1000.0) * 0.10)
                    .filter(|s| s.is_finite() && *s > 0.0)
                    .unwrap_or(0.0);

                let frame = grab_frame_with_fallback(ffmpeg, file, seek_secs)
                    .await
                    .ok_or_else(|| Error::not_found("thumb"))?;
                if let Err(err) = write_atomic(&base_path, &frame).await {
                    tracing::warn!(error = %err, "cannot cache grabbed video thumbnail");
                    let bytes = match size.map(snap_size).filter(|&b| b < BASE_THUMB_WIDTH) {
                        Some(bucket) => resize_image(ffmpeg, &frame, "image/jpeg", bucket)
                            .await
                            .unwrap_or(frame),
                        None => frame,
                    };
                    let len = bytes.len() as u64;
                    return Ok((
                        "image/jpeg",
                        len,
                        Box::pin(io::Cursor::new(bytes)) as ByteStream,
                    ));
                }
            }
        }

        if let Some(bucket) = size.map(snap_size).filter(|&b| b < BASE_THUMB_WIDTH) {
            let resized_key = sidecar_fingerprint(&format!("{base_key}\0{bucket}"));
            let resized_path = self.cache_path("resized", &resized_key);
            if let Ok(cached) = open_cached_jpeg(&resized_path).await {
                return Ok(cached);
            }
            let _permit = self
                .workers
                .acquire()
                .await
                .map_err(|e| Error::internal(e.to_string()))?;
            if let Ok(cached) = open_cached_jpeg(&resized_path).await {
                return Ok(cached);
            }
            if let Ok(base_bytes) = tokio::fs::read(&base_path).await
                && let Ok(resized) = resize_image(ffmpeg, &base_bytes, "image/jpeg", bucket).await
            {
                let _ = write_atomic(&resized_path, &resized).await;
                if let Ok(cached) = open_cached_jpeg(&resized_path).await {
                    return Ok(cached);
                }
                let len = resized.len() as u64;
                return Ok((
                    "image/jpeg",
                    len,
                    Box::pin(io::Cursor::new(resized)) as ByteStream,
                ));
            }
        }

        open_cached_jpeg(&base_path)
            .await
            .map_err(|_| Error::not_found("thumb"))
    }

    /// Extracts and caches a 320px-wide JPEG trickplay frame for timeline hover scrubbing.
    /// Gracefully falls back to existing thumbnails or returns `NotFound` for `.strm` or
    /// when FFmpeg is unavailable.
    pub async fn grab_trickplay_frame(
        &self,
        loc: VideoThumbLocation,
        at_ms: u64,
    ) -> Result<(&'static str, u64, ByteStream)> {
        if super::strm::is_strm_path(&loc.video.relative_path) {
            if let Some(local) = loc.local_thumb
                && let Ok(img) = self.open_image(local, Some(320)).await
            {
                return Ok(img);
            }
            return Err(Error::not_found("trickplay"));
        }

        let Some(ffmpeg) = &self.ffmpeg else {
            return self.open_or_grab_thumb(loc, Some(320)).await;
        };

        let clamped_sec = (at_ms / 1000).min(7 * 24 * 3600);
        let key_prefix = format!(
            "trickplay-v1\0{}\0{}",
            loc.video.root.display(),
            loc.video.relative_path.display()
        );
        let local_thumb = loc.local_thumb;
        let (root, path) = crate::local::track_path(loc.video).await?;
        let (file, video_size, modified_ms) = tokio::task::spawn_blocking(move || {
            let f = root.open_file_blocking(&path)?;
            let meta = f.metadata()?;
            let modified_ms = meta
                .modified()
                .ok()
                .map(|t| clock::to_millis(clock::from_system_time(t)))
                .unwrap_or(0);
            Ok::<_, Error>((f, meta.len(), modified_ms))
        })
        .await
        .map_err(|e| Error::internal(e.to_string()))??;

        let cache_key = sidecar_fingerprint(&format!(
            "{key_prefix}\0{video_size}\0{modified_ms}\0{clamped_sec}"
        ));
        let cache_path = self.cache_path("trickplay", &cache_key);
        if let Ok(cached) = open_cached_jpeg(&cache_path).await {
            return Ok(cached);
        }

        let _permit = self
            .workers
            .acquire()
            .await
            .map_err(|e| Error::internal(e.to_string()))?;
        if let Ok(cached) = open_cached_jpeg(&cache_path).await {
            return Ok(cached);
        }

        match grab_frame_scaled(ffmpeg, file, clamped_sec as f64, 320, "6").await {
            Ok(bytes) => {
                let len = bytes.len() as u64;
                if write_atomic(&cache_path, &bytes).await.is_ok()
                    && let Ok(cached) = open_cached_jpeg(&cache_path).await
                {
                    return Ok(cached);
                }
                Ok((
                    "image/jpeg",
                    len,
                    Box::pin(io::Cursor::new(bytes)) as ByteStream,
                ))
            }
            Err(_) => {
                if let Some(local) = local_thumb
                    && let Ok(img) = self.open_image(local, Some(320)).await
                {
                    return Ok(img);
                }
                Err(Error::not_found("trickplay"))
            }
        }
    }

    pub async fn open_photo(
        &self,
        location: TrackLocation,
        size: Option<u16>,
        original: bool,
    ) -> Result<(&'static str, u64, ByteStream)> {
        let key_prefix = format!(
            "photo-v1\0{}\0{}",
            location.root.display(),
            location.relative_path.display()
        );
        let file_name = location
            .relative_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("photo.jpg")
            .to_owned();
        let (root, path) = crate::local::track_path(location).await?;
        let (mut file, orig_size, modified_ms, mime) = tokio::task::spawn_blocking(move || {
            let mut f = root.open_file_blocking(&path)?;
            let meta = f.metadata()?;
            let len = meta.len();
            if len == 0 {
                return Err(Error::not_found("photo"));
            }
            let mut header = [0u8; 16];
            let hlen = usize::try_from(len.min(16)).unwrap_or(16);
            f.read_exact(&mut header[..hlen])?;
            f.rewind()?;
            let mime = crate::photos::exif::detect_photo_mime(&header[..hlen], &file_name)
                .unwrap_or("application/octet-stream");
            let modified_ms = meta
                .modified()
                .ok()
                .map(|t| clock::to_millis(clock::from_system_time(t)))
                .unwrap_or(0);
            Ok::<_, Error>((f, len, modified_ms, mime))
        })
        .await
        .map_err(|e| Error::internal(e.to_string()))??;

        if original {
            return stream_opened_file(file, mime, orig_size);
        }

        let bucket = snap_size(size.unwrap_or(1920));
        let cache_key = sidecar_fingerprint(&format!(
            "{key_prefix}\0{orig_size}\0{modified_ms}\0{bucket}"
        ));
        let cache_path = self.cache_path("photos", &cache_key);
        if let Ok(cached) = open_cached_jpeg(&cache_path).await {
            return Ok(cached);
        }

        if let Some(ffmpeg) = &self.ffmpeg {
            let _permit = self
                .workers
                .acquire()
                .await
                .map_err(|e| Error::internal(e.to_string()))?;
            if let Ok(cached) = open_cached_jpeg(&cache_path).await {
                return Ok(cached);
            }
            if let Ok(cloned) = file.try_clone()
                && let Ok(resized) = resize_photo_fd(ffmpeg, cloned, bucket).await
            {
                let len = resized.len() as u64;
                if write_atomic(&cache_path, &resized).await.is_ok()
                    && let Ok(cached) = open_cached_jpeg(&cache_path).await
                {
                    return Ok(cached);
                }
                let cursor = io::Cursor::new(resized);
                return Ok(("image/jpeg", len, Box::pin(cursor) as ByteStream));
            }
        }

        let _ = file.seek(SeekFrom::Start(0));
        if matches!(
            mime,
            "image/jpeg" | "image/png" | "image/webp" | "image/gif" | "image/bmp" | "image/avif"
        ) {
            stream_opened_file(file, mime, orig_size)
        } else {
            Err(Error::not_found("photo preview"))
        }
    }

    fn cache_path(&self, kind: &str, key: &str) -> PathBuf {
        let shard = key.get(..2).unwrap_or("00");
        self.cache_dir
            .join(kind)
            .join(shard)
            .join(format!("{key}.jpg"))
    }
}

struct OpenedImage {
    file: File,
    mime: &'static str,
    size: u64,
    modified_ms: i64,
}

fn open_local_image(
    root: &crate::LocalStorage,
    path: &palveni_core::storage::StoragePath,
) -> Result<OpenedImage> {
    let mut file = root.open_file_blocking(path)?;
    let meta = file.metadata()?;
    let size = meta.len();
    if !(3..=MAX_POSTER).contains(&size) {
        return Err(Error::not_found("image"));
    }
    let mut header = [0u8; 12];
    let bytes = &mut header[..usize::try_from(size.min(12)).unwrap_or(12)];
    file.read_exact(bytes)?;
    let mime = crate::artwork::mime(bytes).ok_or_else(|| Error::not_found("image"))?;
    file.rewind()?;
    let modified_ms = meta
        .modified()
        .ok()
        .map(|t| clock::to_millis(clock::from_system_time(t)))
        .unwrap_or(0);
    Ok(OpenedImage {
        file,
        mime,
        size,
        modified_ms,
    })
}

fn stream_opened_file(
    file: File,
    mime: &'static str,
    size: u64,
) -> Result<(&'static str, u64, ByteStream)> {
    let reader = AsyncReadExt::take(tokio::fs::File::from_std(file), size);
    Ok((mime, size, Box::pin(reader) as ByteStream))
}

async fn open_cached_jpeg(path: &Path) -> io::Result<(&'static str, u64, ByteStream)> {
    let file = tokio::fs::File::open(path).await?;
    let size = file.metadata().await?.len();
    if size < 4 {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "empty cache"));
    }
    let reader = AsyncReadExt::take(file, size);
    Ok(("image/jpeg", size, Box::pin(reader) as ByteStream))
}

async fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let seq = TMP_SEQ.fetch_add(1, Ordering::Relaxed);
    let tmp = path.with_extension(format!("tmp.{}.{seq}", std::process::id()));
    let mut file = tokio::fs::File::create(&tmp).await?;
    file.write_all(bytes).await?;
    file.flush().await?;
    drop(file);
    if let Err(err) = tokio::fs::rename(&tmp, path).await {
        let _ = tokio::fs::remove_file(&tmp).await;
        return Err(err);
    }
    Ok(())
}

async fn resize_image(ffmpeg: &Path, data: &[u8], mime: &str, width: u16) -> io::Result<Vec<u8>> {
    let codec = match mime {
        "image/png" => "png",
        "image/jpeg" => "mjpeg",
        "image/webp" => "webp",
        _ => return Err(io::Error::other("unsupported image format")),
    };
    let vf = format!("scale=min(iw\\,{width}):-1,format=yuvj420p");
    let mut child = Command::new(ffmpeg)
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
            &vf,
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
            .take((MAX_RESIZED_BYTES + 1) as u64)
            .read_to_end(&mut bytes)
            .await?;
        if bytes.len() > MAX_RESIZED_BYTES || !bytes.starts_with(&[0xff, 0xd8, 0xff]) {
            return Err(io::Error::other("invalid resized image output"));
        }
        Ok(bytes)
    };
    tokio::time::timeout(Duration::from_secs(8), async {
        let ((), bytes, status) = tokio::try_join!(write, read, child.wait())?;
        if !status.success() {
            return Err(io::Error::other("image resize failed"));
        }
        Ok(bytes)
    })
    .await
    .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "image resize timed out"))?
}

async fn grab_frame_with_fallback(
    ffmpeg: &Path,
    mut file: File,
    seek_secs: f64,
) -> Option<Vec<u8>> {
    if seek_secs > 0.0
        && let Ok(cloned) = file.try_clone()
        && let Ok(bytes) = grab_frame(ffmpeg, cloned, seek_secs).await
    {
        return Some(bytes);
    }
    let _ = file.seek(SeekFrom::Start(0));
    grab_frame(ffmpeg, file, 0.0).await.ok()
}

async fn grab_frame(ffmpeg: &Path, file: File, seek_secs: f64) -> io::Result<Vec<u8>> {
    grab_frame_scaled(ffmpeg, file, seek_secs, BASE_THUMB_WIDTH, "4").await
}

async fn grab_frame_scaled(
    ffmpeg: &Path,
    file: File,
    seek_secs: f64,
    max_width: u16,
    quality: &str,
) -> io::Result<Vec<u8>> {
    let vf = format!("scale=min(iw\\,{max_width}):-2,format=yuvj420p");
    let seek = format!("{seek_secs:.3}");
    let input = format!("{FILE_INPUT}:");
    let mut cmd = Command::new(ffmpeg);
    cmd.args([
        "-hide_banner",
        "-loglevel",
        "error",
        "-nostdin",
        "-max_alloc",
        "67108864",
        "-threads",
        "1",
        "-protocol_whitelist",
        FILE_INPUT,
    ]);
    if seek_secs > 0.0 {
        cmd.args(["-ss", &seek]);
    }
    cmd.args([
        "-i",
        &input,
        "-an",
        "-sn",
        "-dn",
        "-frames:v",
        "1",
        "-filter_threads",
        "1",
        "-vf",
        &vf,
        "-threads",
        "1",
        "-c:v",
        "mjpeg",
        "-q:v",
        quality,
        "-f",
        "image2pipe",
        "pipe:1",
    ])
    .kill_on_drop(true)
    .stdin(Stdio::from(file))
    .stdout(Stdio::piped())
    .stderr(Stdio::null());

    let mut child = cmd.spawn()?;
    let output = child
        .stdout
        .take()
        .ok_or_else(|| io::Error::other("missing ffmpeg output"))?;
    let run = async {
        let mut bytes = Vec::new();
        output
            .take((MAX_RESIZED_BYTES + 1) as u64)
            .read_to_end(&mut bytes)
            .await?;
        let status = child.wait().await?;
        if !status.success()
            || bytes.len() > MAX_RESIZED_BYTES
            || !bytes.starts_with(&[0xff, 0xd8, 0xff])
        {
            return Err(io::Error::other("frame grab failed"));
        }
        Ok(bytes)
    };
    tokio::time::timeout(Duration::from_secs(10), run)
        .await
        .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "frame grab timed out"))?
}

async fn resize_photo_fd(ffmpeg: &Path, file: File, width: u16) -> io::Result<Vec<u8>> {
    let vf = format!("scale=min(iw\\,{width}):-1,format=yuvj420p");
    let input = format!("{FILE_INPUT}:");
    let mut child = Command::new(ffmpeg)
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-nostdin",
            "-max_alloc",
            "134217728",
            "-threads",
            "1",
            "-max_pixels",
            "120000000",
            "-protocol_whitelist",
            FILE_INPUT,
            "-i",
            &input,
            "-an",
            "-sn",
            "-dn",
            "-frames:v",
            "1",
            "-filter_threads",
            "1",
            "-vf",
            &vf,
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
        .stdin(Stdio::from(file))
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    let output = child
        .stdout
        .take()
        .ok_or_else(|| io::Error::other("missing ffmpeg output"))?;
    let run = async {
        let mut bytes = Vec::new();
        output
            .take((MAX_RESIZED_BYTES + 1) as u64)
            .read_to_end(&mut bytes)
            .await?;
        let status = child.wait().await?;
        if !status.success()
            || bytes.len() > MAX_RESIZED_BYTES
            || !bytes.starts_with(&[0xff, 0xd8, 0xff])
        {
            return Err(io::Error::other("photo resize failed"));
        }
        Ok(bytes)
    };
    tokio::time::timeout(Duration::from_secs(10), run)
        .await
        .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "photo resize timed out"))?
}

/// Cheap container-header duration probe via `ffprobe` on an open file descriptor.
pub(crate) async fn probe_duration_ms(ffprobe: &Path, file: File) -> Option<i64> {
    let input = format!("{FILE_INPUT}:");
    let mut child = Command::new(ffprobe)
        .args([
            "-v",
            "error",
            "-threads",
            "1",
            "-max_alloc",
            "67108864",
            "-probesize",
            "262144",
            "-analyzeduration",
            "250000",
            "-protocol_whitelist",
            FILE_INPUT,
            "-show_entries",
            "format=duration",
            "-of",
            "default=nw=1:nk=1",
            "-i",
            &input,
        ])
        .kill_on_drop(true)
        .stdin(Stdio::from(file))
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let output = child.stdout.take()?;
    let run = async {
        let mut buf = Vec::new();
        output.take(128).read_to_end(&mut buf).await.ok()?;
        let status = child.wait().await.ok()?;
        if !status.success() {
            return None;
        }
        let text = std::str::from_utf8(&buf).ok()?.trim();
        let secs: f64 = text.parse().ok()?;
        if !secs.is_finite() || secs <= 0.0 || secs > (7 * 24 * 3600) as f64 {
            return None;
        }
        #[allow(clippy::cast_possible_truncation)]
        let ms = (secs * 1000.0).round() as i64;
        (ms > 0).then_some(ms)
    };
    tokio::time::timeout(Duration::from_secs(2), run)
        .await
        .ok()
        .flatten()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snaps_sizes_to_contract_buckets() {
        assert_eq!(snap_size(1), 96);
        assert_eq!(snap_size(96), 96);
        assert_eq!(snap_size(97), 240);
        assert_eq!(snap_size(240), 240);
        assert_eq!(snap_size(300), 480);
        assert_eq!(snap_size(960), 960);
        assert_eq!(snap_size(1200), 1920);
        assert_eq!(snap_size(3840), 1920);
    }
}

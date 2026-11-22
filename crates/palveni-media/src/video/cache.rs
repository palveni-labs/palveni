//! File-fingerprint JSON cache for `ffprobe` video media info.
use std::{
    fs::File,
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

use palveni_core::ItemId;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::info::ProbedMedia;

const CACHE_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct FileFingerprint {
    pub size: u64,
    pub mtime_secs: i64,
    pub mtime_nanos: u32,
}

impl FileFingerprint {
    pub(crate) fn from_file(file: &File) -> Option<Self> {
        let meta = file.metadata().ok()?;
        let modified = meta.modified().ok()?;
        let (mtime_secs, mtime_nanos) = match modified.duration_since(UNIX_EPOCH) {
            Ok(delta) => (i64::try_from(delta.as_secs()).ok()?, delta.subsec_nanos()),
            Err(err) => {
                let delta = err.duration();
                let secs = i64::try_from(delta.as_secs()).ok()?;
                if delta.subsec_nanos() == 0 {
                    (-secs, 0)
                } else {
                    (-secs.checked_add(1)?, 1_000_000_000 - delta.subsec_nanos())
                }
            }
        };
        Some(Self {
            size: meta.len(),
            mtime_secs,
            mtime_nanos,
        })
    }
}

#[derive(Serialize, Deserialize)]
struct CachedEntry {
    version: u32,
    fingerprint: FileFingerprint,
    media: ProbedMedia,
}

pub(crate) struct MediaInfoCache {
    dir: PathBuf,
}

impl MediaInfoCache {
    pub(crate) fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    fn path(&self, item: ItemId) -> PathBuf {
        self.dir.join(format!("{item}.json"))
    }

    pub(crate) async fn load(
        &self,
        item: ItemId,
        fingerprint: FileFingerprint,
    ) -> Option<ProbedMedia> {
        let bytes = tokio::fs::read(self.path(item)).await.ok()?;
        let entry: CachedEntry = serde_json::from_slice(&bytes).ok()?;
        if entry.version == CACHE_VERSION && entry.fingerprint == fingerprint {
            Some(entry.media)
        } else {
            None
        }
    }

    pub(crate) async fn store(
        &self,
        item: ItemId,
        fingerprint: FileFingerprint,
        media: &ProbedMedia,
    ) {
        let Ok(payload) = serde_json::to_vec(&CachedEntry {
            version: CACHE_VERSION,
            fingerprint,
            media: media.clone(),
        }) else {
            return;
        };
        if let Err(err) = tokio::fs::create_dir_all(&self.dir).await {
            tracing::debug!(dir = %self.dir.display(), error = %err, "cannot create media-info cache directory");
            return;
        }
        let target = self.path(item);
        let tmp = self.dir.join(format!("{item}.{}.tmp", Uuid::now_v7()));
        if let Err(err) = write_atomic(&tmp, &target, &payload).await {
            tracing::debug!(path = %target.display(), error = %err, "cannot write media-info cache");
            let _ = tokio::fs::remove_file(&tmp).await;
        }
    }
}

async fn write_atomic(tmp: &Path, target: &Path, payload: &[u8]) -> std::io::Result<()> {
    tokio::fs::write(tmp, payload).await?;
    tokio::fs::rename(tmp, target).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::video::info::{MediaInfo, VideoStreamInfo};

    fn sample_media() -> ProbedMedia {
        ProbedMedia {
            info: MediaInfo {
                container: Some("mp4".into()),
                duration_ms: Some(12_000),
                size_bytes: Some(1_024),
                bitrate_kbps: Some(680),
                video: Some(VideoStreamInfo {
                    codec: "h264".into(),
                    profile: Some("High".into()),
                    width: 1280,
                    height: 720,
                    frame_rate: Some(24.0),
                    bit_depth: Some(8),
                    hdr: None,
                }),
                audio: vec![],
                subtitles: vec![],
                chapters: vec![],
            },
            video_index: Some(0),
            start_time_ms: 0,
        }
    }

    #[tokio::test]
    async fn cache_hits_matching_fingerprint_and_misses_on_change_or_corruption() {
        let dir = tempfile::tempdir().unwrap();
        let cache = MediaInfoCache::new(dir.path().join("media-info"));
        let item = ItemId::generate();
        let fp = FileFingerprint {
            size: 1024,
            mtime_secs: 1_700_000_000,
            mtime_nanos: 123_456,
        };
        let media = sample_media();

        assert!(cache.load(item, fp).await.is_none());
        cache.store(item, fp, &media).await;
        assert_eq!(cache.load(item, fp).await, Some(media.clone()));

        let changed_size = FileFingerprint { size: 2048, ..fp };
        assert!(cache.load(item, changed_size).await.is_none());

        let changed_mtime = FileFingerprint {
            mtime_nanos: fp.mtime_nanos + 1,
            ..fp
        };
        assert!(cache.load(item, changed_mtime).await.is_none());

        tokio::fs::write(cache.path(item), b"{broken")
            .await
            .unwrap();
        assert!(cache.load(item, fp).await.is_none());
    }
}

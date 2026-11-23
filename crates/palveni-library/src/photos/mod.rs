//! Photos library indexing, EXIF extraction, and direct photo upload.

pub mod exif;

use std::collections::HashMap;
use std::io::Write as _;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use chrono::Datelike;
use palveni_core::events::{EventBus, ServerEvent};
use palveni_core::photos::PhotoInfo;
use palveni_core::storage::StoragePath;
use palveni_core::{ItemId, ItemKind, LibraryId, LibraryKind, UserId, clock};
use palveni_db::items::ItemUpsert;
use palveni_db::photos::PhotoMetadataUpsert;
use palveni_db::{Db, DbError};

use crate::classify::{classify, is_ignored_name, search_text, sort_key, title_for_file};
use crate::{LocalStorage, ScanError, ScanObserver, ScanStep};

static UPLOAD_TMP_SEQ: AtomicU64 = AtomicU64::new(0);
const PHOTO_BATCH_SIZE: usize = 64;
pub const MAX_PHOTO_UPLOAD_BYTES: usize = 25 * 1024 * 1024;

#[derive(Debug, thiserror::Error)]
pub enum PhotoUploadError {
    #[error("{0} not found")]
    NotFound(&'static str),
    #[error("{0}")]
    Invalid(String),
    #[error("{0}")]
    ReadOnly(String),
    #[error("photo exceeds maximum upload size")]
    TooLarge,
    #[error(transparent)]
    Db(#[from] DbError),
    #[error("{0}")]
    Internal(String),
}

pub fn photo_stamp(size_bytes: Option<i64>, modified_at: Option<i64>) -> String {
    format!(
        "photo-v1:{}:{}",
        size_bytes.unwrap_or(0),
        modified_at.unwrap_or(0)
    )
}

fn storage_rel_path(root: &LocalStorage, full_path: &Path) -> Option<StoragePath> {
    let rel = full_path.strip_prefix(root.root()).ok()?;
    let mut sp = StoragePath::root();
    for comp in rel.components() {
        let name = comp.as_os_str().to_str()?;
        sp = sp.join(name).ok()?;
    }
    Some(sp)
}

fn build_metadata_upsert(
    item_id: ItemId,
    stamp: String,
    parsed: exif::ParsedPhoto,
    content_hash: Option<String>,
    fallback_ms: i64,
) -> PhotoMetadataUpsert {
    let fallback_dt = clock::from_millis(fallback_ms).unwrap_or_else(clock::now);
    let taken_at = parsed.taken_at_ms.unwrap_or(fallback_ms);
    let day_bucket = parsed
        .day_bucket
        .unwrap_or_else(|| fallback_dt.format("%Y-%m-%d").to_string());
    let month_bucket = parsed
        .month_bucket
        .unwrap_or_else(|| fallback_dt.format("%Y-%m").to_string());

    PhotoMetadataUpsert {
        item_id,
        stamp,
        taken_at,
        day_bucket,
        month_bucket,
        width: parsed.width.and_then(|v| i32::try_from(v).ok()),
        height: parsed.height.and_then(|v| i32::try_from(v).ok()),
        orientation: i32::from(parsed.orientation.clamp(1, 8)),
        camera_make: parsed.camera_make,
        camera_model: parsed.camera_model,
        lens_model: parsed.lens_model,
        iso: parsed.iso,
        f_number: parsed.f_number,
        exposure_time: parsed.exposure_time,
        focal_length: parsed.focal_length,
        latitude: parsed.latitude,
        longitude: parsed.longitude,
        content_hash,
    }
}

fn read_photo_file(
    item_id: ItemId,
    stamp: String,
    root: &LocalStorage,
    rel: &StoragePath,
    fallback_ms: i64,
) -> PhotoMetadataUpsert {
    match root.open_file_blocking(rel) {
        Ok(mut file) => {
            let file_size = file.metadata().map_or(0, |m| m.len());
            let parsed = exif::parse_photo_reader(&mut file, file_size);
            let content_hash = exif::compute_content_hash(&mut file, file_size).ok();
            build_metadata_upsert(item_id, stamp, parsed, content_hash, fallback_ms)
        }
        Err(_) => build_metadata_upsert(
            item_id,
            stamp,
            exif::ParsedPhoto {
                orientation: 1,
                ..exif::ParsedPhoto::default()
            },
            None,
            fallback_ms,
        ),
    }
}

pub(crate) async fn index_library(
    db: &Db,
    library_id: LibraryId,
    roots: Arc<Vec<LocalStorage>>,
    observer: &dyn ScanObserver,
    allowed: impl Fn(&str) -> bool,
) -> Result<(), ScanError> {
    let candidates = db.photos().scan_index(library_id).await?;
    let now_ms = clock::to_millis(clock::now());
    let mut pending = Vec::new();

    for cand in candidates {
        if observer.is_cancelled() {
            return Err(ScanError::Cancelled);
        }
        if !allowed(&cand.path) {
            continue;
        }
        let expected_stamp = photo_stamp(cand.size_bytes, cand.modified_at);
        if cand.stamp.as_deref() == Some(expected_stamp.as_str()) {
            continue;
        }
        let full = Path::new(&cand.path);
        let Some((root_idx, root)) = roots
            .iter()
            .enumerate()
            .find(|(_, r)| full.starts_with(r.root()))
        else {
            continue;
        };
        let Some(rel) = storage_rel_path(root, full) else {
            continue;
        };
        let fallback_ms = cand.modified_at.unwrap_or(now_ms);
        pending.push((cand.id, expected_stamp, root_idx, rel, fallback_ms));
    }

    let total = pending.len();
    if total == 0 {
        return Ok(());
    }

    let mut done = 0usize;
    for chunk in pending.chunks(PHOTO_BATCH_SIZE) {
        if observer.is_cancelled() {
            return Err(ScanError::Cancelled);
        }
        #[allow(clippy::cast_precision_loss)]
        let frac = done as f64 / total as f64;
        observer.progress(
            Some(frac),
            &ScanStep::Indexing {
                done: u64::try_from(done).unwrap_or(u64::MAX),
                total: u64::try_from(total).unwrap_or(u64::MAX),
            },
        );

        let batch = chunk.to_vec();
        let roots_ref = roots.clone();
        let upserts = tokio::task::spawn_blocking(move || {
            batch
                .into_iter()
                .map(|(id, stamp, root_idx, rel, fallback_ms)| {
                    read_photo_file(id, stamp, &roots_ref[root_idx], &rel, fallback_ms)
                })
                .collect::<Vec<_>>()
        })
        .await
        .map_err(|e| ScanError::Metadata(e.to_string()))?;

        db.photos().upsert_batch(library_id, &upserts).await?;
        done += chunk.len();
    }

    Ok(())
}

fn sanitize_upload_name(raw_name: &str, bytes: &[u8]) -> Result<String, PhotoUploadError> {
    let base = raw_name
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or("")
        .trim()
        .trim_start_matches('.');
    let cleaned: String = base
        .chars()
        .map(|ch| {
            if ch.is_control() || matches!(ch, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|')
            {
                '_'
            } else {
                ch
            }
        })
        .collect();

    let mut candidate = if cleaned.is_empty() {
        "photo".to_owned()
    } else {
        cleaned
    };

    if classify(LibraryKind::Photo, &candidate).is_none() {
        let inferred_ext = match exif::detect_photo_mime(bytes, &candidate) {
            Some("image/jpeg") => Some("jpg"),
            Some("image/png") => Some("png"),
            Some("image/webp") => Some("webp"),
            Some("image/gif") => Some("gif"),
            Some("image/bmp") => Some("bmp"),
            Some("image/avif") => Some("avif"),
            Some("image/heic") => Some("heic"),
            Some("image/tiff") => Some("tiff"),
            Some("image/x-adobe-dng") => Some("dng"),
            Some("image/jxl") => Some("jxl"),
            _ => None,
        };
        if let Some(ext) = inferred_ext {
            candidate = format!("{candidate}.{ext}");
        }
    }

    if candidate.chars().count() > 200
        || is_ignored_name(&candidate)
        || classify(LibraryKind::Photo, &candidate) != Some(ItemKind::Photo)
    {
        return Err(PhotoUploadError::Invalid(
            "Unsupported photo filename or extension".into(),
        ));
    }
    Ok(candidate)
}

fn is_readonly_io(err: &std::io::Error) -> bool {
    matches!(
        err.kind(),
        std::io::ErrorKind::PermissionDenied | std::io::ErrorKind::ReadOnlyFilesystem
    ) || matches!(err.raw_os_error(), Some(13 | 30))
}

pub async fn upload_photo(
    db: &Db,
    bus: &EventBus,
    library_id: LibraryId,
    user_id: UserId,
    include_host_path: bool,
    raw_file_name: &str,
    bytes: Vec<u8>,
) -> Result<PhotoInfo, PhotoUploadError> {
    if bytes.len() > MAX_PHOTO_UPLOAD_BYTES {
        return Err(PhotoUploadError::TooLarge);
    }
    if bytes.len() < 12 || !exif::has_valid_image_magic(&bytes) {
        return Err(PhotoUploadError::Invalid(
            "Uploaded data is not a recognized image format".into(),
        ));
    }

    let library = db
        .libraries()
        .get(library_id)
        .await?
        .ok_or(PhotoUploadError::NotFound("Library"))?;
    if library.kind != LibraryKind::Photo {
        return Err(PhotoUploadError::Invalid(
            "Uploads are only supported for photo libraries".into(),
        ));
    }
    let first_root =
        library.paths.first().cloned().ok_or_else(|| {
            PhotoUploadError::ReadOnly("Photo library has no writable path".into())
        })?;

    let safe_name = sanitize_upload_name(raw_file_name, &bytes)?;
    let storage = LocalStorage::open_registered(Path::new(&first_root))
        .await
        .map_err(|_| PhotoUploadError::ReadOnly("Photo library root is unavailable".into()))?;

    let now = clock::now();
    let yyyy = format!("{:04}", now.year());
    let mm = format!("{:02}", now.month());
    let root_path = storage.root().to_path_buf();

    let (written_path, final_name, size_i64, modified_ms, parsed, content_hash) =
        tokio::task::spawn_blocking(move || -> Result<_, PhotoUploadError> {
            let target_dir = root_path.join("Uploads").join(&yyyy).join(&mm);
            if let Err(err) = std::fs::create_dir_all(&target_dir) {
                if is_readonly_io(&err) {
                    return Err(PhotoUploadError::ReadOnly(
                        "Photo library storage is read-only".into(),
                    ));
                }
                return Err(PhotoUploadError::ReadOnly(format!(
                    "Cannot create Uploads directory: {err}"
                )));
            }

            let (stem, ext) = safe_name
                .rsplit_once('.')
                .map_or((safe_name.as_str(), "jpg"), |(s, e)| (s, e));
            let mut chosen_name = safe_name.clone();
            let mut dest = target_dir.join(&chosen_name);
            let mut suffix = 1u32;
            while dest.exists() && suffix < 10_000 {
                chosen_name = format!("{stem}-{suffix}.{ext}");
                dest = target_dir.join(&chosen_name);
                suffix += 1;
            }

            let seq = UPLOAD_TMP_SEQ.fetch_add(1, Ordering::Relaxed);
            let tmp = target_dir.join(format!(".palveni-upload-{}-{seq}.tmp", std::process::id()));
            let write_res = (|| -> std::io::Result<()> {
                let mut f = std::fs::File::create(&tmp)?;
                f.write_all(&bytes)?;
                f.sync_all()?;
                drop(f);
                std::fs::rename(&tmp, &dest)
            })();
            if let Err(err) = write_res {
                let _ = std::fs::remove_file(&tmp);
                if is_readonly_io(&err) {
                    return Err(PhotoUploadError::ReadOnly(
                        "Photo library storage is read-only".into(),
                    ));
                }
                return Err(PhotoUploadError::ReadOnly(format!(
                    "Cannot write photo to library: {err}"
                )));
            }

            let meta =
                std::fs::metadata(&dest).map_err(|e| PhotoUploadError::Internal(e.to_string()))?;
            let size_i64 = i64::try_from(meta.len()).unwrap_or(i64::MAX);
            let modified_ms = meta
                .modified()
                .ok()
                .map(|t| clock::to_millis(clock::from_system_time(t)))
                .unwrap_or_else(|| clock::to_millis(now));

            let mut cursor = std::io::Cursor::new(&bytes);
            let len_u64 = bytes.len() as u64;
            let parsed = exif::parse_photo_reader(&mut cursor, len_u64);
            let content_hash = exif::compute_content_hash(&mut cursor, len_u64).ok();
            let dest_str = dest
                .to_str()
                .ok_or_else(|| PhotoUploadError::Internal("non-UTF-8 upload path".into()))?
                .to_owned();

            Ok((
                dest_str,
                chosen_name,
                size_i64,
                modified_ms,
                parsed,
                content_hash,
            ))
        })
        .await
        .map_err(|e| PhotoUploadError::Internal(e.to_string()))??;

    let existing = db.items().scan_index(library_id).await?;
    let by_path: HashMap<&str, ItemId> = existing
        .iter()
        .map(|item| (item.path.as_str(), item.id))
        .collect();

    let dir_uploads = storage
        .root()
        .join("Uploads")
        .to_string_lossy()
        .into_owned();
    let dir_year = storage
        .root()
        .join("Uploads")
        .join(format!("{:04}", now.year()))
        .to_string_lossy()
        .into_owned();
    let dir_month = storage
        .root()
        .join("Uploads")
        .join(format!("{:04}", now.year()))
        .join(format!("{:02}", now.month()))
        .to_string_lossy()
        .into_owned();

    let id_uploads = by_path
        .get(dir_uploads.as_str())
        .copied()
        .unwrap_or_else(ItemId::generate);
    let id_year = by_path
        .get(dir_year.as_str())
        .copied()
        .unwrap_or_else(ItemId::generate);
    let id_month = by_path
        .get(dir_month.as_str())
        .copied()
        .unwrap_or_else(ItemId::generate);
    let item_id = by_path
        .get(written_path.as_str())
        .copied()
        .unwrap_or_else(ItemId::generate);

    let yyyy_str = format!("{:04}", now.year());
    let mm_str = format!("{:02}", now.month());
    let title = title_for_file(&final_name);

    let upserts = vec![
        ItemUpsert {
            music: None,
            id: id_uploads,
            parent_id: None,
            kind: ItemKind::Folder,
            title: "Uploads".into(),
            sort_title: sort_key("Uploads"),
            search_text: search_text("Uploads"),
            path: dir_uploads,
            size_bytes: None,
            modified_at: None,
        },
        ItemUpsert {
            music: None,
            id: id_year,
            parent_id: Some(id_uploads),
            kind: ItemKind::Folder,
            title: yyyy_str.clone(),
            sort_title: sort_key(&yyyy_str),
            search_text: search_text(&yyyy_str),
            path: dir_year,
            size_bytes: None,
            modified_at: None,
        },
        ItemUpsert {
            music: None,
            id: id_month,
            parent_id: Some(id_year),
            kind: ItemKind::Folder,
            title: mm_str.clone(),
            sort_title: sort_key(&mm_str),
            search_text: search_text(&mm_str),
            path: dir_month,
            size_bytes: None,
            modified_at: None,
        },
        ItemUpsert {
            music: None,
            id: item_id,
            parent_id: Some(id_month),
            kind: ItemKind::Photo,
            sort_title: sort_key(&title),
            search_text: search_text(&title),
            title,
            path: written_path,
            size_bytes: Some(size_i64),
            modified_at: Some(modified_ms),
        },
    ];

    db.items()
        .apply_scan_batch(library_id, &upserts, &[])
        .await?;
    let stamp = photo_stamp(Some(size_i64), Some(modified_ms));
    let meta_upsert = build_metadata_upsert(item_id, stamp, parsed, content_hash, modified_ms);
    db.photos().upsert_batch(library_id, &[meta_upsert]).await?;

    if let Some(updated_lib) = db.libraries().get(library_id).await? {
        bus.publish(ServerEvent::LibraryUpdated(updated_lib));
    }

    db.photos()
        .photo_info(user_id, item_id, include_host_path)
        .await?
        .ok_or(PhotoUploadError::NotFound("Photo"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Scanner;
    use palveni_core::Role;
    use palveni_db::DbOptions;
    use palveni_db::libraries::NewLibrary;

    struct NoopObserver;
    impl ScanObserver for NoopObserver {
        fn progress(&self, _fraction: Option<f64>, _step: &ScanStep) {}
    }

    #[tokio::test]
    async fn scanner_indexes_photo_metadata_incrementally_and_upload_indexes_immediately() {
        let dir = tempfile::tempdir().unwrap();
        let db_dir = tempfile::tempdir().unwrap();
        let db_base = db_dir.path().display().to_string();
        let db = Db::open(&DbOptions {
            main_url: format!("sqlite://{db_base}/main.db?mode=rwc"),
            ops_url: format!("sqlite://{db_base}/ops.db?mode=rwc"),
            max_connections: Some(4),
            min_connections: Some(1),
        })
        .await
        .unwrap();

        let user = db.users().create("owner", "pw", Role::Admin).await.unwrap();
        let canon = dir.path().canonicalize().unwrap();
        // Minimal PNG 16x9
        let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
        png.extend_from_slice(&13u32.to_be_bytes());
        png.extend_from_slice(b"IHDR");
        png.extend_from_slice(&1600u32.to_be_bytes());
        png.extend_from_slice(&900u32.to_be_bytes());
        png.extend_from_slice(&[8, 6, 0, 0, 0]);
        png.extend_from_slice(&0u32.to_be_bytes());
        png.extend_from_slice(&0u32.to_be_bytes());
        png.extend_from_slice(b"IEND");
        png.extend_from_slice(&0u32.to_be_bytes());
        std::fs::write(canon.join("sunset.png"), &png).unwrap();

        let lib = db
            .libraries()
            .create(NewLibrary {
                name: "Camera Roll".into(),
                kind: LibraryKind::Photo,
                paths: vec![canon.to_str().unwrap().to_owned()],
            })
            .await
            .unwrap();

        let bus = EventBus::new(16, 0, || async { Ok::<_, std::convert::Infallible>(1) });
        let scanner = Scanner::new(db.clone(), bus.clone());
        let summary1 = scanner.scan(lib.id, &NoopObserver).await.unwrap();
        assert_eq!(summary1.added, 1);

        // Second scan sees unchanged stamp
        let summary2 = scanner.scan(lib.id, &NoopObserver).await.unwrap();
        assert_eq!(summary2.unchanged, 1);

        let page = db
            .photos()
            .timeline(
                user.id,
                &palveni_db::photos::PhotoTimelineQuery {
                    limit: 10,
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        assert_eq!(page.total, 1);
        assert_eq!(page.items[0].width, Some(1600));
        assert_eq!(page.items[0].height, Some(900));

        // Immediate upload into Uploads/YYYY/MM
        let uploaded = upload_photo(&db, &bus, lib.id, user.id, false, "vacation.png", png)
            .await
            .unwrap();
        assert_eq!(uploaded.width, Some(1600));
        assert!(uploaded.path.starts_with("Uploads/"));
        assert!(uploaded.content_hash.is_some());

        db.close().await.unwrap();
    }
}

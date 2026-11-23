//! Reading catalog indexer and on-demand archive/ebook/comic/text reader.

pub mod archive;
pub mod comic;
pub mod epub;
#[cfg(test)]
mod tests;
pub mod text;

pub use archive::normalize_archive_path as normalize_archive_path_public;

use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

use palveni_core::reading::{ReadingChapter, ReadingChapterSummary, ReadingKind};
use palveni_core::storage::{StoragePath, StorageProvider};
use palveni_core::{ItemId, LibraryId};
use palveni_db::reading::{ReadingItemLocation, ReadingMetadataUpsert, series_key_for};
use palveni_db::{Db, DbError};
use tokio::sync::Semaphore;

use crate::classify::{classify, extension_of, sort_key};
use crate::local::{LocalStorage, TrackFile};
use crate::{ScanError, ScanObserver};

use self::archive::{ArchiveError, MAX_ENTRY_BYTES};
use self::comic::{detect_image_mime, open_comic_archive, read_validated_image};
use self::epub::{epub_chapter_summaries, open_epub, read_epub_asset, read_epub_chapter};
use self::text::{chapter_summaries, parse_text_book, read_decoded_text, render_text_chapter};

const MAX_SIDECAR_COVER: u64 = 12 * 1024 * 1024;
const INDEX_BATCH_SIZE: usize = 32;
static READER_PERMITS: Semaphore = Semaphore::const_new(4);

#[derive(Debug, thiserror::Error)]
pub enum ReadingError {
    #[error("reading item not found")]
    NotFound,
    #[error("archive or document format `{0}` is not supported for online page/chapter streaming")]
    UnsupportedFormat(String),
    #[error("corrupt or unreadable book/comic: {0}")]
    Corrupt(String),
    #[error("internal error: {0}")]
    Internal(String),
}

impl From<DbError> for ReadingError {
    fn from(err: DbError) -> Self {
        match err {
            DbError::NotFound(_) => Self::NotFound,
            other => Self::Internal(other.to_string()),
        }
    }
}

impl From<ArchiveError> for ReadingError {
    fn from(err: ArchiveError) -> Self {
        match err {
            ArchiveError::UnsupportedCompression(m) => {
                Self::UnsupportedFormat(format!("zip-method-{m}"))
            }
            other => Self::Corrupt(other.to_string()),
        }
    }
}

pub fn stamp_for(size_bytes: Option<i64>, modified_at: Option<i64>) -> String {
    format!(
        "reading-v1:{}:{}",
        size_bytes.unwrap_or(-1),
        modified_at.unwrap_or(-1)
    )
}

pub fn reading_media_type(name_or_format: &str, kind: ReadingKind) -> &'static str {
    let ext =
        extension_of(name_or_format).unwrap_or_else(|| name_or_format.trim().to_ascii_lowercase());
    match ext.as_str() {
        "epub" => "application/epub+zip",
        "pdf" => "application/pdf",
        "txt" => "text/plain; charset=utf-8",
        "md" => "text/markdown; charset=utf-8",
        "mobi" => "application/x-mobipocket-ebook",
        "azw" | "azw3" => "application/vnd.amazon.ebook",
        "djvu" => "image/vnd.djvu",
        "fb2" => "application/x-fictionbook+xml",
        "cbz" => "application/vnd.comicbook+zip",
        "cbr" => "application/vnd.comicbook-rar",
        "cbt" => "application/vnd.comicbook+tar",
        "cb7" => "application/x-cb7",
        "zip" if kind == ReadingKind::Comic => "application/vnd.comicbook+zip",
        "zip" => "application/zip",
        "tar" => "application/x-tar",
        "rar" => "application/vnd.rar",
        "7z" => "application/x-7z-compressed",
        _ => "application/octet-stream",
    }
}

/// Incrementally indexes new, changed, and pre-existing unindexed book/comic items during scan.
pub async fn index_library(
    db: &Db,
    library_id: LibraryId,
    observer: &dyn ScanObserver,
) -> Result<(), ScanError> {
    db.reading().prune_tombstoned(library_id).await?;
    let candidates = db.reading().library_candidates(library_id).await?;
    let pending: Vec<ReadingItemLocation> = candidates
        .into_iter()
        .filter(|c| {
            let expected = stamp_for(c.size_bytes, c.modified_at);
            c.existing_stamp.as_deref() != Some(expected.as_str())
        })
        .collect();

    if pending.is_empty() {
        return Ok(());
    }

    for chunk in pending.chunks(INDEX_BATCH_SIZE) {
        if observer.is_cancelled() {
            return Err(ScanError::Cancelled);
        }
        let mut batch_inputs = Vec::with_capacity(chunk.len());
        for c in chunk {
            batch_inputs.push(CloneableCandidate::from_loc(c));
        }
        let extracted = tokio::task::spawn_blocking(move || {
            let mut out = Vec::with_capacity(batch_inputs.len());
            for item in batch_inputs {
                if let Some(meta) = extract_metadata_blocking(&item) {
                    out.push(meta);
                }
            }
            out
        })
        .await
        .map_err(|e| ScanError::Metadata(e.to_string()))?;

        if !extracted.is_empty() {
            db.reading().upsert_batch(&extracted).await?;
        }
    }

    Ok(())
}

/// Ensures `reading_metadata` is populated for `item_id` (on-demand backfill for pre-existing items).
pub async fn ensure_indexed(db: &Db, item_id: ItemId) -> Result<ReadingItemLocation, ReadingError> {
    let loc = db
        .reading()
        .item_location(item_id)
        .await?
        .ok_or(ReadingError::NotFound)?;
    let expected = stamp_for(loc.size_bytes, loc.modified_at);
    if loc.existing_stamp.as_deref() != Some(expected.as_str()) {
        let candidate = CloneableCandidate::from_loc(&loc);
        let maybe_meta = tokio::task::spawn_blocking(move || extract_metadata_blocking(&candidate))
            .await
            .map_err(|e| ReadingError::Internal(e.to_string()))?;
        if let Some(meta) = maybe_meta {
            db.reading().upsert_batch(&[meta]).await?;
        }
    }
    Ok(loc)
}

pub async fn list_chapters(
    db: &Db,
    item_id: ItemId,
) -> Result<Vec<ReadingChapterSummary>, ReadingError> {
    let loc = ensure_indexed(db, item_id).await?;
    let format = extension_of(&loc.file_path).unwrap_or_default();
    if !matches!(format.as_str(), "epub" | "txt" | "md") {
        return Ok(Vec::new());
    }
    let _permit = READER_PERMITS
        .acquire()
        .await
        .map_err(|e| ReadingError::Internal(e.to_string()))?;
    let fallback_title = loc.title.clone();
    let (storage, rel_path) = crate::local::track_path(loc.location)
        .await
        .map_err(|_| ReadingError::NotFound)?;

    tokio::task::spawn_blocking(move || {
        let file = storage
            .open_file_blocking(&rel_path)
            .map_err(|_| ReadingError::NotFound)?;
        let reader = BufReader::new(file);
        match format.as_str() {
            "epub" => {
                let (_, book) = open_epub(reader)?;
                Ok(epub_chapter_summaries(&book))
            }
            "txt" | "md" => {
                let text =
                    read_decoded_text(reader).map_err(|e| ReadingError::Corrupt(e.to_string()))?;
                let parsed = parse_text_book(&text, format == "md", &fallback_title);
                Ok(chapter_summaries(&parsed))
            }
            _ => Ok(Vec::new()),
        }
    })
    .await
    .map_err(|e| ReadingError::Internal(e.to_string()))?
}

pub async fn read_chapter(
    db: &Db,
    item_id: ItemId,
    index: u32,
) -> Result<ReadingChapter, ReadingError> {
    let loc = ensure_indexed(db, item_id).await?;
    let format = extension_of(&loc.file_path).unwrap_or_default();
    if !matches!(format.as_str(), "epub" | "txt" | "md") {
        return Err(ReadingError::UnsupportedFormat(format));
    }
    let idx = usize::try_from(index).map_err(|_| ReadingError::NotFound)?;
    let _permit = READER_PERMITS
        .acquire()
        .await
        .map_err(|e| ReadingError::Internal(e.to_string()))?;
    let fallback_title = loc.title.clone();
    let (storage, rel_path) = crate::local::track_path(loc.location)
        .await
        .map_err(|_| ReadingError::NotFound)?;

    tokio::task::spawn_blocking(move || {
        let file = storage
            .open_file_blocking(&rel_path)
            .map_err(|_| ReadingError::NotFound)?;
        let reader = BufReader::new(file);
        match format.as_str() {
            "epub" => {
                let (mut zip, book) = open_epub(reader)?;
                read_epub_chapter(&mut zip, &book, idx, item_id)?.ok_or(ReadingError::NotFound)
            }
            "txt" | "md" => {
                let text =
                    read_decoded_text(reader).map_err(|e| ReadingError::Corrupt(e.to_string()))?;
                let is_md = format == "md";
                let parsed = parse_text_book(&text, is_md, &fallback_title);
                render_text_chapter(&parsed, idx, is_md).ok_or(ReadingError::NotFound)
            }
            _ => Err(ReadingError::UnsupportedFormat(format)),
        }
    })
    .await
    .map_err(|e| ReadingError::Internal(e.to_string()))?
}

pub async fn read_asset(
    db: &Db,
    item_id: ItemId,
    asset_path: &str,
) -> Result<(&'static str, Vec<u8>), ReadingError> {
    let loc = ensure_indexed(db, item_id).await?;
    let format = extension_of(&loc.file_path).unwrap_or_default();
    if format != "epub" {
        return Err(ReadingError::NotFound);
    }
    let _permit = READER_PERMITS
        .acquire()
        .await
        .map_err(|e| ReadingError::Internal(e.to_string()))?;
    let asset = asset_path.to_owned();
    let (storage, rel_path) = crate::local::track_path(loc.location)
        .await
        .map_err(|_| ReadingError::NotFound)?;

    tokio::task::spawn_blocking(move || {
        let file = storage
            .open_file_blocking(&rel_path)
            .map_err(|_| ReadingError::NotFound)?;
        let mut zip = archive::ZipArchive::open(BufReader::new(file))?;
        read_epub_asset(&mut zip, &asset)?.ok_or(ReadingError::NotFound)
    })
    .await
    .map_err(|e| ReadingError::Internal(e.to_string()))?
}

pub async fn read_cover(db: &Db, item_id: ItemId) -> Result<(&'static str, Vec<u8>), ReadingError> {
    let loc = ensure_indexed(db, item_id).await?;
    let format = extension_of(&loc.file_path).unwrap_or_default();
    let _permit = READER_PERMITS
        .acquire()
        .await
        .map_err(|e| ReadingError::Internal(e.to_string()))?;
    let (storage, rel_path) = crate::local::track_path(loc.location)
        .await
        .map_err(|_| ReadingError::NotFound)?;

    tokio::task::spawn_blocking(move || {
        // 1. Check same-stem or folder sidecar cover image first if non-archive or fallback
        let sidecar = find_sidecar_cover(&storage, &rel_path);
        match format.as_str() {
            "epub" => {
                if let Ok(file) = storage.open_file_blocking(&rel_path)
                    && let Ok((mut zip, book)) = open_epub(BufReader::new(file))
                    && let Some(cpath) = book.cover_path
                    && let Ok(Some(img)) = read_epub_asset(&mut zip, &cpath)
                {
                    return Ok(img);
                }
                sidecar.ok_or(ReadingError::NotFound)
            }
            "cbz" | "zip" | "cbt" | "tar" => {
                if let Ok(file) = storage.open_file_blocking(&rel_path)
                    && let Ok((mut archive, info)) =
                        open_comic_archive(BufReader::new(file), &format)
                    && let Some(cover_idx) = info.cover_entry
                    && let Ok(img) = read_validated_image(&mut archive, cover_idx)
                {
                    return Ok(img);
                }
                sidecar.ok_or(ReadingError::NotFound)
            }
            _ => sidecar.ok_or(ReadingError::NotFound),
        }
    })
    .await
    .map_err(|e| ReadingError::Internal(e.to_string()))?
}

pub async fn read_page(
    db: &Db,
    item_id: ItemId,
    page: u32,
) -> Result<(&'static str, Vec<u8>), ReadingError> {
    let loc = ensure_indexed(db, item_id).await?;
    let format = extension_of(&loc.file_path).unwrap_or_default();
    if matches!(format.as_str(), "cbr" | "rar" | "7z" | "cb7") {
        return Err(ReadingError::UnsupportedFormat(format));
    }
    if !matches!(format.as_str(), "cbz" | "zip" | "cbt" | "tar") {
        return Err(ReadingError::UnsupportedFormat(format));
    }
    let page_idx = usize::try_from(page).map_err(|_| ReadingError::NotFound)?;
    let _permit = READER_PERMITS
        .acquire()
        .await
        .map_err(|e| ReadingError::Internal(e.to_string()))?;
    let (storage, rel_path) = crate::local::track_path(loc.location)
        .await
        .map_err(|_| ReadingError::NotFound)?;

    tokio::task::spawn_blocking(move || {
        let file = storage
            .open_file_blocking(&rel_path)
            .map_err(|_| ReadingError::NotFound)?;
        let (mut archive, info) = open_comic_archive(BufReader::new(file), &format)?;
        let entry_idx = *info
            .page_entries
            .get(page_idx)
            .ok_or(ReadingError::NotFound)?;
        Ok(read_validated_image(&mut archive, entry_idx)?)
    })
    .await
    .map_err(|e| ReadingError::Internal(e.to_string()))?
}

pub async fn open_reading_file(
    db: &Db,
    item_id: ItemId,
) -> Result<(TrackFile, String, ReadingKind), ReadingError> {
    let loc = db
        .reading()
        .item_location(item_id)
        .await?
        .ok_or(ReadingError::NotFound)?;
    let kind = loc.kind;
    let lib_kind = match kind {
        ReadingKind::Book => palveni_core::LibraryKind::Book,
        ReadingKind::Comic => palveni_core::LibraryKind::Comic,
    };
    let (storage, path) = crate::local::track_path(loc.location)
        .await
        .map_err(|_| ReadingError::NotFound)?;
    let name = path.file_name().unwrap_or_default().to_owned();
    if classify(lib_kind, &name).is_none() {
        return Err(ReadingError::NotFound);
    }
    let source = storage
        .open_file(&path)
        .await
        .map_err(|_| ReadingError::NotFound)?;
    let content_type = reading_media_type(&name, kind);
    Ok((
        TrackFile {
            source,
            content_type,
        },
        name,
        kind,
    ))
}

// ---------------------------------------------------------------------------
// Internal blocking metadata extraction
// ---------------------------------------------------------------------------

struct CloneableCandidate {
    item_id: ItemId,
    library_id: LibraryId,
    kind: ReadingKind,
    title: String,
    file_path: String,
    size_bytes: Option<i64>,
    modified_at: Option<i64>,
    root: std::path::PathBuf,
    relative_path: std::path::PathBuf,
}

impl CloneableCandidate {
    fn from_loc(loc: &ReadingItemLocation) -> Self {
        Self {
            item_id: loc.item_id,
            library_id: loc.library_id,
            kind: loc.kind,
            title: loc.title.clone(),
            file_path: loc.file_path.clone(),
            size_bytes: loc.size_bytes,
            modified_at: loc.modified_at,
            root: loc.location.root.clone(),
            relative_path: loc.location.relative_path.clone(),
        }
    }
}

fn extract_metadata_blocking(c: &CloneableCandidate) -> Option<ReadingMetadataUpsert> {
    let storage = LocalStorage::open_blocking(&c.root).ok()?;
    let mut rel = StoragePath::root();
    for part in &c.relative_path {
        rel = rel.join(part.to_str()?).ok()?;
    }
    let format = extension_of(&c.file_path).unwrap_or_default();
    let stamp = stamp_for(c.size_bytes, c.modified_at);
    let sidecar_has_cover = find_sidecar_cover(&storage, &rel).is_some();

    let (folder_series, volume_idx) = infer_series_and_volume(&c.relative_path, &c.title, c.kind);

    let mut meta = ReadingMetadataUpsert {
        item_id: c.item_id,
        library_id: c.library_id,
        stamp,
        kind: c.kind,
        format: format.clone(),
        title: c.title.clone(),
        sort_title: sort_key(&c.title),
        authors: Vec::new(),
        series_name: folder_series.clone(),
        series_key: folder_series.as_deref().and_then(series_key_for),
        series_index: volume_idx,
        summary: None,
        publisher: None,
        language: None,
        page_count: None,
        has_cover: sidecar_has_cover,
        tags: Vec::new(),
    };

    let Ok(file) = storage.open_file_blocking(&rel) else {
        return Some(meta);
    };
    let reader = BufReader::new(file);

    match format.as_str() {
        "epub" => {
            if let Ok((_, book)) = open_epub(reader) {
                if let Some(t) = book.title.filter(|s| !s.trim().is_empty()) {
                    meta.sort_title = sort_key(&t);
                    meta.title = t;
                }
                meta.authors = book.authors;
                meta.summary = book.summary;
                meta.publisher = book.publisher;
                meta.language = book.language;
                meta.tags = book.tags;
                if let Some(s) = book.series_name.filter(|s| !s.trim().is_empty()) {
                    meta.series_key = series_key_for(&s);
                    meta.series_name = Some(s);
                }
                if book.series_index.is_some() {
                    meta.series_index = book.series_index;
                }
                meta.page_count = i32::try_from(book.spine.len()).ok().filter(|&n| n > 0);
                meta.has_cover = book.cover_path.is_some() || sidecar_has_cover;
            }
        }
        "cbz" | "zip" | "cbt" | "tar" => {
            if let Ok((_, info)) = open_comic_archive(reader, &format) {
                meta.page_count = i32::try_from(info.page_entries.len())
                    .ok()
                    .filter(|&n| n > 0);
                meta.has_cover = info.cover_entry.is_some() || sidecar_has_cover;
            }
        }
        "txt" | "md" => {
            if let Ok(text) = read_decoded_text(reader) {
                let parsed = parse_text_book(&text, format == "md", &c.title);
                if let Some(t) = parsed.title.filter(|s| !s.trim().is_empty()) {
                    meta.sort_title = sort_key(&t);
                    meta.title = t;
                }
                meta.summary = parsed.summary;
                meta.page_count = i32::try_from(parsed.chapters.len()).ok().filter(|&n| n > 0);
            }
        }
        _ => {}
    }

    Some(meta)
}

fn find_sidecar_cover(
    storage: &LocalStorage,
    file_path: &StoragePath,
) -> Option<(&'static str, Vec<u8>)> {
    let parent = file_path.parent().unwrap_or_else(StoragePath::root);
    let file_name = file_path.file_name().unwrap_or_default();
    let stem = file_name
        .rsplit_once('.')
        .map(|(s, _)| s)
        .unwrap_or(file_name);

    let candidates = [
        format!("{stem}.jpg"),
        format!("{stem}.jpeg"),
        format!("{stem}.png"),
        format!("{stem}.webp"),
        "cover.jpg".to_owned(),
        "cover.jpeg".to_owned(),
        "cover.png".to_owned(),
        "cover.webp".to_owned(),
        "folder.jpg".to_owned(),
        "folder.png".to_owned(),
    ];

    for name in candidates {
        let Ok(candidate_path) = parent.join(&name) else {
            continue;
        };
        let Ok(mut f) = storage.open_file_blocking(&candidate_path) else {
            continue;
        };
        if let Some(img) = read_sidecar_image(&mut f) {
            return Some(img);
        }
    }
    None
}

fn read_sidecar_image(file: &mut File) -> Option<(&'static str, Vec<u8>)> {
    let len = file.metadata().ok()?.len();
    if !(12..=MAX_SIDECAR_COVER).contains(&len) {
        return None;
    }
    let cap = usize::try_from(len).ok()?.min(MAX_ENTRY_BYTES);
    let mut buf = Vec::with_capacity(cap);
    file.take(MAX_SIDECAR_COVER + 1)
        .read_to_end(&mut buf)
        .ok()?;
    if u64::try_from(buf.len()).unwrap_or(u64::MAX) > MAX_SIDECAR_COVER {
        return None;
    }
    let mime = detect_image_mime(&buf)?;
    Some((mime, buf))
}

fn infer_series_and_volume(
    relative_path: &Path,
    title: &str,
    kind: ReadingKind,
) -> (Option<String>, Option<f64>) {
    let parent_folder = relative_path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .map(str::trim)
        .filter(|s| !s.is_empty());

    let (title_series, vol_num) = parse_volume_from_title(title);
    if kind == ReadingKind::Comic {
        let series = parent_folder.map(ToOwned::to_owned).or(title_series);
        let idx = vol_num.or_else(|| {
            parent_folder
                .is_some()
                .then(|| parse_trailing_number(title))
                .flatten()
        });
        return (series, idx);
    }

    // For books, only infer series when explicit volume marker is present
    if let Some(vol) = vol_num {
        let series = title_series.or_else(|| parent_folder.map(ToOwned::to_owned));
        return (series, Some(vol));
    }
    (None, None)
}

fn parse_volume_from_title(title: &str) -> (Option<String>, Option<f64>) {
    // Match `Series Vol. 02`, `Series Volume 2`, `Series #02`
    let lower = title.to_ascii_lowercase();
    for marker in [" vol.", " vol ", " volume ", " #"] {
        if let Some(pos) = lower.find(marker) {
            let prefix = title[..pos]
                .trim_end_matches(['-', '_', ':', '—', '·', ' '])
                .trim();
            let after = &title[pos + marker.len()..];
            if let Some(num) = parse_leading_f64(after) {
                let s = (!prefix.is_empty()).then(|| prefix.to_owned());
                return (s, Some(num));
            }
        }
    }
    // Match `系列 第01卷` / `系列 第1册` / `系列 第1话`
    if let Some(di_pos) = title.find('第') {
        let prefix = title[..di_pos]
            .trim_end_matches(['-', '_', ':', '—', '·', ' '])
            .trim();
        let after_di = &title[di_pos + '第'.len_utf8()..];
        let digits: String = after_di
            .chars()
            .take_while(|c| c.is_ascii_digit() || *c == '.' || *c == ' ')
            .collect();
        let unit = after_di[digits.len()..].chars().next();
        if let Some(u) = unit
            && matches!(u, '卷' | '册' | '话' | '集' | '部')
            && let Ok(num) = digits.trim().parse::<f64>()
            && num.is_finite()
        {
            let s = (!prefix.is_empty()).then(|| prefix.to_owned());
            return (s, Some(num));
        }
    }
    (None, None)
}

fn parse_leading_f64(s: &str) -> Option<f64> {
    let trimmed = s.trim_start();
    let num_str: String = trimmed
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    if num_str.is_empty() {
        return None;
    }
    num_str.parse::<f64>().ok().filter(|n| n.is_finite())
}

fn parse_trailing_number(title: &str) -> Option<f64> {
    let trimmed = title.trim_end_matches([')', ']', '】']);
    let rev_digits: String = trimmed
        .chars()
        .rev()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    if rev_digits.is_empty() || rev_digits.len() > 4 {
        return None;
    }
    let digits: String = rev_digits.chars().rev().collect();
    digits.parse::<f64>().ok().filter(|n| n.is_finite())
}

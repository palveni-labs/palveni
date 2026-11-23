//! Comic archive reader (`.cbz`, `.zip`, `.cbt`, `.tar`) with natural page sorting
//! and cover selection.

use std::cmp::Ordering;
use std::io::{Read, Seek};

use super::archive::{ArchiveError, ArchiveReader, MAX_ENTRY_BYTES, TarArchive, ZipArchive};
use crate::classify::extension_of;

#[derive(Clone, Debug)]
pub struct ComicInfo {
    pub page_entries: Vec<usize>,
    pub cover_entry: Option<usize>,
}

pub fn is_comic_image_ext(ext: &str) -> bool {
    matches!(ext, "jpg" | "jpeg" | "png" | "webp" | "gif" | "avif")
}

pub fn detect_image_mime(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(&[0xff, 0xd8, 0xff]) {
        Some("image/jpeg")
    } else if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("image/png")
    } else if bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP") {
        Some("image/webp")
    } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        Some("image/gif")
    } else if bytes.len() >= 12
        && &bytes[4..8] == b"ftyp"
        && matches!(&bytes[8..12], b"avif" | b"avis" | b"mif1")
    {
        Some("image/avif")
    } else {
        None
    }
}

pub fn open_comic_archive<R: Read + Seek>(
    reader: R,
    format: &str,
) -> Result<(ArchiveReader<R>, ComicInfo), ArchiveError> {
    let archive = match format {
        "cbz" | "zip" => ArchiveReader::Zip(ZipArchive::open(reader)?),
        "cbt" | "tar" => ArchiveReader::Tar(TarArchive::open(reader)?),
        _ => return Err(ArchiveError::Corrupt("unsupported comic archive format")),
    };

    let mut pages: Vec<(usize, String)> = Vec::new();
    for (idx, entry) in archive.entries().iter().enumerate() {
        if is_ignored_archive_entry(&entry.name) {
            continue;
        }
        let Some(ext) = extension_of(&entry.name) else {
            continue;
        };
        if is_comic_image_ext(&ext) && entry.uncompressed_size > 0 {
            pages.push((idx, entry.name.clone()));
        }
    }

    pages.sort_by(|a, b| natural_path_cmp(&a.1, &b.1).then_with(|| a.0.cmp(&b.0)));

    let cover_entry = pages
        .iter()
        .find(|(_, name)| is_preferred_cover_name(name))
        .or_else(|| pages.first())
        .map(|(idx, _)| *idx);

    let page_entries = pages.into_iter().map(|(idx, _)| idx).collect();
    Ok((
        archive,
        ComicInfo {
            page_entries,
            cover_entry,
        },
    ))
}

pub fn read_validated_image<R: Read + Seek>(
    archive: &mut ArchiveReader<R>,
    entry_index: usize,
) -> Result<(&'static str, Vec<u8>), ArchiveError> {
    let bytes = archive.read_entry(entry_index, MAX_ENTRY_BYTES)?;
    let mime = detect_image_mime(&bytes)
        .ok_or(ArchiveError::Corrupt("archive entry is not a valid image"))?;
    Ok((mime, bytes))
}

fn is_ignored_archive_entry(path: &str) -> bool {
    for part in path.split('/') {
        let lower = part.to_ascii_lowercase();
        if part.starts_with('.')
            || part.starts_with("__MACOSX")
            || lower == "thumbs.db"
            || lower == "desktop.ini"
        {
            return true;
        }
    }
    false
}

fn is_preferred_cover_name(path: &str) -> bool {
    let file_name = path.rsplit('/').next().unwrap_or(path);
    let stem = file_name
        .rsplit_once('.')
        .map(|(s, _)| s)
        .unwrap_or(file_name)
        .trim()
        .to_ascii_lowercase();
    matches!(
        stem.as_str(),
        "cover" | "front" | "folder" | "00_cover" | "000_cover"
    )
}

/// Natural path comparator (`page_1 < page_2 < page_10`, `ch1/2.jpg < ch1/10.jpg`).
pub fn natural_path_cmp(a: &str, b: &str) -> Ordering {
    let mut sa = a.split('/');
    let mut sb = b.split('/');
    loop {
        match (sa.next(), sb.next()) {
            (Some(pa), Some(pb)) => {
                let ord = natural_segment_cmp(pa, pb);
                if ord != Ordering::Equal {
                    return ord;
                }
            }
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (None, None) => return Ordering::Equal,
        }
    }
}

pub fn natural_segment_cmp(a: &str, b: &str) -> Ordering {
    let mut ca = a.chars().peekable();
    let mut cb = b.chars().peekable();

    loop {
        match (ca.peek(), cb.peek()) {
            (None, None) => return a.cmp(b),
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(&cha), Some(&chb)) if cha.is_ascii_digit() && chb.is_ascii_digit() => {
                let mut da = String::new();
                while let Some(&c) = ca.peek() {
                    if c.is_ascii_digit() {
                        da.push(c);
                        ca.next();
                    } else {
                        break;
                    }
                }
                let mut db = String::new();
                while let Some(&c) = cb.peek() {
                    if c.is_ascii_digit() {
                        db.push(c);
                        cb.next();
                    } else {
                        break;
                    }
                }
                let ta = da.trim_start_matches('0');
                let tb = db.trim_start_matches('0');
                let num_ord = ta
                    .len()
                    .cmp(&tb.len())
                    .then_with(|| ta.cmp(tb))
                    .then_with(|| da.len().cmp(&db.len()));
                if num_ord != Ordering::Equal {
                    return num_ord;
                }
            }
            (Some(_), Some(_)) => {
                let cha = ca.next().unwrap_or('\0');
                let chb = cb.next().unwrap_or('\0');
                let la = cha.to_lowercase().collect::<String>();
                let lb = chb.to_lowercase().collect::<String>();
                let ord = la.cmp(&lb);
                if ord != Ordering::Equal {
                    return ord;
                }
            }
        }
    }
}

//! File classification for the file-level scan.

use palveni_core::{ItemKind, LibraryKind};

const VIDEO_EXTENSIONS: &[&str] = &[
    "3gp", "avi", "flv", "m2ts", "m4v", "mkv", "mov", "mp4", "mpeg", "mpg", "mts", "ogv", "rm",
    "rmvb", "ts", "vob", "webm", "wmv",
];

const AUDIO_EXTENSIONS: &[&str] = &[
    "aac", "aif", "aiff", "ape", "dff", "dsf", "dts", "flac", "m4a", "mka", "mp3", "mpc", "oga",
    "ogg", "opus", "tta", "wav", "wma", "wv",
];

// Photo libraries index still images.
const PHOTO_EXTENSIONS: &[&str] = &[
    "arw", "avif", "bmp", "cr2", "cr3", "dng", "gif", "heic", "heif", "jpeg", "jpg", "jxl", "nef",
    "orf", "png", "raf", "rw2", "tif", "tiff", "webp",
];

const BOOK_EXTENSIONS: &[&str] = &["azw", "azw3", "djvu", "epub", "fb2", "mobi", "pdf"];

const COMIC_EXTENSIONS: &[&str] = &["7z", "cb7", "cbr", "cbt", "cbz", "pdf", "rar", "zip"];

/// Directory and file names that NAS systems and desktop OSes litter into
/// shared folders.
const IGNORED_NAMES: &[&str] = &[
    "@eadir",
    "#recycle",
    "#snapshot",
    "$recycle.bin",
    "lost+found",
    "system volume information",
    "thumbs.db",
    "desktop.ini",
];

/// Lowercased extension of a file name, if it has one.
pub fn extension_of(name: &str) -> Option<String> {
    let (stem, ext) = name.rsplit_once('.')?;
    if stem.is_empty() || ext.is_empty() {
        return None;
    }
    Some(ext.to_ascii_lowercase())
}

pub(crate) fn audio_content_type(name: &str) -> &'static str {
    match extension_of(name).as_deref() {
        Some("aac") => "audio/aac",
        Some("aif" | "aiff") => "audio/aiff",
        Some("dts") => "audio/vnd.dts",
        Some("flac") => "audio/flac",
        Some("m4a" | "m4b") => "audio/mp4",
        Some("mka") => "audio/x-matroska",
        Some("mp3") => "audio/mpeg",
        Some("oga" | "ogg" | "opus") => "audio/ogg",
        Some("wav") => "audio/wav",
        Some("wma") => "audio/x-ms-wma",
        Some("wv") => "audio/x-wavpack",
        Some("ape") => "audio/x-ape",
        Some("mpc") => "audio/x-musepack",
        _ => "application/octet-stream",
    }
}

/// Leaf item kind for a file in a library of `kind`, or `None` if the file
/// is not media for that library.
pub fn classify(kind: LibraryKind, file_name: &str) -> Option<ItemKind> {
    let ext = extension_of(file_name)?;
    let known = match kind {
        LibraryKind::Video => VIDEO_EXTENSIONS,
        LibraryKind::Audiobook | LibraryKind::Podcast if ext == "m4b" => {
            return Some(kind.leaf_item_kind());
        }
        LibraryKind::Music | LibraryKind::Audiobook | LibraryKind::Podcast => AUDIO_EXTENSIONS,
        LibraryKind::Photo => PHOTO_EXTENSIONS,
        LibraryKind::Book => BOOK_EXTENSIONS,
        LibraryKind::Comic => COMIC_EXTENSIONS,
    };
    known
        .contains(&ext.as_str())
        .then_some(kind.leaf_item_kind())
}

/// Hidden entries (dot files, macOS `._` resource forks) and well-known
/// system folders are skipped entirely.
pub fn is_ignored_name(name: &str) -> bool {
    if name.starts_with('.') {
        return true;
    }
    let lower = name.to_lowercase();
    // QNAP thumbnail folders: `.@__thumb` is already hidden; `@Recently-Snapshot` is not.
    lower.starts_with("@recently-snapshot") || IGNORED_NAMES.contains(&lower.as_str())
}

/// Display title for a file: its name without the last extension.
pub fn title_for_file(name: &str) -> String {
    match name.rsplit_once('.') {
        Some((stem, _)) if !stem.is_empty() => stem.to_owned(),
        _ => name.to_owned(),
    }
}

/// Lowercase title used for ordering.
pub fn sort_key(title: &str) -> String {
    title.trim().to_lowercase()
}

/// Normalised title used for searching.
pub fn search_text(title: &str) -> String {
    title.trim().to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_by_library_kind() {
        assert_eq!(
            classify(LibraryKind::Music, "01 - Intro.FLAC"),
            Some(ItemKind::Track)
        );
        assert_eq!(
            classify(LibraryKind::Video, "Movie (2020).mkv"),
            Some(ItemKind::Video)
        );
        assert_eq!(
            classify(LibraryKind::Photo, "IMG_0001.HEIC"),
            Some(ItemKind::Photo)
        );
        assert_eq!(
            classify(LibraryKind::Book, "三体.epub"),
            Some(ItemKind::Book)
        );
        assert_eq!(
            classify(LibraryKind::Comic, "Vol 01.cbz"),
            Some(ItemKind::Comic)
        );
        // Right file, wrong library.
        assert_eq!(
            classify(LibraryKind::Music, "Album.DTS"),
            Some(ItemKind::Track)
        );
        assert_eq!(audio_content_type("Album.DTS"), "audio/vnd.dts");
        assert_eq!(classify(LibraryKind::Music, "Movie.mkv"), None);
        assert_eq!(classify(LibraryKind::Video, "cover.jpg"), None);
        // Sidecars and junk.
        assert_eq!(classify(LibraryKind::Music, "album.cue"), None);
        assert_eq!(classify(LibraryKind::Video, "movie.nfo"), None);
        assert_eq!(classify(LibraryKind::Music, "README"), None);
        assert_eq!(classify(LibraryKind::Music, ".flac"), None);
    }

    #[test]
    fn ignores_hidden_and_system_entries() {
        assert!(is_ignored_name(".DS_Store"));
        assert!(is_ignored_name("._song.flac"));
        assert!(is_ignored_name("@eaDir"));
        assert!(is_ignored_name("#recycle"));
        assert!(is_ignored_name("$RECYCLE.BIN"));
        assert!(is_ignored_name("@Recently-Snapshot"));
        assert!(!is_ignored_name("Albums"));
        assert!(!is_ignored_name("song.flac"));
    }

    #[test]
    fn spoken_audio_remains_distinct_from_music_and_ebooks() {
        assert_eq!(
            classify(LibraryKind::Audiobook, "Book.M4B"),
            Some(ItemKind::Chapter)
        );
        assert_eq!(
            classify(LibraryKind::Podcast, "Episode.mp3"),
            Some(ItemKind::Episode)
        );
        assert_eq!(audio_content_type("Chapter.M4B"), "audio/mp4");
        assert_eq!(classify(LibraryKind::Book, "Book.m4b"), None);
        assert_eq!(classify(LibraryKind::Music, "Book.m4b"), None);
    }

    #[test]
    fn titles_and_keys() {
        assert_eq!(title_for_file("01 Intro.flac"), "01 Intro");
        assert_eq!(title_for_file("archive.tar.gz"), "archive.tar");
        assert_eq!(title_for_file("noext"), "noext");
        assert_eq!(sort_key("  The Wall "), "the wall");
        assert_eq!(extension_of("a.B"), Some("b".to_owned()));
        assert_eq!(extension_of("trailing."), None);
    }
}

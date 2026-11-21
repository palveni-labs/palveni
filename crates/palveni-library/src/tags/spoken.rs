use std::path::Path;

use palveni_core::{ItemKind, music::TrackTags, storage::StoragePath};

fn numbered_folder(name: &str, kind: ItemKind) -> Option<u32> {
    let name = name.trim().to_ascii_lowercase();
    let (prefixes, suffixes): (&[&str], &[char]) = if kind == ItemKind::Episode {
        (
            &["season", "series", "s", "disc", "disk", "cd"],
            &['季', '碟', '盘'],
        )
    } else {
        (&["disc", "disk", "cd"], &['碟', '盘'])
    };
    let number = prefixes
        .iter()
        .find_map(|p| name.strip_prefix(p))
        .or_else(|| {
            name.strip_prefix('第')
                .and_then(|value| value.strip_suffix(suffixes))
        })?;
    number.trim().parse::<u32>().ok().filter(|n| *n > 0)
}

fn collection_tag(album: &str) -> Option<String> {
    let mut title = album.trim();
    if let Some((prefix, suffix)) = title.rsplit_once('第')
        && let Some(number) = suffix.strip_suffix(['章', '集', '回', '节'])
        && !number.is_empty()
        && number
            .chars()
            .all(|c| c.is_ascii_digit() || "零〇一二三四五六七八九十百千万两".contains(c))
    {
        title = prefix;
    } else {
        let lower = title.to_ascii_lowercase();
        for marker in ["chapter ", "episode "] {
            if let Some((prefix, number)) = lower.rsplit_once(marker)
                && (prefix.is_empty() || prefix.ends_with([' ', '-', ':']))
                && !number.is_empty()
                && number.bytes().all(|b| b.is_ascii_digit())
            {
                title = &title[..prefix.len()];
                break;
            }
        }
    }
    let title = title
        .trim_end_matches(|c: char| c.is_whitespace() || matches!(c, '-' | '–' | '—' | ':' | '：'));
    (!title.is_empty()).then(|| title.to_owned())
}

pub(super) fn apply(
    tags: &mut TrackTags,
    relative: &StoragePath,
    root: &Path,
    fallback: &str,
    kind: ItemKind,
) {
    let mut folder = relative.parent().unwrap_or_else(StoragePath::root);
    if let Some(number) = folder.file_name().and_then(|f| numbered_folder(f, kind)) {
        tags.disc_number.get_or_insert(number);
        folder = folder.parent().unwrap_or_else(StoragePath::root);
    }
    let tagged_album = tags.album.as_deref().and_then(collection_tag);
    let loose_file = folder.is_root() && tagged_album.is_none();
    let title = tagged_album.unwrap_or_else(|| {
        folder
            .file_name()
            .or(tags.title.as_deref())
            .unwrap_or(fallback)
            .to_owned()
    });
    let location = if loose_file { relative } else { &folder };
    tags.album = Some(title);
    tags.spoken_location = Some(root.join(location.as_str()).to_string_lossy().into_owned());
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Seek, Write};

    fn chapter(path: &str, mut tags: TrackTags) -> TrackTags {
        let path = StoragePath::parse(path).unwrap();
        apply(
            &mut tags,
            &path,
            Path::new("/books"),
            "Loose",
            ItemKind::Chapter,
        );
        tags
    }

    #[test]
    fn groups_untagged_discs_and_keeps_loose_files_separate() {
        let a = chapter("Book/CD 1/a.mp3", TrackTags::default());
        let b = chapter("Book/Disc 2/b.m4b", TrackTags::default());
        assert_eq!(a.album.as_deref(), Some("Book"));
        assert_eq!(a.spoken_location, b.spoken_location);
        assert_eq!(a.disc_number, Some(1));
        assert_eq!(b.disc_number, Some(2));
        assert_ne!(
            chapter("a.m4b", TrackTags::default()).spoken_location,
            chapter("b.m4b", TrackTags::default()).spoken_location
        );
    }

    #[test]
    fn untagged_recording_retains_its_duration() {
        let mut file = tempfile::tempfile().unwrap();
        // One second of mono 8-bit PCM at 8 kHz.
        file.write_all(b"RIFF").unwrap();
        file.write_all(&8036u32.to_le_bytes()).unwrap();
        file.write_all(b"WAVEfmt \x10\0\0\0\x01\0\x01\0").unwrap();
        file.write_all(&8000u32.to_le_bytes()).unwrap();
        file.write_all(&8000u32.to_le_bytes()).unwrap();
        file.write_all(b"\x01\0\x08\0data").unwrap();
        file.write_all(&8000u32.to_le_bytes()).unwrap();
        file.set_len(8044).unwrap();
        file.rewind().unwrap();
        let tags = super::super::read(file, ItemKind::Chapter).unwrap();
        assert_eq!(tags.duration_ms, Some(1000));
        assert_eq!(tags.title, None);
    }

    #[test]
    fn tags_override_directory_except_numbered_chapter_titles_misused_as_album() {
        let tags = TrackTags {
            title: Some("第12章".into()),
            album: Some("第12章".into()),
            ..Default::default()
        };
        assert_eq!(chapter("Story/a.mp3", tags).album.as_deref(), Some("Story"));
        let tags = TrackTags {
            album: Some("Tagged title".into()),
            ..Default::default()
        };
        assert_eq!(
            chapter("Folder/a.mp3", tags).album.as_deref(),
            Some("Tagged title")
        );
        assert_eq!(numbered_folder("第2季", ItemKind::Episode), Some(2));
        assert_eq!(numbered_folder("Story", ItemKind::Episode), None);
        assert_eq!(
            collection_tag("未来简史 第18章").as_deref(),
            Some("未来简史")
        );
        assert_eq!(collection_tag("1984").as_deref(), Some("1984"));
        assert_eq!(
            collection_tag("Story — Chapter 003").as_deref(),
            Some("Story")
        );
    }
}

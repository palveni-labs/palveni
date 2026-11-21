//! Lyrics from a track's adjacent file or embedded tags. Read on demand so
//! editing a sidecar does not require rescanning the audio library.
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

use lofty::{config::ParseOptions, file::TaggedFileExt, probe::Probe, tag::ItemKey};
use palveni_core::{
    Error, Result,
    music::{Lyrics, LyricsFormat, LyricsSource},
};
use palveni_db::music::TrackLocation;

const MAX_TEXT: usize = 256 * 1024;

pub async fn read(location: TrackLocation) -> Result<Option<Lyrics>> {
    let (storage, path) = crate::local::track_path(location).await?;
    tokio::task::spawn_blocking(move || {
        let audio = storage.open_file_blocking(&path)?;
        let name = path.file_name().ok_or_else(|| Error::not_found("Track"))?;
        let parent = path.parent().ok_or_else(|| Error::not_found("Track"))?;
        for ext in ["lrc", "LRC", "lyrics.txt"] {
            let sidecar = Path::new(name).with_extension(ext);
            let Some(file_name) = sidecar.to_str() else {
                continue;
            };
            let side_path = parent.join(file_name)?;
            if let Ok(file) = storage.open_file_blocking(&side_path)
                && let Some(text) = sidecar_text(file)
            {
                return Ok(Some(document(LyricsSource::Sidecar, text)));
            }
        }
        Ok(embedded(audio).map(|text| document(LyricsSource::Embedded, text)))
    })
    .await
    .map_err(|e| Error::internal(format!("lyrics worker stopped: {e}")))?
}

fn suitable(text: &str) -> bool {
    text.len() <= MAX_TEXT && !text.contains('\0') && !text.trim().is_empty()
}

fn sidecar_text(file: File) -> Option<String> {
    if file.metadata().ok()?.len() > MAX_TEXT as u64 {
        return None;
    }
    let mut bytes = Vec::new();
    file.take(MAX_TEXT as u64 + 1)
        .read_to_end(&mut bytes)
        .ok()?;
    if bytes.len() > MAX_TEXT {
        return None;
    }
    let text = if bytes.starts_with(&[0xff, 0xfe]) || bytes.starts_with(&[0xfe, 0xff]) {
        let (pairs, remainder) = bytes[2..].as_chunks::<2>();
        if !remainder.is_empty() {
            return None;
        }
        let units = pairs.iter().map(|pair| {
            if bytes[0] == 0xff {
                u16::from_le_bytes(*pair)
            } else {
                u16::from_be_bytes(*pair)
            }
        });
        char::decode_utf16(units)
            .collect::<std::result::Result<String, _>>()
            .ok()?
    } else {
        let text = String::from_utf8(bytes).ok()?;
        match text.strip_prefix('\u{feff}') {
            Some(s) => s.to_owned(),
            None => text,
        }
    };
    suitable(&text).then_some(text)
}

fn embedded(file: File) -> Option<String> {
    crate::tags::limit_allocations();
    let options = ParseOptions::new()
        .read_cover_art(false)
        .read_properties(false);
    let audio = Probe::new(BufReader::new(file))
        .options(options)
        .guess_file_type()
        .ok()?
        .read()
        .ok()?;
    audio.tags().iter().find_map(|tag| {
        [ItemKey::Lyrics, ItemKey::UnsyncLyrics]
            .into_iter()
            .flat_map(|key| tag.get_strings(key))
            .find(|text| suitable(text))
            .map(str::to_owned)
    })
}

fn has_timestamp(line: &str) -> bool {
    let Some((stamp, _)) = line
        .trim_start()
        .strip_prefix('[')
        .and_then(|s| s.split_once(']'))
    else {
        return false;
    };
    let Some((minutes, rest)) = stamp.split_once(':') else {
        return false;
    };
    let (seconds, fraction) = rest
        .split_once(['.', ':'])
        .map_or((rest, None), |(s, f)| (s, Some(f)));
    !minutes.is_empty()
        && minutes.bytes().all(|b| b.is_ascii_digit())
        && seconds.len() == 2
        && seconds.parse::<u8>().is_ok_and(|n| n < 60)
        && seconds.bytes().all(|b| b.is_ascii_digit())
        && fraction.is_none_or(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()))
}

fn document(source: LyricsSource, text: String) -> Lyrics {
    let format = if text.lines().any(has_timestamp) {
        LyricsFormat::Lrc
    } else {
        LyricsFormat::Plain
    };
    Lyrics {
        source,
        format,
        text,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Seek, Write};

    fn decode(bytes: &[u8]) -> Option<String> {
        let mut file = tempfile::tempfile().unwrap();
        file.write_all(bytes).unwrap();
        file.rewind().unwrap();
        sidecar_text(file)
    }

    #[test]
    fn accepts_bom_encodings_and_rejects_malformed_or_oversized_text() {
        let text = "[00:09.10]字幕";
        for little in [true, false] {
            let mut bytes = if little {
                vec![0xff, 0xfe]
            } else {
                vec![0xfe, 0xff]
            };
            for unit in text.encode_utf16() {
                bytes.extend(if little {
                    unit.to_le_bytes()
                } else {
                    unit.to_be_bytes()
                });
            }
            assert_eq!(decode(&bytes).as_deref(), Some(text));
            bytes.push(3);
            assert!(decode(&bytes).is_none());
        }
        assert_eq!(decode("\u{feff}test".as_bytes()).as_deref(), Some("test"));
        assert_eq!(decode(&vec![b'x'; MAX_TEXT]).unwrap().len(), MAX_TEXT);
        for bytes in [&vec![b'x'; MAX_TEXT + 1][..], b"test\0text", &[0xff], b"  "] {
            assert!(decode(bytes).is_none());
        }
        assert!(has_timestamp("[00:15.60][01:01]test"));
        assert!(!has_timestamp("[ar:artist]"));
        assert!(!has_timestamp("[00:67]test"));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn inaccessible_sidecars_fall_back_and_replaced_roots_stay_unavailable() {
        let base = tempfile::tempdir().unwrap();
        let root = base.path().canonicalize().unwrap().join("music");
        let outside = base.path().join("elsewhere");
        std::fs::create_dir(&root).unwrap();
        std::fs::create_dir(&outside).unwrap();
        std::fs::write(root.join("song.mp3"), b"fixture").unwrap();
        std::fs::write(outside.join("words"), b"outside").unwrap();
        std::os::unix::fs::symlink(outside.join("words"), root.join("song.lrc")).unwrap();
        std::fs::write(root.join("song.lyrics.txt"), b"inside").unwrap();
        let location = || TrackLocation {
            cue_frames: None,
            root: root.clone(),
            relative_path: "song.mp3".into(),
        };
        assert_eq!(read(location()).await.unwrap().unwrap().text, "inside");
        std::fs::rename(&root, base.path().join("offline")).unwrap();
        std::os::unix::fs::symlink(&outside, &root).unwrap();
        assert!(matches!(read(location()).await, Err(Error::PathEscape)));
    }
}

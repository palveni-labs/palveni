//! Audio metadata is read on the scanner's blocking worker, once per changed track.

mod spoken;

use std::error::Error as _;
use std::fs::File;
use std::io::{self, BufReader};
use std::path::Path;

use lofty::config::{GlobalOptions, ParseOptions, apply_global_options};
use lofty::file::{AudioFile, FileType, TaggedFileExt};
use lofty::probe::Probe;
use lofty::properties::FileProperties;
use lofty::tag::{Accessor, ItemKey};
use palveni_core::{
    ItemKind,
    music::{AudioProperties, TrackTags},
    storage::StoragePath,
};
use palveni_db::items::ItemUpsert;

use crate::{LocalStorage, ScanError};

fn text(value: Option<&str>) -> Option<String> {
    let input = value?.trim();
    let cleaned: String = input
        .chars()
        .filter(|c| !c.is_control())
        .take(1024)
        .collect();
    let name = cleaned.trim();
    (!name.is_empty()).then(|| name.to_owned())
}

fn read(file: File, kind: ItemKind) -> io::Result<TrackTags> {
    let probe = Probe::new(BufReader::new(file)).options(ParseOptions::new().read_cover_art(false));
    let parsed = match probe.guess_file_type()?.read() {
        Ok(file) => file,
        Err(err) => {
            let mut cause = err.source();
            while let Some(error) = cause {
                if let Some(io) = error.downcast_ref::<io::Error>()
                    && !matches!(
                        io.kind(),
                        io::ErrorKind::UnexpectedEof | io::ErrorKind::InvalidData
                    )
                {
                    return Err(io::Error::new(io.kind(), io.to_string()));
                }
                cause = error.source();
            }
            return Ok(TrackTags::default());
        }
    };
    let properties = parsed.properties();
    let duration = properties.duration();
    let mut result = TrackTags {
        duration_ms: (!duration.is_zero())
            .then(|| i64::try_from(duration.as_millis()).unwrap_or(i64::MAX)),
        audio: audio_properties(parsed.file_type(), properties),
        ..Default::default()
    };
    if let Some(tag) = parsed.primary_tag().or_else(|| parsed.first_tag()) {
        result.title = text(tag.title().as_deref());
        result.album = text(tag.album().as_deref());
        if kind == ItemKind::Episode {
            result.album = text(tag.get_string(ItemKey::ShowName)).or(result.album);
        } else if kind == ItemKind::Chapter && result.album.is_none() {
            result.album = text(tag.get_string(ItemKey::Work));
        }
        result.artist = text(tag.artist().as_deref());
        result.album_artist = text(tag.get_string(ItemKey::AlbumArtist));
        result.disc_number = tag.disk().filter(|n| *n != 0);
        result.track_number = tag.track().filter(|n| *n != 0);
        result.year = [
            ItemKey::RecordingDate,
            ItemKey::Year,
            ItemKey::ReleaseDate,
            ItemKey::OriginalReleaseDate,
        ]
        .into_iter()
        .find_map(|key| tag.get_strings(key).find_map(year));
        result.genres = genres(tag.get_strings(ItemKey::Genre));
    }
    Ok(result)
}

/// The leading four-digit year of `2003`, `2003-05-01` or `2003-05-01T10:00`.
pub(crate) fn year(value: &str) -> Option<i32> {
    let digits = value.trim().get(..4)?;
    if !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    digits.parse().ok().filter(|year| (1..=9999).contains(year))
}

const MAX_GENRES: usize = 16;
const MAX_GENRE_CHARS: usize = 128;

/// Splits multi-valued and delimited genre tags and drops case-insensitive repeats.
pub(crate) fn genres<'a>(values: impl Iterator<Item = &'a str>) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for part in values.flat_map(|value| value.split([';', '/', '\0'])) {
        let Some(name) = text(Some(part)) else {
            continue;
        };
        let name: String = name.chars().take(MAX_GENRE_CHARS).collect();
        let name = name.trim_end().to_owned();
        if seen.insert(name.to_lowercase()) {
            out.push(name);
            if out.len() == MAX_GENRES {
                break;
            }
        }
    }
    out
}

fn audio_properties(file_type: FileType, properties: &FileProperties) -> AudioProperties {
    let bit_depth = properties.bit_depth().filter(|n| *n != 0);
    let codec = match file_type {
        FileType::Aac => "aac",
        FileType::Aiff => "aiff",
        FileType::Ape => "ape",
        FileType::Flac => "flac",
        FileType::Mpeg => "mp3",
        // Lofty reports a sample size only for lossless MP4 audio (ALAC, rarely FLAC).
        FileType::Mp4 if bit_depth.is_some() => "alac",
        FileType::Mp4 => "aac",
        FileType::Mpc => "mpc",
        FileType::Opus => "opus",
        FileType::Vorbis => "vorbis",
        FileType::Speex => "speex",
        FileType::Wav => "wav",
        FileType::WavPack => "wavpack",
        _ => "",
    };
    AudioProperties {
        codec: (!codec.is_empty()).then(|| codec.to_owned()),
        bitrate_kbps: properties
            .audio_bitrate()
            .or(properties.overall_bitrate())
            .filter(|n| *n != 0),
        sample_rate_hz: properties.sample_rate().filter(|n| *n != 0),
        bit_depth,
        channels: properties.channels().filter(|n| *n != 0),
    }
}

pub(crate) fn duration_frames(file: File) -> io::Result<Option<i64>> {
    limit_allocations();
    let probe = Probe::new(crate::artwork::bounded_tag_reader(file))
        .options(ParseOptions::new().read_cover_art(false).read_tags(false))
        .guess_file_type()?;
    let parsed = match probe.read() {
        Ok(parsed) => parsed,
        Err(err) => match err.kind() {
            lofty::error::ErrorKind::Io(e)
                if !matches!(
                    e.kind(),
                    io::ErrorKind::InvalidData | io::ErrorKind::UnexpectedEof
                ) =>
            {
                return Err(io::Error::new(e.kind(), e.to_string()));
            }
            _ => return Ok(None),
        },
    };
    let frames = (parsed.properties().duration().as_nanos() * 75 + 500_000_000) / 1_000_000_000;
    Ok(i64::try_from(frames).ok().filter(|n| *n > 0))
}

pub(crate) fn limit_allocations() {
    apply_global_options(
        GlobalOptions::new()
            .allocation_limit(8 * 1024 * 1024)
            .preserve_format_specific_items(false),
    );
}

pub(crate) fn read_batch(
    roots: &[LocalStorage],
    mut rows: Vec<ItemUpsert>,
) -> Result<Vec<ItemUpsert>, ScanError> {
    limit_allocations();
    for row in &mut rows {
        if !row.kind.is_audio() || row.music.as_ref().is_some_and(|t| t.cue.is_some()) {
            continue;
        }
        let (root, relative) = roots
            .iter()
            .find_map(|root| {
                Path::new(&row.path)
                    .strip_prefix(root.root())
                    .ok()
                    .map(|p| (root, p))
            })
            .ok_or(ScanError::Metadata(
                "track is outside its library roots".into(),
            ))?;
        let relative = StoragePath::parse(
            &relative
                .to_string_lossy()
                .replace(std::path::MAIN_SEPARATOR, "/"),
        )
        .map_err(|e| ScanError::Metadata(e.to_string()))?;
        let file = root
            .open_file_blocking(&relative)
            .map_err(|e| ScanError::Metadata(e.to_string()))?;
        let mut tags = read(file, row.kind).map_err(|e| ScanError::Metadata(e.to_string()))?;
        tags.cue_backing = row.music.as_ref().is_some_and(|t| t.cue_backing);
        if row.kind != ItemKind::Track {
            spoken::apply(&mut tags, &relative, root.root(), &row.title, row.kind);
        }
        row.music = Some(tags);
    }
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use lofty::config::WriteOptions;
    use lofty::file::TaggedFileExt;
    use lofty::tag::{ItemKey, ItemValue, Tag, TagItem, TagType};

    use super::*;

    #[test]
    fn years_come_from_the_leading_digits_of_dates() {
        assert_eq!(year("1999"), Some(1999));
        assert_eq!(year(" 2003-05-01T10:00 "), Some(2003));
        assert_eq!(year("0000"), None);
        assert_eq!(year("99"), None);
        assert_eq!(year("19x9"), None);
        assert_eq!(year("二〇〇三"), None);
    }

    #[test]
    fn genres_split_on_delimiters_and_drop_repeats() {
        assert_eq!(
            genres(["Rock; Pop/rock", "Jazz\0Pop", " ", "Soul"].into_iter()),
            ["Rock", "Pop", "Jazz", "Soul"]
        );
        let many: Vec<String> = (0..40).map(|n| format!("g{n}")).collect();
        assert_eq!(genres(many.iter().map(String::as_str)).len(), MAX_GENRES);
        let long = "x".repeat(300);
        assert_eq!(
            genres([long.as_str()].into_iter())[0].len(),
            MAX_GENRE_CHARS
        );
    }

    #[test]
    fn stream_parameters_year_and_genres_are_read_from_a_tagged_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tone.wav");
        let mut file = File::create(&path).unwrap();
        // A quarter second of stereo 16-bit PCM at 44.1 kHz.
        let data = 44_100u32;
        file.write_all(b"RIFF").unwrap();
        file.write_all(&(36 + data).to_le_bytes()).unwrap();
        file.write_all(b"WAVEfmt \x10\0\0\0\x01\0\x02\0").unwrap();
        file.write_all(&44_100u32.to_le_bytes()).unwrap();
        file.write_all(&176_400u32.to_le_bytes()).unwrap();
        file.write_all(b"\x04\0\x10\0data").unwrap();
        file.write_all(&data.to_le_bytes()).unwrap();
        file.write_all(&vec![0; data as usize]).unwrap();
        drop(file);

        let mut tagged = lofty::read_from_path(&path).unwrap();
        let mut tag = Tag::new(TagType::Id3v2);
        tag.insert_text(ItemKey::TrackTitle, "Tone".into());
        tag.insert_text(ItemKey::RecordingDate, "1987-06-05".into());
        tag.push(TagItem::new(
            ItemKey::Genre,
            ItemValue::Text("Ambient;Electronic".into()),
        ));
        tagged.insert_tag(tag);
        tagged.save_to_path(&path, WriteOptions::default()).unwrap();

        let tags = read(File::open(&path).unwrap(), ItemKind::Track).unwrap();
        assert_eq!(tags.title.as_deref(), Some("Tone"));
        assert_eq!(tags.year, Some(1987));
        assert_eq!(tags.genres, ["Ambient", "Electronic"]);
        assert_eq!(tags.audio.codec.as_deref(), Some("wav"));
        assert_eq!(tags.audio.sample_rate_hz, Some(44_100));
        assert_eq!(tags.audio.bit_depth, Some(16));
        assert_eq!(tags.audio.channels, Some(2));
        assert_eq!(tags.audio.bitrate_kbps, Some(1411));
    }
}

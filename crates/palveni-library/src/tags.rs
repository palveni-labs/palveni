//! Audio metadata is read on the scanner's blocking worker, once per changed track.

mod spoken;

use std::error::Error as _;
use std::fs::File;
use std::io::{self, BufReader};
use std::path::Path;

use lofty::config::{GlobalOptions, ParseOptions, apply_global_options};
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::probe::Probe;
use lofty::tag::{Accessor, ItemKey};
use palveni_core::{ItemKind, music::TrackTags, storage::StoragePath};
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
    let duration = parsed.properties().duration();
    let mut result = TrackTags {
        duration_ms: (!duration.is_zero())
            .then(|| i64::try_from(duration.as_millis()).unwrap_or(i64::MAX)),
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
    }
    Ok(result)
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
        if !row.kind.is_audio() {
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
        if row.kind != ItemKind::Track {
            spoken::apply(&mut tags, &relative, root.root(), &row.title, row.kind);
        }
        row.music = Some(tags);
    }
    Ok(rows)
}

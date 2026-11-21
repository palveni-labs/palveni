use std::collections::{BTreeMap, BTreeSet};
use std::io::Cursor;
use std::ops::Range;

use super::{MAX_PACKAGE_BYTES, PinnedRange, SkinError, SkinInfo, checksum, invalid};
use crate::skin_validation::{self, MAX_ASSET_BYTES, MAX_PLUGIN_TOML_BYTES, MAX_SKIN_JSON_BYTES};

const BLOCK: usize = 512;
const MAX_FILES: usize = 256;

#[derive(Clone, Debug)]
pub(super) struct AssetRange {
    pub pinned: PinnedRange,
    pub mime: &'static str,
}

pub(super) struct IndexedSkin {
    pub info: SkinInfo,
    pub assets: BTreeMap<String, AssetRange>,
}

fn raw_entries(bytes: &[u8]) -> Result<BTreeMap<String, Range<usize>>, SkinError> {
    if bytes.len() > MAX_PACKAGE_BYTES {
        return Err(SkinError::TooLarge);
    }
    let mut files = BTreeMap::new();
    let mut seen = BTreeSet::new();
    let mut end = 0;
    let mut archive = tar::Archive::new(Cursor::new(bytes));
    // Raw iteration surfaces link, sparse and extended header types; none
    // may rewrite a subsequent path or describe additional file contents.
    let raw = archive
        .entries()
        .map_err(|_| invalid("Invalid tar archive"))?
        .raw(true);
    for entry in raw {
        let entry = entry.map_err(|_| invalid("Invalid tar entry"))?;
        let header = entry.header();
        if header.as_ustar().is_none() && header.as_gnu().is_none() {
            return Err(invalid("Expected a POSIX tar header"));
        }
        let kind = header.entry_type();
        if !(kind.is_file() || kind.is_dir()) || header.link_name_bytes().is_some() {
            return Err(invalid(
                "Only regular skin package files and directories are allowed",
            ));
        }
        let raw_path = entry.path_bytes();
        let raw_path =
            std::str::from_utf8(&raw_path).map_err(|_| invalid("Invalid tar path encoding"))?;
        if raw_path.starts_with('/') || raw_path.contains('\\') {
            return Err(invalid("Invalid tar path"));
        }
        let path = raw_path.strip_prefix("./").unwrap_or(raw_path);
        let path = if kind.is_dir() {
            path.trim_end_matches('/')
        } else {
            path
        };
        if !seen.insert(path.to_owned()) {
            return Err(invalid("Duplicate tar path"));
        }
        if seen.len() > MAX_FILES + 3 {
            return Err(SkinError::TooLarge);
        }
        let offset = usize::try_from(entry.raw_file_position()).map_err(|_| SkinError::TooLarge)?;
        let size = usize::try_from(entry.size()).map_err(|_| SkinError::TooLarge)?;
        if size > MAX_PACKAGE_BYTES {
            return Err(SkinError::TooLarge);
        }
        let limit = offset.checked_add(size).ok_or(SkinError::TooLarge)?;
        if limit > bytes.len() {
            return Err(invalid("Truncated tar entry"));
        }
        end = limit.next_multiple_of(BLOCK);
        if kind.is_dir() {
            if size != 0 || !matches!(path, "" | "." | "fonts" | "locales") {
                return Err(invalid("Unexpected tar directory"));
            }
            continue;
        }
        if files.len() == MAX_FILES {
            return Err(SkinError::TooLarge);
        }
        let size_limit = match path {
            "skin.json" => MAX_SKIN_JSON_BYTES,
            "plugin.toml" | "locales/en.json" | "locales/zh-CN.json" => MAX_PLUGIN_TOML_BYTES,
            _ if private_license(path) => MAX_PLUGIN_TOML_BYTES,
            _ if public_mime(path).is_some() => MAX_ASSET_BYTES,
            _ => return Err(invalid("Unexpected file in skin package")),
        };
        if size > size_limit {
            return Err(SkinError::TooLarge);
        }
        files.insert(path.to_owned(), offset..limit);
    }
    let padding = bytes
        .get(end..)
        .ok_or_else(|| invalid("Truncated tar archive"))?;
    if !bytes.len().is_multiple_of(BLOCK)
        || padding.len() < 2 * BLOCK
        || padding.iter().any(|&b| b != 0)
    {
        return Err(invalid("Invalid tar end marker"));
    }
    Ok(files)
}

fn safe_filename(name: &str) -> bool {
    name.as_bytes()
        .first()
        .is_some_and(u8::is_ascii_alphanumeric)
        && name.len() <= 128
        && name
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_' | b'.'))
}

fn private_license(path: &str) -> bool {
    path.strip_prefix("fonts/LICENSE-")
        .and_then(|s| s.strip_suffix(".txt"))
        .is_some_and(safe_filename)
}

fn public_mime(path: &str) -> Option<&'static str> {
    match path {
        "preview.png" => Some("image/png"),
        "preview.jpg" => Some("image/jpeg"),
        "preview.avif" => Some("image/avif"),
        "preview.webp" => Some("image/webp"),
        _ if path
            .strip_prefix("fonts/")
            .is_some_and(|s| safe_filename(s) && s.ends_with(".woff2")) =>
        {
            Some("font/woff2")
        }
        _ => None,
    }
}

fn u32_be(bytes: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_be_bytes(
        bytes.get(offset..offset + 4)?.try_into().ok()?,
    ))
}

fn matches_type(mime: &str, bytes: &[u8]) -> bool {
    match mime {
        "font/woff2" => {
            bytes.len() >= 48
                && bytes.starts_with(b"wOF2")
                && u32_be(bytes, 8).is_some_and(|n| n as usize == bytes.len())
        }
        "image/png" => {
            bytes.starts_with(b"\x89PNG\r\n\x1a\n\x00\x00\x00\x0dIHDR")
                && bytes.ends_with(b"IEND\xaeB\x60\x82")
        }
        "image/jpeg" => bytes.starts_with(&[0xff, 0xd8, 0xff]) && bytes.ends_with(&[0xff, 0xd9]),
        "image/webp" => {
            bytes.starts_with(b"RIFF")
                && bytes.get(8..12) == Some(b"WEBP")
                && bytes
                    .get(12..16)
                    .is_some_and(|chunk| matches!(chunk, b"VP8 " | b"VP8L" | b"VP8X"))
        }
        "image/avif" => {
            let box_len = u32_be(bytes, 0).unwrap_or(0) as usize;
            bytes.get(4..8) == Some(b"ftyp")
                && box_len >= 16
                && box_len <= bytes.len()
                && (matches!(bytes.get(8..12), Some(b"avif" | b"avis"))
                    || bytes[16..box_len]
                        .as_chunks::<4>()
                        .0
                        .iter()
                        .any(|brand| matches!(brand, b"avif" | b"avis")))
        }
        _ => false,
    }
}

pub(super) fn index_package(bytes: &[u8]) -> Result<IndexedSkin, SkinError> {
    let files = raw_entries(bytes)?;
    let get = |name: &str| {
        files
            .get(name)
            .map(|range| &bytes[range.clone()])
            .ok_or_else(|| invalid("Missing required skin file"))
    };
    let skin = skin_validation::validate_skin(get("plugin.toml")?, get("skin.json")?)
        .map_err(SkinError::Invalid)?;
    for (path, expected) in &skin.fonts {
        let font = get(path)?;
        if expected.is_some_and(|len| len != font.len() as u64) {
            return Err(invalid("Font length does not match manifest"));
        }
    }
    if let Some(preview) = &skin.preview {
        get(preview)?;
    }

    let mut assets = BTreeMap::new();
    for (name, range) in files {
        let asset = &bytes[range.clone()];
        let mime = match name.as_str() {
            "plugin.toml" => continue,
            "locales/en.json" | "locales/zh-CN.json" => {
                let values: BTreeMap<String, String> =
                    serde_json::from_slice(asset).map_err(|_| invalid("Invalid skin locale"))?;
                if values
                    .keys()
                    .any(|key| !matches!(key.as_str(), "name" | "description"))
                {
                    return Err(invalid("Unknown skin locale key"));
                }
                continue;
            }
            _ if private_license(&name) => {
                std::str::from_utf8(asset)
                    .map_err(|_| invalid("Font license must be plain text"))?;
                continue;
            }
            "skin.json" => "application/json",
            _ => {
                if !skin.fonts.contains_key(&name) && skin.preview.as_ref() != Some(&name) {
                    return Err(invalid("Undeclared skin resource"));
                }
                let mime =
                    public_mime(&name).ok_or_else(|| invalid("Invalid skin resource type"))?;
                if !matches_type(mime, asset) {
                    return Err(invalid("Skin resource content does not match its type"));
                }
                mime
            }
        };
        assets.insert(
            name,
            AssetRange {
                mime,
                pinned: PinnedRange {
                    offset: range.start,
                    len: range.len(),
                    digest: checksum(asset),
                },
            },
        );
    }
    let preview_url = skin
        .preview
        .as_ref()
        .map(|name| format!("/api/v1/skins/installed/{}/assets/{name}", skin.id));
    Ok(IndexedSkin {
        info: SkinInfo {
            id: skin.id,
            plugin_id: skin.plugin_id,
            version: skin.version,
            name: skin.name,
            description: skin.description,
            size_bytes: bytes.len(),
            preview_url,
        },
        assets,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn archive(entries: &[(&str, tar::EntryType, usize)]) -> Vec<u8> {
        let mut builder = tar::Builder::new(Vec::new());
        for &(name, kind, size) in entries {
            let mut header = tar::Header::new_ustar();
            header.set_entry_type(kind);
            header.set_size(size as u64);
            header.set_mode(0o644);
            let bytes = vec![0; size];
            builder
                .append_data(&mut header, name, bytes.as_slice())
                .expect("append");
        }
        builder.into_inner().expect("archive")
    }

    #[test]
    fn archive_accepts_only_the_expected_regular_files_and_complete_terminator() {
        let file = tar::EntryType::Regular;
        assert!(raw_entries(&archive(&[("plugin.toml", file, 1)])).is_ok());
        for malformed in [
            archive(&[("plugin.toml", file, 1), ("plugin.toml", file, 1)]),
            archive(&[("skin.json", tar::EntryType::Link, 0)]),
            archive(&[("skin.json", tar::EntryType::Symlink, 0)]),
            archive(&[("index.html", file, 0)]),
        ] {
            assert!(matches!(
                raw_entries(&malformed),
                Err(SkinError::Invalid(_))
            ));
        }
        let mut incomplete = archive(&[("skin.json", file, 1)]);
        incomplete.truncate(incomplete.len() - BLOCK);
        assert!(raw_entries(&incomplete).is_err());
        let mut concatenated = archive(&[("skin.json", file, 1)]);
        concatenated.extend(archive(&[("plugin.toml", file, 1)]));
        assert!(raw_entries(&concatenated).is_err());
        let oversized = archive(&[("skin.json", file, MAX_SKIN_JSON_BYTES + 1)]);
        assert!(matches!(raw_entries(&oversized), Err(SkinError::TooLarge)));
    }
}

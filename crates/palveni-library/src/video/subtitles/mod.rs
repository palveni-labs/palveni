//! Read-only subtitle sidecars for indexed local and mounted videos.
mod webvtt;

use std::{collections::BTreeMap, io::Read, path::Path};

use palveni_core::{
    Error, Result,
    storage::StoragePath,
    video::{SubtitleFormat, SubtitleList, SubtitleTrack},
};
use palveni_db::music::TrackLocation;

use crate::local::{LocalStorage, track_path};

const MAX_TRACKS: usize = 32;
const MAX_ENTRIES: usize = 65_536;
const MAX_BYTES: usize = 2 * 1024 * 1024;

fn language(value: &str) -> Option<String> {
    let named = match value {
        "简体中文" | "簡體中文" | "简体" | "简中" | "簡中" => Some("zh-Hans"),
        "繁體中文" | "繁体中文" | "繁体" | "繁體" | "繁中" => Some("zh-Hant"),
        "中文" | "chinese" => Some("zh"),
        "english" => Some("en"),
        "japanese" | "日语" | "日語" => Some("ja"),
        "korean" | "韩语" | "韓語" => Some("ko"),
        "french" => Some("fr"),
        "spanish" => Some("es"),
        _ => None,
    };
    if let Some(named) = named {
        return Some(named.to_owned());
    }
    let mut parts = value.split('-');
    let first = parts.next()?;
    if !(2..=3).contains(&first.len()) || !first.bytes().all(|b| b.is_ascii_alphabetic()) {
        return None;
    }
    let mut name = match first.to_ascii_lowercase().as_str() {
        "chs" => "zh-Hans".into(),
        "cht" => "zh-Hant".into(),
        "zho" | "chi" => "zh".into(),
        "eng" => "en".into(),
        "jpn" => "ja".into(),
        "kor" => "ko".into(),
        "fra" | "fre" => "fr".into(),
        "ger" | "deu" => "de".into(),
        "spa" => "es".into(),
        s => s.to_owned(),
    };
    let mut region = false;
    if name.contains('-') && parts.clone().next().is_some() {
        return None;
    }
    for (index, subtag) in parts.enumerate() {
        if index >= 2 || region {
            return None;
        }
        let alpha = subtag.bytes().all(|b| b.is_ascii_alphabetic());
        let part = match subtag.len() {
            4 if index == 0 && alpha => {
                let mut script = subtag.to_ascii_lowercase();
                script[..1].make_ascii_uppercase();
                script
            }
            2 if alpha => {
                region = true;
                subtag.to_ascii_uppercase()
            }
            3 if subtag.bytes().all(|b| b.is_ascii_digit()) => {
                region = true;
                subtag.to_owned()
            }
            _ => return None,
        };
        name.push('-');
        name.push_str(&part);
    }
    Some(name)
}

fn descriptor(id: &str) -> Option<SubtitleTrack> {
    if id.len() > 64 {
        return None;
    }
    let mut tags = id.split('.');
    let format = match tags.next_back()?.to_ascii_lowercase().as_str() {
        "srt" => SubtitleFormat::Srt,
        "vtt" => SubtitleFormat::Vtt,
        _ => return None,
    };
    let mut track = SubtitleTrack {
        id: id.to_owned(),
        language: None,
        forced: false,
        hearing_impaired: false,
        format,
    };
    for tag in tags {
        match tag.to_ascii_lowercase().as_str() {
            "forced" if !track.forced => track.forced = true,
            "sdh" | "cc" if !track.hearing_impaired => track.hearing_impaired = true,
            "forced" | "sdh" | "cc" => return None,
            s if track.language.is_none() => track.language = Some(language(s)?),
            _ => return None,
        }
    }
    Some(track)
}

struct VideoDirectory {
    storage: LocalStorage,
    parent: StoragePath,
    stem: String,
}

async fn directory(location: TrackLocation) -> Result<VideoDirectory> {
    let (storage, path) = track_path(location)
        .await
        .map_err(|_| Error::not_found("Video"))?;
    tokio::task::spawn_blocking(move || {
        storage
            .open_file_blocking(&path)
            .map_err(|_| Error::not_found("Video"))?;
        let parent = path.parent().ok_or_else(|| Error::not_found("Video"))?;
        let stem = path
            .file_name()
            .and_then(|name| Path::new(name).file_stem())
            .and_then(|name| name.to_str())
            .ok_or_else(|| Error::not_found("Video"))?
            .to_owned();
        Ok(VideoDirectory {
            storage,
            parent,
            stem,
        })
    })
    .await
    .map_err(|e| Error::internal(e.to_string()))?
}

pub async fn list(location: TrackLocation) -> Result<SubtitleList> {
    let video = directory(location).await?;
    tokio::task::spawn_blocking(move || {
        let mut tracks = BTreeMap::new();
        let mut truncated = false;
        let prefix = format!("{}.", video.stem);
        for (index, entry) in video.storage.directory_entries(&video.parent)?.enumerate() {
            if index == MAX_ENTRIES {
                truncated = true;
                break;
            }
            let Ok(entry) = entry else {
                continue;
            };
            let Ok(name) = entry.file_name().into_string() else {
                continue;
            };
            let Some(track) = name.strip_prefix(&prefix).and_then(descriptor) else {
                continue;
            };
            if !entry.file_type().is_ok_and(|kind| kind.is_file())
                || !entry
                    .metadata()
                    .is_ok_and(|meta| meta.len() <= MAX_BYTES as u64)
            {
                continue;
            }
            tracks.insert(track.id.clone(), track);
            if tracks.len() > MAX_TRACKS {
                tracks.pop_last();
                truncated = true;
            }
        }
        Ok(SubtitleList {
            tracks: tracks.into_values().collect(),
            truncated,
        })
    })
    .await
    .map_err(|e| Error::internal(e.to_string()))?
}

pub async fn read(location: TrackLocation, id: &str) -> Result<String> {
    let track = descriptor(id).ok_or_else(|| Error::invalid("Invalid subtitle identifier"))?;
    let video = directory(location).await?;
    tokio::task::spawn_blocking(move || {
        let file = video
            .storage
            .open_subtitle_file(&video.parent, &format!("{}.{}", video.stem, track.id))?;
        if file.metadata()?.len() > MAX_BYTES as u64 {
            return Err(Error::invalid("Subtitle exceeds 2 MiB"));
        }
        let mut bytes = Vec::new();
        file.take(MAX_BYTES as u64 + 1).read_to_end(&mut bytes)?;
        if bytes.len() > MAX_BYTES {
            return Err(Error::invalid("Subtitle exceeds 2 MiB"));
        }
        webvtt::render(&bytes, track.format)
            .ok_or_else(|| Error::invalid("Invalid or unsupported subtitle content"))
    })
    .await
    .map_err(|e| Error::internal(e.to_string()))?
}

#[cfg(test)]
mod tests;

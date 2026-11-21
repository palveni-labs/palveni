use super::*;
use palveni_core::music::{CueSegment, TrackTags};
use std::io::Read;

const MAX_SHEET: u64 = 256 * 1024;

#[derive(Default)]
struct Sheet {
    album: Option<String>,
    artist: Option<String>,
    tracks: Vec<Track>,
}
struct Track {
    file: String,
    number: u32,
    title: Option<String>,
    artist: Option<String>,
    start: Option<i64>,
}

fn label(text: &str) -> Option<String> {
    let value = if text.starts_with('"') {
        text.strip_prefix('"')?.strip_suffix('"')?
    } else {
        text
    };
    (!value.is_empty() && value.chars().count() <= 1024 && !value.chars().any(char::is_control))
        .then(|| value.to_owned())
}
fn frame(text: &str) -> Option<i64> {
    let mut fields = text.split(':');
    let mut next = || {
        let s = fields.next()?;
        (!s.is_empty() && s.len() <= 4 && s.bytes().all(|b| b.is_ascii_digit()))
            .then(|| s.parse::<i64>().ok())
            .flatten()
    };
    let (minute, second, frame) = (next()?, next()?, next()?);
    (fields.next().is_none() && second < 60 && frame < 75)
        .then_some((minute * 60 + second) * 75 + frame)
}
fn parse(text: &str) -> Option<Sheet> {
    let mut sheet = Sheet::default();
    let mut source = None;
    let mut active = false;
    for line in text.trim_start_matches('\u{feff}').lines().map(str::trim) {
        let Some((key, value)) = line.split_once(char::is_whitespace) else {
            continue;
        };
        let value = value.trim();
        match key.to_ascii_uppercase().as_str() {
            "FILE" => {
                let (name, _) = value.rsplit_once(char::is_whitespace)?;
                let name = label(name.trim_end())?;
                if name.contains(['/', '\\', ':'])
                    || is_ignored_name(&name)
                    || classify(LibraryKind::Music, &name).is_none()
                {
                    return None;
                }
                source = Some(name);
                active = false;
            }
            "TRACK" => {
                let (n, kind) = value.split_once(char::is_whitespace)?;
                let number = n.parse::<u32>().ok()?;
                if !(1..=99).contains(&number)
                    || !kind.trim().eq_ignore_ascii_case("AUDIO")
                    || sheet.tracks.last().is_some_and(|t| t.number >= number)
                {
                    return None;
                }
                sheet.tracks.push(Track {
                    file: source.clone()?,
                    number,
                    title: None,
                    artist: None,
                    start: None,
                });
                active = true;
            }
            "TITLE" | "PERFORMER" => {
                let title = key.eq_ignore_ascii_case("TITLE");
                let value = Some(label(value)?);
                if active {
                    let track = sheet.tracks.last_mut()?;
                    if title {
                        track.title = value;
                    } else {
                        track.artist = value;
                    }
                } else if sheet.tracks.is_empty() {
                    if title {
                        sheet.album = value;
                    } else {
                        sheet.artist = value;
                    }
                }
            }
            "INDEX" => {
                let (index, at) = value.split_once(char::is_whitespace)?;
                if index == "01" {
                    let track = sheet.tracks.last_mut()?;
                    if !active || track.start.replace(frame(at.trim())?).is_some() {
                        return None;
                    }
                }
            }
            _ => {}
        }
    }
    (!sheet.tracks.is_empty()).then_some(sheet)
}

struct Projected {
    sources: HashSet<String>,
    files: Vec<FoundFile>,
}
fn confined_file(
    storage: &LocalStorage,
    path: &StoragePath,
) -> Result<Option<std::fs::File>, ScanError> {
    match storage.open_file_blocking(path) {
        Ok(file) => Ok(Some(file)),
        Err(palveni_core::Error::Io(e)) => Err(ScanError::Metadata(e.to_string())),
        Err(_) => Ok(None),
    }
}

fn read_sheet(storage: &LocalStorage, path: &StoragePath) -> Result<Option<String>, ScanError> {
    let Some(file) = confined_file(storage, path)? else {
        return Ok(None);
    };
    let mut bytes = Vec::new();
    file.take(MAX_SHEET + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| ScanError::Metadata(e.to_string()))?;
    if bytes.len() as u64 > MAX_SHEET {
        return Ok(None);
    }
    Ok(String::from_utf8(bytes).ok())
}

fn project(
    storage: &LocalStorage,
    dir: &StoragePath,
    entry: &DirEntry,
    text: &str,
    sheet: Sheet,
    audio: &HashMap<String, DirEntry>,
    durations: &HashMap<String, Option<i64>>,
) -> Option<Projected> {
    let path = dir.join(&entry.name).ok()?;
    let album = sheet.album.unwrap_or_else(|| title_for_file(&entry.name));
    let stamp = palveni_db::music::cue_fingerprint(text);
    let mut sources = HashSet::new();
    let mut files = Vec::new();
    let mut last: Option<&str> = None;
    for (index, track) in sheet.tracks.iter().enumerate() {
        let entry = audio.get(&track.file)?;
        if last != Some(&track.file) && !sources.insert(track.file.clone()) {
            return None;
        }
        last = Some(&track.file);
        let source = dir.join(&track.file).ok()?;
        let end_of_file = (*durations.get(&track.file)?)?;
        let start = track.start?;
        let end = match sheet.tracks.get(index + 1) {
            Some(next) if next.file == track.file => next.start?,
            _ => end_of_file,
        };
        if end <= start || end > end_of_file || (end - start) * 2352 > i64::from(u32::MAX) - 36 {
            return None;
        }
        let title = track
            .title
            .clone()
            .unwrap_or_else(|| format!("Track {:02}", track.number));
        let virtual_path = storage.display_path(&path.join(&format!("{:02}", track.number)).ok()?);
        let tags = TrackTags {
            title: Some(title.clone()),
            artist: track.artist.clone().or_else(|| sheet.artist.clone()),
            album_artist: sheet.artist.clone(),
            album: Some(album.clone()),
            track_number: Some(track.number),
            duration_ms: Some((end - start) * 1000 / 75),
            cue: Some(CueSegment {
                source: storage.display_path(&source),
                start_frame: start,
                end_frame: end,
                stamp: stamp.clone(),
            }),
            ..TrackTags::default()
        };
        files.push(FoundFile {
            path: virtual_path,
            parent: (!dir.is_root()).then(|| storage.display_path(dir)),
            name: title,
            kind: ItemKind::Track,
            size: Some(44 + (end - start) * 2352),
            modified_ms: entry.modified.map(clock::to_millis),
            music: Some(tags),
        });
    }
    Some(Projected { sources, files })
}
pub(super) async fn collect(
    storage: &LocalStorage,
    dir: &StoragePath,
    entries: &[DirEntry],
) -> Result<(HashSet<String>, Vec<FoundFile>), ScanError> {
    let sheets: Vec<_> = entries
        .iter()
        .filter(|e| {
            e.kind == EntryKind::File
                && e.name.to_ascii_lowercase().ends_with(".cue")
                && !is_ignored_name(&e.name)
        })
        .cloned()
        .collect();
    if sheets.is_empty() {
        return Ok((HashSet::new(), Vec::new()));
    }
    let audio: HashMap<_, _> = entries
        .iter()
        .filter(|e| e.kind == EntryKind::File && classify(LibraryKind::Music, &e.name).is_some())
        .map(|e| (e.name.clone(), e.clone()))
        .collect();
    let root = storage.clone();
    let dir = dir.clone();
    tokio::task::spawn_blocking(move || {
        let mut projected = Vec::new();
        let mut durations = HashMap::new();
        for entry in &sheets {
            let Ok(path) = dir.join(&entry.name) else {
                continue;
            };
            let Some(text) = read_sheet(&root, &path)? else {
                continue;
            };
            let Some(sheet) = parse(&text) else { continue };
            for track in &sheet.tracks {
                if durations.contains_key(&track.file) || !audio.contains_key(&track.file) {
                    continue;
                }
                let Ok(source) = dir.join(&track.file) else {
                    continue;
                };
                let duration = match confined_file(&root, &source)? {
                    Some(file) => crate::tags::duration_frames(file)
                        .map_err(|e| ScanError::Metadata(e.to_string()))?,
                    None => None,
                };
                durations.insert(track.file.clone(), duration);
            }
            if let Some(result) = project(&root, &dir, entry, &text, sheet, &audio, &durations) {
                projected.push(result);
            }
        }
        let mut references = HashMap::new();
        for s in &projected {
            for name in &s.sources {
                *references.entry(name.clone()).or_insert(0) += 1;
            }
        }
        let mut sources = HashSet::new();
        let mut files = Vec::new();
        for sheet in projected {
            if sheet.sources.iter().all(|n| references[n] == 1) {
                sources.extend(sheet.sources);
                files.extend(sheet.files);
            }
        }
        Ok((sources, files))
    })
    .await
    .map_err(|e| ScanError::Metadata(e.to_string()))?
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn conflicting_malformed_oversize_and_escaping_sheets_keep_original_files() {
        use std::fs;
        let temp = tempfile::tempdir().unwrap();
        let directory = temp.path().join("music");
        fs::create_dir(&directory).unwrap();
        let root = LocalStorage::open(&directory).await.unwrap();
        let path = StoragePath::root();
        let mut audio=b"RIFF\0\0\0\0WAVEfmt \x10\0\0\0\x01\0\x01\0\x40\x1f\0\0\x80\x3e\0\0\x02\0\x10\0data\0\0\0\0".to_vec();
        audio[4..8].copy_from_slice(&16036u32.to_le_bytes());
        audio[40..44].copy_from_slice(&16000u32.to_le_bytes());
        audio.resize(16044, 0);
        fs::write(directory.join("source.wav"), &audio).unwrap();
        let content = "FILE \"source.wav\" WAVE\nTRACK 01 AUDIO\nINDEX 01 00:00:00";
        let sheet = directory.join("one.cue");
        fs::write(&sheet, content).unwrap();
        let other = directory.join("two.cue");
        fs::write(&other, content).unwrap();
        async fn count(root: &LocalStorage, path: &StoragePath) -> usize {
            let entries = root.list_dir(path).await.unwrap();
            collect(root, path, &entries).await.unwrap().1.len()
        }
        assert_eq!(count(&root, &path).await, 0);
        fs::remove_file(&other).unwrap();
        assert_eq!(count(&root, &path).await, 1);
        fs::write(&sheet, format!("{content}\nREM {}", "x".repeat(256 * 1024))).unwrap();
        assert_eq!(count(&root, &path).await, 0);
        fs::write(&sheet, b"\xff\xfe").unwrap();
        assert_eq!(count(&root, &path).await, 0);
        #[cfg(unix)]
        {
            let external = temp.path().join("elsewhere.wav");
            fs::write(&external, audio).unwrap();
            std::os::unix::fs::symlink(&external, directory.join("link.wav")).unwrap();
            fs::write(&sheet, content.replace("source.wav", "link.wav")).unwrap();
            assert_eq!(count(&root, &path).await, 0);
        }
    }
    #[test]
    fn rejects_paths_duplicate_tracks_and_invalid_times() {
        for invalid in [
            "../out.wav",
            "sub/in.wav",
            "\\server.wav",
            "C:in.wav",
            "index.cue",
            ".hidden.wav",
        ] {
            assert!(
                parse(&format!(
                    "FILE \"{invalid}\" WAVE\nTRACK 01 AUDIO\nINDEX 01 00:00:00"
                ))
                .is_none()
            );
        }
        for invalid in ["00:01:75", "00:60:00", "-2:00:00", "00:01:", "1:2:3:4"] {
            assert!(frame(invalid).is_none());
        }
        assert!(
            parse("FILE \"in.wav\" WAVE\nTRACK 01 AUDIO\nINDEX 01 00:00:00\nTRACK 01 AUDIO")
                .is_none()
        );
        assert!(
            parse("FILE \"in.wav\" WAVE\nTRACK 01 AUDIO\nINDEX 01 00:00:00\nINDEX 01 00:00:12")
                .is_none()
        );
        assert_eq!(frame("01:02:03"), Some(4653));
    }
}

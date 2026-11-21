//! Library-scoped independent music search. The primary database is the source of truth.
use std::{
    collections::{HashMap, HashSet},
    fs::{self, File},
    io::{BufReader, BufWriter, Write},
    path::{Path, PathBuf},
    sync::Arc,
};

use palveni_core::{
    Error, LibraryId, Result,
    music::{MusicSearchHit, MusicSearchKind},
};
use palveni_db::{Db, music::search::SearchTrack};
use pinyin::ToPinyin;
use serde::{Deserialize, Serialize};
use tokio::{
    sync::{Mutex, Semaphore},
    task::spawn_blocking,
};

const FORMAT: u8 = 2;

pub struct MusicSearch {
    directory: PathBuf,
    cached: Mutex<Option<Arc<Index>>>,
    readers: Semaphore,
}

pub struct Matches {
    pub entries: Vec<MusicSearchHit>,
    pub next: Option<usize>,
    pub revision: i64,
}

#[derive(Deserialize, Serialize)]
struct Index {
    version: u8,
    library: LibraryId,
    revision: i64,
    artists: Vec<Entry>,
    albums: Vec<Entry>,
    tracks: Vec<Entry>,
}

#[derive(Deserialize, Serialize)]
struct Entry {
    hit: MusicSearchHit,
    words: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    sample: Option<String>,
}

fn normalize(text: &str) -> String {
    if text.is_ascii() {
        return text.to_ascii_lowercase();
    }
    fast2s::convert(text).to_lowercase()
}

pub fn terms(query: &str) -> Result<Vec<String>> {
    let query = query.trim();
    if query.is_empty() || query.chars().count() > 128 || query.chars().any(char::is_control) {
        return Err(Error::invalid(
            "Expected a search query of 1 to 128 characters",
        ));
    }
    let terms: Vec<_> = query.split_whitespace().map(normalize).collect();
    if terms.len() > 8 {
        return Err(Error::invalid("Use at most eight search terms"));
    }
    Ok(terms)
}

impl Entry {
    fn new(hit: MusicSearchHit, album_artist: Option<&str>) -> Self {
        let fields = [
            Some(hit.title.as_str()),
            hit.artist.as_deref(),
            hit.album.as_deref(),
            album_artist,
        ];
        let mut words = String::new();
        let mut previous = Vec::with_capacity(fields.len());
        for field in fields.into_iter().flatten() {
            if previous.contains(&field) {
                continue;
            }
            if !words.is_empty() {
                words.push('\0');
            }
            let canonical = normalize(field);
            if canonical.chars().any(|c| c.to_pinyin().is_some()) {
                let mut initials = String::new();
                for c in canonical.chars() {
                    if let Some(pinyin) = c.to_pinyin() {
                        words.push_str(pinyin.plain());
                        initials.push(pinyin.plain().as_bytes()[0] as char);
                    } else {
                        words.push(c);
                        initials.push(c);
                    }
                }
                words.push('\0');
                words.push_str(&initials);
                words.push('\0');
            }
            words.push_str(&canonical);
            previous.push(field);
        }
        Self {
            hit,
            words,
            sample: None,
        }
    }
}

#[derive(Default)]
struct Builder {
    artists: HashMap<String, Entry>,
    albums: HashMap<(String, String), Entry>,
    tracks: Vec<Entry>,
}

impl Builder {
    fn groups(&mut self, track: &SearchTrack) {
        if let Some(name) = &track.album_artist {
            self.artists
                .entry(track.artist_key.clone())
                .or_insert_with(|| {
                    let mut entry = Entry::new(
                        MusicSearchHit {
                            id: track.artist_key.clone(),
                            title: name.clone(),
                            artist: None,
                            album: None,
                            artist_id: None,
                        },
                        None,
                    );
                    entry.sample = Some(track.id.clone());
                    entry
                });
        }
        if let Some(name) = &track.album {
            self.albums
                .entry((track.artist_key.clone(), track.album_key.clone()))
                .or_insert_with(|| {
                    let mut entry = Entry::new(
                        MusicSearchHit {
                            id: track.album_key.clone(),
                            title: name.clone(),
                            artist: track.album_artist.clone(),
                            album: None,
                            artist_id: Some(track.artist_key.clone()),
                        },
                        None,
                    );
                    entry.sample = Some(track.id.clone());
                    entry
                });
        }
    }

    fn extend(&mut self, tracks: Vec<SearchTrack>) {
        for track in tracks {
            self.groups(&track);
            self.tracks.push(Entry::new(
                MusicSearchHit {
                    id: track.id,
                    title: track.title,
                    artist: track.artist,
                    album: track.album,
                    artist_id: None,
                },
                track.album_artist.as_deref(),
            ));
        }
    }

    fn finish(self, library: LibraryId, revision: i64) -> Index {
        Index {
            version: FORMAT,
            library,
            revision,
            artists: ordered(self.artists.into_values().collect()),
            albums: ordered(self.albums.into_values().collect()),
            tracks: ordered(self.tracks),
        }
    }
}

fn order(entries: &mut [Entry]) {
    // The first field is the pinyin or lowercase title. IDs distinguish equal titles.
    entries.sort_unstable_by(|a, b| {
        a.words
            .split('\0')
            .next()
            .cmp(&b.words.split('\0').next())
            .then_with(|| a.hit.artist_id.cmp(&b.hit.artist_id))
            .then_with(|| a.hit.id.cmp(&b.hit.id))
    });
}

fn ordered(mut entries: Vec<Entry>) -> Vec<Entry> {
    order(&mut entries);
    entries
}

impl Index {
    fn load(path: &Path, library: LibraryId, revision: i64) -> Option<Self> {
        let index: Self = serde_json::from_reader(BufReader::new(File::open(path).ok()?)).ok()?;
        (index.version == FORMAT && index.library == library && index.revision <= revision)
            .then_some(index)
    }

    fn affected_groups(&self, changes: &HashSet<String>) -> Vec<(String, Option<String>)> {
        self.artists
            .iter()
            .chain(&self.albums)
            .filter(|row| row.sample.as_ref().is_some_and(|id| changes.contains(id)))
            .map(|row| match &row.hit.artist_id {
                None => (row.hit.id.clone(), None),
                Some(artist) => (artist.clone(), Some(row.hit.id.clone())),
            })
            .collect()
    }

    fn apply(
        &mut self,
        ids: &HashSet<String>,
        tracks: Vec<SearchTrack>,
        samples: Vec<SearchTrack>,
        revision: i64,
    ) {
        let mut updates = Builder::default();
        for sample in &samples {
            updates.groups(sample);
        }
        updates.extend(tracks);
        self.tracks.retain(|row| !ids.contains(&row.hit.id));
        if !updates.tracks.is_empty() {
            self.tracks.extend(updates.tracks);
            order(&mut self.tracks);
        }
        fn retain_group(
            row: &mut Entry,
            replacement: Option<Entry>,
            changed: &HashSet<String>,
        ) -> bool {
            if row.sample.as_ref().is_some_and(|id| changed.contains(id)) {
                if let Some(replacement) = replacement {
                    *row = replacement;
                } else {
                    return false;
                }
            }
            true
        }
        self.artists.retain_mut(|row| {
            let replacement = updates.artists.remove(&row.hit.id);
            retain_group(row, replacement, ids)
        });
        self.albums.retain_mut(|row| {
            let replacement = if updates.albums.is_empty() {
                None
            } else {
                row.hit
                    .artist_id
                    .as_ref()
                    .and_then(|artist| updates.albums.remove(&(artist.clone(), row.hit.id.clone())))
            };
            retain_group(row, replacement, ids)
        });
        if !updates.artists.is_empty() {
            self.artists.extend(updates.artists.into_values());
            order(&mut self.artists);
        }
        if !updates.albums.is_empty() {
            self.albums.extend(updates.albums.into_values());
            order(&mut self.albums);
        }
        self.revision = revision;
    }

    fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(directory) = path.parent() {
            fs::create_dir_all(directory)?;
        }
        let temporary = path.with_extension(format!("{}.tmp", std::process::id()));
        let mut writer = BufWriter::new(File::create(&temporary)?);
        serde_json::to_writer(&mut writer, self)?;
        writer.flush()?;
        drop(writer);
        fs::rename(temporary, path)
    }

    fn matches(
        &self,
        kind: MusicSearchKind,
        terms: &[String],
        offset: usize,
        limit: usize,
    ) -> Result<Matches> {
        let rows = match kind {
            MusicSearchKind::Artist => &self.artists,
            MusicSearchKind::Album => &self.albums,
            MusicSearchKind::Track => &self.tracks,
        };
        if offset > rows.len() {
            return Err(Error::invalid("Invalid music search position"));
        }
        let mut entries = Vec::with_capacity(limit);
        let mut next = None;
        for (i, entry) in rows.iter().enumerate().skip(offset) {
            if terms.iter().all(|s| entry.words.contains(s)) {
                if entries.len() == limit {
                    next = Some(i);
                    break;
                }
                entries.push(entry.hit.clone());
            }
        }
        Ok(Matches {
            entries,
            next,
            revision: self.revision,
        })
    }
}

fn join_error(err: tokio::task::JoinError) -> Error {
    Error::internal(err.to_string())
}

impl MusicSearch {
    pub fn new(directory: PathBuf) -> Self {
        Self {
            directory,
            cached: Mutex::new(None),
            readers: Semaphore::new(2),
        }
    }

    async fn index(
        &self,
        db: &Db,
        library: LibraryId,
        expected: Option<i64>,
    ) -> Result<Arc<Index>> {
        let mut slot = self.cached.lock().await;
        let revision = db.music().search_revision(library).await?;
        if expected.is_some_and(|r| r != revision) {
            return Err(Error::conflict("Music library changed; restart the search"));
        }
        if let Some(index) = slot.as_ref()
            && index.library == library
            && index.revision == revision
        {
            return Ok(index.clone());
        }
        let path = self.directory.join(format!("music-{library}.json"));
        let saved = match slot.take() {
            Some(active) if active.library == library => Arc::try_unwrap(active).ok(),
            _ => {
                let path = path.clone();
                spawn_blocking(move || Index::load(&path, library, revision))
                    .await
                    .map_err(join_error)?
            }
        };
        let saved = if let Some(index) = saved {
            if index.revision == revision {
                Some(index)
            } else {
                self.update(db, index, revision).await?
            }
        } else {
            None
        };
        let index = if let Some(index) = saved {
            index
        } else {
            let mut builder = Builder::default();
            let mut after = String::new();
            loop {
                let batch = db.music().search_tracks(library, &after).await?;
                let Some(last) = batch.last() else {
                    break;
                };
                after.clone_from(&last.id);
                builder = spawn_blocking(move || {
                    builder.extend(batch);
                    builder
                })
                .await
                .map_err(join_error)?;
            }
            if db.music().search_revision(library).await? != revision {
                return Err(Error::conflict("Music library changed; restart the search"));
            }
            spawn_blocking(move || {
                let index = builder.finish(library, revision);
                if let Err(error) = index.save(&path) {
                    tracing::warn!(%error, "could not persist music search index");
                }
                index
            })
            .await
            .map_err(join_error)?
        };
        let index = Arc::new(index);
        *slot = Some(index.clone());
        Ok(index)
    }

    async fn update(&self, db: &Db, index: Index, revision: i64) -> Result<Option<Index>> {
        let library = index.library;
        let Some(changes) = db.music().search_changes(library, index.revision).await? else {
            return Ok(None);
        };
        let ids: HashSet<_> = changes.ids.into_iter().collect();
        let (mut index, ids, groups) = spawn_blocking(move || {
            let groups = index.affected_groups(&ids);
            (index, ids, groups)
        })
        .await
        .map_err(join_error)?;
        let mut samples = Vec::new();
        for (artist, album) in groups {
            if let Some(sample) = db
                .music()
                .search_group_sample(library, &artist, album.as_deref())
                .await?
            {
                samples.push(sample);
            }
        }
        if db.music().search_revision(library).await? != revision {
            return Err(Error::conflict("Music library changed; restart the search"));
        }
        spawn_blocking(move || {
            index.apply(&ids, changes.tracks, samples, revision);
            Some(index)
        })
        .await
        .map_err(join_error)
    }

    pub async fn forget(&self, library: LibraryId) {
        let mut slot = self.cached.lock().await;
        if slot.as_ref().is_some_and(|index| index.library == library) {
            *slot = None;
        }
        let path = self.directory.join(format!("music-{library}.json"));
        if let Err(error) = tokio::fs::remove_file(path).await
            && error.kind() != std::io::ErrorKind::NotFound
        {
            tracing::warn!(%error, "could not remove music search index");
        }
    }

    pub async fn search(
        &self,
        db: &Db,
        library: LibraryId,
        kind: MusicSearchKind,
        query: Vec<String>,
        limit: usize,
        cursor: Option<(i64, usize)>,
    ) -> Result<Matches> {
        let _permit = self
            .readers
            .acquire()
            .await
            .map_err(|e| Error::internal(e.to_string()))?;
        let index = self.index(db, library, cursor.map(|c| c.0)).await?;
        let start = cursor.map_or(0, |c| c.1);
        spawn_blocking(move || index.matches(kind, &query, start, limit.clamp(1, 200)))
            .await
            .map_err(join_error)?
    }
}

#[cfg(test)]
mod tests;

//! Independent per-library music indexes. The primary database is the source of truth.
use std::{
    collections::{HashMap, HashSet, VecDeque},
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
const CACHE_BUDGET: usize = 64 * 1024 * 1024;
mod global;
pub use global::{GlobalCursor, GlobalMatches};

pub struct MusicSearch {
    directory: PathBuf,
    cached: Mutex<VecDeque<(Arc<Index>, usize)>>,
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
    fn sort_key(&self) -> (&str, Option<&str>, &str) {
        (
            self.words.split('\0').next().unwrap_or_default(),
            self.hit.artist_id.as_deref(),
            &self.hit.id,
        )
    }

    fn contains_terms(&self, terms: &[String]) -> bool {
        terms.iter().all(|s| self.words.contains(s))
    }

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
    entries.sort_unstable_by(|a, b| a.sort_key().cmp(&b.sort_key()));
}

fn ordered(mut entries: Vec<Entry>) -> Vec<Entry> {
    order(&mut entries);
    entries
}

impl Index {
    fn rows(&self, kind: MusicSearchKind) -> &[Entry] {
        match kind {
            MusicSearchKind::Artist => &self.artists,
            MusicSearchKind::Album => &self.albums,
            MusicSearchKind::Track => &self.tracks,
        }
    }

    fn memory_bytes(&self) -> usize {
        let mut bytes = size_of::<Self>();
        for rows in [&self.artists, &self.albums, &self.tracks] {
            bytes += rows.capacity() * size_of::<Entry>();
            for entry in rows {
                let hit = &entry.hit;
                bytes += entry.words.capacity() + hit.id.capacity() + hit.title.capacity();
                bytes += [&entry.sample, &hit.artist, &hit.artist_id, &hit.album]
                    .into_iter()
                    .flatten()
                    .map(String::capacity)
                    .sum::<usize>();
            }
        }
        bytes
    }

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
        let rows = self.rows(kind);
        if offset > rows.len() {
            return Err(Error::invalid("Invalid music search position"));
        }
        let mut entries = Vec::with_capacity(limit);
        let mut next = None;
        for (i, entry) in rows.iter().enumerate().skip(offset) {
            if entry.contains_terms(terms) {
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
            cached: Mutex::new(VecDeque::new()),
            readers: Semaphore::new(2),
        }
    }

    async fn index(&self, db: &Db, library: LibraryId, revision: i64) -> Result<Arc<Index>> {
        let mut cached = self.cached.lock().await;
        let active = cached
            .iter()
            .position(|(index, _)| index.library == library)
            .and_then(|i| cached.remove(i));
        if let Some((index, weight)) = active.as_ref()
            && index.revision >= revision
        {
            cached.push_back((index.clone(), *weight));
            return if index.revision == revision {
                Ok(index.clone())
            } else {
                Err(Error::conflict("Music library changed; restart the search"))
            };
        }
        if cached
            .front()
            .is_some_and(|(_, weight)| *weight > CACHE_BUDGET)
        {
            cached.pop_front();
        }
        let path = self.directory.join(format!("music-{library}.json"));
        let saved = match active.and_then(|(index, _)| Arc::try_unwrap(index).ok()) {
            Some(index) => Some(index),
            None => {
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
        let (index, weight) = spawn_blocking(move || {
            let weight = index.memory_bytes();
            (Arc::new(index), weight)
        })
        .await
        .map_err(join_error)?;
        // Keep small indexes hot together. A single larger active library fits as before.
        let mut total = weight + cached.iter().map(|(_, bytes)| *bytes).sum::<usize>();
        while total > CACHE_BUDGET || cached.len() >= 8 {
            let Some((_, evicted)) = cached.pop_front() else {
                break;
            };
            total -= evicted;
        }
        cached.push_back((index.clone(), weight));
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
        let mut cached = self.cached.lock().await;
        cached.retain(|(index, _)| index.library != library);
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
        let revision = db.music().search_revision(library).await?;
        if cursor.is_some_and(|c| c.0 != revision) {
            return Err(Error::conflict("Music library changed; restart the search"));
        }
        let index = self.index(db, library, revision).await?;
        let start = cursor.map_or(0, |c| c.1);
        spawn_blocking(move || index.matches(kind, &query, start, limit.clamp(1, 200)))
            .await
            .map_err(join_error)?
    }
}

#[cfg(test)]
mod tests;

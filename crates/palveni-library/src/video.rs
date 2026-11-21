//! Local film and TV sidecars. Discovery uses the scanner's existing directory listing.
mod nfo;
mod recognition;
pub mod subtitles;
#[cfg(test)]
mod tests;

use crate::classify::extension_of;
use crate::{LocalStorage, ScanError, ScanObserver, ScanStep};
use palveni_core::{
    Error, LibraryId, Result,
    storage::{ByteStream, DirEntry, EntryKind, StoragePath, StorageProvider},
};
use palveni_db::{
    Db,
    video::{VideoMetadata, sidecar_fingerprint},
};
use std::collections::HashMap;
use std::fmt::Write as _;
use std::io::{Read, Seek};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

const MAX_NFO: u64 = 512 * 1024;
const MAX_POSTER: u64 = 12 * 1024 * 1024;

pub(super) fn storage_path(root: &LocalStorage, absolute: &Path) -> Option<StoragePath> {
    let mut path = StoragePath::root();
    for part in absolute.strip_prefix(root.root()).ok()? {
        path = path.join(part.to_str()?).ok()?;
    }
    Some(path)
}

#[derive(Clone, Debug)]
struct Sidecar {
    path: PathBuf,
    bytes: u64,
    modified: i64,
}

#[derive(Default, Debug)]
struct Folder {
    files: HashMap<String, Sidecar>,
    films: usize,
}

impl Folder {
    fn file(&self, name: &str) -> Option<&Sidecar> {
        self.files.get(&name.to_ascii_lowercase())
    }
    fn poster(&self, stems: &[&str]) -> Option<&Sidecar> {
        stems.iter().find_map(|name| {
            ["jpg", "jpeg", "png", "webp"].iter().find_map(|ext| {
                self.file(&format!("{name}.{ext}"))
                    .filter(|s| s.bytes <= MAX_POSTER)
            })
        })
    }
}

#[derive(Default, Debug)]
pub(crate) struct Inventory {
    folders: HashMap<PathBuf, Folder>,
}

impl Inventory {
    pub(crate) fn observe(
        &mut self,
        storage: &LocalStorage,
        dir: &StoragePath,
        entries: &[DirEntry],
    ) {
        let parent = PathBuf::from(storage.display_path(dir));
        let mut folder = Folder::default();
        for entry in entries
            .iter()
            .filter(|e| e.kind == EntryKind::File && !crate::classify::is_ignored_name(&e.name))
        {
            if crate::classify::classify(palveni_core::LibraryKind::Video, &entry.name).is_some() {
                folder.films += 1;
            }
            if matches!(
                extension_of(&entry.name).as_deref(),
                Some("nfo" | "jpg" | "jpeg" | "png" | "webp")
            ) {
                let Some(bytes) = entry.size else { continue };
                if bytes > MAX_POSTER {
                    continue;
                }
                folder.files.insert(
                    entry.name.to_ascii_lowercase(),
                    Sidecar {
                        path: parent.join(&entry.name),
                        bytes,
                        modified: entry
                            .modified
                            .map(palveni_core::clock::to_millis)
                            .unwrap_or(0),
                    },
                );
            }
        }
        if folder.films > 0 || !folder.files.is_empty() {
            self.folders.insert(parent, folder);
        }
    }

    fn candidate(&self, path: &Path, root: &Path) -> Option<Candidate> {
        let folder_path = path.parent()?;
        let folder = self.folders.get(folder_path)?;
        let stem = path.file_stem()?.to_str()?;
        let named_nfo = folder.file(&format!("{stem}.nfo"));
        let file_nfo = named_nfo
            .or_else(|| {
                (folder.films == 1)
                    .then(|| folder.file("movie.nfo"))
                    .flatten()
            })
            .filter(|s| s.bytes <= MAX_NFO);
        let season_folder = folder_path
            .file_name()
            .and_then(|n| n.to_str())
            .and_then(recognition::season_folder);
        let show_dir = if season_folder.is_some() && folder_path != root {
            folder_path.parent()?
        } else {
            folder_path
        };
        let show_folder = self.folders.get(show_dir);
        let show_nfo = show_folder
            .and_then(|d| d.file("tvshow.nfo"))
            .filter(|s| s.bytes <= MAX_NFO);
        let mark = recognition::episode_marker(stem);
        let movie_poster = folder
            .poster(&[&format!("{stem}-poster"), &format!("{stem}-cover"), stem])
            .or_else(|| {
                (folder.films == 1)
                    .then(|| folder.poster(&["poster", "movie-poster", "folder", "cover"]))
                    .flatten()
            });
        let marker_start = mark
            .map(|m| m.start)
            .or_else(|| recognition::episode_only(stem).map(|(start, _)| start));
        let folder_names_series = marker_start.is_some_and(|start| {
            let by_file = recognition::clean_title(&stem[..start]);
            show_dir
                .file_name()
                .and_then(|s| s.to_str())
                .is_some_and(|name| {
                    recognition::title_year(name)
                        .0
                        .eq_ignore_ascii_case(&by_file)
                })
        });
        let show_poster = show_folder
            .filter(|_| show_nfo.is_some() || season_folder.is_some() || folder_names_series)
            .and_then(|d| d.poster(&["poster", "tvshow-poster", "folder", "cover"]));
        let season = season_folder.or(mark.map(|m| m.season));
        let season_poster = season
            .and_then(|s| {
                show_folder.and_then(|d| {
                    if s == 0 {
                        d.poster(&["season-specials-poster", "season00-poster", "season00"])
                    } else {
                        d.poster(&[&format!("season{s:02}-poster"), &format!("season{s:02}")])
                    }
                })
            })
            .or_else(|| season_folder.and_then(|_| folder.poster(&["poster", "folder", "cover"])));
        let episode_poster = folder.poster(&[&format!("{stem}-thumb"), &format!("{stem}-poster")]);
        let owns_folder = folder.films == 1 && folder_path != root;
        let mut signature = format!("video-sidecars-1/{owns_folder}");
        for file in [
            file_nfo,
            show_nfo,
            movie_poster,
            show_poster,
            season_poster,
            episode_poster,
        ] {
            signature.push('|');
            if let Some(file) = file {
                let p = file.path.to_str()?;
                let _ = write!(
                    signature,
                    "{}:{p}/{}:{}",
                    p.len(),
                    file.bytes,
                    file.modified
                );
            }
        }
        Some(Candidate {
            path: path.to_owned(),
            show_dir: show_dir.to_owned(),
            file_nfo: file_nfo.map(|s| s.path.clone()),
            show_nfo: show_nfo.map(|s| s.path.clone()),
            movie_poster: movie_poster.map(|s| s.path.clone()),
            show_poster: show_poster.map(|s| s.path.clone()),
            season_poster: season_poster.map(|s| s.path.clone()),
            episode_poster: episode_poster.map(|s| s.path.clone()),
            season_folder,
            stamp: sidecar_fingerprint(&signature),
            owns_folder,
        })
    }

    pub(crate) async fn index(
        &self,
        db: &Db,
        library: LibraryId,
        roots: Arc<Vec<LocalStorage>>,
        observer: &dyn ScanObserver,
        allowed: impl Fn(&str) -> bool,
    ) -> std::result::Result<(), ScanError> {
        let started = Instant::now();
        let indexed = db.video().scan_index(library).await?;
        let examined = indexed.len();
        let mut pending = Vec::new();
        for (file, (id, stamp)) in indexed {
            if observer.is_cancelled() {
                return Err(ScanError::Cancelled);
            }
            if !allowed(&file) {
                continue;
            }
            let path = Path::new(&file);
            let Some((root_idx, root)) = roots
                .iter()
                .enumerate()
                .find(|(_, r)| path.starts_with(r.root()))
            else {
                continue;
            };
            if let Some(candidate) = self.candidate(path, root.root())
                && stamp.as_ref() != Some(&candidate.stamp)
            {
                pending.push((id, root_idx, candidate));
            }
        }
        // Keep episodes next to each other so a batch reads their shared NFO only once.
        pending.sort_unstable_by(|left, right| left.2.path.cmp(&right.2.path));
        let total = pending.len();
        let mut done = 0;
        let mut read_time = Duration::ZERO;
        let mut longest_read = Duration::ZERO;
        let mut write_time = Duration::ZERO;
        let mut pending = pending.into_iter();
        while done < total {
            if observer.is_cancelled() {
                return Err(ScanError::Cancelled);
            }
            observer.progress(
                Some(done as f64 / total as f64),
                &ScanStep::Indexing {
                    done: done as u64,
                    total: total as u64,
                },
            );
            let batch: Vec<_> = pending.by_ref().take(48).collect();
            let batch_size = batch.len();
            let roots = roots.clone();
            let (rows, batch_read_time) = tokio::task::spawn_blocking(move || {
                let started = Instant::now();
                let mut shared = HashMap::new();
                let rows = batch
                    .into_iter()
                    .map(|(id, index, candidate)| candidate.read(id, &roots[index], &mut shared))
                    .collect::<Vec<_>>();
                (rows, started.elapsed())
            })
            .await
            .map_err(|e| ScanError::Metadata(e.to_string()))?;
            read_time += batch_read_time;
            longest_read = longest_read.max(batch_read_time);
            let write_started = Instant::now();
            db.video().save_batch(library, rows).await?;
            write_time += write_started.elapsed();
            done += batch_size;
        }
        tracing::info!(
            %library, examined, updated = done,
            elapsed_seconds = started.elapsed().as_secs_f64(),
            worker_read_seconds = read_time.as_secs_f64(),
            longest_read_batch_seconds = longest_read.as_secs_f64(),
            database_write_seconds = write_time.as_secs_f64(),
            "video metadata indexing completed"
        );
        Ok(())
    }
}

struct Candidate {
    path: PathBuf,
    show_dir: PathBuf,
    file_nfo: Option<PathBuf>,
    show_nfo: Option<PathBuf>,
    movie_poster: Option<PathBuf>,
    show_poster: Option<PathBuf>,
    season_poster: Option<PathBuf>,
    episode_poster: Option<PathBuf>,
    season_folder: Option<i32>,
    owns_folder: bool,
    stamp: String,
}

fn path_string(path: Option<&PathBuf>) -> Option<String> {
    path.and_then(|p| p.to_str()).map(str::to_owned)
}

impl Candidate {
    fn read(
        self,
        id: palveni_core::ItemId,
        root: &LocalStorage,
        cache: &mut HashMap<PathBuf, Option<nfo::Nfo>>,
    ) -> VideoMetadata {
        let file = self
            .file_nfo
            .as_ref()
            .and_then(|p| nfo::read(root, p))
            .unwrap_or_default();
        let show = self
            .show_nfo
            .as_ref()
            .and_then(|p| {
                cache
                    .entry(p.clone())
                    .or_insert_with(|| nfo::read(root, p))
                    .clone()
            })
            .filter(|s| s.root == "tvshow")
            .unwrap_or_default();
        let stem = self.path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        let marker = recognition::episode_marker(stem);
        let episode_only = recognition::episode_only(stem);
        let series = file.root == "episodedetails"
            || (file.root != "movie"
                && (marker.is_some()
                    || episode_only.is_some()
                    || self.season_folder.is_some()
                    || show.root == "tvshow"));
        let name = self
            .show_dir
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(stem);
        let folder_title = recognition::title_year(name);
        let film_title = recognition::title_year(stem);
        let metadata_title = file.title.clone();
        let season = series.then(|| {
            file.season
                .or_else(|| marker.map(|m| m.season))
                .or(self.season_folder)
                .unwrap_or(1)
        });
        let episode = series
            .then(|| {
                file.episode
                    .or_else(|| marker.map(|m| m.episode))
                    .or_else(|| episode_only.map(|(_, n)| n))
            })
            .flatten();
        let (work_group, work_title, year, work_overview, work_poster) = if series {
            let owns_series = self.season_folder.is_some() || self.show_nfo.is_some();
            let fallback_title = if owns_series {
                folder_title.0.clone()
            } else {
                marker
                    .map(|m| m.start)
                    .or_else(|| episode_only.map(|(start, _)| start))
                    .and_then(|start| stem.get(..start))
                    .map(recognition::clean_title)
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| folder_title.0.clone())
            };
            let title = if owns_series {
                show.title
            } else {
                file.show_title.clone()
            }
            .unwrap_or(fallback_title);
            // A flat directory may contain several different series.
            let group = if owns_series {
                self.show_dir.to_string_lossy().into_owned()
            } else {
                format!("{}\n{}", self.show_dir.display(), title.to_lowercase())
            };
            (
                format!("series\n{group}"),
                title,
                show.year.or(folder_title.1),
                show.plot,
                path_string(
                    self.show_poster
                        .as_ref()
                        .or(self.season_poster.as_ref())
                        .or(self.episode_poster.as_ref()),
                ),
            )
        } else {
            let fallback = if self.owns_folder && folder_title.1.is_some() {
                folder_title.clone()
            } else {
                film_title.clone()
            };
            (
                format!("movie\n{}", self.path.display()),
                metadata_title.clone().unwrap_or(fallback.0),
                file.year.or(fallback.1),
                file.plot.clone(),
                path_string(self.movie_poster.as_ref()),
            )
        };
        let season_poster =
            path_string(self.season_poster.as_ref()).or_else(|| work_poster.clone());
        let episode_poster =
            path_string(self.episode_poster.as_ref()).or_else(|| season_poster.clone());
        VideoMetadata {
            item_id: id,
            stamp: self.stamp,
            work_group,
            work_title,
            year,
            work_overview,
            work_poster,
            season,
            season_poster,
            episode,
            title: metadata_title.unwrap_or(film_title.0),
            overview: file.plot,
            episode_poster,
        }
    }
}

pub async fn open_poster(
    location: palveni_db::music::TrackLocation,
) -> Result<(&'static str, u64, ByteStream)> {
    let (root, path) = crate::local::track_path(location).await?;
    tokio::task::spawn_blocking(move || {
        let mut file = root.open_file_blocking(&path)?;
        let size = file.metadata()?.len();
        if !(3..=MAX_POSTER).contains(&size) {
            return Err(Error::not_found("poster"));
        }
        let mut header = [0u8; 12];
        let bytes = &mut header[..size.min(12) as usize];
        file.read_exact(bytes)?;
        let media = crate::artwork::mime(bytes).ok_or_else(|| Error::not_found("poster"))?;
        file.rewind()?;
        let reader = tokio::io::AsyncReadExt::take(tokio::fs::File::from_std(file), size);
        Ok((media, size, Box::pin(reader) as ByteStream))
    })
    .await
    .map_err(|e| Error::internal(e.to_string()))?
}

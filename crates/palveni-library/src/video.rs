//! Local film and TV sidecars. Discovery uses the scanner's existing directory listing.
mod images;
mod nfo;
mod recognition;
pub mod subtitles;
#[cfg(test)]
mod tests;

pub use images::{VideoImages, snap_size};

use crate::classify::extension_of;
use crate::{LocalStorage, ScanError, ScanObserver, ScanStep};
use palveni_core::{
    Error, LibraryId, Result,
    storage::{ByteStream, DirEntry, EntryKind, StoragePath, StorageProvider},
};
use palveni_db::{
    Db,
    video::{VideoMetadata, VideoPersonInput, sidecar_fingerprint},
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
    actors: HashMap<String, Sidecar>,
    videos: HashMap<String, (u64, i64)>,
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
        let mut folder = self.folders.remove(&parent).unwrap_or_default();
        for entry in entries
            .iter()
            .filter(|e| e.kind == EntryKind::File && !crate::classify::is_ignored_name(&e.name))
        {
            let modified = entry
                .modified
                .map(palveni_core::clock::to_millis)
                .unwrap_or(0);
            if crate::classify::classify(palveni_core::LibraryKind::Video, &entry.name).is_some() {
                folder.films += 1;
                folder.videos.insert(
                    entry.name.to_ascii_lowercase(),
                    (entry.size.unwrap_or(0), modified),
                );
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
                        modified,
                    },
                );
            }
        }
        if folder.films > 0 || !folder.files.is_empty() || !folder.actors.is_empty() {
            self.folders.insert(parent, folder);
        }
    }

    pub(crate) fn observe_actors(
        &mut self,
        storage: &LocalStorage,
        parent_dir: &StoragePath,
        actors_dir: &StoragePath,
        entries: &[DirEntry],
    ) {
        let parent = PathBuf::from(storage.display_path(parent_dir));
        let actors_root = PathBuf::from(storage.display_path(actors_dir));
        let folder = self.folders.entry(parent).or_default();
        for entry in entries
            .iter()
            .filter(|e| e.kind == EntryKind::File && !crate::classify::is_ignored_name(&e.name))
        {
            if !matches!(
                extension_of(&entry.name).as_deref(),
                Some("jpg" | "jpeg" | "png" | "webp")
            ) {
                continue;
            }
            let Some(bytes) = entry.size else { continue };
            if bytes > MAX_POSTER {
                continue;
            }
            let modified = entry
                .modified
                .map(palveni_core::clock::to_millis)
                .unwrap_or(0);
            folder.actors.insert(
                entry.name.to_lowercase(),
                Sidecar {
                    path: actors_root.join(&entry.name),
                    bytes,
                    modified,
                },
            );
        }
    }

    fn bdmv_work(
        &self,
        folder_path: &Path,
        root: &Path,
    ) -> Option<(PathBuf, PathBuf, Option<String>)> {
        let is_stream = folder_path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.eq_ignore_ascii_case("stream"));
        if !is_stream {
            return None;
        }
        let bdmv_dir = folder_path.parent()?;
        let is_bdmv = bdmv_dir
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.eq_ignore_ascii_case("bdmv"));
        if !is_bdmv {
            return None;
        }
        let disc_root = bdmv_dir.parent()?;
        if disc_root == root || !disc_root.starts_with(root) {
            return None;
        }
        if let Some(disc_name) = disc_root.file_name().and_then(|n| n.to_str())
            && recognition::disc_folder(disc_name)
            && let Some(parent) = disc_root.parent()
            && parent != root
            && parent.starts_with(root)
            && self.folders.get(parent).is_none_or(|f| f.films == 0)
            && parent
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| !recognition::is_generic_category_dir(n))
        {
            return Some((
                parent.to_owned(),
                disc_root.to_owned(),
                Some(disc_name.to_owned()),
            ));
        }
        Some((disc_root.to_owned(), disc_root.to_owned(), None))
    }

    fn candidate(&self, path: &Path, root: &Path) -> Option<Candidate> {
        let folder_path = path.parent()?;
        let folder = self.folders.get(folder_path)?;
        let file_name = path.file_name()?.to_str()?;
        let stem = path.file_stem()?.to_str()?;
        let bdmv = self.bdmv_work(folder_path, root);
        let bdmv_work_dir = bdmv.as_ref().map(|(w, _, _)| w.as_path());
        let bdmv_disc_root = bdmv.as_ref().map(|(_, d, _)| d.as_path());
        let bdmv_work_folder = bdmv_work_dir.and_then(|d| self.folders.get(d));
        let bdmv_disc_folder = bdmv_disc_root
            .filter(|d| Some(*d) != bdmv_work_dir)
            .and_then(|d| self.folders.get(d));
        let named_nfo = folder.file(&format!("{stem}.nfo"));
        let bdmv_nfo = bdmv_work_dir.and_then(|dir| {
            let dir_stem = dir.file_name().and_then(|n| n.to_str());
            bdmv_work_folder
                .and_then(|f| {
                    dir_stem
                        .and_then(|name| f.file(&format!("{name}.nfo")))
                        .or_else(|| f.file("movie.nfo"))
                })
                .or_else(|| bdmv_disc_folder.and_then(|f| f.file("movie.nfo")))
        });
        let file_nfo = named_nfo
            .or(bdmv_nfo)
            .or_else(|| {
                (folder.films == 1)
                    .then(|| folder.file("movie.nfo"))
                    .flatten()
            })
            .filter(|s| s.bytes <= MAX_NFO);
        let season_folder = if bdmv.is_some() {
            None
        } else {
            folder_path
                .file_name()
                .and_then(|n| n.to_str())
                .and_then(recognition::season_folder)
        };
        let show_dir = if let Some(dir) = bdmv_work_dir {
            dir
        } else if season_folder.is_some() && folder_path != root {
            folder_path.parent()?
        } else {
            folder_path
        };
        let show_folder = self.folders.get(show_dir);
        let show_nfo = if bdmv.is_some() {
            None
        } else {
            show_folder
                .and_then(|d| d.file("tvshow.nfo"))
                .filter(|s| s.bytes <= MAX_NFO)
        };
        let mark = if bdmv.is_some() {
            None
        } else {
            recognition::episode_marker(stem)
        };
        let movie_poster = folder
            .poster(&[&format!("{stem}-poster"), &format!("{stem}-cover"), stem])
            .or_else(|| {
                bdmv_work_folder
                    .and_then(|f| f.poster(&["poster", "movie-poster", "folder", "cover"]))
                    .or_else(|| {
                        bdmv_disc_folder
                            .and_then(|f| f.poster(&["poster", "movie-poster", "folder", "cover"]))
                    })
            })
            .or_else(|| {
                (folder.films == 1)
                    .then(|| folder.poster(&["poster", "movie-poster", "folder", "cover"]))
                    .flatten()
            });
        let movie_backdrop = folder
            .poster(&[
                &format!("{stem}-fanart"),
                &format!("{stem}-backdrop"),
                &format!("{stem}-landscape"),
            ])
            .or_else(|| {
                let stems = [
                    "fanart",
                    "backdrop",
                    "landscape",
                    "movie-fanart",
                    "movie-backdrop",
                ];
                bdmv_work_folder
                    .and_then(|f| f.poster(&stems))
                    .or_else(|| bdmv_disc_folder.and_then(|f| f.poster(&stems)))
            })
            .or_else(|| {
                (folder.films == 1)
                    .then(|| {
                        folder.poster(&[
                            "fanart",
                            "backdrop",
                            "landscape",
                            "movie-fanart",
                            "movie-backdrop",
                        ])
                    })
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
        let owns_show_art = show_nfo.is_some() || season_folder.is_some() || folder_names_series;
        let show_poster = show_folder
            .filter(|_| owns_show_art)
            .and_then(|d| d.poster(&["poster", "tvshow-poster", "folder", "cover"]));
        let show_backdrop = show_folder.filter(|_| owns_show_art).and_then(|d| {
            d.poster(&[
                "fanart",
                "backdrop",
                "landscape",
                "tvshow-fanart",
                "tvshow-backdrop",
            ])
        });
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
        let episode_thumb = folder.poster(&[&format!("{stem}-thumb"), stem]);
        let movie_thumb = folder.poster(&[&format!("{stem}-thumb")]);
        let episode_poster =
            folder.poster(&[&format!("{stem}-thumb"), &format!("{stem}-poster"), stem]);

        let mut actors: HashMap<String, PathBuf> = HashMap::new();
        let mut actor_sig: Vec<(&str, &Sidecar)> = Vec::new();
        if let Some(show_folder) = show_folder
            && show_dir != folder_path
        {
            for (k, v) in &show_folder.actors {
                actors.insert(k.clone(), v.path.clone());
                actor_sig.push((k.as_str(), v));
            }
        }
        if let Some(disc_folder) = bdmv_disc_folder {
            for (k, v) in &disc_folder.actors {
                actors.insert(k.clone(), v.path.clone());
                actor_sig.push((k.as_str(), v));
            }
        }
        for (k, v) in &folder.actors {
            actors.insert(k.clone(), v.path.clone());
            actor_sig.push((k.as_str(), v));
        }
        actor_sig.sort_unstable_by(|a, b| a.0.cmp(b.0).then_with(|| a.1.path.cmp(&b.1.path)));

        let owns_folder = folder.films == 1 && folder_path != root;
        let (v_bytes, v_mod) = folder
            .videos
            .get(&file_name.to_ascii_lowercase())
            .copied()
            .unwrap_or((0, 0));
        let mut signature = format!("video-sidecars-2/{owns_folder}/{v_bytes}:{v_mod}");
        if let Some((ref work_dir, _, ref disc_label)) = bdmv {
            let _ = write!(
                signature,
                "/v3-bdmv:{}:{}",
                work_dir.display(),
                disc_label.as_deref().unwrap_or("")
            );
        }
        if season_folder.is_some_and(|s| s >= 1000) || mark.is_some_and(|m| m.season >= 1000) {
            signature.push_str("/v3-s4");
        }
        for file in [
            file_nfo,
            show_nfo,
            movie_poster,
            show_poster,
            season_poster,
            episode_poster,
            movie_backdrop,
            show_backdrop,
            episode_thumb,
            movie_thumb,
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
        for (k, file) in actor_sig {
            if let Some(p) = file.path.to_str() {
                let _ = write!(
                    signature,
                    "|a:{k}={}:{p}/{}:{}",
                    p.len(),
                    file.bytes,
                    file.modified
                );
            }
        }
        let show_dir_owned = show_dir.to_owned();
        let (bdmv_work_dir, disc_label) = match bdmv {
            Some((work_dir, _, label)) => (Some(work_dir), label),
            None => (None, None),
        };
        Some(Candidate {
            path: path.to_owned(),
            folder_path: folder_path.to_owned(),
            show_dir: show_dir_owned,
            bdmv_work_dir,
            disc_label,
            file_nfo: file_nfo.map(|s| s.path.clone()),
            show_nfo: show_nfo.map(|s| s.path.clone()),
            movie_poster: movie_poster.map(|s| s.path.clone()),
            show_poster: show_poster.map(|s| s.path.clone()),
            season_poster: season_poster.map(|s| s.path.clone()),
            episode_poster: episode_poster.map(|s| s.path.clone()),
            movie_backdrop: movie_backdrop.map(|s| s.path.clone()),
            show_backdrop: show_backdrop.map(|s| s.path.clone()),
            episode_thumb: episode_thumb.map(|s| s.path.clone()),
            movie_thumb: movie_thumb.map(|s| s.path.clone()),
            actors,
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
        ffprobe: Option<&Path>,
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
            let roots_for_read = roots.clone();
            let (mut read_rows, batch_read_time) = tokio::task::spawn_blocking(move || {
                let started = Instant::now();
                let mut shared = HashMap::new();
                let rows = batch
                    .into_iter()
                    .map(|(id, index, candidate)| {
                        let path = candidate.path.clone();
                        let (meta, fallback_runtime) =
                            candidate.read(id, &roots_for_read[index], &mut shared);
                        (meta, index, path, fallback_runtime)
                    })
                    .collect::<Vec<_>>();
                (rows, started.elapsed())
            })
            .await
            .map_err(|e| ScanError::Metadata(e.to_string()))?;

            // Cheap duration fallback when NFO provides no runtime.
            for (meta, root_idx, path, fallback_runtime) in &mut read_rows {
                if meta.runtime_ms.is_none() {
                    let probed = if let Some(ffprobe) = ffprobe {
                        let root = roots[*root_idx].clone();
                        let path_clone = path.clone();
                        let opened = tokio::task::spawn_blocking(move || {
                            let sp = storage_path(&root, &path_clone)?;
                            root.open_file_blocking(&sp).ok()
                        })
                        .await
                        .ok()
                        .flatten();
                        match opened {
                            Some(file) => images::probe_duration_ms(ffprobe, file).await,
                            None => None,
                        }
                    } else {
                        None
                    };
                    meta.runtime_ms = probed.or(*fallback_runtime);
                }
            }

            let rows: Vec<VideoMetadata> = read_rows.into_iter().map(|(m, _, _, _)| m).collect();
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
    folder_path: PathBuf,
    show_dir: PathBuf,
    bdmv_work_dir: Option<PathBuf>,
    disc_label: Option<String>,
    file_nfo: Option<PathBuf>,
    show_nfo: Option<PathBuf>,
    movie_poster: Option<PathBuf>,
    show_poster: Option<PathBuf>,
    season_poster: Option<PathBuf>,
    episode_poster: Option<PathBuf>,
    movie_backdrop: Option<PathBuf>,
    show_backdrop: Option<PathBuf>,
    episode_thumb: Option<PathBuf>,
    movie_thumb: Option<PathBuf>,
    actors: HashMap<String, PathBuf>,
    season_folder: Option<i32>,
    owns_folder: bool,
    stamp: String,
}

fn path_string(path: Option<&PathBuf>) -> Option<String> {
    path.and_then(|p| p.to_str()).map(str::to_owned)
}

fn rating_tenths(rating: Option<f64>) -> Option<i32> {
    rating.and_then(|r| {
        if !r.is_finite() || !(0.0..=10.0).contains(&r) {
            return None;
        }
        #[allow(clippy::cast_possible_truncation)]
        let tenths = (r * 10.0).round() as i32;
        Some(tenths.clamp(0, 100))
    })
}

fn validate_local_image(root: &LocalStorage, candidate: &Path) -> Option<String> {
    let ext = candidate
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)?;
    if !matches!(ext.as_str(), "jpg" | "jpeg" | "png" | "webp") {
        return None;
    }
    let sp = storage_path(root, candidate)?;
    let mut file = root.open_file_blocking(&sp).ok()?;
    let size = file.metadata().ok()?.len();
    if !(3..=MAX_POSTER).contains(&size) {
        return None;
    }
    let mut header = [0u8; 12];
    let n = usize::try_from(size.min(12)).ok()?;
    file.read_exact(&mut header[..n]).ok()?;
    crate::artwork::mime(&header[..n])?;
    candidate.to_str().map(str::to_owned)
}

fn resolve_person_image(
    root: &LocalStorage,
    base_dir: &Path,
    person: &nfo::Person,
    actors: &HashMap<String, PathBuf>,
) -> Option<String> {
    if let Some(raw_thumb) = person.thumb.as_deref().map(str::trim)
        && !raw_thumb.is_empty()
        && !raw_thumb.contains("://")
        && !raw_thumb.starts_with("//")
    {
        let normalized = raw_thumb.replace('\\', "/");
        let candidate = if Path::new(&normalized).is_absolute() {
            PathBuf::from(&normalized)
        } else {
            base_dir.join(&normalized)
        };
        if let Some(valid) = validate_local_image(root, &candidate) {
            return Some(valid);
        }
    }
    let trimmed = person.name.trim();
    if trimmed.is_empty() {
        return None;
    }
    let underscored = trimmed.replace(' ', "_");
    for stem in [underscored.as_str(), trimmed] {
        for ext in ["jpg", "jpeg", "png", "webp"] {
            let key = format!("{stem}.{ext}").to_lowercase();
            if let Some(path) = actors.get(&key)
                && let Some(valid) = validate_local_image(root, path)
            {
                return Some(valid);
            }
        }
    }
    None
}

fn map_people(
    root: &LocalStorage,
    base_dir: &Path,
    people: &[nfo::Person],
    actors: &HashMap<String, PathBuf>,
) -> Vec<VideoPersonInput> {
    people
        .iter()
        .map(|p| VideoPersonInput {
            kind: p.kind,
            name: p.name.clone(),
            role: p.role.clone(),
            image: resolve_person_image(root, base_dir, p, actors),
        })
        .collect()
}

impl Candidate {
    fn read(
        self,
        id: palveni_core::ItemId,
        root: &LocalStorage,
        cache: &mut HashMap<PathBuf, Option<nfo::Nfo>>,
    ) -> (VideoMetadata, Option<i64>) {
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
        let is_bdmv = self.bdmv_work_dir.is_some();
        let marker = (!is_bdmv)
            .then(|| recognition::episode_marker(stem))
            .flatten();
        let episode_only = (!is_bdmv)
            .then(|| recognition::episode_only(stem))
            .flatten();
        let series = !is_bdmv
            && (file.root == "episodedetails"
                || (file.root != "movie"
                    && (marker.is_some()
                        || episode_only.is_some()
                        || self.season_folder.is_some()
                        || show.root == "tvshow")));
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

        let (
            work_group,
            work_title,
            year,
            work_overview,
            work_poster,
            work_original_title,
            work_tagline,
            work_rating,
            work_official_rating,
            work_premiered,
            work_studios,
            work_backdrop,
            genres,
            people,
            thumb,
            fallback_runtime,
        ) = if series {
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
                show.title.clone()
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
            let genres = if !show.genres.is_empty() {
                show.genres.clone()
            } else {
                file.genres.clone()
            };
            let people_src = if !show.people.is_empty() {
                &show.people
            } else {
                &file.people
            };
            let people = map_people(root, &self.show_dir, people_src, &self.actors);
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
                show.original_title,
                show.tagline,
                rating_tenths(show.rating),
                show.official_rating,
                show.premiered,
                show.studios,
                path_string(self.show_backdrop.as_ref()),
                genres,
                people,
                path_string(self.episode_thumb.as_ref()),
                show.runtime_ms,
            )
        } else if let Some(ref bdmv_dir) = self.bdmv_work_dir {
            let people = map_people(root, bdmv_dir, &file.people, &self.actors);
            (
                format!("movie\n{}", bdmv_dir.display()),
                metadata_title.clone().unwrap_or(folder_title.0),
                file.year.or(folder_title.1),
                file.plot.clone(),
                path_string(self.movie_poster.as_ref()),
                file.original_title,
                file.tagline,
                rating_tenths(file.rating),
                file.official_rating,
                file.premiered.clone(),
                file.studios,
                path_string(self.movie_backdrop.as_ref()),
                file.genres,
                people,
                path_string(self.movie_thumb.as_ref()),
                None,
            )
        } else {
            let fallback = if self.owns_folder && folder_title.1.is_some() {
                folder_title.clone()
            } else {
                film_title.clone()
            };
            let people = map_people(root, &self.folder_path, &file.people, &self.actors);
            (
                format!("movie\n{}", self.path.display()),
                metadata_title.clone().unwrap_or(fallback.0),
                file.year.or(fallback.1),
                file.plot.clone(),
                path_string(self.movie_poster.as_ref()),
                file.original_title,
                file.tagline,
                rating_tenths(file.rating),
                file.official_rating,
                file.premiered.clone(),
                file.studios,
                path_string(self.movie_backdrop.as_ref()),
                file.genres,
                people,
                path_string(self.movie_thumb.as_ref()),
                None,
            )
        };
        let item_title = if is_bdmv {
            match self.disc_label {
                Some(ref label) => format!("{label} · {stem}"),
                None => stem.to_owned(),
            }
        } else {
            metadata_title.unwrap_or(film_title.0)
        };
        let season_poster =
            path_string(self.season_poster.as_ref()).or_else(|| work_poster.clone());
        let episode_poster =
            path_string(self.episode_poster.as_ref()).or_else(|| season_poster.clone());
        let meta = VideoMetadata {
            item_id: id,
            stamp: self.stamp,
            work_group,
            work_title,
            year,
            work_overview,
            work_poster,
            work_original_title,
            work_tagline,
            work_rating,
            work_official_rating,
            work_premiered,
            work_studios,
            work_backdrop,
            season,
            season_poster,
            episode,
            title: item_title,
            overview: file.plot,
            episode_poster,
            runtime_ms: file.runtime_ms,
            rating: rating_tenths(file.rating),
            premiered: file.premiered,
            thumb,
            genres,
            people,
        };
        (meta, fallback_runtime)
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
        let bytes = &mut header[..usize::try_from(size.min(12)).unwrap_or(12)];
        file.read_exact(bytes)?;
        let media = crate::artwork::mime(bytes).ok_or_else(|| Error::not_found("poster"))?;
        file.rewind()?;
        let reader = tokio::io::AsyncReadExt::take(tokio::fs::File::from_std(file), size);
        Ok((media, size, Box::pin(reader) as ByteStream))
    })
    .await
    .map_err(|e| Error::internal(e.to_string()))?
}

//! File-level library scanner.
//!
//! 1. Walk every library root through its storage provider (blocking I/O
//!    happens on the blocking pool, see [`LocalStorage`]).
//! 2. Keep files whose extension matches the library kind; synthesise
//!    folder items for the directories that contain them.
//! 3. Diff against the stored index by path, size and mtime.
//! 4. Write changes in short batched transactions; tombstone vanished items.
//!
//! Safety rules for NAS setups: a root that cannot be opened (unmounted
//! volume) or a directory that cannot be read keeps its existing items —
//! only files under successfully listed directories can be tombstoned.

mod cue;
mod reviews;

use palveni_core::music::TrackTags;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};

use palveni_core::events::{EventBus, ServerEvent};
use palveni_core::storage::{DirEntry, EntryKind, StoragePath, StorageProvider};
use palveni_core::{ItemId, ItemKind, JobStep, LibraryId, LibraryKind, clock};
use palveni_db::items::{IndexedItem, ItemUpsert};
use palveni_db::{Db, DbError};
use serde::Serialize;

use crate::classify::{classify, is_ignored_name, search_text, sort_key, title_for_file};
use crate::local::LocalStorage;

pub use palveni_core::model::{SCAN_JOB_KIND, ScanJobPayload};

/// Rows per write transaction. Small enough to keep write locks short on
/// SQLite and well under every backend's bind-parameter limit.
const BATCH_SIZE: usize = 500;
/// Progress is reported every this many files during discovery.
const DISCOVERY_REPORT_EVERY: usize = 1_000;

/// Receives progress from a running scan.
pub trait ScanObserver: Send + Sync {
    /// `fraction` is 0..=1, or `None` while the total is unknown.
    fn progress(&self, fraction: Option<f64>, step: &ScanStep);

    fn is_cancelled(&self) -> bool {
        false
    }
}

/// Scan steps with their contract codes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScanStep {
    /// `scan.discovering` (`found`).
    Discovering { found: u64 },
    /// `scan.indexing` (`done`, `total`).
    Indexing { done: u64, total: u64 },
    /// `scan.summary` (`added`, `updated`, `removed`, `unchanged`,
    /// `unavailablePaths`, `unreadableFolders`).
    Summary {
        added: u64,
        updated: u64,
        removed: u64,
        unchanged: u64,
        /// Library paths that could not be read at all; their items were kept.
        unavailable_paths: u64,
        /// Folders skipped because they could not be listed; their items were kept.
        unreadable_folders: u64,
    },
}

impl ScanStep {
    pub const DISCOVERING: &'static str = "scan.discovering";
    pub const INDEXING: &'static str = "scan.indexing";
    pub const SUMMARY: &'static str = "scan.summary";

    /// Machine-readable form for `Job.step`.
    pub fn to_job_step(&self) -> JobStep {
        match *self {
            ScanStep::Discovering { found } => JobStep::new(Self::DISCOVERING).with("found", found),
            ScanStep::Indexing { done, total } => JobStep::new(Self::INDEXING)
                .with("done", done)
                .with("total", total),
            ScanStep::Summary {
                added,
                updated,
                removed,
                unchanged,
                unavailable_paths,
                unreadable_folders,
            } => JobStep::new(Self::SUMMARY)
                .with("added", added)
                .with("updated", updated)
                .with("removed", removed)
                .with("unchanged", unchanged)
                .with("unavailablePaths", unavailable_paths)
                .with("unreadableFolders", unreadable_folders),
        }
    }

    /// English rendering for `Job.message`.
    pub fn message(&self) -> String {
        match *self {
            ScanStep::Discovering { found } => format!("Discovering files: {found} found"),
            ScanStep::Indexing { done, total } => format!("Updating index: {done} of {total}"),
            ScanStep::Summary {
                added,
                updated,
                removed,
                unchanged,
                unavailable_paths,
                unreadable_folders,
            } => {
                let mut text = format!(
                    "{added} added, {updated} updated, {removed} removed, {unchanged} unchanged"
                );
                // A clean scan keeps the short form; anything that was skipped
                // must be visible, because skipped items were deliberately kept.
                if unavailable_paths > 0 {
                    text.push_str(&format!("; {unavailable_paths} library paths unavailable"));
                }
                if unreadable_folders > 0 {
                    text.push_str(&format!("; {unreadable_folders} folders unreadable"));
                }
                text
            }
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanSummary {
    pub files_seen: u64,
    pub added: u64,
    pub updated: u64,
    pub removed: u64,
    pub unchanged: u64,
    pub skipped_roots: u64,
    pub unreadable_dirs: u64,
}

impl ScanSummary {
    pub fn step(&self) -> ScanStep {
        ScanStep::Summary {
            added: self.added,
            updated: self.updated,
            removed: self.removed,
            unchanged: self.unchanged,
            unavailable_paths: self.skipped_roots,
            unreadable_folders: self.unreadable_dirs,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ScanError {
    #[error("library not found")]
    LibraryNotFound,
    #[error("reading music metadata: {0}")]
    Metadata(String),
    #[error("mount information is unavailable or changed during the scan")]
    MountChanged,
    #[error("the missing items changed before the review could be applied")]
    ReviewChanged,
    #[error("scan cancelled")]
    Cancelled,
    #[error("none of the library's paths could be read")]
    NoReadableRoot,
    #[error(transparent)]
    Db(DbError),
}

impl From<DbError> for ScanError {
    fn from(err: DbError) -> Self {
        match err {
            DbError::NotFound("library") => ScanError::LibraryNotFound,
            other => ScanError::Db(other),
        }
    }
}

#[derive(Clone, Debug)]
struct FoundFolder {
    path: String,
    parent: Option<String>,
    name: String,
    depth: usize,
}

#[derive(Clone, Debug)]
struct FoundFile {
    music: Option<TrackTags>,
    path: String,
    parent: Option<String>,
    name: String,
    kind: ItemKind,
    size: Option<i64>,
    modified_ms: Option<i64>,
}

#[derive(Default)]
struct WalkOutcome {
    video: crate::video::Inventory,
    /// Keyed by absolute path; BTreeMap keeps parents before children when
    /// sorted by depth later.
    folders: BTreeMap<String, FoundFolder>,
    files: Vec<FoundFile>,
    /// Absolute paths under which nothing may be tombstoned.
    protected: Vec<String>,
    skipped_roots: u64,
    unreadable_dirs: u64,
}

impl WalkOutcome {
    fn add_file(
        &mut self,
        storage: &LocalStorage,
        rel: &StoragePath,
        kind: ItemKind,
        entry: &DirEntry,
        backing: bool,
    ) {
        let parent_display = |p: Option<StoragePath>| -> Option<String> {
            p.filter(|p| !p.is_root()).map(|p| storage.display_path(&p))
        };
        // Register missing ancestor folders; stop at the first known one
        // because its own ancestors are then known too.
        let mut current = rel.parent();
        while let Some(dir) = current {
            if dir.is_root() {
                break;
            }
            let display = storage.display_path(&dir);
            if self.folders.contains_key(&display) {
                break;
            }
            let next = dir.parent();
            self.folders.insert(
                display.clone(),
                FoundFolder {
                    path: display,
                    parent: parent_display(next.clone()),
                    name: dir.file_name().unwrap_or_default().to_owned(),
                    depth: dir.depth(),
                },
            );
            current = next;
        }
        self.files.push(FoundFile {
            music: backing.then(|| TrackTags {
                cue_backing: true,
                ..Default::default()
            }),
            path: storage.display_path(rel),
            parent: parent_display(rel.parent()),
            name: entry.name.clone(),
            kind,
            size: entry.size.and_then(|s| i64::try_from(s).ok()),
            modified_ms: entry.modified.map(clock::to_millis),
        });
    }

    fn is_protected(&self, path: &str) -> bool {
        let candidate = Path::new(path);
        self.protected
            .iter()
            .any(|dir| candidate.starts_with(Path::new(dir)))
    }
}

#[derive(Clone)]
pub struct Scanner {
    db: Db,
    bus: EventBus,
    ffprobe: Option<PathBuf>,
}

impl Scanner {
    pub fn new(db: Db, bus: EventBus) -> Self {
        Scanner {
            db,
            bus,
            ffprobe: None,
        }
    }

    pub fn with_ffprobe(mut self, ffprobe: Option<PathBuf>) -> Self {
        self.ffprobe = ffprobe;
        self
    }

    pub async fn scan(
        &self,
        library_id: LibraryId,
        observer: &dyn ScanObserver,
    ) -> Result<ScanSummary, ScanError> {
        self.scan_reviewed(library_id, observer, None).await
    }

    pub async fn scan_reviewed(
        &self,
        library_id: LibraryId,
        observer: &dyn ScanObserver,
        revision: Option<&str>,
    ) -> Result<ScanSummary, ScanError> {
        self.scan_with_mounts(library_id, observer, revision, crate::mounts::snapshot)
            .await
    }

    async fn scan_with_mounts<F>(
        &self,
        library_id: LibraryId,
        observer: &dyn ScanObserver,
        revision: Option<&str>,
        read_mounts: impl Fn() -> F,
    ) -> Result<ScanSummary, ScanError>
    where
        F: std::future::Future<Output = std::io::Result<crate::mounts::MountTable>>,
    {
        let library = self
            .db
            .libraries()
            .get(library_id)
            .await?
            .ok_or(ScanError::LibraryNotFound)?;
        observer.progress(None, &ScanStep::Discovering { found: 0 });

        let table = read_mounts().await.map_err(|_| ScanError::MountChanged)?;
        let sources = table.sources(&library.paths);
        let (mut scopes, protected) =
            reviews::ReviewScopes::load(&self.db, library_id, &library.paths, &table).await?;
        let mut walk = WalkOutcome {
            protected,
            ..WalkOutcome::default()
        };
        let mounts_changed = |current: &crate::mounts::MountTable,
                              scopes: &reviews::ReviewScopes| {
            current.sources(&library.paths) != sources || !scopes.unchanged(current)
        };
        walk.unreadable_dirs = count(
            walk.protected
                .iter()
                .filter(|p| !library.paths.contains(p))
                .count(),
        );
        let mut walked_any = false;
        let mut roots = Vec::new();
        for root in &library.paths {
            if observer.is_cancelled() {
                return Err(ScanError::Cancelled);
            }
            if walk.is_protected(root) {
                walk.skipped_roots += 1;
                continue;
            }
            match LocalStorage::open_registered(Path::new(root)).await {
                Ok(storage) => {
                    if walk_root(&storage, library.kind, observer, &mut walk).await? {
                        roots.push(storage);
                        walked_any = true;
                    }
                }
                Err(err) => {
                    tracing::warn!(%library_id, root = %root, error = %err, "library path unavailable; keeping its items");
                    walk.protected.push(root.clone());
                    walk.skipped_roots += 1;
                }
            }
        }
        if !walked_any {
            return Err(ScanError::NoReadableRoot);
        }

        observer.progress(
            None,
            &ScanStep::Discovering {
                found: count(walk.files.len()),
            },
        );
        if mounts_changed(
            &read_mounts().await.map_err(|_| ScanError::MountChanged)?,
            &scopes,
        ) {
            return Err(ScanError::MountChanged);
        }
        let existing = self.db.items().scan_index(library_id).await?;
        let confirmed = scopes
            .inspect(&self.db, library_id, revision, &existing, &walk)
            .await?;
        let video = std::mem::take(&mut walk.video);
        scopes.restrict(&mut walk);
        let mut plan = plan_changes(library_id, walk, &existing, &scopes);
        if let Some((scope, revision)) = confirmed {
            if observer.is_cancelled() {
                return Err(ScanError::Cancelled);
            }
            if mounts_changed(
                &read_mounts().await.map_err(|_| ScanError::MountChanged)?,
                &scopes,
            ) {
                return Err(ScanError::MountChanged);
            }
            let targets: HashSet<_> = existing
                .iter()
                .filter(|item| scopes.owner(&item.path) == Some(scope.as_str()))
                .map(|item| item.id)
                .collect();
            let (approved, ordinary): (Vec<_>, _) = plan
                .tombstones
                .into_iter()
                .partition(|id| targets.contains(id));
            if !self
                .db
                .scan_reviews()
                .apply_confirmed_on(library_id, &revision, scopes.source(&scope), &approved)
                .await?
            {
                return Err(ScanError::ReviewChanged);
            }
            plan.tombstones = ordinary;
        }

        let total = plan.upserts.len() + plan.tombstones.len();
        let mut done = 0usize;
        report_writes(observer, done, total);
        let audio = library.kind.leaf_item_kind().is_audio();
        let batch_size = if audio { 64 } else { BATCH_SIZE };
        let roots = std::sync::Arc::new(roots);
        for chunk in plan.upserts.chunks(batch_size) {
            if observer.is_cancelled() {
                return Err(ScanError::Cancelled);
            }
            let tagged;
            let rows = if audio {
                let roots = roots.clone();
                let batch = chunk.to_vec();
                tagged =
                    tokio::task::spawn_blocking(move || crate::tags::read_batch(&roots, batch))
                        .await
                        .map_err(|e| ScanError::Metadata(e.to_string()))??;
                tagged.as_slice()
            } else {
                chunk
            };
            self.db
                .items()
                .apply_scan_batch(library_id, rows, &[])
                .await?;
            done += chunk.len();
            report_writes(observer, done, total);
        }
        for chunk in plan.tombstones.chunks(BATCH_SIZE) {
            if observer.is_cancelled() {
                return Err(ScanError::Cancelled);
            }
            self.db
                .items()
                .apply_scan_batch(library_id, &[], chunk)
                .await?;
            done += chunk.len();
            report_writes(observer, done, total);
        }

        if library.kind == LibraryKind::Music {
            self.db.music().refresh_albums(library_id).await?;
        }
        if library.kind == LibraryKind::Video {
            video
                .index(
                    &self.db,
                    library_id,
                    roots,
                    self.ffprobe.as_deref(),
                    observer,
                    |path| !scopes.held(path),
                )
                .await?;
        }
        if !self
            .db
            .libraries()
            .mark_scanned(library_id, clock::now())
            .await?
        {
            return Err(ScanError::LibraryNotFound);
        }
        if let Some(updated) = self.db.libraries().get(library_id).await? {
            self.bus.publish(ServerEvent::LibraryUpdated(updated));
        }
        if plan.summary.skipped_roots > 0 || plan.summary.unreadable_dirs > 0 {
            tracing::warn!(
                %library_id,
                unavailable_paths = plan.summary.skipped_roots,
                unreadable_folders = plan.summary.unreadable_dirs,
                "scan finished with parts of the library unreadable"
            );
        }
        observer.progress(Some(1.0), &plan.summary.step());
        Ok(plan.summary)
    }
}

fn count(n: usize) -> u64 {
    u64::try_from(n).unwrap_or(u64::MAX)
}

fn report_writes(observer: &dyn ScanObserver, done: usize, total: usize) {
    let step = ScanStep::Indexing {
        done: count(done),
        total: count(total),
    };
    if total == 0 {
        observer.progress(Some(1.0), &step);
        return;
    }
    // Precision loss is irrelevant for a progress bar.
    #[allow(clippy::cast_precision_loss)]
    let fraction = done as f64 / total as f64;
    observer.progress(Some(fraction), &step);
}

async fn walk_root(
    storage: &LocalStorage,
    kind: LibraryKind,
    observer: &dyn ScanObserver,
    out: &mut WalkOutcome,
) -> Result<bool, ScanError> {
    let mut stack = vec![StoragePath::root()];
    let mut next_report = DISCOVERY_REPORT_EVERY;
    while let Some(dir) = stack.pop() {
        if observer.is_cancelled() {
            return Err(ScanError::Cancelled);
        }
        if out.is_protected(&storage.display_path(&dir)) {
            continue;
        }
        let entries = match storage.list_dir(&dir).await {
            Ok(entries) => entries,
            Err(err) => {
                let dir_path = storage.display_path(&dir);
                tracing::warn!(dir = %dir_path, error = %err, "cannot read folder; keeping its items");
                out.protected.push(dir_path);
                if dir.is_root() {
                    out.skipped_roots += 1;
                    return Ok(false);
                }
                out.unreadable_dirs += 1;
                continue;
            }
        };
        if kind == LibraryKind::Video {
            out.video.observe(storage, &dir, &entries);
            for entry in entries.iter().filter(|e| {
                e.kind == EntryKind::Directory && e.name.eq_ignore_ascii_case(".actors")
            }) {
                if let Ok(actors_dir) = dir.join(&entry.name)
                    && let Ok(actor_entries) = storage.list_dir(&actors_dir).await
                {
                    out.video
                        .observe_actors(storage, &dir, &actors_dir, &actor_entries);
                }
            }
        }
        let mut cue_sources = HashSet::new();
        if kind == LibraryKind::Music {
            let (sources, files) = cue::collect(storage, &dir, &entries).await?;
            cue_sources = sources;
            out.files.extend(files);
        }
        for entry in entries {
            if is_ignored_name(&entry.name) {
                continue;
            }
            let Ok(child) = dir.join(&entry.name) else {
                continue;
            };
            match entry.kind {
                EntryKind::Directory => stack.push(child),
                EntryKind::File => {
                    if let Some(item_kind) = classify(kind, &entry.name) {
                        out.add_file(
                            storage,
                            &child,
                            item_kind,
                            &entry,
                            cue_sources.contains(&entry.name),
                        );
                    }
                }
                EntryKind::Other => {}
            }
        }
        if out.files.len() >= next_report {
            observer.progress(
                None,
                &ScanStep::Discovering {
                    found: count(out.files.len()),
                },
            );
            next_report = out.files.len() + DISCOVERY_REPORT_EVERY;
        }
    }
    Ok(true)
}

struct ChangePlan {
    /// Folders (parents first) followed by files.
    upserts: Vec<ItemUpsert>,
    tombstones: Vec<ItemId>,
    summary: ScanSummary,
}

fn plan_changes(
    library_id: LibraryId,
    walk: WalkOutcome,
    existing: &[IndexedItem],
    scopes: &reviews::ReviewScopes,
) -> ChangePlan {
    let by_path: HashMap<&str, &IndexedItem> =
        existing.iter().map(|e| (e.path.as_str(), e)).collect();
    let mut summary = ScanSummary {
        files_seen: count(
            walk.files
                .iter()
                .filter(|f| f.music.as_ref().is_none_or(|m| m.cue.is_none()))
                .count(),
        ),
        skipped_roots: walk.skipped_roots,
        unreadable_dirs: walk.unreadable_dirs,
        ..ScanSummary::default()
    };

    let mut folders: Vec<&FoundFolder> = walk.folders.values().collect();
    folders.sort_by(|a, b| a.depth.cmp(&b.depth).then_with(|| a.path.cmp(&b.path)));

    let mut folder_ids: HashMap<&str, ItemId> = HashMap::with_capacity(folders.len());
    for folder in &folders {
        let id = by_path
            .get(folder.path.as_str())
            .map_or_else(ItemId::generate, |e| e.id);
        folder_ids.insert(folder.path.as_str(), id);
    }
    let parent_id_of = |parent: &Option<String>| -> Option<ItemId> {
        parent.as_deref().and_then(|p| folder_ids.get(p).copied())
    };

    let mut seen: HashSet<&str> = HashSet::with_capacity(folders.len() + walk.files.len());
    let mut upserts = Vec::new();

    for folder in &folders {
        seen.insert(folder.path.as_str());
        let Some(id) = folder_ids.get(folder.path.as_str()).copied() else {
            continue;
        };
        let parent_id = parent_id_of(&folder.parent);
        let unchanged = by_path
            .get(folder.path.as_str())
            .is_some_and(|e| !e.deleted && e.kind == ItemKind::Folder && e.parent_id == parent_id);
        if !unchanged {
            upserts.push(ItemUpsert {
                music: None,
                id,
                parent_id,
                kind: ItemKind::Folder,
                title: folder.name.clone(),
                sort_title: sort_key(&folder.name),
                search_text: search_text(&folder.name),
                path: folder.path.clone(),
                size_bytes: None,
                modified_at: None,
            });
        }
    }

    for file in &walk.files {
        if !seen.insert(file.path.as_str()) {
            continue;
        }
        let parent_id = parent_id_of(&file.parent);
        let existing = by_path.get(file.path.as_str()).copied();
        let cue = file.music.as_ref().and_then(|t| t.cue.as_ref());
        let cue_stamp = if file.music.as_ref().is_some_and(|t| t.cue_backing) {
            Some("backing")
        } else {
            cue.map(|c| c.stamp.as_str())
        };
        let id = match existing {
            Some(e)
                if !e.deleted
                    && (!file.kind.is_audio() || e.music_indexed)
                    && e.cue_stamp.as_deref() == cue_stamp
                    && e.kind == file.kind
                    && e.parent_id == parent_id
                    && e.size_bytes == file.size
                    && e.modified_at == file.modified_ms =>
            {
                summary.unchanged += 1;
                continue;
            }
            Some(e) => {
                if e.deleted {
                    summary.added += 1;
                } else {
                    summary.updated += 1;
                }
                e.id
            }
            None => {
                summary.added += 1;
                if cue.is_some() {
                    palveni_db::music::cue_id(library_id, &file.path)
                } else {
                    ItemId::generate()
                }
            }
        };
        let title = if cue.is_some() {
            file.name.clone()
        } else {
            title_for_file(&file.name)
        };
        upserts.push(ItemUpsert {
            music: file.music.clone(),
            id,
            parent_id,
            kind: file.kind,
            sort_title: sort_key(&title),
            search_text: search_text(&title),
            title,
            path: file.path.clone(),
            size_bytes: file.size,
            modified_at: file.modified_ms,
        });
    }

    let mut retained_folders = HashSet::new();
    for item in existing
        .iter()
        .filter(|item| !item.deleted && (walk.is_protected(&item.path) || scopes.held(&item.path)))
    {
        for ancestor in Path::new(&item.path).ancestors().skip(1) {
            if !retained_folders.insert(ancestor) {
                break;
            }
        }
    }
    let tombstones: Vec<ItemId> = existing
        .iter()
        .filter(|e| {
            !e.deleted
                && !seen.contains(e.path.as_str())
                && !walk.is_protected(&e.path)
                && !scopes.held(&e.path)
                && (e.kind != ItemKind::Folder || !retained_folders.contains(Path::new(&e.path)))
        })
        .map(|e| {
            if e.kind != ItemKind::Folder {
                summary.removed += 1;
            }
            e.id
        })
        .collect();

    ChangePlan {
        upserts,
        tombstones,
        summary,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn indexed(
        path: &str,
        kind: ItemKind,
        size: Option<i64>,
        modified: Option<i64>,
    ) -> IndexedItem {
        IndexedItem {
            id: ItemId::generate(),
            path: path.to_owned(),
            parent_id: None,
            kind,
            size_bytes: size,
            modified_at: modified,
            deleted: false,
            music_indexed: true,
            cue_stamp: None,
        }
    }

    fn file(path: &str, size: i64, modified: i64) -> FoundFile {
        FoundFile {
            music: None,
            path: path.to_owned(),
            parent: None,
            name: path.rsplit('/').next().unwrap().to_owned(),
            kind: ItemKind::Track,
            size: Some(size),
            modified_ms: Some(modified),
        }
    }

    #[test]
    fn steps_match_the_contract() {
        let discovering = ScanStep::Discovering { found: 12 };
        assert_eq!(
            serde_json::to_value(discovering.to_job_step()).unwrap(),
            serde_json::json!({ "code": "scan.discovering", "params": { "found": 12 } })
        );
        assert_eq!(discovering.message(), "Discovering files: 12 found");

        let indexing = ScanStep::Indexing { done: 3, total: 9 };
        assert_eq!(
            serde_json::to_value(indexing.to_job_step()).unwrap(),
            serde_json::json!({ "code": "scan.indexing", "params": { "done": 3, "total": 9 } })
        );
        assert_eq!(indexing.message(), "Updating index: 3 of 9");

        let summary = ScanSummary {
            files_seen: 10,
            added: 1,
            updated: 2,
            removed: 3,
            unchanged: 4,
            skipped_roots: 0,
            unreadable_dirs: 0,
        }
        .step();
        assert_eq!(
            serde_json::to_value(summary.to_job_step()).unwrap(),
            serde_json::json!({
                "code": "scan.summary",
                "params": {
                    "added": 1, "updated": 2, "removed": 3, "unchanged": 4,
                    "unavailablePaths": 0, "unreadableFolders": 0
                }
            })
        );
        assert_eq!(
            summary.message(),
            "1 added, 2 updated, 3 removed, 4 unchanged"
        );

        let partial = ScanSummary {
            files_seen: 4,
            unchanged: 4,
            skipped_roots: 1,
            unreadable_dirs: 2,
            ..ScanSummary::default()
        }
        .step();
        assert_eq!(
            serde_json::to_value(partial.to_job_step()).unwrap()["params"]["unavailablePaths"],
            1
        );
        assert_eq!(
            partial.message(),
            "0 added, 0 updated, 0 removed, 4 unchanged; 1 library paths unavailable; 2 folders unreadable"
        );
    }

    #[test]
    fn detects_added_changed_unchanged_and_removed() {
        let mut walk = WalkOutcome::default();
        walk.files.push(file("/m/same.flac", 10, 100));
        walk.files.push(file("/m/changed.flac", 20, 200));
        walk.files.push(file("/m/new.flac", 30, 300));
        let existing = vec![
            indexed("/m/same.flac", ItemKind::Track, Some(10), Some(100)),
            indexed("/m/changed.flac", ItemKind::Track, Some(20), Some(199)),
            indexed("/m/gone.flac", ItemKind::Track, Some(5), Some(50)),
        ];
        let changed_id = existing[1].id;
        let gone_id = existing[2].id;

        let plan = plan_changes(
            LibraryId::generate(),
            walk,
            &existing,
            &reviews::ReviewScopes::default(),
        );
        assert_eq!(plan.summary.unchanged, 1);
        assert_eq!(plan.summary.updated, 1);
        assert_eq!(plan.summary.added, 1);
        assert_eq!(plan.summary.removed, 1);
        assert_eq!(plan.tombstones, vec![gone_id]);
        assert_eq!(plan.upserts.len(), 2);
        assert!(plan.upserts.iter().any(|u| u.id == changed_id));
        let new = plan
            .upserts
            .iter()
            .find(|u| u.path == "/m/new.flac")
            .unwrap();
        assert_eq!(new.title, "new");
    }

    #[test]
    fn protected_paths_are_not_tombstoned() {
        let mut walk = WalkOutcome::default();
        walk.protected.push("/mnt/usb".to_owned());
        let existing = vec![
            indexed("/mnt/usb/a.flac", ItemKind::Track, Some(1), Some(1)),
            indexed("/mnt/usb2/b.flac", ItemKind::Track, Some(1), Some(1)),
        ];
        let keep = existing[0].id;
        let plan = plan_changes(
            LibraryId::generate(),
            walk,
            &existing,
            &reviews::ReviewScopes::default(),
        );
        assert!(!plan.tombstones.contains(&keep));
        // `/mnt/usb2` is a different directory, not a child of `/mnt/usb`.
        assert_eq!(plan.tombstones.len(), 1);
    }

    #[test]
    fn folders_come_before_their_children() {
        let mut walk = WalkOutcome::default();
        for (path, parent, depth) in [("/m/a/b", Some("/m/a"), 2usize), ("/m/a", None, 1usize)] {
            walk.folders.insert(
                path.to_owned(),
                FoundFolder {
                    path: path.to_owned(),
                    parent: parent.map(str::to_owned),
                    name: path.rsplit('/').next().unwrap().to_owned(),
                    depth,
                },
            );
        }
        walk.files.push(FoundFile {
            parent: Some("/m/a/b".to_owned()),
            ..file("/m/a/b/x.flac", 1, 1)
        });
        let plan = plan_changes(
            LibraryId::generate(),
            walk,
            &[],
            &reviews::ReviewScopes::default(),
        );
        let kinds: Vec<ItemKind> = plan.upserts.iter().map(|u| u.kind).collect();
        assert_eq!(
            kinds,
            vec![ItemKind::Folder, ItemKind::Folder, ItemKind::Track]
        );
        assert_eq!(plan.upserts[1].parent_id, Some(plan.upserts[0].id));
        assert_eq!(plan.upserts[2].parent_id, Some(plan.upserts[1].id));
    }

    #[tokio::test]
    async fn walk_classifies_files_and_builds_folders() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join("Artist/Album")).unwrap();
        std::fs::create_dir_all(root.join("@eaDir/junk")).unwrap();
        std::fs::create_dir_all(root.join("Empty")).unwrap();
        std::fs::write(root.join("Artist/Album/01.flac"), b"x").unwrap();
        std::fs::write(root.join("Artist/Album/cover.jpg"), b"x").unwrap();
        std::fs::write(root.join("@eaDir/junk/02.flac"), b"x").unwrap();
        std::fs::write(root.join("loose.mp3"), b"x").unwrap();

        struct Quiet;
        impl ScanObserver for Quiet {
            fn progress(&self, _fraction: Option<f64>, _step: &ScanStep) {}
        }

        let storage = LocalStorage::open(root).await.unwrap();
        let mut walk = WalkOutcome::default();
        walk_root(&storage, LibraryKind::Music, &Quiet, &mut walk)
            .await
            .unwrap();
        let mut names: Vec<&str> = walk.files.iter().map(|f| f.name.as_str()).collect();
        names.sort_unstable();
        assert_eq!(names, vec!["01.flac", "loose.mp3"]);
        let folder_names: Vec<&str> = walk.folders.values().map(|f| f.name.as_str()).collect();
        assert_eq!(folder_names.len(), 2);
        assert!(folder_names.contains(&"Artist"));
        assert!(folder_names.contains(&"Album"));
    }
}

#[cfg(test)]
mod mount_tests;

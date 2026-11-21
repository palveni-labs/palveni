//! Native filesystem notifications are hints to reconcile an index. The
//! callback retains at most one dirty flag per library, even during imports.
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};

use notify::{
    Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher,
    event::{AccessKind, AccessMode, MetadataKind, ModifyKind},
};
use palveni_core::LibraryId;

#[derive(Default)]
pub struct Changes {
    pub libraries: BTreeSet<LibraryId>,
    pub rearm: bool,
}

fn under_root(path: &Path, root: &Path, data_dir: &Path) -> bool {
    if let Ok(relative) = path.strip_prefix(data_dir)
        && (relative.starts_with("skins")
            || relative.to_str().is_some_and(|name| {
                matches!(
                    name,
                    "node.key"
                        | "palveni.toml"
                        | "palveni.db"
                        | "palveni-ops.db"
                        | "palveni.db-wal"
                        | "palveni.db-shm"
                        | "palveni-ops.db-wal"
                        | "palveni-ops.db-shm"
                )
            }))
    {
        return false;
    }
    path.strip_prefix(root).is_ok_and(|relative| {
        !relative.components().any(|part| {
            part.as_os_str()
                .to_str()
                .is_some_and(crate::classify::is_ignored_name)
        })
    })
}

impl Changes {
    fn record(
        &mut self,
        result: notify::Result<Event>,
        roots: &[(LibraryId, PathBuf)],
        data_dir: &Path,
    ) {
        let event = match result {
            Ok(event) if !event.need_rescan() => event,
            _ => {
                self.libraries.extend(roots.iter().map(|(id, _)| *id));
                self.rearm = true;
                return;
            }
        };
        match event.kind {
            EventKind::Access(kind) if kind != AccessKind::Close(AccessMode::Write) => return,
            EventKind::Modify(ModifyKind::Metadata(MetadataKind::AccessTime)) => return,
            _ if event.paths.is_empty() => {
                self.libraries.extend(roots.iter().map(|(id, _)| *id));
                self.rearm = true;
                return;
            }
            _ => (),
        }
        for (id, root) in roots {
            if event
                .paths
                .iter()
                .any(|path| under_root(path, root, data_dir))
            {
                self.libraries.insert(*id);
                if matches!(
                    event.kind,
                    EventKind::Remove(_)
                        | EventKind::Create(_)
                        | EventKind::Modify(ModifyKind::Name(_))
                ) && event.paths.iter().any(|p| p == root)
                {
                    self.rearm = true;
                }
            }
        }
    }
}

#[derive(Default)]
pub struct LocalWatcher {
    _backend: Option<RecommendedWatcher>,
    changes: Arc<Mutex<Changes>>,
    pub unavailable: BTreeSet<LibraryId>,
}

impl LocalWatcher {
    /// Install one native backend for all libraries. Nested/shared roots
    /// share a kernel watch but each affected library still receives a hint.
    pub fn open(roots: Vec<(LibraryId, PathBuf)>, data_dir: PathBuf) -> Self {
        if roots.is_empty() {
            return Self::default();
        }
        let changes = Arc::new(Mutex::new(Changes::default()));
        let sink = changes.clone();
        let routes = roots.clone();
        let handler = move |event| {
            sink.lock()
                .unwrap_or_else(PoisonError::into_inner)
                .record(event, &routes, &data_dir);
        };
        let mut backend =
            RecommendedWatcher::new(handler, Config::default().with_follow_symlinks(false))
                .map_err(|e| {
                    tracing::warn!(error=%e,"native filesystem notifications unavailable");
                })
                .ok();
        let mut sorted = roots;
        sorted.sort_by(|a, b| a.1.cmp(&b.1));
        let mut covered: Vec<PathBuf> = Vec::new();
        let mut unavailable = BTreeSet::new();
        for (id, root) in sorted {
            // Do not follow a configured root that has since been replaced by a symlink.
            if !root.canonicalize().is_ok_and(|canonical| canonical == root) || !root.is_dir() {
                unavailable.insert(id);
                continue;
            }
            if covered.iter().any(|ancestor| root.starts_with(ancestor)) {
                continue;
            }
            if backend
                .as_mut()
                .is_some_and(|w| w.watch(&root, RecursiveMode::Recursive).is_ok())
            {
                covered.push(root);
            } else {
                unavailable.insert(id);
            }
        }
        Self {
            _backend: backend,
            changes,
            unavailable,
        }
    }

    pub fn drain(&self) -> Changes {
        std::mem::take(&mut *self.changes.lock().unwrap_or_else(PoisonError::into_inner))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use notify::event::{CreateKind, DataChange};
    #[test]
    fn overlapping_roots_coalesce_and_scanner_reads_do_not_trigger_work() {
        let a = LibraryId::generate();
        let b = LibraryId::generate();
        let root = PathBuf::from("/media/music");
        let roots = vec![(a, root.clone()), (b, root.join("Disc"))];
        let data = root.join("server-data");
        let mut hints = Changes::default();
        let file = root.join("Disc/track.flac");
        for (kind, path) in [
            (
                EventKind::Access(AccessKind::Close(AccessMode::Read)),
                file.clone(),
            ),
            (EventKind::Create(CreateKind::File), data.join("palveni.db")),
            (
                EventKind::Modify(ModifyKind::Data(DataChange::Any)),
                root.join(".cache/tmp"),
            ),
        ] {
            hints.record(Ok(Event::new(kind).add_path(path)), &roots, &data);
        }
        hints.record(
            Ok(Event::new(EventKind::Access(AccessKind::Read))),
            &roots,
            &data,
        );
        assert!(hints.libraries.is_empty());
        assert!(under_root(&file, &root, Path::new("/media")));
        assert!(under_root(&data.join("music/song.mp3"), &root, &data));
        for _ in 0..100 {
            hints.record(
                Ok(
                    Event::new(EventKind::Access(AccessKind::Close(AccessMode::Write)))
                        .add_path(file.clone()),
                ),
                &roots,
                &data,
            );
        }
        assert_eq!(hints.libraries, BTreeSet::from([a, b]));
        hints.record(Err(notify::Error::generic("events lost")), &roots, &data);
        assert!(hints.rearm);
    }
}

use super::{FoundFile, WalkOutcome};
use crate::mounts::MountTable;
use palveni_core::{ItemId, ItemKind, LibraryId};
use palveni_db::{Db, DbError, items::IndexedItem};
use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

#[derive(Default)]
pub(super) struct ReviewScopes {
    identities: HashMap<String, String>,
    pending: HashMap<String, String>,
    held: HashSet<String>,
}

#[derive(Default)]
struct Inventory {
    physical: usize,
    missing: Vec<(ItemId, String)>,
    incomplete: bool,
}

impl Inventory {
    fn suspicious(&self) -> bool {
        let missing = self.missing.len();
        missing > 0
            && (missing == self.physical
                || (missing >= 100 && missing >= self.physical - self.physical / 10))
    }
}

impl ReviewScopes {
    pub(super) async fn load(
        db: &Db,
        library: LibraryId,
        roots: &[String],
        table: &MountTable,
    ) -> Result<(Self, Vec<String>), DbError> {
        let mut sources = table.sources(roots);
        let mut protected = HashSet::new();
        let mut pending = HashMap::new();
        for review in db.scan_reviews().list(library).await? {
            if !db
                .scan_reviews()
                .matches_source(
                    library,
                    &review.revision,
                    &table.covering_identity(&review.scope),
                )
                .await?
            {
                // Do not pin a replacement as the first recognized mount of a previously unpinned root.
                sources.remove(&review.scope);
                protected.insert(review.scope.clone());
            }
            pending.insert(review.scope, review.revision);
        }
        protected.extend(db.libraries().unavailable_mounts(library, &sources).await?);
        let identities = roots
            .iter()
            .chain(sources.keys())
            .chain(&protected)
            .chain(pending.keys())
            .map(|scope| (scope.clone(), table.covering_identity(scope)))
            .collect();
        Ok((
            Self {
                identities,
                pending,
                held: HashSet::new(),
            },
            protected.into_iter().collect(),
        ))
    }

    pub(super) fn unchanged(&self, current: &MountTable) -> bool {
        self.identities
            .iter()
            .all(|(scope, identity)| current.covering_identity(scope) == *identity)
    }

    pub(super) fn source(&self, scope: &str) -> &str {
        &self.identities[scope]
    }

    pub(super) fn owner(&self, path: &str) -> Option<&str> {
        Path::new(path).ancestors().find_map(|dir| {
            self.identities
                .get_key_value(dir.to_str()?)
                .map(|(scope, _)| scope.as_str())
        })
    }

    pub(super) fn held(&self, path: &str) -> bool {
        !self.held.is_empty()
            && self
                .owner(path)
                .is_some_and(|scope| self.held.contains(scope))
    }

    pub(super) async fn inspect(
        &mut self,
        db: &Db,
        library: LibraryId,
        requested: Option<&str>,
        existing: &[IndexedItem],
        walk: &WalkOutcome,
    ) -> Result<Option<(String, String)>, DbError> {
        let mut inventories: HashMap<_, Inventory> = self
            .identities
            .keys()
            .map(|scope| (scope.clone(), Inventory::default()))
            .collect();

        for dir in &walk.protected {
            // A failed parent listing may also hide registered submounts.
            if let Some(inventory) = self.owner(dir).and_then(|scope| inventories.get_mut(scope)) {
                inventory.incomplete = true;
            }
            for (scope, inventory) in &mut inventories {
                if Path::new(scope).starts_with(dir) {
                    inventory.incomplete = true;
                }
            }
        }
        let observed: HashSet<_> = walk
            .files
            .iter()
            .filter(|f| physical_file(f))
            .map(|f| f.path.as_str())
            .collect();
        for item in existing.iter().filter(|item| {
            !item.deleted
                && item.kind != ItemKind::Folder
                && item
                    .cue_stamp
                    .as_deref()
                    .is_none_or(|stamp| stamp == "backing")
        }) {
            let Some(inventory) = self
                .owner(&item.path)
                .and_then(|scope| inventories.get_mut(scope))
            else {
                continue;
            };
            inventory.physical += 1;
            if !observed.contains(item.path.as_str()) {
                inventory.missing.push((item.id, item.path.clone()));
            }
        }

        let mut confirmed = None;
        for (scope, inventory) in inventories {
            let previous = self.pending.get(&scope);
            if previous.is_none() && !inventory.suspicious() {
                continue;
            }
            if inventory.incomplete {
                self.held.insert(scope);
                continue;
            }
            if inventory.missing.is_empty() {
                if let Some(previous) = previous
                    && !db.scan_reviews().clear(library, previous).await?
                {
                    self.held.insert(scope);
                }
                continue;
            }
            let review = db
                .scan_reviews()
                .record_on(library, &scope, self.source(&scope), &inventory.missing)
                .await?;
            if requested == Some(review.revision.as_str()) && previous == Some(&review.revision) {
                confirmed = Some((scope, review.revision));
            } else {
                self.held.insert(scope);
            }
        }
        Ok(confirmed)
    }

    pub(super) fn restrict(&self, walk: &mut WalkOutcome) {
        if self.held.is_empty() {
            return;
        }
        walk.files.retain(|f| !self.held(&f.path));
        let mut required_folders = HashSet::new();
        for file in &walk.files {
            for dir in Path::new(&file.path).ancestors().skip(1) {
                if !required_folders.insert(dir) {
                    break;
                }
            }
        }
        // A healthy nested mount can still need folders in its held parent scope.
        walk.folders
            .retain(|path, _| !self.held(path) || required_folders.contains(Path::new(path)));
    }
}

fn physical_file(file: &FoundFile) -> bool {
    file.music.as_ref().is_none_or(|tags| tags.cue.is_none())
}

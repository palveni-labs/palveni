//! Local directory storage.
//!
//! Every path handed out or accepted is resolved against the canonical root
//! and re-canonicalised, so `..` components and symlinks cannot reach
//! outside the root. Symlinked files inside the root are followed;
//! symlinked directories are reported as `Other` so a walk cannot loop.
//! All filesystem calls run on tokio's blocking pool.

use std::fs::Metadata;
use std::io::SeekFrom;
use std::path::{Path, PathBuf};

use async_trait::async_trait;
use palveni_core::clock;
use palveni_core::error::{Error, Result};
use palveni_core::storage::{
    ByteRange, ByteStream, DirEntry, EntryKind, FileStat, StoragePath, StorageProvider,
};
use tokio::io::{AsyncReadExt, AsyncSeekExt};

#[derive(Clone, Debug)]
pub struct LocalStorage {
    /// Canonical absolute root.
    root: PathBuf,
    /// `root` as UTF-8, used for display paths.
    root_display: String,
}

impl LocalStorage {
    /// Opens an existing directory as a storage root.
    pub async fn open(root: &Path) -> Result<Self> {
        let root = root.to_path_buf();
        run_blocking(move || Self::open_blocking(&root)).await
    }

    pub fn open_blocking(root: &Path) -> Result<Self> {
        if !root.is_absolute() {
            return Err(Error::invalid(format!(
                "library path must be absolute: {}",
                root.display()
            )));
        }
        let canonical = std::fs::canonicalize(root).map_err(|e| map_io(e, root))?;
        let meta = std::fs::metadata(&canonical).map_err(|e| map_io(e, &canonical))?;
        if !meta.is_dir() {
            return Err(Error::invalid(format!(
                "not a directory: {}",
                canonical.display()
            )));
        }
        let root_display = canonical
            .to_str()
            .ok_or_else(|| Error::invalid("library path is not valid UTF-8"))?
            .to_owned();
        Ok(LocalStorage {
            root: canonical,
            root_display,
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub(crate) fn open_file_blocking(&self, path: &StoragePath) -> Result<std::fs::File> {
        let resolved = self.resolve(path)?;
        if !std::fs::metadata(&resolved)
            .map_err(|e| map_io(e, &resolved))?
            .is_file()
        {
            return Err(Error::invalid("expected a regular audio file"));
        }
        std::fs::File::open(&resolved).map_err(|e| map_io(e, &resolved))
    }

    /// Resolves a storage path to a canonical host path inside the root.
    fn resolve(&self, path: &StoragePath) -> Result<PathBuf> {
        let mut joined = self.root.clone();
        for component in path.components() {
            joined.push(component);
        }
        let canonical = std::fs::canonicalize(&joined).map_err(|e| map_io(e, &joined))?;
        if canonical.starts_with(&self.root) {
            Ok(canonical)
        } else {
            Err(Error::PathEscape)
        }
    }

    fn list_dir_blocking(&self, path: &StoragePath) -> Result<Vec<DirEntry>> {
        let dir = self.resolve(path)?;
        let reader = std::fs::read_dir(&dir).map_err(|e| map_io(e, &dir))?;
        let mut entries = Vec::new();
        for entry in reader {
            let entry = match entry {
                Ok(entry) => entry,
                Err(err) => {
                    tracing::warn!(dir = %dir.display(), error = %err, "skipping unreadable directory entry");
                    continue;
                }
            };
            let Ok(name) = entry.file_name().into_string() else {
                tracing::warn!(path = %entry.path().display(), "skipping entry whose name is not valid UTF-8");
                continue;
            };
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            let (kind, meta) = if file_type.is_symlink() {
                self.follow_symlink(&entry.path())
            } else if file_type.is_dir() {
                (EntryKind::Directory, entry.metadata().ok())
            } else if file_type.is_file() {
                (EntryKind::File, entry.metadata().ok())
            } else {
                (EntryKind::Other, None)
            };
            entries.push(make_entry(name, kind, meta.as_ref()));
        }
        Ok(entries)
    }

    /// Symlinks are only followed to regular files that stay inside the root.
    fn follow_symlink(&self, link: &Path) -> (EntryKind, Option<Metadata>) {
        match std::fs::canonicalize(link) {
            Ok(target) if target.starts_with(&self.root) => match std::fs::metadata(&target) {
                Ok(meta) if meta.is_file() => (EntryKind::File, Some(meta)),
                _ => (EntryKind::Other, None),
            },
            _ => (EntryKind::Other, None),
        }
    }

    fn stat_blocking(&self, path: &StoragePath) -> Result<FileStat> {
        let resolved = self.resolve(path)?;
        let meta = std::fs::metadata(&resolved).map_err(|e| map_io(e, &resolved))?;
        let kind = if meta.is_dir() {
            EntryKind::Directory
        } else if meta.is_file() {
            EntryKind::File
        } else {
            EntryKind::Other
        };
        let entry = make_entry(String::new(), kind, Some(&meta));
        Ok(FileStat {
            kind: entry.kind,
            size: entry.size,
            modified: entry.modified,
        })
    }
}

fn make_entry(name: String, kind: EntryKind, meta: Option<&Metadata>) -> DirEntry {
    let size = match (kind, meta) {
        (EntryKind::File, Some(meta)) => Some(meta.len()),
        _ => None,
    };
    let modified = meta
        .and_then(|m| m.modified().ok())
        .map(clock::from_system_time);
    DirEntry {
        name,
        kind,
        size,
        modified,
    }
}

fn map_io(err: std::io::Error, path: &Path) -> Error {
    if err.kind() == std::io::ErrorKind::NotFound {
        Error::not_found(path.display().to_string())
    } else {
        Error::Io(err)
    }
}

async fn run_blocking<T, F>(f: F) -> Result<T>
where
    F: FnOnce() -> Result<T> + Send + 'static,
    T: Send + 'static,
{
    tokio::task::spawn_blocking(f)
        .await
        .map_err(|e| Error::internal(format!("blocking filesystem task failed: {e}")))?
}

#[async_trait]
impl StorageProvider for LocalStorage {
    fn display_path(&self, path: &StoragePath) -> String {
        if path.is_root() {
            return self.root_display.clone();
        }
        let mut joined = self.root.clone();
        for component in path.components() {
            joined.push(component);
        }
        // Components are UTF-8 by construction, so this is lossless.
        joined.to_string_lossy().into_owned()
    }

    async fn list_dir(&self, path: &StoragePath) -> Result<Vec<DirEntry>> {
        let this = self.clone();
        let path = path.clone();
        run_blocking(move || this.list_dir_blocking(&path)).await
    }

    async fn stat(&self, path: &StoragePath) -> Result<FileStat> {
        let this = self.clone();
        let path = path.clone();
        run_blocking(move || this.stat_blocking(&path)).await
    }

    async fn open_range(&self, path: &StoragePath, range: ByteRange) -> Result<ByteStream> {
        let this = self.clone();
        let path = path.clone();
        let resolved = run_blocking(move || this.resolve(&path)).await?;
        let mut file = tokio::fs::File::open(&resolved)
            .await
            .map_err(|e| map_io(e, &resolved))?;
        if range.start > 0 {
            file.seek(SeekFrom::Start(range.start)).await?;
        }
        let stream: ByteStream = match range.length() {
            Some(len) => Box::pin(file.take(len)),
            None => Box::pin(file),
        };
        Ok(stream)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn lists_and_confines() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("Album")).unwrap();
        std::fs::write(dir.path().join("Album/01.flac"), b"abcdef").unwrap();
        let storage = LocalStorage::open(dir.path()).await.unwrap();

        let root = storage.list_dir(&StoragePath::root()).await.unwrap();
        assert_eq!(root.len(), 1);
        assert_eq!(root[0].kind, EntryKind::Directory);

        let album = StoragePath::parse("Album").unwrap();
        let files = storage.list_dir(&album).await.unwrap();
        assert_eq!(files[0].name, "01.flac");
        assert_eq!(files[0].size, Some(6));

        let mut stream = storage
            .open_range(
                &album.join("01.flac").unwrap(),
                ByteRange {
                    start: 2,
                    end: Some(4),
                },
            )
            .await
            .unwrap();
        let mut buf = Vec::new();
        stream.read_to_end(&mut buf).await.unwrap();
        assert_eq!(buf, b"cd");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn symlinks_cannot_escape() {
        let outside = tempfile::tempdir().unwrap();
        std::fs::write(outside.path().join("secret.flac"), b"x").unwrap();
        let dir = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink(
            outside.path().join("secret.flac"),
            dir.path().join("link.flac"),
        )
        .unwrap();
        std::os::unix::fs::symlink(outside.path(), dir.path().join("linkdir")).unwrap();
        let storage = LocalStorage::open(dir.path()).await.unwrap();

        let entries = storage.list_dir(&StoragePath::root()).await.unwrap();
        assert!(entries.iter().all(|e| e.kind == EntryKind::Other));

        let escaped = storage
            .stat(&StoragePath::parse("linkdir/secret.flac").unwrap())
            .await;
        assert!(matches!(escaped, Err(Error::PathEscape)));
    }

    #[tokio::test]
    async fn rejects_relative_roots() {
        assert!(LocalStorage::open(Path::new("relative/dir")).await.is_err());
    }
}

//! Storage abstraction.
//!
//! Scanners and streaming code address files through a [`StorageProvider`]
//! and a root-relative [`StoragePath`]; they never touch host paths directly.
//! Implementations are responsible for confining every resolved path to
//! their root.

use std::fmt;
use std::pin::Pin;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use tokio::io::AsyncRead;

use crate::error::{Error, Result};

/// A path relative to a storage root, using `/` as the separator.
///
/// The empty path is the root. Components are never empty, `.` or `..`, and
/// never contain `/`, `\` or NUL, so a `StoragePath` cannot express an escape
/// from its root on its own (symlinks are the provider's job).
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct StoragePath(String);

impl StoragePath {
    pub fn root() -> Self {
        StoragePath(String::new())
    }

    /// Parses a `/`-separated relative path. Leading and trailing slashes
    /// are rejected rather than silently stripped.
    pub fn parse(text: &str) -> Result<Self> {
        if text.is_empty() {
            return Ok(Self::root());
        }
        for component in text.split('/') {
            validate_component(component)?;
        }
        Ok(StoragePath(text.to_owned()))
    }

    /// Appends one component.
    pub fn join(&self, name: &str) -> Result<Self> {
        validate_component(name)?;
        if self.0.is_empty() {
            Ok(StoragePath(name.to_owned()))
        } else {
            Ok(StoragePath(format!("{}/{}", self.0, name)))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn is_root(&self) -> bool {
        self.0.is_empty()
    }

    /// Parent path; `None` for the root.
    pub fn parent(&self) -> Option<StoragePath> {
        if self.0.is_empty() {
            return None;
        }
        match self.0.rsplit_once('/') {
            Some((parent, _)) => Some(StoragePath(parent.to_owned())),
            None => Some(StoragePath::root()),
        }
    }

    /// Last component; `None` for the root.
    pub fn file_name(&self) -> Option<&str> {
        if self.0.is_empty() {
            None
        } else {
            Some(self.0.rsplit('/').next().unwrap_or(&self.0))
        }
    }

    pub fn components(&self) -> impl Iterator<Item = &str> {
        self.0.split('/').filter(|c| !c.is_empty())
    }

    /// Number of components (0 for the root).
    pub fn depth(&self) -> usize {
        self.components().count()
    }
}

fn validate_component(component: &str) -> Result<()> {
    let bad = component.is_empty()
        || component == "."
        || component == ".."
        || component.contains(['/', '\\', '\0']);
    if bad {
        Err(Error::invalid(format!(
            "invalid path component `{component}`"
        )))
    } else {
        Ok(())
    }
}

impl fmt::Debug for StoragePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "StoragePath({:?})", self.0)
    }
}

impl fmt::Display for StoragePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.is_empty() {
            f.write_str("/")
        } else {
            f.write_str(&self.0)
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntryKind {
    File,
    Directory,
    /// Sockets, devices, dangling or escaping symlinks, symlinked directories…
    Other,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DirEntry {
    pub name: String,
    pub kind: EntryKind,
    /// Size in bytes for files.
    pub size: Option<u64>,
    pub modified: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileStat {
    pub kind: EntryKind,
    pub size: Option<u64>,
    pub modified: Option<DateTime<Utc>>,
}

/// Byte range `[start, end)`; `end == None` reads to the end of the file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ByteRange {
    pub start: u64,
    pub end: Option<u64>,
}

impl ByteRange {
    pub const fn full() -> Self {
        ByteRange {
            start: 0,
            end: None,
        }
    }

    /// Number of bytes covered, if bounded.
    pub fn length(&self) -> Option<u64> {
        self.end.map(|end| end.saturating_sub(self.start))
    }
}

/// A readable byte stream returned by [`StorageProvider::open_range`].
pub type ByteStream = Pin<Box<dyn AsyncRead + Send>>;

/// A representation whose length and range reads refer to the same open source.
#[async_trait]
pub trait OpenedFile: Send {
    fn size(&self) -> u64;
    async fn read_range(self: Box<Self>, range: ByteRange) -> Result<ByteStream>;
}

/// Lists, inspects and opens files relative to the configured storage root.
#[async_trait]
pub trait StorageProvider: Send + Sync {
    /// Human-readable location of `path` (an absolute host path for local
    /// storage). Used for display and as the stable identity of an item.
    fn display_path(&self, path: &StoragePath) -> String;

    async fn list_dir(&self, path: &StoragePath) -> Result<Vec<DirEntry>>;

    async fn stat(&self, path: &StoragePath) -> Result<FileStat>;

    async fn open_file(&self, path: &StoragePath) -> Result<Box<dyn OpenedFile>>;

    async fn open_range(&self, path: &StoragePath, range: ByteRange) -> Result<ByteStream> {
        self.open_file(path).await?.read_range(range).await
    }
}

#[derive(Clone, Copy, serde::Serialize, utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum MountProtocol {
    Smb,
    Nfs,
    Webdav,
    Sshfs,
    Rclone,
    Network,
}

#[derive(Clone, serde::Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MountedDirectory {
    pub path: String,
    pub protocol: MountProtocol,
    pub read_only: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_and_join() {
        let p = StoragePath::parse("Music/Album").unwrap();
        assert_eq!(p.depth(), 2);
        let f = p.join("01 Intro.flac").unwrap();
        assert_eq!(f.as_str(), "Music/Album/01 Intro.flac");
        assert_eq!(f.file_name(), Some("01 Intro.flac"));
        assert_eq!(f.parent(), Some(p.clone()));
        assert_eq!(p.parent().unwrap().parent(), Some(StoragePath::root()));
        assert_eq!(StoragePath::root().parent(), None);
        assert_eq!(StoragePath::root().join("a").unwrap().as_str(), "a");
    }

    #[test]
    fn rejects_escapes() {
        for bad in [
            "../etc", "a/../b", "/abs", "a//b", "a/", "./a", "a\\b", "nul\0",
        ] {
            assert!(StoragePath::parse(bad).is_err(), "{bad:?}");
        }
        assert!(StoragePath::root().join("..").is_err());
        assert!(StoragePath::root().join("a/b").is_err());
    }

    #[test]
    fn ranges() {
        assert_eq!(ByteRange::full().length(), None);
        assert_eq!(
            ByteRange {
                start: 10,
                end: Some(25)
            }
            .length(),
            Some(15)
        );
        assert_eq!(
            ByteRange {
                start: 30,
                end: Some(25)
            }
            .length(),
            Some(0)
        );
    }
}

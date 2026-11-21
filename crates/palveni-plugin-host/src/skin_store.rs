//! Installation and serving of data-only skin packages.
mod archive;

use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::{PoisonError, RwLock};

use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use self::archive::{AssetRange, IndexedSkin, index_package};

pub const MAX_PACKAGE_BYTES: usize = 16 * 1024 * 1024;

#[derive(Debug, thiserror::Error)]
pub enum SkinError {
    #[error("Skin not found")]
    NotFound,
    #[error("Skin ID or plugin ID already in use")]
    Conflict,
    #[error("Skin package exceeds size or entry limit")]
    TooLarge,
    #[error("{0}")]
    Invalid(String),
    #[error("Bundled skin package is unavailable")]
    Unavailable,
    #[error("Skin storage I/O error: {0}")]
    Io(#[from] io::Error),
}

#[derive(utoipa::ToSchema, Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkinInfo {
    pub id: String,
    pub plugin_id: String,
    pub version: String,
    pub name: BTreeMap<String, String>,
    pub description: BTreeMap<String, String>,
    pub size_bytes: usize,
    #[schema(required = true)]
    pub preview_url: Option<String>,
}

#[derive(utoipa::ToSchema, Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SkinSource {
    Official,
    Upload,
}

#[derive(utoipa::ToSchema, Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledSkin {
    #[serde(flatten)]
    pub skin: SkinInfo,
    pub installed_at: String,
    pub source: SkinSource,
}

#[derive(utoipa::ToSchema, Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistrySkin {
    #[serde(flatten)]
    pub skin: SkinInfo,
    pub published_at: String,
    pub download_url: String,
    pub sha256: String,
    #[serde(default)]
    #[schema(required = true)]
    pub installed: bool,
    #[schema(ignore)]
    #[serde(skip_serializing)]
    filename: String,
    #[schema(ignore)]
    #[serde(skip_serializing)]
    preview: PinnedRange,
}

#[derive(Debug)]
pub struct SkinAsset {
    pub mime: &'static str,
    pub bytes: Vec<u8>,
}

#[derive(Debug)]
struct Installation {
    summary: InstalledSkin,
    filename: String,
    assets: BTreeMap<String, AssetRange>,
}

#[derive(Debug)]
pub struct SkinStore {
    dir: PathBuf,
    bundles: PathBuf,
    registry: Vec<RegistrySkin>,
    installations: RwLock<BTreeMap<String, Installation>>,
}

fn invalid(message: &str) -> SkinError {
    SkinError::Invalid(message.into())
}

fn checksum(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

// The configured data directory itself may live on a mounted volume. Its
// child directories and package files are managed by the host, never plugins.
fn verify_directory(dir: &Path) -> Result<(), SkinError> {
    if !fs::symlink_metadata(dir)?.is_dir() || dir.canonicalize()? != dir {
        return Err(invalid("Skin directory must not be a symbolic link"));
    }
    Ok(())
}

fn regular_file(path: &Path) -> Result<File, SkinError> {
    verify_directory(
        path.parent()
            .ok_or_else(|| invalid("Missing parent directory"))?,
    )?;
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_file() {
        return Err(invalid("Skin package must be a regular file"));
    }
    let file = File::open(path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let opened = file.metadata()?;
        if opened.dev() != metadata.dev() || opened.ino() != metadata.ino() {
            return Err(invalid("Skin package changed while opening"));
        }
    }
    Ok(file)
}

fn bounded_read(path: &Path) -> Result<Vec<u8>, SkinError> {
    let file = regular_file(path)?;
    if file.metadata()?.len() > MAX_PACKAGE_BYTES as u64 {
        return Err(SkinError::TooLarge);
    }
    let mut bytes = Vec::new();
    file.take(MAX_PACKAGE_BYTES as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_PACKAGE_BYTES {
        return Err(SkinError::TooLarge);
    }
    Ok(bytes)
}

impl SkinStore {
    pub fn open(data_dir: &Path) -> Result<Self, SkinError> {
        fs::create_dir_all(data_dir)?;
        let dir = data_dir.canonicalize()?.join("skins");
        match fs::create_dir(&dir) {
            Err(err) if err.kind() == io::ErrorKind::AlreadyExists => (),
            result => result?,
        }
        verify_directory(&dir)?;
        let beside_exe = std::env::current_exe()?.with_file_name("skin-packages");
        let bundles = if beside_exe.exists() {
            beside_exe
        } else {
            PathBuf::from(concat!(env!("OUT_DIR"), "/skin-packages"))
        };
        let registry = serde_json::from_str(include_str!(concat!(
            env!("OUT_DIR"),
            "/skin-registry.json"
        )))
        .map_err(|e| invalid(&e.to_string()))?;
        let store = Self {
            dir,
            bundles,
            registry,
            installations: RwLock::default(),
        };
        store.reload()?;
        Ok(store)
    }

    fn official(&self, id: &str) -> Result<&RegistrySkin, SkinError> {
        self.registry
            .iter()
            .find(|skin| skin.skin.id == id)
            .ok_or(SkinError::NotFound)
    }

    pub fn installed(&self) -> Result<Vec<InstalledSkin>, SkinError> {
        Ok(self
            .installations
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .values()
            .map(|entry| entry.summary.clone())
            .collect())
    }

    pub fn registry(&self) -> Result<Vec<RegistrySkin>, SkinError> {
        let installed = self
            .installations
            .read()
            .unwrap_or_else(PoisonError::into_inner);
        Ok(self
            .registry
            .iter()
            .cloned()
            .map(|mut item| {
                item.installed = installed.contains_key(&item.skin.id);
                item
            })
            .collect())
    }

    pub fn package(&self, id: &str) -> Result<SkinAsset, SkinError> {
        let spec = self.official(id)?;
        let bytes = bounded_read(&self.bundles.join(&spec.filename)).map_err(bundle_error)?;
        verify_pin(spec, &bytes)?;
        Ok(SkinAsset {
            mime: "application/x-tar",
            bytes,
        })
    }

    pub fn preview(&self, id: &str) -> Result<SkinAsset, SkinError> {
        let spec = self.official(id)?;
        let file = regular_file(&self.bundles.join(&spec.filename)).map_err(bundle_error)?;
        spec.preview.read(file, "image/png")
    }

    pub fn install_official(&self, id: &str) -> Result<InstalledSkin, SkinError> {
        if let Some(entry) = self.current_official(id)? {
            return Ok(entry);
        }
        self.install_official_bytes(id, &self.package(id)?.bytes)
    }

    fn current_official(&self, id: &str) -> Result<Option<InstalledSkin>, SkinError> {
        self.official(id)?;
        Ok(self
            .installations
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .get(id)
            .map(|e| e.summary.clone()))
    }

    pub fn install_official_bytes(
        &self,
        id: &str,
        bytes: &[u8],
    ) -> Result<InstalledSkin, SkinError> {
        verify_pin(self.official(id)?, bytes)?;
        self.install(bytes, SkinSource::Official)
    }

    pub fn upload(&self, bytes: &[u8]) -> Result<InstalledSkin, SkinError> {
        self.install(bytes, SkinSource::Upload)
    }

    fn check_identity(
        &self,
        info: &SkinInfo,
        digest: &str,
        source: SkinSource,
    ) -> Result<(), SkinError> {
        let reserved = self
            .registry
            .iter()
            .find(|s| s.skin.id == info.id || s.skin.plugin_id == info.plugin_id);
        match (reserved, source) {
            (Some(spec), SkinSource::Official)
                if spec.sha256 == digest
                    && spec.skin.id == info.id
                    && spec.skin.plugin_id == info.plugin_id =>
            {
                Ok(())
            }
            (None, SkinSource::Upload) => Ok(()),
            _ => Err(SkinError::Conflict),
        }
    }

    fn install(&self, bytes: &[u8], source: SkinSource) -> Result<InstalledSkin, SkinError> {
        let index = index_package(bytes)?;
        let digest = checksum(bytes);
        self.check_identity(&index.info, &digest, source)?;
        let mut entries = self
            .installations
            .write()
            .unwrap_or_else(PoisonError::into_inner);
        if let Some(existing) = entries.get(&index.info.id) {
            if source == SkinSource::Official && existing.summary.source == source {
                return Ok(existing.summary.clone());
            }
            return Err(SkinError::Conflict);
        }
        if entries
            .values()
            .any(|e| e.summary.skin.plugin_id == index.info.plugin_id)
        {
            return Err(SkinError::Conflict);
        }
        verify_directory(&self.dir)?;
        let now = Utc::now();
        let filename = format!(
            "{}-{}-{digest}.tar",
            source.as_str(),
            now.timestamp_millis()
        );
        let mut temp = tempfile::NamedTempFile::new_in(&self.dir)?;
        temp.write_all(bytes)?;
        temp.as_file().sync_all()?;
        temp.persist_noclobber(self.dir.join(&filename))
            .map_err(|e| SkinError::Io(e.error))?;
        let installation = Installation::new(index, filename, now, source);
        let summary = installation.summary.clone();
        entries.insert(summary.skin.id.clone(), installation);
        Ok(summary)
    }

    pub fn remove(&self, id: &str) -> Result<(), SkinError> {
        let mut entries = self
            .installations
            .write()
            .unwrap_or_else(PoisonError::into_inner);
        let entry = entries.get(id).ok_or(SkinError::NotFound)?;
        verify_directory(&self.dir)?;
        fs::remove_file(self.dir.join(&entry.filename))?;
        entries.remove(id);
        Ok(())
    }

    pub fn manifest(&self, id: &str) -> Result<(SkinAsset, SkinSource), SkinError> {
        self.read_asset(id, "skin.json")
    }

    pub fn asset(&self, id: &str, path: &str) -> Result<SkinAsset, SkinError> {
        if path == "skin.json" {
            return Err(SkinError::NotFound);
        }
        self.read_asset(id, path).map(|(asset, _)| asset)
    }

    fn read_asset(&self, id: &str, key: &str) -> Result<(SkinAsset, SkinSource), SkinError> {
        let (file, range, source) = {
            let entries = self
                .installations
                .read()
                .unwrap_or_else(PoisonError::into_inner);
            let entry = entries.get(id).ok_or(SkinError::NotFound)?;
            let range = entry.assets.get(key).ok_or(SkinError::NotFound)?.clone();
            (
                regular_file(&self.dir.join(&entry.filename))?,
                range,
                entry.summary.source,
            )
        };
        Ok((range.pinned.read(file, range.mime)?, source))
    }

    fn reload(&self) -> Result<(), SkinError> {
        let mut loaded = self
            .installations
            .write()
            .unwrap_or_else(PoisonError::into_inner);
        for entry in fs::read_dir(&self.dir)? {
            let entry = entry?;
            let Some(filename) = entry.file_name().to_str().map(str::to_owned) else {
                continue;
            };
            if !filename.ends_with(".tar") {
                continue;
            }
            let package = self.reopen(&filename);
            match package {
                Ok(p)
                    if !loaded.contains_key(&p.summary.skin.id)
                        && !loaded.values().any(|other| {
                            other.summary.skin.plugin_id == p.summary.skin.plugin_id
                        }) =>
                {
                    loaded.insert(p.summary.skin.id.clone(), p);
                }
                _ => tracing::warn!("skipping invalid or conflicting installed skin package"),
            }
        }
        Ok(())
    }

    fn reopen(&self, filename: &str) -> Result<Installation, SkinError> {
        let mut parts = filename
            .strip_suffix(".tar")
            .ok_or_else(|| invalid("Missing suffix"))?
            .split('-');
        let source = match parts.next() {
            Some("official") => SkinSource::Official,
            Some("upload") => SkinSource::Upload,
            _ => return Err(invalid("Unknown skin source")),
        };
        let time = parts
            .next()
            .and_then(|s| s.parse().ok())
            .and_then(DateTime::from_timestamp_millis)
            .ok_or_else(|| invalid("Invalid installation time"))?;
        let digest = parts.next().ok_or_else(|| invalid("Missing checksum"))?;
        if parts.next().is_some() || digest.len() != 64 {
            return Err(invalid("Invalid skin filename"));
        }
        let bytes = bounded_read(&self.dir.join(filename))?;
        if checksum(&bytes) != digest {
            return Err(invalid("Stored skin checksum mismatch"));
        }
        let index = index_package(&bytes)?;
        self.check_identity(&index.info, digest, source)?;
        Ok(Installation::new(index, filename.into(), time, source))
    }
}

impl SkinSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Official => "official",
            Self::Upload => "upload",
        }
    }
}

impl Installation {
    fn new(index: IndexedSkin, filename: String, time: DateTime<Utc>, source: SkinSource) -> Self {
        Self {
            summary: InstalledSkin {
                skin: index.info,
                installed_at: time.to_rfc3339_opts(SecondsFormat::Millis, true),
                source,
            },
            filename,
            assets: index.assets,
        }
    }
}

fn verify_pin(spec: &RegistrySkin, bytes: &[u8]) -> Result<(), SkinError> {
    if bytes.len() > MAX_PACKAGE_BYTES {
        return Err(SkinError::TooLarge);
    }
    if bytes.len() != spec.skin.size_bytes || checksum(bytes) != spec.sha256 {
        return Err(invalid("Official skin package checksum mismatch"));
    }
    Ok(())
}

fn bundle_error(err: SkinError) -> SkinError {
    match err {
        SkinError::Io(err) if err.kind() == io::ErrorKind::NotFound => SkinError::Unavailable,
        other => other,
    }
}

#[derive(Clone, Debug, Deserialize)]
struct PinnedRange {
    offset: usize,
    len: usize,
    digest: String,
}

impl PinnedRange {
    fn read(&self, mut file: File, mime: &'static str) -> Result<SkinAsset, SkinError> {
        file.seek(SeekFrom::Start(self.offset as u64))?;
        let mut bytes = vec![0; self.len];
        file.read_exact(&mut bytes)?;
        if checksum(&bytes) != self.digest {
            return Err(invalid("Skin resource checksum mismatch"));
        }
        Ok(SkinAsset { mime, bytes })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn third_party(base: &[u8], edit: impl Fn(&str, &mut Vec<u8>)) -> Vec<u8> {
        let mut writer = tar::Builder::new(Vec::new());
        let mut reader = tar::Archive::new(Cursor::new(base));
        for file in reader.entries().expect("entries") {
            let mut file = file.expect("file");
            let name = file.path().expect("name").into_owned();
            let mut bytes = Vec::new();
            file.read_to_end(&mut bytes).expect("content");
            if name == Path::new("skin.json") {
                let mut json: serde_json::Value = serde_json::from_slice(&bytes).expect("manifest");
                json["id"] = "custom".into();
                bytes = serde_json::to_vec(&json).expect("json");
            }
            if name == Path::new("plugin.toml") {
                bytes = std::str::from_utf8(&bytes)
                    .expect("plugin")
                    .replace("com.palveni.skin.native", "org.example.skin.custom")
                    .into_bytes();
            }
            edit(name.to_str().expect("text path"), &mut bytes);
            let mut header = file.header().clone();
            header.set_size(bytes.len() as u64);
            writer
                .append_data(&mut header, name, &bytes[..])
                .expect("append");
        }
        writer.into_inner().expect("archive")
    }

    #[test]
    fn official_packages_are_pinned_idempotent_and_reloadable() {
        let tmp = tempfile::tempdir().expect("temp");
        let store = SkinStore::open(tmp.path()).expect("store");
        for spec in store.registry().expect("registry") {
            let id = &spec.skin.id;
            let pkg = store.package(id).expect("official package");
            assert_eq!(pkg.bytes.len(), spec.skin.size_bytes);
            let preview = store.preview(id).expect("registry preview");
            let first = store
                .install_official_bytes(id, &pkg.bytes)
                .expect("install");
            assert_eq!(first.source, SkinSource::Official);
            assert_eq!(
                store.install_official(id).expect("retry").installed_at,
                first.installed_at
            );
            let asset = store.asset(id, "preview.png").expect("preview");
            assert_eq!(asset.bytes, preview.bytes);
            assert!(matches!(
                store.asset(id, "plugin.toml"),
                Err(SkinError::NotFound)
            ));
            assert!(matches!(
                store.install_official_bytes(id, b"other"),
                Err(SkinError::Invalid(_))
            ));
        }
        let summaries = serde_json::to_value(store.installed().expect("installed")).expect("json");
        drop(store);
        let restored = SkinStore::open(tmp.path()).expect("restart");
        assert_eq!(
            serde_json::to_value(restored.installed().expect("list")).expect("json"),
            summaries
        );
        let (manifest, source) = restored.manifest("ink").expect("manifest");
        assert_eq!(source, SkinSource::Official);
        assert_eq!(manifest.mime, "application/json");
    }

    #[test]
    fn uploaded_files_are_validated_as_a_unit_and_removal_persists() {
        let tmp = tempfile::tempdir().expect("temp");
        let store = SkinStore::open(tmp.path()).expect("store");
        let base = store.package("native").expect("base").bytes;
        assert!(matches!(store.upload(&base), Err(SkinError::Conflict)));
        let invalid = third_party(&base, |path, bytes| {
            if path.ends_with(".woff2") {
                bytes[0] = 0;
            }
        });
        assert!(matches!(store.upload(&invalid), Err(SkinError::Invalid(_))));
        assert!(store.installed().expect("list").is_empty());
        let valid = third_party(&base, |_, _| {});
        let summary = store.upload(&valid).expect("upload");
        assert_eq!(summary.skin.id, "custom");
        assert_eq!(summary.source, SkinSource::Upload);
        assert!(matches!(store.upload(&valid), Err(SkinError::Conflict)));
        for bad in [
            "../plugin.toml",
            "skin.json",
            "locales/en.json",
            "fonts/LICENSE-inter.txt",
            "/preview.png",
        ] {
            assert!(matches!(
                store.asset("custom", bad),
                Err(SkinError::NotFound)
            ));
        }
        drop(store);
        let store = SkinStore::open(tmp.path()).expect("reload");
        assert_eq!(store.installed().expect("list").len(), 1);
        let (_, source) = store.manifest("custom").expect("manifest");
        assert_eq!(source, SkinSource::Upload);
        store.remove("custom").expect("remove");
        drop(store);
        assert!(
            SkinStore::open(tmp.path())
                .expect("reload")
                .installed()
                .expect("list")
                .is_empty()
        );
    }

    #[cfg(unix)]
    #[test]
    fn storage_rejects_symlinked_directory_and_revalidates_changed_bytes() {
        use std::os::unix::fs::symlink;
        let tmp = tempfile::tempdir().expect("temp");
        let other = tempfile::tempdir().expect("other");
        symlink(other.path(), tmp.path().join("skins")).expect("symlink");
        assert!(SkinStore::open(tmp.path()).is_err());
        assert_eq!(fs::read_dir(other.path()).expect("dir").count(), 0);
        fs::remove_file(tmp.path().join("skins")).expect("remove symlink");
        let store = SkinStore::open(tmp.path()).expect("store");
        store.install_official("gear").expect("install");
        let package = fs::read_dir(tmp.path().join("skins"))
            .expect("entries")
            .flatten()
            .find(|e| e.path().extension().is_some_and(|s| s == "tar"))
            .expect("archive")
            .path();
        drop(store);
        let mut data = fs::read(&package).expect("read");
        data[0] ^= 1;
        fs::write(package, &data).expect("write");
        let reloaded = SkinStore::open(tmp.path()).expect("reload");
        assert!(reloaded.installed().expect("list").is_empty());
    }
}

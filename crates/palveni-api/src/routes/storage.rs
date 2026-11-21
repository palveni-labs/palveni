use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};

use axum::{Json, extract::Query};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{ApiError, auth::AdminUser};

const DIRECTORY_LIMIT: usize = 200;

#[derive(Clone, Copy, Serialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum MountProtocol {
    Smb,
    Nfs,
    Webdav,
    Sshfs,
    Rclone,
    Network,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MountedDirectory {
    path: String,
    protocol: MountProtocol,
    read_only: bool,
}

#[derive(Serialize, ToSchema)]
pub struct MountList {
    mounts: Vec<MountedDirectory>,
}

#[derive(Deserialize)]
pub struct FolderQuery {
    mount: String,
    #[serde(default)]
    folder: String,
}

#[derive(Serialize, ToSchema)]
pub struct StorageFolders {
    directories: Vec<String>,
    truncated: bool,
}

fn protocol(fs: &str) -> Option<MountProtocol> {
    match fs {
        "cifs" | "smb3" | "smbfs" => Some(MountProtocol::Smb),
        "nfs" | "nfs4" => Some(MountProtocol::Nfs),
        "davfs" | "davfs2" | "fuse.davfs" | "webdav" => Some(MountProtocol::Webdav),
        "sshfs" | "fuse.sshfs" => Some(MountProtocol::Sshfs),
        "rclone" | "fuse.rclone" => Some(MountProtocol::Rclone),
        "ceph" | "glusterfs" | "fuse.glusterfs" | "fuse.s3fs" => Some(MountProtocol::Network),
        _ => None,
    }
}

fn unescape_mount_path(raw: &str) -> String {
    raw.replace("\\040", " ")
        .replace("\\011", "\t")
        .replace("\\012", "\n")
        .replace("\\134", "\\")
}

fn linux_mounts(info: &str) -> Vec<MountedDirectory> {
    let mut found = std::collections::BTreeMap::new();
    for line in info.lines() {
        let Some((mount, superblock)) = line.split_once(" - ") else {
            continue;
        };
        let mut fields = mount.split_whitespace().skip(4);
        let (Some(path), Some(options)) = (fields.next(), fields.next()) else {
            continue;
        };
        let Some(kind) = superblock.split_whitespace().next().and_then(protocol) else {
            continue;
        };
        let path = unescape_mount_path(path);
        if Path::new(&path).is_absolute() {
            found.insert(
                path.clone(),
                MountedDirectory {
                    path,
                    protocol: kind,
                    read_only: options.split(',').any(|x| x == "ro"),
                },
            );
        }
    }
    found.into_values().collect()
}

async fn mounted() -> Result<Vec<MountedDirectory>, ApiError> {
    if cfg!(target_os = "linux") {
        let info = tokio::fs::read_to_string("/proc/self/mountinfo")
            .await
            .map_err(ApiError::internal)?;
        return Ok(linux_mounts(&info));
    }
    #[cfg(target_os = "macos")]
    {
        let output = tokio::process::Command::new("/sbin/mount")
            .output()
            .await
            .map_err(ApiError::internal)?;
        let listing = String::from_utf8_lossy(&output.stdout);
        let mut result = Vec::new();
        for line in listing.lines() {
            let Some((_, tail)) = line.split_once(" on ") else {
                continue;
            };
            let Some((path, options)) = tail.rsplit_once(" (") else {
                continue;
            };
            let mut opts = options.trim_end_matches(')').split(',').map(str::trim);
            if let Some(kind) = opts.next().and_then(protocol) {
                result.push(MountedDirectory {
                    path: path.to_owned(),
                    protocol: kind,
                    read_only: opts.any(|v| v == "read-only"),
                });
            }
        }
        result.sort_by(|a, b| a.path.cmp(&b.path));
        return Ok(result);
    }
    #[allow(unreachable_code)]
    Ok(Vec::new())
}

pub async fn mounts(_: AdminUser) -> Result<Json<MountList>, ApiError> {
    Ok(Json(MountList {
        mounts: mounted().await?,
    }))
}

pub async fn folders(
    _: AdminUser,
    query: Result<Query<FolderQuery>, axum::extract::rejection::QueryRejection>,
) -> Result<Json<StorageFolders>, ApiError> {
    let Query(query) = query?;
    let mount = mounted()
        .await?
        .into_iter()
        .find(|row| row.path == query.mount)
        .ok_or_else(|| ApiError::not_found("Mount is no longer available"))?;
    inspect_directory(Path::new(&mount.path), &query.folder)
        .await
        .map(Json)
}

async fn inspect_directory(root: &Path, relative: &str) -> Result<StorageFolders, ApiError> {
    let missing = || ApiError::not_found("Directory is unavailable");
    if relative.starts_with('/')
        || relative.contains(['\\', '\0'])
        || relative.split('/').any(|s| s == "." || s == "..")
    {
        return Err(
            ApiError::validation("Expected a relative directory inside the mount").field("folder"),
        );
    }
    let mut path = PathBuf::from(root);
    for component in Path::new(relative).components() {
        let Component::Normal(name) = component else {
            return Err(missing());
        };
        path.push(name);
        if !tokio::fs::symlink_metadata(&path)
            .await
            .map_err(|_| missing())?
            .is_dir()
        {
            return Err(missing());
        }
    }
    if tokio::fs::canonicalize(&path)
        .await
        .map_err(|_| missing())?
        != path
    {
        return Err(missing());
    }
    let mut entries = tokio::fs::read_dir(path).await.map_err(|_| missing())?;
    let mut names = BTreeSet::new();
    let mut truncated = false;
    while let Some(entry) = entries.next_entry().await.map_err(|_| missing())? {
        let Ok(name) = entry.file_name().into_string() else {
            continue;
        };
        if name.starts_with('.') || !entry.file_type().await.map_err(|_| missing())?.is_dir() {
            continue;
        }
        names.insert(name);
        if names.len() > DIRECTORY_LIMIT {
            names.pop_last();
            truncated = true;
        }
    }
    Ok(StorageFolders {
        directories: names.into_iter().collect(),
        truncated,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[cfg(unix)]
    async fn folder_picker_lists_only_real_directories_inside_the_mount() {
        let base = std::env::temp_dir().join(format!(
            "palveni-mount-picker-{}",
            palveni_core::LibraryId::generate()
        ));
        let root = base.join("share");
        tokio::fs::create_dir_all(root.join("Music")).await.unwrap();
        tokio::fs::create_dir_all(base.join("elsewhere"))
            .await
            .unwrap();
        std::os::unix::fs::symlink(base.join("elsewhere"), root.join("link")).unwrap();
        let canonical = tokio::fs::canonicalize(&root).await.unwrap();
        let page = inspect_directory(&canonical, "").await.unwrap();
        assert_eq!(page.directories, ["Music"]);
        assert!(inspect_directory(&canonical, "link").await.is_err());
        assert!(inspect_directory(&canonical, "../elsewhere").await.is_err());
        tokio::fs::remove_dir_all(base).await.unwrap();
    }

    #[test]
    fn mount_options_and_credentials_are_never_returned() {
        let info = "12 1 0:32 / /media/a\\040b ro,nosuid - cifs //example/share ro,username=private\n13 1 8:0 / /data rw - ext4 /dev/sda1 rw";
        let list = linux_mounts(info);
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].path, "/media/a b");
        assert!(list[0].read_only);
        assert!(!serde_json::to_string(&list).unwrap().contains("private"));
    }
}

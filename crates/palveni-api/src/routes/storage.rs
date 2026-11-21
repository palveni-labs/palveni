use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};

use axum::{Json, extract::Query};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{ApiError, auth::AdminUser};

const DIRECTORY_LIMIT: usize = 200;

pub use palveni_core::storage::{MountProtocol, MountedDirectory};

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

async fn mounted() -> Result<Vec<MountedDirectory>, ApiError> {
    Ok(palveni_library::mounts::snapshot()
        .await
        .map_err(ApiError::internal)?
        .network_directories())
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
}

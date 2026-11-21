//! Visible filesystems used for directory selection and network mount continuity.
use palveni_core::storage::{MountProtocol, MountedDirectory};
use std::{collections::BTreeMap, io, path::Path};

struct Filesystem {
    kind: String,
    source: String,
    root: String,
    read_only: bool,
}

impl Filesystem {
    fn identity(&self, mount: &str) -> String {
        format!("{}\0{}\0{}\0{}", self.kind, self.source, self.root, mount)
    }
}

#[derive(Default)]
pub struct MountTable(BTreeMap<String, Filesystem>);

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

impl MountTable {
    pub fn network_directories(&self) -> Vec<MountedDirectory> {
        self.0
            .iter()
            .filter_map(|(path, fs)| {
                Some(MountedDirectory {
                    path: path.clone(),
                    protocol: protocol(&fs.kind)?,
                    read_only: fs.read_only,
                })
            })
            .collect()
    }

    pub(crate) fn covering_identity(&self, path: &str) -> String {
        Path::new(path)
            .ancestors()
            .find_map(|p| self.0.get_key_value(p.to_str()?))
            .map(|(mount, filesystem)| filesystem.identity(mount))
            .unwrap_or_default()
    }

    pub(crate) fn sources(&self, roots: &[String]) -> BTreeMap<String, String> {
        let mut out = BTreeMap::new();
        for root in roots {
            let root = Path::new(root);
            // A library may start below a mount point, or contain several independent mounts.
            let covering = root
                .ancestors()
                .find_map(|p| self.0.get_key_value(p.to_str()?));
            let children = self
                .0
                .iter()
                .filter(|(p, _)| Path::new(p).starts_with(root));
            for (mount, fs) in covering.into_iter().chain(children) {
                if protocol(&fs.kind).is_some() {
                    let scope = if root.starts_with(mount) {
                        root
                    } else {
                        Path::new(mount)
                    };
                    out.insert(scope.to_string_lossy().into_owned(), fs.identity(mount));
                }
            }
        }
        out
    }
}

#[cfg(any(target_os = "linux", test))]
pub(crate) fn linux_mounts(info: &str) -> MountTable {
    fn unescape(raw: &str) -> String {
        raw.replace("\\040", " ")
            .replace("\\011", "\t")
            .replace("\\012", "\n")
            .replace("\\134", "\\")
    }
    let mut table = MountTable::default();
    for line in info.lines() {
        let Some((left, right)) = line.split_once(" - ") else {
            continue;
        };
        let mut fields = left.split_whitespace().skip(3);
        let (Some(root), Some(path), Some(options)) = (fields.next(), fields.next(), fields.next())
        else {
            continue;
        };
        let mut device = right.split_whitespace();
        let (Some(kind), Some(source)) = (device.next(), device.next()) else {
            continue;
        };
        let path = unescape(path);
        if Path::new(&path).is_absolute() {
            table.0.insert(
                path,
                Filesystem {
                    kind: kind.to_owned(),
                    source: source.to_owned(),
                    root: root.to_owned(),
                    read_only: options.split(',').any(|o| o == "ro"),
                },
            );
        }
    }
    table
}

pub async fn snapshot() -> io::Result<MountTable> {
    #[cfg(target_os = "linux")]
    {
        let bytes = tokio::fs::read("/proc/self/mountinfo").await?;
        let table = linux_mounts(&String::from_utf8_lossy(&bytes));
        if !table.0.contains_key("/") {
            return Err(io::Error::other("incomplete mount table"));
        }
        return Ok(table);
    }
    #[cfg(target_os = "macos")]
    {
        let output = tokio::process::Command::new("/sbin/mount").output().await?;
        if !output.status.success() {
            return Err(io::Error::other("cannot read mount table"));
        }
        let mut table = MountTable::default();
        for line in String::from_utf8_lossy(&output.stdout).lines() {
            let Some((source, tail)) = line.split_once(" on ") else {
                continue;
            };
            let Some((path, options)) = tail.rsplit_once(" (") else {
                continue;
            };
            let mut options = options.trim_end_matches(')').split(',').map(str::trim);
            let Some(kind) = options.next() else { continue };
            table.0.insert(
                path.into(),
                Filesystem {
                    kind: kind.into(),
                    source: source.into(),
                    root: "/".into(),
                    read_only: options.any(|option| option == "read-only"),
                },
            );
        }
        if !table.0.contains_key("/") {
            return Err(io::Error::other("incomplete mount table"));
        }
        return Ok(table);
    }
    #[allow(unreachable_code)]
    Ok(MountTable::default())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn public_list_and_scan_roots_omit_credentials_and_distinguish_local_submounts() {
        let info = r"12 1 0:32 / /media/a\040b ro,nosuid - cifs //nas/share ro,username=test-private
13 1 8:0 / /media/a\040b/offline rw - ext4 /dev/sda1 rw
14 1 8:1 / /data rw - ext4 /dev/sda2 rw";
        let mounts = linux_mounts(info);
        let list = mounts.network_directories();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].path, "/media/a b");
        assert!(list[0].read_only);
        let public = serde_json::to_string(&list).unwrap();
        assert!(!public.contains("test-private") && !public.contains("//nas/share"));
        assert!(
            mounts
                .sources(&["/media/a b/offline/music".into()])
                .is_empty()
        );
        assert!(
            mounts
                .sources(&["/media/a b/online".into()])
                .contains_key("/media/a b/online")
        );
        assert!(
            mounts
                .sources(&["/media".into()])
                .contains_key("/media/a b")
        );
    }
}

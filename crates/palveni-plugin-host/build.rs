use std::error::Error;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use sha2::{Digest, Sha256};

fn collect(root: &Path, dir: &Path, names: &mut Vec<PathBuf>) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let kind = entry.file_type()?;
        if kind.is_dir() {
            collect(root, &path, names)?;
        } else if kind.is_file() {
            names.push(path.strip_prefix(root).map_err(io::Error::other)?.into());
        } else {
            return Err(io::Error::other(format!(
                "not a regular file: {}",
                path.display()
            )));
        }
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let sources = Path::new("../../plugins");
    let out = PathBuf::from(std::env::var_os("OUT_DIR").ok_or("missing OUT_DIR")?);
    let packages = out.join("skin-packages");
    fs::create_dir_all(&packages)?;
    let mut index = Vec::new();
    for id in ["native", "ink", "gear"] {
        let dir = sources.join(format!("skin-{id}"));
        println!("cargo:rerun-if-changed={}", dir.display());
        let mut names = Vec::new();
        collect(&dir, &dir, &mut names)?;
        names.sort();
        let mut tar = tar::Builder::new(Vec::new());
        let mut preview = None;
        for name in names {
            let bytes = fs::read(dir.join(&name))?;
            if name == Path::new("preview.png") {
                preview = Some(json!({
                    "offset": tar.get_ref().len() + 512,
                    "len": bytes.len(),
                    "digest": hex::encode(Sha256::digest(&bytes))
                }));
            }
            let mut header = tar::Header::new_ustar();
            header.set_entry_type(tar::EntryType::Regular);
            header.set_mode(0o644);
            header.set_uid(0);
            header.set_gid(0);
            header.set_mtime(0);
            header.set_size(bytes.len() as u64);
            tar.append_data(&mut header, name, &bytes[..])?;
        }
        let bytes = tar.into_inner()?;
        let manifest: Value = serde_json::from_slice(&fs::read(dir.join("skin.json"))?)?;
        let plugin: toml::Value = toml::from_str(&fs::read_to_string(dir.join("plugin.toml"))?)?;
        let version = plugin["plugin"]["version"]
            .as_str()
            .ok_or("missing version")?;
        let filename = format!("skin-{id}-{version}.tar");
        fs::write(packages.join(&filename), &bytes)?;
        index.push(json!({
            "id": id, "pluginId": plugin["plugin"]["id"].as_str().ok_or("missing ID")?,
            "name": manifest["name"], "description": manifest["description"], "version": version,
            "sizeBytes": bytes.len(), "previewUrl": format!("/api/v1/skins/registry/{id}/preview"),
            "publishedAt": "2026-11-21T00:00:00Z", "sha256": hex::encode(Sha256::digest(&bytes)),
            "downloadUrl": format!("https://github.com/palveni-labs/plugins/releases/download/skins-v{version}/{filename}"),
            "filename": filename, "preview": preview.ok_or("missing official preview")?
        }));
    }
    fs::write(out.join("skin-registry.json"), serde_json::to_vec(&index)?)?;
    Ok(())
}

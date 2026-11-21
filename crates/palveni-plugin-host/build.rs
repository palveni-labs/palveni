use std::collections::HashSet;
use std::error::Error;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const CATALOG: &str = include_str!("catalog.json");

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
    println!("cargo:rerun-if-changed=catalog.json");
    let catalog: Value = serde_json::from_str(CATALOG)?;
    if catalog["schema"] != 1 {
        return Err("unsupported catalogue format".into());
    }
    let mut skin_ids = HashSet::new();
    let mut plugin_ids = HashSet::new();
    for entry in catalog["plugins"]
        .as_array()
        .ok_or("missing catalogue entries")?
    {
        let id = entry["skinId"].as_str().ok_or("missing skin identifier")?;
        if !(2..=63).contains(&id.len())
            || !id
                .bytes()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-' || c == b'.')
            || id.starts_with(['.', '-'])
            || id.contains("..")
            || !skin_ids.insert(id)
        {
            return Err(format!("invalid or duplicate catalogue skin identifier: {id}").into());
        }
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
        let plugin_id = plugin["plugin"]["id"].as_str().ok_or("missing plugin ID")?;
        let digest = hex::encode(Sha256::digest(&bytes));
        let releases = entry["releases"].as_array().ok_or("missing releases")?;
        let mut matching = releases
            .iter()
            .filter(|release| release["version"] == version);
        let release = matching
            .next()
            .ok_or("bundled release missing from catalogue")?;
        if matching.next().is_some() || !release["yanked"].is_null() {
            return Err(format!("duplicate or withdrawn bundled release: {id} {version}").into());
        }
        let url = release["url"].as_str().ok_or("missing download URL")?;
        let revision = url
            .strip_prefix("https://raw.githubusercontent.com/palveni-labs/plugins/")
            .and_then(|path| path.strip_suffix(&format!("/packages/{filename}")))
            .ok_or("unexpected catalogue package URL")?;
        if revision.len() != 40 || !revision.bytes().all(|c| c.is_ascii_hexdigit()) {
            return Err("package must point to an immutable Git revision".into());
        }
        if plugin["plugin"]["runtime"].as_str() != Some("none")
            || plugin["provides"]["skin"].as_str() != Some("skin.json")
            || manifest["id"] != id
            || entry["id"] != plugin_id
            || !plugin_ids.insert(plugin_id.to_owned())
            || entry["name"] != manifest["name"]["en"]
            || entry["description"] != manifest["description"]["en"]
            || release["runtime"] != "none"
            || release["permissions"] != json!({})
            || release["api"].as_str() != plugin["compat"]["api"].as_str()
            || release["size"] != bytes.len()
            || release["sha256"] != digest
        {
            return Err(
                format!("offline package does not match the reviewed catalogue: {id}").into(),
            );
        }
        fs::write(packages.join(&filename), &bytes)?;
        index.push(json!({
            "id": id, "pluginId": plugin_id,
            "name": manifest["name"], "description": manifest["description"], "version": version,
            "sizeBytes": bytes.len(), "previewUrl": format!("/api/v1/skins/registry/{id}/preview"),
            "publishedAt": release["publishedAt"].as_str().ok_or("missing publication date")?, "sha256": digest,
            "downloadUrl": url,
            "filename": filename, "preview": preview.ok_or("missing official preview")?
        }));
    }
    fs::write(out.join("skin-registry.json"), serde_json::to_vec(&index)?)?;
    Ok(())
}

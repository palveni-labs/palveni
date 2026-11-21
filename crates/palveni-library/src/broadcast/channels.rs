use super::{label, public_media_url, xml};
use palveni_core::broadcast::{ImportedEntry, MAX_BROADCAST_ENTRIES};
use reqwest::Url;
use std::collections::HashMap;

#[derive(Default)]
struct Channel {
    title: String,
    group: String,
    guide_id: Option<String>,
    guide_name: Option<String>,
}

#[derive(Default)]
struct Catalogue {
    records: Vec<(ImportedEntry, Option<String>, Option<String>)>,
    indexes: HashMap<String, usize>,
}

impl Catalogue {
    fn add(&mut self, info: &Channel, url: &str, base: Option<&Url>) -> Result<(), &'static str> {
        let Some(url) = public_media_url(url, base) else {
            return Ok(());
        };
        let title = if info.title.trim().is_empty() {
            label(url.host_str().unwrap_or_default())
        } else {
            label(&info.title)
        };
        let group = label(&info.group);
        let key = format!("{}\0{}", group.to_lowercase(), title.to_lowercase());
        let url = url.to_string();
        if let Some(index) = self.indexes.get(&key) {
            let (row, guide_id, guide_name) = &mut self.records[*index];
            if guide_id.is_none() {
                guide_id.clone_from(&info.guide_id);
            }
            if guide_name.is_none() {
                guide_name.clone_from(&info.guide_name);
            }
            if !row.streams.contains(&url) {
                if row.streams.len() >= 12 {
                    return Err("A channel exceeds 12 alternate stream URLs");
                }
                row.streams.push(url);
            }
        } else {
            if self.records.len() >= MAX_BROADCAST_ENTRIES {
                return Err("Too many channels");
            }
            self.indexes.insert(key.clone(), self.records.len());
            self.records.push((
                ImportedEntry {
                    key,
                    title,
                    group: (!group.is_empty()).then_some(group),
                    streams: vec![url],
                    published_at: None,
                    programmes: vec![],
                },
                info.guide_id.clone(),
                info.guide_name.clone(),
            ));
        }
        Ok(())
    }
}
fn split_info(line: &str) -> (&str, &str) {
    let mut quote = None;
    for (i, c) in line.char_indices() {
        match c {
            '\'' | '"' if quote.is_none() => quote = Some(c),
            c if quote == Some(c) => quote = None,
            ',' if quote.is_none() => return (&line[..i], line[i + 1..].trim()),
            _ => {}
        }
    }
    (line, "")
}

fn attributes(info: &str) -> HashMap<String, String> {
    let mut attrs = HashMap::new();
    let Some((_, mut input)) = info.split_once(char::is_whitespace) else {
        return attrs;
    };
    loop {
        input = input.trim_start();
        let Some((key, rest)) = input.split_once('=') else {
            break;
        };
        if key.contains(char::is_whitespace) {
            input = input.split_once(char::is_whitespace).map_or("", |(_, s)| s);
            continue;
        }
        let rest = rest.trim_start();
        let (value, tail) = match rest.chars().next() {
            Some(q @ ('\'' | '"')) => {
                let Some((v, t)) = rest[1..].split_once(q) else {
                    break;
                };
                (v, t)
            }
            Some(_) => rest.split_once(char::is_whitespace).unwrap_or((rest, "")),
            None => break,
        };
        attrs.insert(key.to_ascii_lowercase(), label(value));
        input = tail;
    }
    attrs
}

pub(super) fn parse(
    content: &str,
    guide: Option<&str>,
    base: Option<&Url>,
) -> Result<Vec<ImportedEntry>, &'static str> {
    let mut result = Catalogue::default();
    let mut group = String::new();
    let mut pending = None;
    for raw in content.lines() {
        let line = raw.trim();
        if line.starts_with("#EXT-X-") {
            return Err("An HLS media stream is not a channel catalogue");
        }
        if let Some(info) = line.strip_prefix("#EXTINF:") {
            let (metadata, title) = split_info(info);
            let mut attrs = attributes(metadata);
            pending = Some(Channel {
                title: if title.is_empty() {
                    attrs.get("tvg-name").cloned().unwrap_or_default()
                } else {
                    label(title)
                },
                group: attrs.remove("group-title").unwrap_or_else(|| group.clone()),
                guide_id: attrs.remove("tvg-id"),
                guide_name: attrs.remove("tvg-name"),
            });
        } else if let Some(value) = line.strip_prefix("#EXTGRP:") {
            if let Some(channel) = pending.as_mut() {
                channel.group = label(value);
            } else {
                group = label(value);
            }
        } else if line.is_empty() || line.starts_with('#') {
            continue;
        } else if let Some(channel) = pending.take() {
            result.add(&channel, line, base)?;
        } else if line.starts_with("http://") || line.starts_with("https://") {
            result.add(
                &Channel {
                    group: group.clone(),
                    ..Default::default()
                },
                line,
                base,
            )?;
        } else if let Some((title, urls)) = line.split_once([',', '，']) {
            if urls.trim().eq_ignore_ascii_case("#genre#") {
                group = label(title);
                continue;
            }
            let info = Channel {
                title: label(title),
                group: group.clone(),
                ..Default::default()
            };
            for url in urls.split('#') {
                let url = url.trim().split_once('$').map_or(url.trim(), |(u, _)| u);
                result.add(&info, url, base)?;
            }
        }
    }
    if let Some(guide) = guide {
        let guide = xml::guide(guide)?;
        let mut guide_bytes = 0usize;
        for (row, guide_id, guide_name) in &mut result.records {
            if let Some(programmes) = guide
                .for_channel(
                    guide_id.as_deref(),
                    guide_name.as_deref().unwrap_or(&row.title),
                )
                .or_else(|| guide.for_channel(None, &row.title))
            {
                guide_bytes += programmes.iter().map(|p| p.title.len() + 96).sum::<usize>();
                if guide_bytes > 8 * 1024 * 1024 {
                    return Err("Combined programme guide exceeds the import limit");
                }
                row.programmes = programmes.to_vec();
            }
        }
    }
    Ok(result.records.into_iter().map(|(r, _, _)| r).collect())
}

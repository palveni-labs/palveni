use std::collections::{HashMap, HashSet};

use chrono::{DateTime, NaiveDateTime, TimeDelta, Utc};
use palveni_core::broadcast::{BroadcastProgramme, ImportedEntry, MAX_BROADCAST_ENTRIES};
use quick_xml::{
    Reader, XmlVersion,
    events::{BytesStart, Event},
};
use reqwest::Url;

use super::{label, public_media_url};

const XML_ERROR: &str = "Invalid or unsupported XML document";
#[derive(Default)]
struct Node {
    name: String,
    attrs: HashMap<String, String>,
    text: String,
    children: Vec<Node>,
}
impl Node {
    fn from_start(tag: &BytesStart<'_>) -> Result<Self, &'static str> {
        let mut node = Self {
            name: tag.local_name().into_inner().to_ascii_lowercase(),
            ..Self::default()
        };
        for a in tag.attributes() {
            let a = a.map_err(|_| XML_ERROR)?;
            let attr = a.key.into_inner().to_ascii_lowercase();
            let value = a
                .normalized_value(XmlVersion::Implicit1_0)
                .map_err(|_| XML_ERROR)?;
            node.attrs.insert(attr, value.into_owned());
            if node.attrs.len() > 64 {
                return Err(XML_ERROR);
            }
        }
        Ok(node)
    }
    fn child(&self, name: &str) -> Option<&Self> {
        self.children.iter().find(|n| n.name == name)
    }
    fn field(&self, name: &str) -> Option<&str> {
        self.child(name)
            .map(|n| n.text.trim())
            .filter(|s| !s.is_empty())
    }
    fn attr(&self, name: &str) -> Option<&str> {
        self.attrs.get(name).map(String::as_str)
    }
}

fn document(content: &str, xmltv: bool) -> Result<Node, &'static str> {
    let mut reader = Reader::from_str(content.trim_start_matches('\u{feff}'));
    reader.config_mut().expand_empty_elements = true;
    let mut stack = vec![Node::default()];
    let mut count = 0u32;
    let mut declared = false;
    loop {
        let top = stack.last_mut().ok_or(XML_ERROR)?;
        match reader.read_event().map_err(|_| XML_ERROR)? {
            Event::Start(tag) => {
                count += 1;
                if count > 50_000 || stack.len() > 48 {
                    return Err(XML_ERROR);
                }
                stack.push(Node::from_start(&tag)?);
            }
            Event::End(_) => {
                let node = stack.pop().ok_or(XML_ERROR)?;
                stack.last_mut().ok_or(XML_ERROR)?.children.push(node);
            }
            Event::Text(text) => top.text.push_str(&text),
            Event::CData(text) => top.text.push_str(&text),
            Event::GeneralRef(reference) => {
                if let Some(c) = reference.resolve_char_ref().map_err(|_| XML_ERROR)? {
                    top.text.push(c);
                } else {
                    top.text.push_str(match &*reference {
                        "amp" => "&",
                        "lt" => "<",
                        "gt" => ">",
                        "quot" => "\"",
                        "apos" => "'",
                        _ => return Err(XML_ERROR),
                    });
                }
            }
            Event::DocType(declaration) => {
                // XMLTV exporters commonly name this DTD. Never load it or accept an internal subset.
                let mut fields = declaration.split_ascii_whitespace();
                if !xmltv
                    || declared
                    || count != 0
                    || !matches!(
                        (fields.next(), fields.next(), fields.next(), fields.next()),
                        (
                            Some("tv"),
                            Some("SYSTEM"),
                            Some("\"xmltv.dtd\"" | "'xmltv.dtd'"),
                            None
                        )
                    )
                {
                    return Err("Unsupported XML document type");
                }
                declared = true;
            }
            Event::Eof => break,
            _ => {}
        }
    }
    if stack.len() != 1 {
        return Err(XML_ERROR);
    }
    let mut root = stack.pop().ok_or(XML_ERROR)?;
    if root.children.len() != 1 || !root.text.trim().is_empty() {
        return Err(XML_ERROR);
    }
    root.children.pop().ok_or(XML_ERROR)
}

pub(super) fn episodes(
    content: &str,
    base: Option<&Url>,
) -> Result<Vec<ImportedEntry>, &'static str> {
    let root = document(content, false)?;
    let (container, entry_tag) = match root.name.as_str() {
        "rss" => (
            root.child("channel").ok_or("RSS channel is missing")?,
            "item",
        ),
        "feed" => (&root, "entry"),
        _ => return Err("A podcast source must be an RSS or Atom feed"),
    };
    let mut seen = HashSet::new();
    let mut result = Vec::new();
    for item in container.children.iter().filter(|n| n.name == entry_tag) {
        let mut streams = Vec::new();
        for link in &item.children {
            let target = match link.name.as_str() {
                "enclosure" => link.attr("url"),
                "link" if link.attr("rel") == Some("enclosure") => link.attr("href"),
                _ => None,
            };
            if let Some(mime) = link.attr("type")
                && !(mime.starts_with("audio/")
                    || mime.starts_with("video/")
                    || mime == "application/octet-stream")
            {
                continue;
            }
            if let Some(u) = target.and_then(|s| public_media_url(s, base)) {
                let url = u.to_string();
                if !streams.contains(&url) && streams.len() < 12 {
                    streams.push(url);
                }
            }
        }
        let Some(first) = streams.first() else {
            continue;
        };
        let key = item
            .field("guid")
            .or_else(|| item.field("id"))
            .unwrap_or(first)
            .to_owned();
        if key.len() > 4096 {
            return Err("Podcast episode identifier is too long");
        }
        if !seen.insert(key.clone()) {
            continue;
        }
        if result.len() >= MAX_BROADCAST_ENTRIES {
            return Err("Too many podcast episodes");
        }
        let title = item
            .field("title")
            .map(label)
            .unwrap_or_else(|| label(&key));
        let published_at = item
            .field("pubdate")
            .or_else(|| item.field("published"))
            .or_else(|| item.field("updated"))
            .and_then(|s| {
                DateTime::parse_from_rfc2822(s)
                    .or_else(|_| DateTime::parse_from_rfc3339(s))
                    .ok()
            })
            .map(|d| d.to_utc());
        result.push(ImportedEntry {
            key,
            title,
            group: None,
            streams,
            published_at,
            programmes: vec![],
        });
    }
    Ok(result)
}

fn xmltv_time(value: &str) -> Option<DateTime<Utc>> {
    if let Ok(time) = DateTime::parse_from_str(value.trim(), "%Y%m%d%H%M%S %z") {
        return Some(time.to_utc());
    }
    NaiveDateTime::parse_from_str(value.trim(), "%Y%m%d%H%M%S")
        .ok()
        .map(|dt| dt.and_utc())
}

pub(super) struct Guide {
    programmes: HashMap<String, Vec<BroadcastProgramme>>,
    names: HashMap<String, Option<String>>,
}
impl Guide {
    pub(super) fn for_channel(
        &self,
        id: Option<&str>,
        title: &str,
    ) -> Option<&[BroadcastProgramme]> {
        let by_name = self
            .names
            .get(&title.to_lowercase())
            .and_then(Option::as_deref);
        id.and_then(|k| self.programmes.get(k))
            .or_else(|| by_name.and_then(|k| self.programmes.get(k)))
            .map(Vec::as_slice)
    }
}

pub(super) fn guide(content: &str) -> Result<Guide, &'static str> {
    let tv = document(content, true)?;
    if tv.name != "tv" {
        return Err("Programme guide must use XMLTV");
    }
    let mut result = Guide {
        programmes: HashMap::new(),
        names: HashMap::new(),
    };
    for node in tv.children {
        match node.name.as_str() {
            "channel" => {
                if let Some(id) = node.attr("id") {
                    for name in node.children.iter().filter(|n| n.name == "display-name") {
                        let key = label(&name.text).to_lowercase();
                        result
                            .names
                            .entry(key)
                            .and_modify(|entry| {
                                if entry.as_deref() != Some(id) {
                                    *entry = None;
                                }
                            })
                            .or_insert_with(|| Some(id.to_owned()));
                    }
                }
            }
            "programme" => {
                let (Some(channel), Some(start), Some(title)) = (
                    node.attr("channel"),
                    node.attr("start").and_then(xmltv_time),
                    node.field("title"),
                ) else {
                    continue;
                };
                let fallback = start.checked_add_signed(TimeDelta::hours(1));
                let Some(end) = node.attr("stop").and_then(xmltv_time).or(fallback) else {
                    continue;
                };
                if end <= start {
                    continue;
                }
                result
                    .programmes
                    .entry(channel.to_owned())
                    .or_default()
                    .push(BroadcastProgramme {
                        title: label(title),
                        start,
                        end,
                    });
            }
            _ => {}
        }
    }
    for list in result.programmes.values_mut() {
        list.sort_by_key(|p| p.start);
        list.dedup_by(|a, b| a.start == b.start && a.title == b.title);
        if list.len() > 512 {
            return Err("A channel exceeds 512 scheduled programmes");
        }
    }
    Ok(result)
}

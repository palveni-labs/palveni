use super::*;
use quick_xml::{Reader, events::Event};

const PROPERTIES: &str = r#"<?xml version="1.0" encoding="utf-8"?><d:propfind xmlns:d="DAV:"><d:prop><d:displayname/><d:resourcetype/><d:getcontenttype/></d:prop></d:propfind>"#;

fn relative(value: &str) -> Result<Vec<&str>> {
    let parts: Vec<_> = value.split('/').collect();
    if value.len() > 4096 || parts.len() > 64 || !parts.iter().all(|p| safe_component(p)) {
        return Err(RemoteError::Invalid);
    }
    Ok(parts)
}

pub(super) fn file(source: &Source<'_>, id: &str) -> Result<Url> {
    let (kind, path) = id.split_once(':').ok_or(RemoteError::Invalid)?;
    if !matches!(kind, "audio" | "video") {
        return Err(RemoteError::Invalid);
    }
    source.endpoint(&relative(path)?)
}

pub(super) async fn entries(
    source: &Source<'_>,
    parent: &str,
    offset: u32,
    limit: u32,
) -> Result<ConnectionPage> {
    let mut path = if parent.is_empty() {
        vec![]
    } else {
        relative(parent.strip_prefix("folder:").ok_or(RemoteError::Invalid)?)?
    };
    path.push("");
    let url = source.endpoint(&path)?;
    let method = Method::from_bytes(b"PROPFIND").map_err(|_| RemoteError::Invalid)?;
    let req = source
        .request(method, url.clone())?
        .header("Depth", "1")
        .header(header::CONTENT_TYPE, "application/xml; charset=utf-8")
        .body(PROPERTIES);
    let response = source.send(req).await?;
    if response.status() != StatusCode::MULTI_STATUS {
        return Err(RemoteError::Unavailable);
    }
    let bytes = small_body(response).await?;
    let base = source.address.clone();
    tokio::task::spawn_blocking(move || {
        let xml = xml_text(&bytes)?;
        let mut entries = parse(&xml, &base, &url)?;
        entries.sort_unstable_by(|a, b| {
            (a.kind != Kind::Folder)
                .cmp(&(b.kind != Kind::Folder))
                .then_with(|| a.id.cmp(&b.id))
        });
        entries.dedup_by(|a, b| a.id == b.id);
        Ok(page(entries, offset, limit))
    })
    .await
    .map_err(|_| RemoteError::Unavailable)?
}

fn xml_text(bytes: &[u8]) -> Result<std::borrow::Cow<'_, str>> {
    if bytes.starts_with(&[0xff, 0xfe]) || bytes.starts_with(&[0xfe, 0xff]) {
        let (pairs, remainder) = bytes[2..].as_chunks::<2>();
        if !remainder.is_empty() {
            return Err(RemoteError::Unavailable);
        }
        let little = bytes[0] == 0xff;
        let units = pairs.iter().copied().map(|b| {
            if little {
                u16::from_le_bytes(b)
            } else {
                u16::from_be_bytes(b)
            }
        });
        return char::decode_utf16(units)
            .collect::<std::result::Result<String, _>>()
            .map(Into::into)
            .map_err(|_| RemoteError::Unavailable);
    }
    std::str::from_utf8(bytes)
        .map(|s| s.trim_start_matches('\u{feff}').into())
        .map_err(|_| RemoteError::Unavailable)
}

#[derive(Default)]
struct Props {
    name: String,
    mime: String,
    folder: bool,
    status: String,
}
#[derive(Default)]
struct Resource {
    href: String,
    props: Props,
    found: bool,
    status: String,
}

fn success(status: &str) -> bool {
    status
        .split_whitespace()
        .nth(1)
        .and_then(|v| v.parse::<u16>().ok())
        .is_some_and(|n| (200..300).contains(&n))
}

fn entry(resource: Resource, base: &Url, parent: &Url) -> Option<ConnectionEntry> {
    if !resource.found || (!resource.status.is_empty() && !success(&resource.status)) {
        return None;
    }
    let url = scoped_href(base, parent, resource.href.trim())?;
    let root = components(base)?;
    let folder = components(parent)?;
    let child = components(&url)?;
    if child.len() != folder.len() + 1 || !child.starts_with(&folder) {
        return None;
    }
    let leaf = child.last()?;
    let name = named(&resource.props.name).or_else(|| named(leaf))?;
    let kind = if resource.props.folder {
        Kind::Folder
    } else {
        file_kind(leaf, resource.props.mime.trim())
    };
    let prefix = match kind {
        Kind::Folder => "folder",
        Kind::Audio => "audio",
        Kind::Video => "video",
        Kind::File => "file",
    };
    Some(ConnectionEntry {
        id: format!("{prefix}:{}", child[root.len()..].join("/")),
        name,
        kind,
    })
}

fn parse(xml: &str, base: &Url, parent: &Url) -> Result<Vec<ConnectionEntry>> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().expand_empty_elements = true;
    let mut tags = Vec::<String>::new();
    let mut resource = Resource::default();
    let mut props = Props::default();
    let mut out = Vec::new();
    let mut responses = 0;
    let mut root_seen = false;
    let mut text = String::new();
    loop {
        match reader.read_event().map_err(|_| RemoteError::Unavailable)? {
            Event::Start(e) => {
                if tags.len() >= 48 {
                    return Err(RemoteError::Unavailable);
                }
                let name = e.local_name().as_ref().to_string();
                if tags.is_empty() {
                    if root_seen || name != "multistatus" {
                        return Err(RemoteError::Unavailable);
                    }
                    root_seen = true;
                }
                tags.push(name);
                text.clear();
                match tags.as_slice() {
                    [_, r] if r == "response" => {
                        responses += 1;
                        if responses > MAX_ROWS {
                            return Err(RemoteError::Unavailable);
                        }
                        resource = Resource::default();
                    }
                    [_, r, p] if r == "response" && p == "propstat" => props = Props::default(),
                    [_, r, p, group, ty, c]
                        if r == "response"
                            && p == "propstat"
                            && group == "prop"
                            && ty == "resourcetype"
                            && c == "collection" =>
                    {
                        props.folder = true
                    }
                    _ => {}
                }
            }
            Event::End(_) => {
                match tags.as_slice() {
                    [_, r, k] if r == "response" && k == "href" => {
                        resource.href = text.trim().to_owned()
                    }
                    [_, r, k] if r == "response" && k == "status" => {
                        resource.status = text.trim().to_owned()
                    }
                    [_, r, p, k] if r == "response" && p == "propstat" && k == "status" => {
                        props.status = text.trim().to_owned()
                    }
                    [_, r, p, g, k] if r == "response" && p == "propstat" && g == "prop" => {
                        match k.as_str() {
                            "displayname" => props.name = text.trim().to_owned(),
                            "getcontenttype" => props.mime = text.trim().to_owned(),
                            _ => {}
                        }
                    }
                    [_, r, p] if r == "response" && p == "propstat" && success(&props.status) => {
                        resource.found = true;
                        resource.props.folder |= props.folder;
                        if !props.name.is_empty() {
                            resource.props.name = std::mem::take(&mut props.name);
                        }
                        if !props.mime.is_empty() {
                            resource.props.mime = std::mem::take(&mut props.mime);
                        }
                    }
                    [_, r] if r == "response" => {
                        if (!resource.found
                            || (!resource.status.is_empty() && !success(&resource.status)))
                            && scoped_href(base, parent, resource.href.trim())
                                .is_some_and(|url| components(&url) == components(parent))
                        {
                            return Err(RemoteError::Unavailable);
                        }
                        if let Some(row) = entry(std::mem::take(&mut resource), base, parent) {
                            out.push(row);
                        }
                    }
                    _ => {}
                }
                tags.pop().ok_or(RemoteError::Unavailable)?;
                text.clear();
            }
            Event::Text(v) => text.push_str(&v.xml10_content()),
            Event::CData(v) => text.push_str(&v.xml10_content()),
            Event::GeneralRef(v) => {
                if let Some(c) = v.resolve_char_ref().map_err(|_| RemoteError::Unavailable)? {
                    text.push(c);
                } else {
                    text.push(match &*v {
                        "amp" => '&',
                        "lt" => '<',
                        "gt" => '>',
                        "apos" => '\'',
                        "quot" => '"',
                        _ => return Err(RemoteError::Unavailable),
                    });
                }
            }
            Event::DocType(_) => return Err(RemoteError::Unavailable),
            Event::Eof => {
                return if root_seen && tags.is_empty() {
                    Ok(out)
                } else {
                    Err(RemoteError::Unavailable)
                };
            }
            _ => {}
        }
    }
}

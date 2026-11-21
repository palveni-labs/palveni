mod media_servers;
#[cfg(test)]
mod tests;
mod webdav;

use std::{io, sync::LazyLock, time::Duration};

use futures_util::{StreamExt, TryStreamExt};
use palveni_core::connections::{
    ConnectionEntry, ConnectionEntryKind as Kind, ConnectionKind, ConnectionPage, Credentials,
};
use reqwest::{Client, Method, RequestBuilder, Response, StatusCode, Url, header};
use tokio::io::AsyncReadExt;
use tokio_util::io::StreamReader;

use super::{RemoteError, RemoteStream};

type Result<T> = std::result::Result<T, RemoteError>;
const LIMIT_BYTES: usize = 8 * 1024 * 1024;
const MAX_ROWS: usize = 20_000;

static CLIENT: LazyLock<Result<Client>> = LazyLock::new(|| {
    Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(5))
        .read_timeout(Duration::from_secs(25))
        .pool_max_idle_per_host(4)
        .build()
        .map_err(|_| RemoteError::Unavailable)
});

struct Source<'a> {
    kind: ConnectionKind,
    address: &'a Url,
    credentials: &'a Credentials,
}

impl Source<'_> {
    fn endpoint(&self, segments: &[&str]) -> Result<Url> {
        let mut url = self.address.clone();
        let mut path = url.path_segments_mut().map_err(|_| RemoteError::Invalid)?;
        path.pop_if_empty();
        for piece in segments {
            path.push(piece);
        }
        drop(path);
        Ok(url)
    }

    fn request(&self, method: Method, url: Url) -> Result<RequestBuilder> {
        if components(self.address).is_none() {
            return Err(RemoteError::Invalid);
        }
        let client = CLIENT.as_ref().map_err(|_| RemoteError::Unavailable)?;
        let mut req = client
            .request(method, url)
            .header(header::ACCEPT_ENCODING, "identity");
        if self.kind == ConnectionKind::Webdav {
            if !self.credentials.username.is_empty() || !self.credentials.secret.is_empty() {
                req = req.basic_auth(&self.credentials.username, Some(&self.credentials.secret));
            }
        } else {
            if self.credentials.secret.is_empty() {
                return Err(RemoteError::Invalid);
            }
            let mut token = header::HeaderValue::from_str(&self.credentials.secret)
                .map_err(|_| RemoteError::Invalid)?;
            token.set_sensitive(true);
            req = match self.kind {
                ConnectionKind::Plex => req
                    .header("X-Plex-Token", token)
                    .header("X-Plex-Client-Identifier", "palveni-server")
                    .header("X-Plex-Product", "Palveni"),
                ConnectionKind::Emby | ConnectionKind::Jellyfin => {
                    const CLIENT_INFO: &str = "MediaBrowser Client=\"Palveni\", Device=\"Palveni Server\", DeviceId=\"palveni-server\", Version=\"1\"";
                    if self.credentials.secret.contains(['"', '\\']) {
                        return Err(RemoteError::Invalid);
                    }
                    let mut auth = header::HeaderValue::from_str(&format!(
                        "{CLIENT_INFO}, Token=\"{}\"",
                        self.credentials.secret
                    ))
                    .map_err(|_| RemoteError::Invalid)?;
                    auth.set_sensitive(true);
                    req.header(header::AUTHORIZATION, auth)
                        .header("X-Emby-Token", token)
                        .header("X-Emby-Authorization", CLIENT_INFO)
                }
                _ => return Err(RemoteError::Invalid),
            };
        }
        Ok(req)
    }

    async fn send(&self, req: RequestBuilder) -> Result<Response> {
        let client = CLIENT.as_ref().map_err(|_| RemoteError::Unavailable)?;
        let template = req.build().map_err(|_| RemoteError::Invalid)?;
        let mut url = template.url().clone();
        for _ in 0..=3 {
            let mut request = template.try_clone().ok_or(RemoteError::Invalid)?;
            *request.url_mut() = url.clone();
            let response = client
                .execute(request)
                .await
                .map_err(|_| RemoteError::Unavailable)?;
            if !matches!(response.status().as_u16(), 301 | 302 | 303 | 307 | 308) {
                return Ok(response);
            }
            let reference = response
                .headers()
                .get(header::LOCATION)
                .and_then(|h| h.to_str().ok())
                .ok_or(RemoteError::Unavailable)?;
            url = scoped_href(self.address, &url, reference).ok_or(RemoteError::Unavailable)?;
        }
        Err(RemoteError::Unavailable)
    }
}

fn safe_component(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 255
        && !matches!(value, "." | "..")
        && !value
            .chars()
            .any(|c| c.is_control() || matches!(c, '/' | '\\'))
        && !value
            .as_bytes()
            .windows(3)
            .any(|w| w[0] == b'%' && w[1].is_ascii_hexdigit() && w[2].is_ascii_hexdigit())
}

fn decode(segment: &str) -> Option<String> {
    let mut input = segment.as_bytes().iter().copied();
    let mut out = Vec::with_capacity(segment.len());
    while let Some(b) = input.next() {
        out.push(if b == b'%' {
            let a = char::from(input.next()?).to_digit(16)?;
            let c = char::from(input.next()?).to_digit(16)?;
            u8::try_from(a * 16 + c).ok()?
        } else {
            b
        });
    }
    let value = String::from_utf8(out).ok()?;
    safe_component(&value).then_some(value)
}

fn components(url: &Url) -> Option<Vec<String>> {
    let path = url.path().strip_prefix('/')?;
    let path = path.strip_suffix('/').unwrap_or(path);
    if path.is_empty() {
        return Some(vec![]);
    }
    if path.len() > 4096 {
        return None;
    }
    let parts: Vec<_> = path.split('/').map(decode).collect::<Option<_>>()?;
    (parts.len() <= 64).then_some(parts)
}

fn scoped_href(base: &Url, from: &Url, reference: &str) -> Option<Url> {
    if reference.is_empty()
        || reference.len() > 4096
        || reference.contains('\\')
        || reference.chars().any(char::is_control)
    {
        return None;
    }
    let url = from.join(reference).ok()?;
    if url.origin() != base.origin()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || !components(&url)?.starts_with(&components(base)?)
    {
        return None;
    }
    Some(url)
}

async fn small_body(response: Response) -> Result<Vec<u8>> {
    if !response.status().is_success()
        || response
            .content_length()
            .is_some_and(|n| n > LIMIT_BYTES as u64)
    {
        return Err(RemoteError::Unavailable);
    }
    let mut stream = response.bytes_stream();
    let mut out = Vec::new();
    while let Some(part) = stream.next().await {
        let part = part.map_err(|_| RemoteError::Unavailable)?;
        if out.len().saturating_add(part.len()) > LIMIT_BYTES {
            return Err(RemoteError::Unavailable);
        }
        out.extend_from_slice(&part);
    }
    Ok(out)
}

fn page(entries: Vec<ConnectionEntry>, offset: u32, limit: u32) -> ConnectionPage {
    let end = offset.saturating_add(limit);
    let has_more = (end as usize) < entries.len();
    ConnectionPage {
        entries: entries
            .into_iter()
            .skip(offset as usize)
            .take(limit as usize)
            .collect(),
        next_offset: has_more.then_some(end),
    }
}

fn named(value: &str) -> Option<String> {
    let value = value.trim();
    (!value.is_empty() && value.chars().count() <= 512 && !value.chars().any(char::is_control))
        .then(|| value.to_owned())
}

fn file_kind(name: &str, mime: &str) -> Kind {
    use crate::classify::classify;
    use palveni_core::LibraryKind;
    if mime.starts_with("audio/")
        || classify(LibraryKind::Music, name).is_some()
        || classify(LibraryKind::Audiobook, name).is_some()
    {
        Kind::Audio
    } else if mime.starts_with("video/") || classify(LibraryKind::Video, name).is_some() {
        Kind::Video
    } else {
        Kind::File
    }
}

pub(super) async fn entries(
    kind: ConnectionKind,
    address: &Url,
    credentials: &Credentials,
    parent: &str,
    offset: u32,
    limit: u32,
) -> Result<ConnectionPage> {
    let source = Source {
        kind,
        address,
        credentials,
    };
    let limit = limit.clamp(1, 200);
    if kind == ConnectionKind::Webdav {
        webdav::entries(&source, parent, offset, limit).await
    } else {
        media_servers::entries(&source, parent, offset, limit).await
    }
}

#[derive(Clone, Copy)]
enum Range {
    Full,
    Start(u64, Option<u64>),
    Suffix(u64),
    Invalid,
}

impl Range {
    fn parse(value: Option<&str>) -> Self {
        let Some(s) = value else {
            return Self::Full;
        };
        let Some((a, b)) = s.strip_prefix("bytes=").and_then(|s| s.split_once('-')) else {
            return Self::Invalid;
        };
        fn num(s: &str) -> Option<u64> {
            (!s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()))
                .then(|| s.parse().ok())
                .flatten()
        }
        if s.len() <= 64 {
            match (a, b) {
                ("", b) => {
                    if let Some(n) = num(b).filter(|n| *n != 0) {
                        return Self::Suffix(n);
                    }
                }
                (a, "") => {
                    if let Some(n) = num(a) {
                        return Self::Start(n, None);
                    }
                }
                (a, b) => {
                    if let (Some(first), Some(last)) = (num(a), num(b))
                        && first <= last
                    {
                        return Self::Start(first, Some(last));
                    }
                }
            }
        }
        Self::Invalid
    }
    fn bounds(self, total: u64) -> Result<(u64, u64)> {
        match self {
            Self::Full => Ok((0, total)),
            Self::Start(n, end) if n < total => {
                Ok((n, end.map_or(total, |v| v.saturating_add(1).min(total))))
            }
            Self::Suffix(n) if total != 0 => Ok((total.saturating_sub(n), total)),
            _ => Err(RemoteError::Range { total }),
        }
    }
}

pub(super) async fn open(
    kind: ConnectionKind,
    address: &Url,
    credentials: &Credentials,
    id: &str,
    range: Option<&str>,
) -> Result<RemoteStream> {
    let source = Source {
        kind,
        address,
        credentials,
    };
    let url = if kind == ConnectionKind::Webdav {
        webdav::file(&source, id)?
    } else {
        media_servers::file(&source, id).await?
    };
    let mut request = source.request(Method::GET, url)?;
    let selection = Range::parse(range);
    if let Some(raw) = range {
        request = request.header(
            header::RANGE,
            if matches!(selection, Range::Invalid) {
                "bytes=0-0"
            } else {
                raw
            },
        );
    }
    let response = source.send(request).await?;
    let headers = response.headers();
    let content_range = headers
        .get(header::CONTENT_RANGE)
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.strip_prefix("bytes "));
    let (total, origin_start, origin_end) = match response.status() {
        StatusCode::RANGE_NOT_SATISFIABLE => {
            let reported = content_range
                .and_then(|s| s.strip_prefix("*/"))
                .and_then(|s| s.parse().ok());
            let total = match reported {
                Some(n) => n,
                None => {
                    // Some media servers omit the original size from their 416 response.
                    let head = source.request(Method::HEAD, response.url().clone())?;
                    let head = source.send(head).await?;
                    if head.status() != StatusCode::OK {
                        return Err(RemoteError::Unavailable);
                    }
                    head.headers()
                        .get(header::CONTENT_LENGTH)
                        .and_then(|v| v.to_str().ok())
                        .and_then(|n| n.parse().ok())
                        .ok_or(RemoteError::Unavailable)?
                }
            };
            return Err(RemoteError::Range { total });
        }
        StatusCode::PARTIAL_CONTENT => {
            let (span, total) = content_range
                .and_then(|s| s.split_once('/'))
                .ok_or(RemoteError::Unavailable)?;
            let (start, last) = span.split_once('-').ok_or(RemoteError::Unavailable)?;
            let number = |s: &str| s.parse::<u64>().map_err(|_| RemoteError::Unavailable);
            let (total, start, last) = (number(total)?, number(start)?, number(last)?);
            if last < start || last >= total {
                return Err(RemoteError::Unavailable);
            }
            (total, start, last + 1)
        }
        StatusCode::OK => {
            let size = response.content_length().ok_or(RemoteError::Unavailable)?;
            (size, 0, size)
        }
        _ => return Err(RemoteError::Unavailable),
    };
    let (start, end) = selection.bounds(total)?;
    if origin_start != start
        || origin_end < end
        || response
            .content_length()
            .is_some_and(|len| len != origin_end - origin_start)
        || headers
            .get(header::CONTENT_ENCODING)
            .is_some_and(|v| v != "identity")
    {
        return Err(RemoteError::Unavailable);
    }
    let declared = headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(';').next())
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    let leaf = response
        .url()
        .path_segments()
        .and_then(|mut p| p.next_back())
        .and_then(decode)
        .unwrap_or_default();
    let mime = if (declared.starts_with("audio/") || declared.starts_with("video/"))
        && declared.len() <= 100
        && declared
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-./+".contains(&b))
    {
        declared
    } else if matches!(
        declared.as_str(),
        "" | "application/octet-stream"
            | "binary/octet-stream"
            | "application/x-matroska"
            | "application/ogg"
    ) {
        match file_kind(&leaf, "") {
            Kind::Audio => crate::classify::audio_content_type(&leaf),
            Kind::Video => crate::local::video_content_type(&leaf),
            _ if kind != ConnectionKind::Webdav => "application/octet-stream",
            _ => return Err(RemoteError::Invalid),
        }
        .to_owned()
    } else {
        return Err(RemoteError::Unavailable);
    };
    let reader = StreamReader::new(
        response
            .bytes_stream()
            .map_err(|_| io::Error::other("remote media read failed")),
    );
    Ok(RemoteStream {
        reader: Box::pin(reader.take(end - start)),
        start,
        total,
        len: end - start,
        mime,
        partial: range.is_some(),
    })
}

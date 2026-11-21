//! Parse user-managed channel catalogues and podcast feeds into one import snapshot.
mod channels;
mod remote;
#[cfg(test)]
mod tests;
mod xml;

use palveni_core::broadcast::{
    BroadcastKind, ImportedEntry, MAX_BROADCAST_BYTES, MAX_BROADCAST_ENTRIES,
};
use reqwest::Url;

pub use remote::fetch_public;

pub fn public_media_url(input: &str, base: Option<&Url>) -> Option<Url> {
    let input = input.trim();
    if input.is_empty() || input.len() > 4096 || input.chars().any(|c| c.is_control() || c == '\\')
    {
        return None;
    }
    let url = Url::parse(input)
        .or_else(|err| base.map_or(Err(err), |base| base.join(input)))
        .ok()?;
    let has_secret = url.query_pairs().any(|(k, _)| {
        matches!(
            k.to_ascii_lowercase().as_str(),
            "password"
                | "passwd"
                | "token"
                | "access_token"
                | "auth_token"
                | "auth"
                | "key"
                | "pass"
                | "secret"
                | "credential"
                | "access_key"
                | "client_secret"
                | "api_key"
                | "apikey"
                | "authorization"
                | "signature"
                | "sig"
                | "x-amz-credential"
                | "x-amz-security-token"
                | "x-amz-signature"
        )
    });
    (matches!(url.scheme(), "http" | "https")
        && url.host().is_some()
        && url.username().is_empty()
        && url.password().is_none()
        && !has_secret
        && url.as_str().len() <= 4096)
        .then_some(url)
}

pub fn parse_catalog(
    kind: BroadcastKind,
    content: &str,
    guide: Option<&str>,
    base: Option<&Url>,
) -> Result<Vec<ImportedEntry>, &'static str> {
    if content.len() > MAX_BROADCAST_BYTES || guide.is_some_and(|s| s.len() > MAX_BROADCAST_BYTES) {
        return Err("The source or programme guide exceeds 1 MiB");
    }
    let content = content.trim_start_matches('\u{feff}').trim();
    let result = match kind {
        BroadcastKind::Podcast => {
            if guide.is_some() {
                return Err("Podcast sources do not accept a programme guide");
            }
            xml::episodes(content, base)?
        }
        BroadcastKind::Iptv | BroadcastKind::Radio => channels::parse(content, guide, base)?,
    };
    if result.is_empty() || result.len() > MAX_BROADCAST_ENTRIES {
        return Err("A source must contain 1–5000 playable entries");
    }
    Ok(result)
}

fn label(input: &str) -> String {
    input
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(256)
        .collect()
}

//! Safe `.strm` remote stream pointer parsing and SSRF validation.

use std::io::Read;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::path::Path;

use palveni_core::{Error, Result};
use palveni_db::music::TrackLocation;

pub const MAX_STRM_BYTES: u64 = 8 * 1024;
const MAX_URL_LEN: usize = 4096;

/// Returns true when the path has a `.strm` extension (case-insensitive).
pub fn is_strm_path(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("strm"))
}

/// Returns true when the filename or path string ends with `.strm`.
pub fn is_strm_name(name: &str) -> bool {
    is_strm_path(Path::new(name))
}

/// Reads a local `.strm` file inside its registered library root (bounded to 8 KiB)
/// and returns its validated `http://` or `https://` target URL.
pub async fn read_strm_location(location: TrackLocation) -> Result<String> {
    if !is_strm_path(&location.relative_path) {
        return Err(Error::invalid("Not a .strm file"));
    }
    let file = crate::local::open_audio_input(location).await?;
    tokio::task::spawn_blocking(move || read_strm_file(file))
        .await
        .map_err(|e| Error::internal(format!("blocking strm read failed: {e}")))?
}

/// Reads and validates a `.strm` target URL from an open file descriptor.
pub fn read_strm_file(file: std::fs::File) -> Result<String> {
    let meta = file.metadata()?;
    if meta.len() > MAX_STRM_BYTES {
        return Err(Error::invalid(".strm file exceeds maximum size of 8 KiB"));
    }
    let mut buf = Vec::with_capacity(usize::try_from(meta.len()).unwrap_or(0));
    file.take(MAX_STRM_BYTES + 1).read_to_end(&mut buf)?;
    if buf.len() as u64 > MAX_STRM_BYTES {
        return Err(Error::invalid(".strm file exceeds maximum size of 8 KiB"));
    }
    let text =
        std::str::from_utf8(&buf).map_err(|_| Error::invalid(".strm file must be valid UTF-8"))?;
    parse_and_validate_strm_content(text)
}

/// Extracts the first non-empty non-comment line from `.strm` text and validates its URL.
pub fn parse_and_validate_strm_content(raw: &str) -> Result<String> {
    let stripped = raw.strip_prefix('\u{feff}').unwrap_or(raw);
    let line = stripped
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with('#'))
        .ok_or_else(|| Error::invalid(".strm file does not contain a target URL"))?;
    validate_strm_url(line)
}

/// Validates that a `.strm` target URL uses strictly `http://` or `https://`
/// and does not target blocked SSRF endpoints (loopback, link-local, unspecified,
/// or cloud metadata hosts) while permitting normal public and LAN media servers.
pub fn validate_strm_url(raw: &str) -> Result<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.len() > MAX_URL_LEN {
        return Err(Error::invalid("Invalid .strm URL length"));
    }
    if trimmed.chars().any(char::is_control) {
        return Err(Error::invalid(".strm URL contains control characters"));
    }
    let parsed =
        reqwest::Url::parse(trimmed).map_err(|_| Error::invalid("Malformed URL in .strm file"))?;
    match parsed.scheme() {
        "http" | "https" => {}
        _ => {
            return Err(Error::invalid(
                ".strm URL must use http:// or https:// scheme",
            ));
        }
    }
    if parsed.host().is_none() {
        return Err(Error::invalid(".strm URL is missing a host"));
    }
    check_host(&parsed)?;
    Ok(parsed.into())
}

fn check_host(parsed: &reqwest::Url) -> Result<()> {
    let host_str = parsed
        .host_str()
        .ok_or_else(|| Error::invalid(".strm URL is missing a host"))?;
    let trimmed = host_str
        .trim_start_matches('[')
        .trim_end_matches(']')
        .trim_end_matches('.');
    if trimmed.is_empty() {
        return Err(Error::invalid(".strm URL is missing a host"));
    }
    if let Ok(ip) = trimmed.parse::<std::net::IpAddr>() {
        match ip {
            std::net::IpAddr::V4(v4) => {
                if is_blocked_ipv4(v4) {
                    return Err(Error::invalid(".strm URL targets a blocked address"));
                }
            }
            std::net::IpAddr::V6(v6) => {
                if is_blocked_ipv6(v6) {
                    return Err(Error::invalid(".strm URL targets a blocked address"));
                }
            }
        }
        return Ok(());
    }
    let lower = trimmed.to_ascii_lowercase();
    if lower == "localhost"
        || lower.ends_with(".localhost")
        || lower == "metadata"
        || lower == "metadata.google.internal"
        || lower == "metadata.goog"
        || lower == "metadata.azure.com"
        || lower == "instance-data"
    {
        return Err(Error::invalid(".strm URL targets a blocked host"));
    }
    Ok(())
}

fn is_blocked_ipv4(ip: Ipv4Addr) -> bool {
    let octets = ip.octets();
    ip.is_unspecified()
        || octets[0] == 0
        || ip.is_loopback()
        || ip.is_link_local()
        || ip.is_broadcast()
        || ip.is_multicast()
}

fn is_blocked_ipv6(ip: Ipv6Addr) -> bool {
    if let Some(v4) = ip.to_ipv4_mapped().or_else(|| ip.to_ipv4())
        && is_blocked_ipv4(v4)
    {
        return true;
    }
    let segs = ip.segments();
    let is_link_local = (segs[0] & 0xffc0) == 0xfe80;
    let is_aws_imds = ip == Ipv6Addr::new(0xfd00, 0x0ec2, 0, 0, 0, 0, 0, 0x0254);
    ip.is_unspecified() || ip.is_loopback() || ip.is_multicast() || is_link_local || is_aws_imds
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_public_and_lan_strm_urls() {
        let content = "\u{feff}# CloudDrive 115 link\n\n  https://cdn.example.com/movies/film.mkv?token=abc  \nhttps://ignored.example.com/2.mkv\n";
        assert_eq!(
            parse_and_validate_strm_content(content).unwrap(),
            "https://cdn.example.com/movies/film.mkv?token=abc"
        );
        assert_eq!(
            validate_strm_url("http://192.168.1.50:5244/d/115/Movie.mp4").unwrap(),
            "http://192.168.1.50:5244/d/115/Movie.mp4"
        );
        assert_eq!(
            validate_strm_url("http://10.0.0.9:19798/static/show.mkv").unwrap(),
            "http://10.0.0.9:19798/static/show.mkv"
        );
    }

    #[test]
    fn rejects_ssrf_and_non_http_schemes() {
        for bad in [
            "file:///etc/passwd",
            "javascript:alert(1)",
            "ftp://example.com/movie.mkv",
            "http://169.254.169.254/latest/meta-data/",
            "http://[::ffff:169.254.169.254]/latest/meta-data/",
            "http://metadata.google.internal/computeMetadata/v1/",
            "http://0.0.0.0/stream.mp4",
            "http://127.0.0.1:8080/internal",
            "http://[::1]:8080/internal",
            "http://localhost:8080/internal",
            "http://[fe80::1]/stream.mp4",
            "http://[fd00:ec2::254]/latest/meta-data/",
            "not a url",
            "",
        ] {
            assert!(
                validate_strm_url(bad).is_err(),
                "expected SSRF / scheme rejection for {bad}"
            );
        }
    }

    #[test]
    fn rejects_oversized_strm_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("huge.strm");
        let payload = format!("https://example.com/{}", "a".repeat(9000));
        std::fs::write(&path, payload).unwrap();
        let file = std::fs::File::open(&path).unwrap();
        assert!(read_strm_file(file).is_err());
    }
}

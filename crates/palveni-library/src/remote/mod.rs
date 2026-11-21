//! Read-only clients for network sources explicitly registered by an administrator.
mod http;
mod smb;

use std::pin::Pin;
use std::sync::{Arc, LazyLock};
use std::time::Duration;

use palveni_core::connections::{ConnectionKind, ConnectionPage, Credentials};
use reqwest::Url;
use tokio::{
    io::AsyncRead,
    sync::{OwnedSemaphorePermit, Semaphore},
    time::timeout,
};

#[derive(Debug, thiserror::Error)]
pub enum RemoteError {
    #[error("invalid source or media entry")]
    Invalid,
    #[error("source unavailable")]
    Unavailable,
    #[error("unsatisfiable range")]
    Range { total: u64 },
}

pub struct RemoteStream {
    pub reader: Pin<Box<dyn AsyncRead + Send>>,
    pub len: u64,
    pub total: u64,
    pub start: u64,
    pub mime: String,
    pub partial: bool,
}

static BROWSE_SLOTS: LazyLock<Arc<Semaphore>> = LazyLock::new(|| Arc::new(Semaphore::new(8)));
static STREAM_SLOTS: LazyLock<Arc<Semaphore>> = LazyLock::new(|| Arc::new(Semaphore::new(8)));

pub fn parse_address(kind: ConnectionKind, raw: &str) -> Result<Url, RemoteError> {
    if raw.len() > 2048 || raw.chars().any(char::is_control) || raw.contains('\\') {
        return Err(RemoteError::Invalid);
    }
    let url = Url::parse(raw).map_err(|_| RemoteError::Invalid)?;
    let valid_scheme = match kind {
        ConnectionKind::Smb => url.scheme() == "smb" && !url.path().trim_matches('/').is_empty(),
        _ => matches!(url.scheme(), "https" | "http"),
    };
    if !valid_scheme
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(RemoteError::Invalid);
    }
    Ok(url)
}

pub fn validate_entry(entry: &str) -> Result<(), RemoteError> {
    if entry.is_empty() || entry.len() > 4096 || entry.chars().any(char::is_control) {
        return Err(RemoteError::Invalid);
    }
    Ok(())
}

pub async fn entries(
    kind: ConnectionKind,
    address: &Url,
    credentials: &Credentials,
    parent: &str,
    offset: u32,
    limit: u32,
) -> Result<ConnectionPage, RemoteError> {
    if !parent.is_empty() {
        validate_entry(parent)?;
    }
    let _permit = BROWSE_SLOTS
        .try_acquire()
        .map_err(|_| RemoteError::Unavailable)?;
    timeout(Duration::from_secs(25), async {
        match kind {
            ConnectionKind::Smb => smb::entries(address, credentials, parent, offset, limit).await,
            _ => http::entries(kind, address, credentials, parent, offset, limit).await,
        }
    })
    .await
    .map_err(|_| RemoteError::Unavailable)?
}

pub async fn open(
    kind: ConnectionKind,
    address: &Url,
    credentials: &Credentials,
    id: &str,
    range: Option<&str>,
) -> Result<(RemoteStream, OwnedSemaphorePermit), RemoteError> {
    validate_entry(id)?;
    let permit = STREAM_SLOTS
        .clone()
        .try_acquire_owned()
        .map_err(|_| RemoteError::Unavailable)?;
    let stream = timeout(Duration::from_secs(25), async {
        match kind {
            ConnectionKind::Smb => smb::open(address, credentials, id, range).await,
            _ => http::open(kind, address, credentials, id, range).await,
        }
    })
    .await
    .map_err(|_| RemoteError::Unavailable)??;
    Ok((stream, permit))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn address_requires_protocol_and_separate_credentials() {
        for raw in [
            "file:///tmp/media",
            "https://example.test/?token=bad",
            "https://user:secret@example.test/",
            "https://example.test/#anchor",
        ] {
            assert!(parse_address(ConnectionKind::Webdav, raw).is_err());
        }
        assert!(parse_address(ConnectionKind::Smb, "smb://example.test").is_err());
        assert!(parse_address(ConnectionKind::Smb, "smb://example.test/Music").is_ok());
        assert!(parse_address(ConnectionKind::Webdav, "https://example.test/dav/music").is_ok());
    }
}

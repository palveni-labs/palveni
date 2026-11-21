pub mod auth;
pub mod broadcast;
pub mod connections;
pub mod events;
pub mod items;
pub mod jobs;
pub mod libraries;
pub mod music;
pub mod personal;
pub mod playback;
pub mod plugins;
pub mod settings;
pub mod skins;
pub mod spoken;
pub mod storage;
pub mod subsonic;
pub mod system;
pub mod video;

use std::str::FromStr;

use palveni_db::audit::AuditEntry;

use crate::error::ApiError;
use crate::state::AppState;

pub(crate) const DEFAULT_PAGE_SIZE: u32 = 50;
pub(crate) const MAX_PAGE_SIZE: u32 = 200;

/// Records an audit entry. Failures are logged, never surfaced: auditing
/// must not break the operation it describes.
pub(crate) async fn audit(state: &AppState, entry: AuditEntry) {
    let action = entry.action;
    if let Err(err) = state.db.audit().record(entry).await {
        tracing::warn!(action, error = %err, "cannot write audit log entry");
    }
}

/// `?limit=`: default when absent, clamped to the maximum, at least 1.
pub(crate) fn parse_limit(raw: Option<&str>, default: u32, max: u32) -> Result<u32, ApiError> {
    match raw.map(str::trim).filter(|s| !s.is_empty()) {
        None => Ok(default),
        Some(text) => {
            let value: u32 = text.parse().map_err(|_| {
                ApiError::validation("limit must be a positive integer").field("limit")
            })?;
            if value == 0 {
                return Err(ApiError::validation("limit must be at least 1").field("limit"));
            }
            Ok(value.min(max))
        }
    }
}

/// Parses an ID from a URL path segment. Malformed IDs cannot name an
/// existing resource, so they are reported as `404 not_found`.
pub(crate) fn path_id<T: FromStr>(raw: &str, what: &str) -> Result<T, ApiError> {
    raw.parse::<T>()
        .map_err(|_| ApiError::not_found(format!("{what} not found")))
}

/// Parses an optional ID from a query parameter (`400` when malformed).
pub(crate) fn query_id<T: FromStr>(raw: Option<&str>, name: &str) -> Result<Option<T>, ApiError> {
    match raw.map(str::trim).filter(|s| !s.is_empty()) {
        None => Ok(None),
        Some(text) => text
            .parse::<T>()
            .map(Some)
            .map_err(|_| ApiError::validation(format!("{name} is not a valid ID")).field(name)),
    }
}

#[cfg(test)]
mod tests {
    use palveni_core::LibraryId;

    use super::*;

    #[test]
    fn limits() {
        assert_eq!(parse_limit(None, 50, 200).unwrap(), 50);
        assert_eq!(parse_limit(Some(""), 50, 200).unwrap(), 50);
        assert_eq!(parse_limit(Some("10"), 50, 200).unwrap(), 10);
        assert_eq!(parse_limit(Some("5000"), 50, 200).unwrap(), 200);
        assert!(parse_limit(Some("0"), 50, 200).is_err());
        assert!(parse_limit(Some("-1"), 50, 200).is_err());
        assert!(parse_limit(Some("ten"), 50, 200).is_err());
    }

    #[test]
    fn ids() {
        let id = LibraryId::generate();
        let parsed: Option<LibraryId> = query_id(Some(&id.to_string()), "libraryId").unwrap();
        assert_eq!(parsed, Some(id));
        assert_eq!(query_id::<LibraryId>(None, "libraryId").unwrap(), None);
        let err = query_id::<LibraryId>(Some("nope"), "libraryId").unwrap_err();
        assert_eq!(err.code(), "validation_failed");
        let err = path_id::<LibraryId>("nope", "Library").unwrap_err();
        assert_eq!(err.code(), "not_found");
    }
}

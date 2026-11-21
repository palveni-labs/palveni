//! Plugin permissions.
//!
//! Everything is denied unless declared in `[permissions]` and granted by
//! an administrator. Unknown permission keys are rejected rather than
//! ignored: a permission the host does not understand must never be
//! silently treated as granted or as absent.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::error::ManifestError;

/// Upper bound for a plugin's private key/value storage.
pub const MAX_STORAGE_QUOTA: u64 = 1024 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LibraryAccess {
    /// Read item metadata.
    Metadata,
    /// Read media file contents by byte range (fingerprinting, …).
    Files,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum UserStateAccess {
    Read,
    ReadWrite,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct Permissions {
    /// Host names reachable over HTTPS; `*.example.org` covers subdomains.
    pub network: Vec<String>,
    /// Private storage quota, e.g. `"10MB"`.
    pub storage: Option<String>,
    /// Secret names readable by the plugin.
    pub secrets: Vec<String>,
    pub library: Vec<LibraryAccess>,
    pub user_state: Option<UserStateAccess>,
    /// Event names the plugin may subscribe to.
    pub events: Vec<String>,
    pub schedule: bool,
    pub http_endpoint: bool,
    pub oauth: bool,
}

/// One permission that a new version has and the old one did not.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PermissionChange {
    NetworkHost(String),
    StorageQuota { from: Option<u64>, to: u64 },
    Secret(String),
    Library(LibraryAccess),
    UserState(UserStateAccess),
    Event(String),
    Schedule,
    HttpEndpoint,
    OAuth,
}

impl Permissions {
    pub fn is_empty(&self) -> bool {
        *self == Permissions::default()
    }

    /// Storage quota in bytes.
    pub fn storage_quota(&self) -> Result<Option<u64>, ManifestError> {
        match &self.storage {
            None => Ok(None),
            Some(text) => parse_size(text).map(Some).ok_or_else(|| {
                ManifestError::invalid(
                    "permissions.storage",
                    format!("`{text}` is not a size such as \"10MB\""),
                )
            }),
        }
    }

    pub fn validate(&self) -> Result<(), ManifestError> {
        for host in &self.network {
            if !is_valid_network_host(host) {
                return Err(ManifestError::invalid(
                    "permissions.network",
                    format!(
                        "`{host}` is not an allowed public host name (no IPs, ports, schemes or local names)"
                    ),
                ));
            }
        }
        if let Some(quota) = self.storage_quota()?
            && (quota == 0 || quota > MAX_STORAGE_QUOTA)
        {
            return Err(ManifestError::invalid(
                "permissions.storage",
                "quota must be between 1 byte and 1GB",
            ));
        }
        for name in &self.secrets {
            if !is_identifier(name, 64) {
                return Err(ManifestError::invalid(
                    "permissions.secrets",
                    format!("`{name}` must match [a-z0-9_]{{1,64}}"),
                ));
            }
        }
        for event in &self.events {
            let ok = !event.is_empty()
                && event.len() <= 64
                && event
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '.' || c == '_');
            if !ok {
                return Err(ManifestError::invalid(
                    "permissions.events",
                    format!("`{event}` is not an event name"),
                ));
            }
        }
        Ok(())
    }

    /// Permissions in `self` that `previous` did not grant. A non-empty
    /// result means an upgrade needs the administrator's consent again.
    pub fn escalations_from(&self, previous: &Permissions) -> Vec<PermissionChange> {
        let mut changes = Vec::new();
        for host in &self.network {
            if !host_covered(host, &previous.network) {
                changes.push(PermissionChange::NetworkHost(host.clone()));
            }
        }
        let new_quota = self.storage_quota().ok().flatten();
        let old_quota = previous.storage_quota().ok().flatten();
        if let Some(to) = new_quota
            && old_quota.is_none_or(|from| to > from)
        {
            changes.push(PermissionChange::StorageQuota {
                from: old_quota,
                to,
            });
        }
        let old_secrets: BTreeSet<&String> = previous.secrets.iter().collect();
        for secret in &self.secrets {
            if !old_secrets.contains(secret) {
                changes.push(PermissionChange::Secret(secret.clone()));
            }
        }
        for access in &self.library {
            if !previous.library.contains(access) {
                changes.push(PermissionChange::Library(*access));
            }
        }
        if let Some(access) = self.user_state {
            let escalated = match previous.user_state {
                None => true,
                Some(UserStateAccess::Read) => access == UserStateAccess::ReadWrite,
                Some(UserStateAccess::ReadWrite) => false,
            };
            if escalated {
                changes.push(PermissionChange::UserState(access));
            }
        }
        for event in &self.events {
            if !previous.events.contains(event) {
                changes.push(PermissionChange::Event(event.clone()));
            }
        }
        if self.schedule && !previous.schedule {
            changes.push(PermissionChange::Schedule);
        }
        if self.http_endpoint && !previous.http_endpoint {
            changes.push(PermissionChange::HttpEndpoint);
        }
        if self.oauth && !previous.oauth {
            changes.push(PermissionChange::OAuth);
        }
        changes
    }
}

/// Parses `"512"`, `"64KB"`, `"10MB"`, `"1GB"` (1024-based; `KiB`/`MiB`/`GiB`
/// accepted too). Returns `None` for anything else.
pub fn parse_size(text: &str) -> Option<u64> {
    let text = text.trim();
    let split = text
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(text.len());
    let (digits, unit) = text.split_at(split);
    let value: u64 = digits.parse().ok()?;
    let multiplier: u64 = match unit.trim().to_ascii_uppercase().as_str() {
        "" | "B" => 1,
        "K" | "KB" | "KIB" => 1024,
        "M" | "MB" | "MIB" => 1024 * 1024,
        "G" | "GB" | "GIB" => 1024 * 1024 * 1024,
        _ => return None,
    };
    value.checked_mul(multiplier)
}

fn is_identifier(text: &str, max_len: usize) -> bool {
    !text.is_empty()
        && text.len() <= max_len
        && text
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

/// Public DNS names only, optionally with a leading `*.` wildcard.
/// The host additionally blocks private addresses at request time; this
/// check rejects what is obviously wrong in a manifest.
pub fn is_valid_network_host(host: &str) -> bool {
    let name = host.strip_prefix("*.").unwrap_or(host);
    if name.is_empty() || name.len() > 253 || name != name.to_ascii_lowercase() {
        return false;
    }
    let labels: Vec<&str> = name.split('.').collect();
    if labels.len() < 2 {
        return false;
    }
    let label_ok = |label: &&str| {
        !label.is_empty()
            && label.len() <= 63
            && !label.starts_with('-')
            && !label.ends_with('-')
            && label
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    };
    if !labels.iter().all(label_ok) {
        return false;
    }
    // An all-numeric last label means an IPv4 literal.
    if labels
        .last()
        .is_some_and(|tld| tld.chars().all(|c| c.is_ascii_digit()))
    {
        return false;
    }
    const LOCAL_SUFFIXES: [&str; 6] = [
        "localhost",
        "local",
        "lan",
        "internal",
        "home.arpa",
        "intranet",
    ];
    !LOCAL_SUFFIXES
        .iter()
        .any(|suffix| name == *suffix || name.ends_with(&format!(".{suffix}")))
}

fn host_covered(host: &str, granted: &[String]) -> bool {
    granted.iter().any(|g| {
        if g == host {
            return true;
        }
        match g.strip_prefix("*.") {
            // `*.example.org` covers `api.example.org` and `*.api.example.org`.
            Some(base) => {
                let candidate = host.strip_prefix("*.").unwrap_or(host);
                candidate.ends_with(&format!(".{base}"))
            }
            None => false,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes() {
        assert_eq!(parse_size("10MB"), Some(10 * 1024 * 1024));
        assert_eq!(parse_size("512"), Some(512));
        assert_eq!(parse_size("64 KiB"), Some(64 * 1024));
        assert_eq!(parse_size("1gb"), Some(1024 * 1024 * 1024));
        assert_eq!(parse_size("MB"), None);
        assert_eq!(parse_size("10TB"), None);
        assert_eq!(parse_size("-1MB"), None);
    }

    #[test]
    fn network_hosts() {
        for ok in [
            "api.themoviedb.org",
            "image.tmdb.org",
            "*.example.org",
            "xn--fiqs8s.cn",
        ] {
            assert!(is_valid_network_host(ok), "{ok}");
        }
        for bad in [
            "localhost",
            "nas.local",
            "router.lan",
            "192.168.1.1",
            "10.0.0.1",
            "https://api.example.org",
            "api.example.org:8443",
            "API.example.org",
            "example",
            "*.",
            "-bad.example.org",
            "a..b.org",
        ] {
            assert!(!is_valid_network_host(bad), "{bad}");
        }
    }

    #[test]
    fn unknown_permissions_are_rejected() {
        let parsed: Result<Permissions, _> = toml::from_str("filesystem = true");
        assert!(parsed.is_err());
    }

    #[test]
    fn kebab_case_keys() {
        let parsed: Permissions = toml::from_str(
            "user-state = \"read-write\"\nhttp-endpoint = true\nlibrary = [\"files\"]",
        )
        .unwrap();
        assert_eq!(parsed.user_state, Some(UserStateAccess::ReadWrite));
        assert!(parsed.http_endpoint);
        assert_eq!(parsed.library, vec![LibraryAccess::Files]);
    }

    #[test]
    fn escalation_detection() {
        let old = Permissions {
            network: vec!["*.tmdb.org".into()],
            storage: Some("10MB".into()),
            secrets: vec!["api_key".into()],
            user_state: Some(UserStateAccess::Read),
            ..Permissions::default()
        };
        let same_scope = Permissions {
            network: vec!["image.tmdb.org".into()],
            storage: Some("5MB".into()),
            secrets: vec!["api_key".into()],
            user_state: Some(UserStateAccess::Read),
            ..Permissions::default()
        };
        assert!(same_scope.escalations_from(&old).is_empty());

        let more = Permissions {
            network: vec!["*.tmdb.org".into(), "api.trakt.tv".into()],
            storage: Some("20MB".into()),
            secrets: vec!["api_key".into(), "client_secret".into()],
            user_state: Some(UserStateAccess::ReadWrite),
            oauth: true,
            ..Permissions::default()
        };
        let changes = more.escalations_from(&old);
        assert!(changes.contains(&PermissionChange::NetworkHost("api.trakt.tv".into())));
        assert!(changes.contains(&PermissionChange::StorageQuota {
            from: Some(10 * 1024 * 1024),
            to: 20 * 1024 * 1024
        }));
        assert!(changes.contains(&PermissionChange::Secret("client_secret".into())));
        assert!(changes.contains(&PermissionChange::UserState(UserStateAccess::ReadWrite)));
        assert!(changes.contains(&PermissionChange::OAuth));
        assert_eq!(changes.len(), 5);
    }

    #[test]
    fn validation() {
        let ok = Permissions {
            network: vec!["api.example.org".into()],
            storage: Some("10MB".into()),
            secrets: vec!["api_key".into()],
            events: vec!["item.added".into()],
            ..Permissions::default()
        };
        assert!(ok.validate().is_ok());
        let bad_secret = Permissions {
            secrets: vec!["API KEY".into()],
            ..Permissions::default()
        };
        assert!(bad_secret.validate().is_err());
        let huge = Permissions {
            storage: Some("2GB".into()),
            ..Permissions::default()
        };
        assert!(huge.validate().is_err());
    }
}

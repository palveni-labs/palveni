//! Contract (WIT package `palveni:plugin`) version compatibility.
//!
//! Plugins declare a semver range (`[compat] api = "^0.1"`). The host
//! supports its current contract version and the previous major one. It
//! binds a plugin to the newest compatible version.

use semver::{Version, VersionReq};

use crate::error::ManifestError;

/// Version of `wit/` implemented by this host.
pub const HOST_CONTRACT_VERSION: &str = "0.1.0";

/// Contract versions this host can serve, newest first.
pub fn supported_contract_versions() -> Vec<Version> {
    // Only one contract exists so far; the previous major joins this list
    // when 1.0 ships.
    match Version::parse(HOST_CONTRACT_VERSION) {
        Ok(version) => vec![version],
        Err(_) => Vec::new(),
    }
}

/// Parses a compatibility range.
pub fn parse_range(range: &str) -> Result<VersionReq, ManifestError> {
    VersionReq::parse(range.trim())
        .map_err(|e| ManifestError::invalid("compat.api", format!("`{range}`: {e}")))
}

/// Checks `range` against this host.
pub fn check_compat(range: &str) -> Result<Version, ManifestError> {
    check_compat_with(range, &supported_contract_versions())
}

/// Checks `range` against an explicit list of supported versions and
/// returns the newest matching one.
pub fn check_compat_with(range: &str, supported: &[Version]) -> Result<Version, ManifestError> {
    let req = parse_range(range)?;
    supported
        .iter()
        .filter(|v| req.matches(v))
        .max()
        .cloned()
        .ok_or_else(|| ManifestError::Incompatible {
            required: range.trim().to_owned(),
            available: supported
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", "),
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(s: &str) -> Version {
        Version::parse(s).unwrap()
    }

    #[test]
    fn caret_ranges_on_zero_major() {
        assert_eq!(check_compat("^0.1").unwrap(), v("0.1.0"));
        assert!(check_compat("^0.2").is_err());
        assert!(check_compat(">=1.0").is_err());
        assert!(check_compat("not a range").is_err());
    }

    #[test]
    fn picks_newest_supported_match() {
        let supported = [v("2.1.0"), v("1.4.2")];
        assert_eq!(check_compat_with("^1", &supported).unwrap(), v("1.4.2"));
        assert_eq!(
            check_compat_with(">=1.0, <3", &supported).unwrap(),
            v("2.1.0")
        );
        let err = check_compat_with("^3", &supported).unwrap_err();
        assert!(matches!(err, ManifestError::Incompatible { .. }));
    }
}

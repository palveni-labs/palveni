//! Timestamp helpers.
//!
//! Timestamps are stored as integer milliseconds since the Unix epoch and
//! exposed as `DateTime<Utc>` (serialised as RFC 3339). `now()` truncates to
//! milliseconds so a value returned straight from a create call is identical
//! to the same value read back from the database later.

use std::time::SystemTime;

use chrono::{DateTime, Utc};

/// Current time, truncated to millisecond precision.
pub fn now() -> DateTime<Utc> {
    let exact = Utc::now();
    from_millis(exact.timestamp_millis()).unwrap_or(exact)
}

pub fn to_millis(t: DateTime<Utc>) -> i64 {
    t.timestamp_millis()
}

/// Returns `None` when `ms` is outside the range chrono can represent.
pub fn from_millis(ms: i64) -> Option<DateTime<Utc>> {
    DateTime::<Utc>::from_timestamp_millis(ms)
}

pub fn from_system_time(t: SystemTime) -> DateTime<Utc> {
    let exact = DateTime::<Utc>::from(t);
    from_millis(exact.timestamp_millis()).unwrap_or(exact)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn millis_round_trip() {
        let t = now();
        assert_eq!(from_millis(to_millis(t)), Some(t));
    }

    #[test]
    fn serialises_as_rfc3339_utc_with_optional_millis() {
        // Whole seconds: no fractional part (contract: omitted when zero).
        let whole = from_millis(1_795_163_400_000).unwrap();
        assert_eq!(
            serde_json::to_string(&whole).unwrap(),
            "\"2026-11-20T08:30:00Z\""
        );
        // Otherwise exactly three digits.
        let millis = from_millis(1_795_163_400_120).unwrap();
        assert_eq!(
            serde_json::to_string(&millis).unwrap(),
            "\"2026-11-20T08:30:00.120Z\""
        );
    }
}

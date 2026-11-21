//! Small conversions between stored representations and domain types.

use std::str::FromStr;

use chrono::{DateTime, Utc};
use palveni_core::clock;
use sha2::{Digest, Sha256};

use crate::error::DbError;

pub(crate) fn parse_id<T: FromStr>(value: &str, what: &'static str) -> Result<T, DbError> {
    value
        .parse::<T>()
        .map_err(|_| DbError::Corrupt(format!("invalid {what} `{value}`")))
}

pub(crate) fn parse_opt_id<T: FromStr>(
    value: Option<&str>,
    what: &'static str,
) -> Result<Option<T>, DbError> {
    value.map(|v| parse_id(v, what)).transpose()
}

pub(crate) fn ts(ms: i64) -> Result<DateTime<Utc>, DbError> {
    clock::from_millis(ms).ok_or_else(|| DbError::Corrupt(format!("timestamp out of range: {ms}")))
}

pub(crate) fn opt_ts(ms: Option<i64>) -> Result<Option<DateTime<Utc>>, DbError> {
    ms.map(ts).transpose()
}

pub(crate) fn now_ms() -> i64 {
    clock::to_millis(clock::now())
}

/// Lowercase hex SHA-256 of a string.
pub(crate) fn sha256_hex(input: &str) -> String {
    let digest = Sha256::digest(input.as_bytes());
    hex::encode(&digest[..])
}

/// Truncates to at most `max` characters (not bytes): `VARCHAR(n)` limits
/// are in characters on PostgreSQL and MySQL.
pub(crate) fn truncate_chars(text: &str, max: usize) -> String {
    match text.char_indices().nth(max) {
        Some((byte_index, _)) => text[..byte_index].to_owned(),
        None => text.to_owned(),
    }
}

/// Saturating conversion for row counts.
pub(crate) fn count_to_i64(count: u64) -> i64 {
    i64::try_from(count).unwrap_or(i64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_is_lowercase_hex() {
        assert_eq!(
            sha256_hex("abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn truncation_counts_characters() {
        assert_eq!(truncate_chars("音乐库", 2), "音乐");
        assert_eq!(truncate_chars("abc", 5), "abc");
        assert_eq!(truncate_chars("abc", 3), "abc");
    }
}

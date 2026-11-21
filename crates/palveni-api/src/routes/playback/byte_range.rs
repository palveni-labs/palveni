use std::ops::Range;

use axum::http::{
    HeaderMap,
    header::{IF_RANGE, RANGE},
};

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Selection {
    Full,
    Partial(Range<u64>),
    Unsatisfiable,
}

/// Parse a single HTTP byte range into an exclusive Rust range.
pub(super) fn select(headers: &HeaderMap, size: u64) -> Selection {
    let mut values = headers.get_all(RANGE).iter();
    let Some(value) = values.next().and_then(|v| v.to_str().ok()) else {
        return Selection::Full;
    };
    if values.next().is_some() || headers.contains_key(IF_RANGE) {
        return Selection::Full;
    }
    let Some((unit, spec)) = value.trim().split_once('=') else {
        return Selection::Full;
    };
    if !unit.eq_ignore_ascii_case("bytes") {
        return Selection::Full;
    }
    let Some((first, last)) = spec.split_once('-') else {
        return Selection::Full;
    };
    if first.is_empty() {
        return match decimal(last) {
            None => Selection::Full,
            Some(0) => Selection::Unsatisfiable,
            Some(_) if size == 0 => Selection::Unsatisfiable,
            Some(count) => Selection::Partial(size.saturating_sub(count)..size),
        };
    }
    let Some(start) = decimal(first) else {
        return Selection::Full;
    };
    let end = if last.is_empty() {
        size
    } else {
        match decimal(last) {
            Some(end) if end >= start => end.saturating_add(1).min(size),
            _ => return Selection::Full,
        }
    };
    if start >= size {
        Selection::Unsatisfiable
    } else {
        Selection::Partial(start..end)
    }
}

// A decimal longer than u64 still denotes an offset beyond any supported file.
fn decimal(text: &str) -> Option<u64> {
    if text.is_empty() {
        return None;
    }
    text.bytes().try_fold(0u64, |n, b| {
        b.is_ascii_digit()
            .then(|| n.saturating_mul(10).saturating_add(u64::from(b - b'0')))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    #[test]
    fn probes_seeks_suffixes_and_empty_files() {
        for (value, size, expected) in [
            ("bytes=0-1", 8, Selection::Partial(0..2)),
            ("bytes=0-1", 1, Selection::Partial(0..1)),
            ("bytes=5-", 8, Selection::Partial(5..8)),
            ("bytes=-2", 8, Selection::Partial(6..8)),
            ("bytes=-20", 8, Selection::Partial(0..8)),
            (
                "bytes=2-99999999999999999999999",
                8,
                Selection::Partial(2..8),
            ),
            (
                "bytes=99999999999999999999999-",
                8,
                Selection::Unsatisfiable,
            ),
            ("bytes=8-", 8, Selection::Unsatisfiable),
            ("bytes=-0", 8, Selection::Unsatisfiable),
            ("bytes=0-", 0, Selection::Unsatisfiable),
            ("bytes=-1", 0, Selection::Unsatisfiable),
            ("bytes=3-1", 8, Selection::Full),
            ("bytes=0-1,4-5", 8, Selection::Full),
            ("other=1-2", 8, Selection::Full),
            ("bytes=--", 8, Selection::Full),
        ] {
            let mut headers = HeaderMap::new();
            headers.insert(RANGE, HeaderValue::from_static(value));
            assert_eq!(select(&headers, size), expected, "{value}");
        }
    }
}

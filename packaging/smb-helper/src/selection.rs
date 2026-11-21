use std::ops::Range;

pub const MAX_PATH_BYTES: usize = 2048;
pub const MAX_DEPTH: usize = 64;

pub fn component(name: &str) -> bool {
    !name.is_empty()
        && !matches!(name, "." | "..")
        && !name.ends_with(['.', ' '])
        && name.encode_utf16().count() <= 255
        && !name.chars().any(|c| {
            c.is_control() || matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|')
        })
}

fn decoded_segment(text: &str) -> Option<String> {
    let input = text.as_bytes();
    let mut bytes = Vec::with_capacity(input.len());
    let mut at = 0;
    while at < input.len() {
        if input[at] == b'%' {
            let mut decoded = [0];
            hex::decode_to_slice(input.get(at + 1..at + 3)?, &mut decoded).ok()?;
            bytes.push(decoded[0]);
            at += 3;
        } else {
            bytes.push(input[at]);
            at += 1;
        }
    }
    let name = String::from_utf8(bytes).ok()?;
    component(&name).then_some(name)
}

pub fn share_path(address_path: &str) -> Option<(String, Vec<String>)> {
    if address_path.len() > MAX_PATH_BYTES {
        return None;
    }
    let path = address_path.strip_prefix('/')?;
    let path = path.strip_suffix('/').unwrap_or(path);
    let mut segments = path.split('/');
    let share = decoded_segment(segments.next()?)?;
    if share.eq_ignore_ascii_case("ipc$") {
        return None;
    }
    let base: Vec<_> = segments.map(decoded_segment).collect::<Option<_>>()?;
    (base.len() <= MAX_DEPTH).then_some((share, base))
}

/// IDs only represent names below the configured directory. The authority and share always come from the saved connection.
pub fn item_path(id: &str) -> Option<String> {
    if id.len() > MAX_PATH_BYTES * 2 {
        return None;
    }
    if id.is_empty() {
        return Some(String::new());
    }
    let path = String::from_utf8(hex::decode(id).ok()?).ok()?;
    let mut segments = path.split('/');
    (segments.clone().count() <= MAX_DEPTH && segments.all(component)).then_some(path)
}

pub fn child_id(parent: &str, name: &str) -> Option<String> {
    if !component(name) {
        return None;
    }
    let child = if parent.is_empty() {
        name.to_owned()
    } else {
        format!("{parent}/{name}")
    };
    if child.len() > MAX_PATH_BYTES || child.split('/').count() > MAX_DEPTH {
        return None;
    }
    Some(hex::encode(child))
}

fn decimal(text: &str) -> Option<u64> {
    if text.is_empty() {
        return None;
    }
    text.bytes().try_fold(0u64, |n, b| {
        b.is_ascii_digit()
            .then(|| n.saturating_mul(10).saturating_add(u64::from(b - b'0')))
    })
}

pub fn bytes(header: Option<&str>, size: u64) -> Option<(Range<u64>, bool)> {
    let full = Some((0..size, false));
    let Some((unit, spec)) = header.and_then(|h| h.trim().split_once('=')) else {
        return full;
    };
    if !unit.eq_ignore_ascii_case("bytes") {
        return full;
    }
    let Some((first, last)) = spec.split_once('-') else {
        return full;
    };
    if first.is_empty() {
        return match decimal(last) {
            Some(0) => None,
            Some(_) if size == 0 => None,
            Some(count) => Some((size.saturating_sub(count)..size, true)),
            None => full,
        };
    }
    let Some(start) = decimal(first) else {
        return full;
    };
    let end = if last.is_empty() {
        size
    } else {
        match decimal(last) {
            Some(end) if end >= start => end.saturating_add(1).min(size),
            _ => return full,
        }
    };
    (start < size).then_some((start..end, true))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn share_and_items_accept_unicode_and_stay_relative() {
        let (share, base) = share_path("/%E4%B9%90%E5%BA%93/Radio+Drama/").unwrap();
        assert_eq!(share, "乐库");
        assert_eq!(base, ["Radio+Drama"]);
        assert_eq!(
            item_path(&child_id("电台", "April.m4a").unwrap()).unwrap(),
            "电台/April.m4a"
        );
        for path in [
            "/Music/%2Fshift",
            "/Music/%5Cshift",
            "/IPC$",
            "/Music/%2e%2e",
            "/Music//foo",
            "/Music/%FF",
        ] {
            assert!(share_path(path).is_none());
        }
        for name in [
            "../file.mp3",
            "album/.. /file",
            "s:ads",
            r"\\elsewhere\share",
            "/abs",
            "a//b",
        ] {
            assert!(item_path(&hex::encode(name)).is_none());
        }
    }

    #[test]
    fn ranges_match_original_file_streaming() {
        assert_eq!(bytes(None, 8), Some((0..8, false)));
        assert_eq!(bytes(Some("bytes=-3"), 8), Some((5..8, true)));
        assert_eq!(bytes(Some("bytes=4-"), 8), Some((4..8, true)));
        assert_eq!(
            bytes(Some("bytes=7-99999999999999999999999"), 8),
            Some((7..8, true))
        );
        assert_eq!(bytes(Some("bytes=0-"), 0), None);
        assert_eq!(bytes(Some("bytes=-0"), 8), None);
        assert_eq!(bytes(Some("bytes=3-1"), 8), Some((0..8, false)));
        assert_eq!(bytes(Some("bytes=0-1,5-6"), 8), Some((0..8, false)));
    }
}

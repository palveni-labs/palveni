#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct EpisodeMarker {
    pub start: usize,
    pub season: i32,
    pub episode: i32,
}

fn digits(bytes: &[u8], at: usize, max: usize) -> Option<(i32, usize)> {
    let slice = bytes.get(at..)?;
    let len = slice.iter().take_while(|b| b.is_ascii_digit()).count();
    if len == 0 || len > max {
        return None;
    }
    let value = std::str::from_utf8(&slice[..len]).ok()?.parse().ok()?;
    Some((value, at + len))
}

pub(super) fn episode_marker(name: &str) -> Option<EpisodeMarker> {
    let bytes = name.as_bytes();
    for (start, &ch) in bytes.iter().enumerate() {
        if start > 0 && bytes[start - 1].is_ascii_alphanumeric() {
            continue;
        }
        let marked = ch.eq_ignore_ascii_case(&b's');
        let Some((season, mid)) = digits(
            bytes,
            start + usize::from(marked),
            if marked { 4 } else { 2 },
        ) else {
            continue;
        };
        let split = mid
            + bytes[mid..]
                .iter()
                .take_while(|&&c| matches!(c, b' ' | b'.' | b'_' | b'-'))
                .count();
        let marker = if marked { b'e' } else { b'x' };
        if !bytes
            .get(split)
            .is_some_and(|c| c.eq_ignore_ascii_case(&marker))
        {
            continue;
        }
        if let Some((episode, _)) = digits(bytes, split + 1, 4) {
            return Some(EpisodeMarker {
                start,
                season,
                episode,
            });
        }
    }
    None
}

pub(super) fn episode_only(name: &str) -> Option<(usize, i32)> {
    for (start, _) in name.match_indices('第') {
        let offset = start + '第'.len_utf8();
        if let Some((n, end)) = digits(name.as_bytes(), offset, 4)
            && name[end..].starts_with('集')
        {
            return Some((start, n));
        }
    }
    let bytes = name.as_bytes();
    for (index, &c) in bytes.iter().enumerate() {
        if c.eq_ignore_ascii_case(&b'e')
            && (index == 0 || !bytes[index - 1].is_ascii_alphanumeric())
        {
            let i = index + 1;
            let p = usize::from(bytes.get(i).is_some_and(|b| b.eq_ignore_ascii_case(&b'p')));
            if let Some((n, _)) = digits(bytes, i + p, 4) {
                return Some((index, n));
            }
        }
    }
    None
}

pub(super) fn season_folder(name: &str) -> Option<i32> {
    let name = name.trim().to_ascii_lowercase();
    if matches!(name.as_str(), "specials" | "special" | "特别篇") {
        return Some(0);
    }
    let n = name
        .strip_prefix("season")
        .or_else(|| name.strip_prefix('s'))
        .or_else(|| name.strip_prefix('第')?.strip_suffix('季'))?
        .trim_matches([' ', '.', '_', '-']);
    if n.len() > 4 {
        return None;
    }
    super::nfo::number(n)
}

pub(super) fn disc_folder(name: &str) -> bool {
    let lower = name.trim().to_ascii_lowercase();
    let Some(rest) = lower
        .strip_prefix("disc")
        .or_else(|| lower.strip_prefix("disk"))
        .or_else(|| lower.strip_prefix("cd"))
        .or_else(|| lower.strip_prefix("bd"))
    else {
        return false;
    };
    let rest = rest.trim_matches([' ', '.', '_', '-']);
    !rest.is_empty() && rest.len() <= 3 && rest.bytes().all(|b| b.is_ascii_digit())
}

pub(super) fn is_generic_category_dir(name: &str) -> bool {
    matches!(
        name.trim().to_ascii_lowercase().as_str(),
        "电影"
            | "剧集"
            | "电视剧"
            | "动漫"
            | "动画"
            | "动画电影"
            | "纪录片"
            | "综艺"
            | "其他"
            | "movies"
            | "movie"
            | "films"
            | "film"
            | "tv"
            | "shows"
            | "series"
            | "video"
            | "videos"
            | "download"
            | "downloads"
            | "other"
    )
}

pub(super) fn clean_title(name: &str) -> String {
    name.replace(['.', '_'], " ")
        .trim_matches([' ', '-', '[', ']', '(', ')'])
        .trim()
        .to_owned()
}

pub(super) fn title_year(name: &str) -> (String, Option<i32>) {
    for (idx, ch) in name.char_indices() {
        if !matches!(ch, '(' | '[') {
            continue;
        }
        let tail = &name[idx + 1..];
        if let Some(year) = tail
            .get(..4)
            .and_then(super::nfo::number)
            .filter(|n| (1880..=2199).contains(n))
            && matches!(tail.as_bytes().get(4), Some(b')' | b']'))
        {
            let title = clean_title(&name[..idx]);
            if !title.is_empty() {
                return (title, Some(year));
            }
        }
    }
    // Release names commonly put the year between the title and codec tags.
    let bytes = name.as_bytes();
    for (idx, _) in name.char_indices().rev() {
        if idx == 0 || !matches!(bytes[idx - 1], b'.' | b' ' | b'_' | b'-') {
            continue;
        }
        if let Some((year, end)) = digits(bytes, idx, 4)
            && end - idx == 4
            && (1880..=chrono::Datelike::year(&chrono::Utc::now()) + 2).contains(&year)
            && bytes
                .get(end)
                .is_none_or(|c| matches!(c, b'.' | b' ' | b'_' | b'-'))
        {
            let title = clean_title(&name[..idx]);
            if !title.is_empty() {
                return (title, Some(year));
            }
        }
    }
    (clean_title(name), None)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn identifies_episode_markers_without_splitting_unicode() {
        assert_eq!(
            episode_marker("三体.S02E003.1080p").map(|m| (m.season, m.episode)),
            Some((2, 3))
        );
        assert!(episode_marker("Glass01Ene").is_none());
        assert_eq!(
            episode_marker("A Show.2x03").map(|m| (m.season, m.episode)),
            Some((2, 3))
        );
        assert!(episode_marker("Movie.1920x1080").is_none());
        assert_eq!(episode_only("电视剧.第18集").map(|(_, n)| n), Some(18));
        assert_eq!(episode_only("Show - EP07").map(|(_, n)| n), Some(7));
        assert_eq!(season_folder("Season 01"), Some(1));
        assert_eq!(season_folder("Season 2021"), Some(2021));
        assert_eq!(season_folder("第2季"), Some(2));
        assert_eq!(season_folder("第2021季"), Some(2021));
        assert_eq!(
            episode_marker("不疯不狂不爱你 - S2021E01 - Mad for Each Other 2021")
                .map(|m| (m.season, m.episode)),
            Some((2021, 1))
        );
        assert!(disc_folder("DISC1"));
        assert!(disc_folder("Disk 2"));
        assert!(!disc_folder("Discovery"));
        assert_eq!(title_year("电影名字 (2020)").1, Some(2020));
        assert_eq!(
            title_year("A.Movie.2021.2160p.HEVC"),
            ("A Movie".into(), Some(2021))
        );
    }
}

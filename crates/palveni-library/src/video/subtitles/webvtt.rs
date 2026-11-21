use palveni_core::video::SubtitleFormat;
use std::fmt::Write;

fn timestamp(s: &str, format: SubtitleFormat) -> Option<u64> {
    let decimal = if format == SubtitleFormat::Srt {
        ','
    } else {
        '.'
    };
    let (time, ms) = s.split_once(decimal)?;
    if ms.len() != 3 || !ms.bytes().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let mut parts = time.rsplit(':');
    let seconds = parts.next()?;
    let minutes = parts.next()?;
    let hours = parts.next();
    if parts.next().is_some() || (hours.is_none() && format == SubtitleFormat::Srt) {
        return None;
    }
    let pair = |s: &str| {
        (s.len() == 2 && s.bytes().all(|c| c.is_ascii_digit()))
            .then(|| s.parse::<u64>().ok())
            .flatten()
            .filter(|n| *n < 60)
    };
    let hours = match hours {
        Some(h) if (2..=6).contains(&h.len()) && h.bytes().all(|c| c.is_ascii_digit()) => {
            h.parse::<u64>().ok()?
        }
        None => 0,
        _ => return None,
    };
    Some(((hours * 60 + pair(minutes)?) * 60 + pair(seconds)?) * 1000 + ms.parse::<u64>().ok()?)
}

fn write_time(out: &mut String, t: u64) {
    let _ = write!(
        out,
        "{:02}:{:02}:{:02}.{:03}",
        t / 3_600_000,
        t / 60_000 % 60,
        t / 1000 % 60,
        t % 1000
    );
}

fn text(out: &mut String, mut rest: &str) {
    const LITERALS: [&str; 12] = [
        "<b>", "</b>", "<i>", "</i>", "<u>", "</u>", "&amp;", "&lt;", "&gt;", "&nbsp;", "&lrm;",
        "&rlm;",
    ];
    while let Some(i) = rest.find(['<', '>', '&']) {
        out.push_str(&rest[..i]);
        rest = &rest[i..];
        if let Some(tag) = LITERALS.iter().find(|tag| rest.starts_with(**tag)) {
            out.push_str(tag);
            rest = &rest[tag.len()..];
        } else {
            out.push_str(match rest.as_bytes()[0] {
                b'<' => "&lt;",
                b'>' => "&gt;",
                _ => "&amp;",
            });
            rest = &rest[1..];
        }
    }
    out.push_str(rest);
}

pub(super) fn render(bytes: &[u8], format: SubtitleFormat) -> Option<String> {
    let raw = std::str::from_utf8(bytes)
        .ok()?
        .trim_start_matches('\u{feff}');
    if raw
        .chars()
        .any(|c| c.is_control() && !matches!(c, '\r' | '\n' | '\t'))
    {
        return None;
    }
    let normalized = raw.replace("\r\n", "\n").replace('\r', "\n");
    let mut lines = normalized.lines();
    if format == SubtitleFormat::Vtt {
        let header = lines.next()?;
        if header != "WEBVTT" && !header.starts_with("WEBVTT ") && !header.starts_with("WEBVTT\t") {
            return None;
        }
        lines
            .by_ref()
            .take_while(|line| !line.trim().is_empty())
            .for_each(drop);
    }
    let mut out = String::from("WEBVTT\n\n");
    let mut cues = 0;
    while let Some(first) = lines.by_ref().find(|l| !l.trim().is_empty()) {
        if format == SubtitleFormat::Vtt
            && (matches!(first, "STYLE" | "REGION" | "NOTE")
                || first.starts_with("NOTE ")
                || first.starts_with("NOTE\t"))
        {
            lines
                .by_ref()
                .take_while(|l| !l.trim().is_empty())
                .for_each(drop);
            continue;
        }
        let timing = if first.contains("-->") {
            first
        } else {
            lines.next()?
        };
        let (start, end) = timing.trim().split_once("-->")?;
        let start = timestamp(start.trim(), format)?;
        let end = timestamp(end.split_whitespace().next()?, format)?;
        if end <= start {
            return None;
        }
        cues += 1;
        if cues > 32768 {
            return None;
        }
        write_time(&mut out, start);
        out.push_str(" --> ");
        write_time(&mut out, end);
        out.push('\n');
        for line in lines.by_ref().take_while(|line| !line.trim().is_empty()) {
            text(&mut out, line);
            out.push('\n');
        }
        out.push('\n');
        if out.len() > 8 * 1024 * 1024 {
            return None;
        }
    }
    (cues > 0).then_some(out)
}

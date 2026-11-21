use std::{
    fs::File,
    io,
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};

use palveni_core::spoken::{SpokenChapter, SpokenChapterList};
use serde::Deserialize;
use tokio::{process::Command, sync::Semaphore};

use crate::{
    discovery::locate,
    process::{FILE_INPUT, bounded_output, configure_file_input},
};

pub struct AudiobookChapters {
    ffprobe: Option<PathBuf>,
    slots: Semaphore,
}

impl AudiobookChapters {
    pub(crate) fn new(configured: Option<&Path>) -> Self {
        Self {
            ffprobe: locate("ffprobe", configured).map(|tool| tool.path),
            slots: Semaphore::new(2),
        }
    }

    pub async fn list(&self, file: File) -> io::Result<SpokenChapterList> {
        let Some(ffprobe) = &self.ffprobe else {
            return Ok(SpokenChapterList::default());
        };
        let _slot = self
            .slots
            .try_acquire()
            .map_err(|_| io::Error::from(io::ErrorKind::WouldBlock))?;
        let mut cmd = Command::new(ffprobe);
        configure_file_input(&mut cmd, "mov");
        cmd.args([
            "-nofind_stream_info",
            "-show_entries",
            "chapter=start_time:chapter_tags=title",
            "-of",
            "json",
            "-i",
            &format!("{FILE_INPUT}:"),
        ])
        .stdin(Stdio::from(file));
        match bounded_output(cmd, 512 * 1024, Duration::from_secs(5)).await {
            Ok(data) => Ok(parse_chapters(&data)),
            Err(err) if err.kind() == io::ErrorKind::TimedOut => Err(err),
            Err(_) => Ok(SpokenChapterList::default()),
        }
    }
}

#[derive(Deserialize, Default)]
struct ChapterTags {
    title: Option<String>,
}

#[derive(Deserialize)]
struct Chapter {
    start_time: Option<String>,
    #[serde(default)]
    tags: ChapterTags,
}

fn milliseconds(seconds: &str) -> Option<u64> {
    let (whole, fraction) = seconds.split_once('.').unwrap_or((seconds, ""));
    if !whole.bytes().all(|b| b.is_ascii_digit()) || !fraction.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let millis = fraction
        .bytes()
        .zip([100, 10, 1])
        .map(|(b, weight)| u64::from(b - b'0') * weight)
        .sum::<u64>();
    whole
        .parse::<u64>()
        .ok()?
        .checked_mul(1000)?
        .checked_add(millis)
}

fn parse_chapters(data: &[u8]) -> SpokenChapterList {
    #[derive(Deserialize)]
    struct Listing {
        chapters: Vec<Chapter>,
    }
    let Ok(list) = serde_json::from_slice::<Listing>(data) else {
        return SpokenChapterList::default();
    };
    let mut chapters: Vec<_> = list
        .chapters
        .into_iter()
        .filter_map(|chapter| {
            let start_ms = milliseconds(chapter.start_time.as_deref()?)?;
            let title = chapter
                .tags
                .title
                .map(|name| name.trim().chars().take(256).collect::<String>())
                .filter(|name| !name.is_empty());
            Some(SpokenChapter { start_ms, title })
        })
        .collect();
    chapters.sort_by_key(|chapter| chapter.start_ms);
    chapters.dedup_by_key(|chapter| chapter.start_ms);
    chapters.truncate(1024);
    SpokenChapterList { chapters }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sorts_valid_positions_and_keeps_unnamed_chapters() {
        let data = br#"{"chapters":[{"start_time":"12.034999","tags":{"title":" Harbor "}},{"start_time":"0.0"},{"start_time":"0.000999"},{"start_time":"-1.0"},{"start_time":"18446744073709551615"}]}"#;
        let parsed = parse_chapters(data);
        assert_eq!(parsed.chapters.len(), 2);
        assert_eq!(parsed.chapters[0].start_ms, 0);
        assert!(parsed.chapters[0].title.is_none());
        assert_eq!(parsed.chapters[1].start_ms, 12034);
        assert_eq!(parsed.chapters[1].title.as_deref(), Some("Harbor"));
    }
}

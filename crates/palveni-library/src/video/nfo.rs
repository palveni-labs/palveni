use quick_xml::{Reader, events::Event};
use std::io::Read;
use std::path::Path;

use super::{MAX_NFO, storage_path};
use crate::LocalStorage;

#[derive(Default, Clone, Debug)]
pub(super) struct Nfo {
    pub root: String,
    pub title: Option<String>,
    pub show_title: Option<String>,
    pub plot: Option<String>,
    pub year: Option<i32>,
    pub season: Option<i32>,
    pub episode: Option<i32>,
}

pub(super) fn number(value: &str) -> Option<i32> {
    value
        .trim()
        .parse::<i32>()
        .ok()
        .filter(|v| (0..=65_535).contains(v))
}

impl Nfo {
    fn set(&mut self, key: &str, text: &str) {
        let text = text.trim();
        if text.is_empty() {
            return;
        }
        match key {
            "title" => self.title = Some(text.chars().take(512).collect()),
            "showtitle" => self.show_title = Some(text.chars().take(512).collect()),
            "plot" => self.plot = Some(text.chars().take(4096).collect()),
            "year" => self.year = number(text).filter(|n| (1000..=9999).contains(n)),
            "premiered" | "aired" if self.year.is_none() => {
                self.year = text
                    .get(..4)
                    .and_then(number)
                    .filter(|n| (1000..=9999).contains(n))
            }
            "season" => self.season = number(text),
            "episode" => self.episode = number(text),
            _ => {}
        }
    }
}

pub(super) fn parse(text: &str) -> Option<Nfo> {
    let mut reader = Reader::from_str(text.trim_start_matches('\u{feff}'));
    let mut nfo = Nfo::default();
    let mut depth = 0usize;
    let mut key = String::new();
    let mut value = String::new();
    loop {
        match reader.read_event().ok()? {
            Event::Start(tag) => {
                depth += 1;
                if depth > 64 {
                    return None;
                }
                if depth == 1 {
                    if !nfo.root.is_empty() {
                        return None;
                    }
                    nfo.root = tag.local_name().as_ref().to_ascii_lowercase();
                    if !matches!(nfo.root.as_str(), "movie" | "tvshow" | "episodedetails") {
                        return None;
                    }
                } else if depth == 2 {
                    key = tag.local_name().as_ref().to_ascii_lowercase();
                    value.clear();
                }
            }
            Event::End(_) => {
                if depth == 2 {
                    nfo.set(&key, &value);
                }
                depth = depth.checked_sub(1)?;
            }
            Event::Text(event) if depth == 2 => value.push_str(&event.xml10_content()),
            Event::CData(event) if depth == 2 => value.push_str(&event.xml10_content()),
            Event::GeneralRef(event) if depth == 2 => {
                if let Some(c) = event.resolve_char_ref().ok()? {
                    value.push(c);
                } else {
                    value.push(match &*event {
                        "amp" => '&',
                        "apos" => '\'',
                        "lt" => '<',
                        "gt" => '>',
                        "quot" => '"',
                        _ => return None,
                    });
                }
            }
            Event::DocType(_) => return None,
            Event::Eof => return (depth == 0 && !nfo.root.is_empty()).then_some(nfo),
            _ => {}
        }
    }
}

pub(super) fn read(root: &LocalStorage, absolute: &Path) -> Option<Nfo> {
    let path = storage_path(root, absolute)?;
    let file = root.open_file_blocking(&path).ok()?;
    if file.metadata().ok()?.len() > MAX_NFO {
        return None;
    }
    let mut text = String::new();
    file.take(MAX_NFO + 1).read_to_string(&mut text).ok()?;
    if text.len() as u64 > MAX_NFO {
        return None;
    }
    parse(&text)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reads_direct_fields_and_bounded_xml() {
        let n=parse("<movie><title>A &amp; B</title><actor><title>Other</title></actor><plot><![CDATA[Story <1>]]></plot><year>2021</year></movie>").unwrap();
        assert_eq!(n.title.as_deref(), Some("A & B"));
        assert_eq!(n.plot.as_deref(), Some("Story <1>"));
        assert_eq!(n.year, Some(2021));
        assert!(parse("<!DOCTYPE movie><movie><title>A</title></movie>").is_none());
        assert!(parse("<movie><title>x").is_none());
    }
}

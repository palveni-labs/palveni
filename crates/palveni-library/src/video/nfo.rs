//! Kodi / Emby / Jellyfin `.nfo` files. Only local fields are read; URLs are
//! kept as raw strings and resolved (or ignored) by the caller.
use palveni_core::video::VideoPersonKind;
use quick_xml::{Reader, events::Event};
use std::io::Read;
use std::path::Path;

use super::{MAX_NFO, storage_path};
use crate::LocalStorage;

const MAX_PEOPLE: usize = 100;
const MAX_LIST: usize = 32;

#[derive(Clone, Debug, PartialEq)]
pub(super) struct Person {
    pub name: String,
    pub role: Option<String>,
    pub kind: VideoPersonKind,
    pub thumb: Option<String>,
    pub order: Option<i32>,
}

#[derive(Default, Clone, Debug)]
pub(super) struct Nfo {
    pub root: String,
    pub title: Option<String>,
    pub show_title: Option<String>,
    pub original_title: Option<String>,
    pub tagline: Option<String>,
    pub plot: Option<String>,
    pub year: Option<i32>,
    pub season: Option<i32>,
    pub episode: Option<i32>,
    pub genres: Vec<String>,
    pub studios: Vec<String>,
    pub runtime_ms: Option<i64>,
    /// 0–10.
    pub rating: Option<f64>,
    pub official_rating: Option<String>,
    /// `YYYY-MM-DD`.
    pub premiered: Option<String>,
    pub people: Vec<Person>,
}

pub(super) fn number(value: &str) -> Option<i32> {
    value
        .trim()
        .parse::<i32>()
        .ok()
        .filter(|v| (0..=65_535).contains(v))
}

fn bounded(text: &str, max: usize) -> String {
    text.chars().take(max).collect()
}

fn leading_number(text: &str) -> Option<i64> {
    let digits: String = text
        .trim()
        .chars()
        .take_while(char::is_ascii_digit)
        .take(9)
        .collect();
    digits.parse().ok()
}

fn date(text: &str) -> Option<String> {
    let head = text.trim().get(..10)?;
    let b = head.as_bytes();
    let shape = b.iter().enumerate().all(|(i, c)| match i {
        4 | 7 => *c == b'-',
        _ => c.is_ascii_digit(),
    });
    let month = number(&head[5..7]).filter(|m| (1..=12).contains(m));
    let day = number(&head[8..10]).filter(|d| (1..=31).contains(d));
    (shape && month.is_some() && day.is_some()).then(|| head.to_owned())
}

/// `Rated PG-13`, `US:PG-13` and `PG-13` all become `PG-13`.
fn certification(text: &str) -> Option<String> {
    let text = text.trim();
    let text = text
        .strip_prefix("Rated ")
        .or_else(|| text.strip_prefix("rated "))
        .unwrap_or(text);
    let text = text.rsplit(':').next().unwrap_or(text).trim();
    (!text.is_empty()).then(|| bounded(text, 32))
}

fn rating(text: &str, max: f64) -> Option<f64> {
    let value: f64 = text.trim().replace(',', ".").parse().ok()?;
    let max = if max > 0.0 { max } else { 10.0 };
    let scaled = value * 10.0 / max;
    (scaled.is_finite() && (0.0..=10.0).contains(&scaled)).then(|| (scaled * 10.0).round() / 10.0)
}

fn push_unique(list: &mut Vec<String>, text: &str, max_len: usize) {
    // Kodi writes one value per element; some tools join them with " / ".
    for part in text.split(" / ").flat_map(|p| p.split('|')) {
        let part = part.trim();
        if part.is_empty() || list.len() >= MAX_LIST {
            continue;
        }
        let part = bounded(part, max_len);
        if !list.iter().any(|v| v.eq_ignore_ascii_case(&part)) {
            list.push(part);
        }
    }
}

#[derive(Default)]
struct Rating {
    max: f64,
    default: bool,
    value: Option<f64>,
    raw: Option<String>,
}

#[derive(Default)]
struct State {
    nfo: Nfo,
    actor: Option<Person>,
    rating: Option<Rating>,
    ratings: Vec<(bool, f64)>,
    direct_rating: Option<f64>,
    duration_seconds: Option<i64>,
}

impl State {
    fn person(&mut self, kind: VideoPersonKind, text: &str) {
        for name in text.split(" / ") {
            let name = name.trim();
            if name.is_empty() || self.nfo.people.len() >= MAX_PEOPLE {
                continue;
            }
            if self
                .nfo
                .people
                .iter()
                .any(|p| p.kind == kind && p.name == name)
            {
                continue;
            }
            self.nfo.people.push(Person {
                name: bounded(name, 256),
                role: None,
                kind,
                thumb: None,
                order: None,
            });
        }
    }

    fn field(&mut self, key: &str, text: &str) {
        let nfo = &mut self.nfo;
        match key {
            "title" => nfo.title = Some(bounded(text, 512)),
            "showtitle" => nfo.show_title = Some(bounded(text, 512)),
            "originaltitle" => nfo.original_title = Some(bounded(text, 512)),
            "tagline" => nfo.tagline = Some(bounded(text, 1024)),
            "plot" => nfo.plot = Some(bounded(text, 4096)),
            "outline" if nfo.plot.is_none() => nfo.plot = Some(bounded(text, 4096)),
            "year" => nfo.year = number(text).filter(|n| (1000..=9999).contains(n)),
            "premiered" | "aired" | "releasedate" => {
                if let Some(day) = date(text) {
                    if nfo.year.is_none() {
                        nfo.year = number(&day[..4]).filter(|n| (1000..=9999).contains(n));
                    }
                    if key != "releasedate" || nfo.premiered.is_none() {
                        nfo.premiered = Some(day);
                    }
                }
            }
            "season" => nfo.season = number(text),
            "episode" => nfo.episode = number(text),
            "genre" => push_unique(&mut nfo.genres, text, 128),
            "studio" => push_unique(&mut nfo.studios, text, 256),
            "runtime" => {
                nfo.runtime_ms = leading_number(text)
                    .filter(|m| (1..=60 * 24 * 7).contains(m))
                    .map(|m| m * 60_000);
            }
            "rating" => self.direct_rating = rating(text, 10.0),
            "mpaa" | "certification" if nfo.official_rating.is_none() => {
                nfo.official_rating = certification(text);
            }
            "director" => self.person(VideoPersonKind::Director, text),
            "credits" | "writer" => self.person(VideoPersonKind::Writer, text),
            "producer" => self.person(VideoPersonKind::Producer, text),
            _ => {}
        }
    }

    fn actor_field(&mut self, key: &str, text: &str) {
        let Some(actor) = self.actor.as_mut() else {
            return;
        };
        match key {
            "name" => actor.name = bounded(text, 256),
            "role" => actor.role = Some(bounded(text, 256)),
            "thumb" => actor.thumb = Some(bounded(text, 1024)),
            "order" | "sortorder" => actor.order = number(text),
            "type" => {
                actor.kind = match text.to_ascii_lowercase().as_str() {
                    "director" => VideoPersonKind::Director,
                    "writer" => VideoPersonKind::Writer,
                    "producer" => VideoPersonKind::Producer,
                    _ => VideoPersonKind::Actor,
                }
            }
            _ => {}
        }
    }

    fn finish_actor(&mut self) {
        let Some(actor) = self.actor.take() else {
            return;
        };
        if actor.name.trim().is_empty() || self.nfo.people.len() >= MAX_PEOPLE {
            return;
        }
        if !self
            .nfo
            .people
            .iter()
            .any(|p| p.kind == actor.kind && p.name == actor.name)
        {
            self.nfo.people.push(actor);
        }
    }

    fn end(&mut self, path: &[String], text: &str) {
        let text = text.trim();
        let keys: Vec<&str> = path.iter().skip(1).map(String::as_str).collect();
        match keys.as_slice() {
            ["actor"] => self.finish_actor(),
            ["actor", key] if !text.is_empty() => self.actor_field(key, text),
            ["ratings", "rating"] => {
                if let Some(r) = self.rating.take() {
                    // Some writers put the score directly inside <rating>.
                    let value = r
                        .value
                        .or_else(|| r.raw.as_deref().and_then(|t| rating(t, r.max)));
                    if let Some(value) = value {
                        self.ratings.push((r.default, value));
                    }
                }
            }
            ["ratings", "rating", "value"] => {
                if let Some(r) = self.rating.as_mut() {
                    r.value = rating(text, r.max);
                }
            }
            ["fileinfo", "streamdetails", "video", "durationinseconds"] => {
                self.duration_seconds = leading_number(text).filter(|s| *s > 0);
            }
            [key] if !text.is_empty() => self.field(key, text),
            _ => {}
        }
    }

    fn finish(mut self) -> Nfo {
        self.nfo.rating = self
            .ratings
            .iter()
            .find(|(default, _)| *default)
            .or_else(|| self.ratings.first())
            .map(|(_, v)| *v)
            .or(self.direct_rating);
        if self.nfo.runtime_ms.is_none() {
            self.nfo.runtime_ms = self
                .duration_seconds
                .filter(|s| *s <= 7 * 24 * 3600)
                .map(|s| s * 1000);
        }
        // Cast before crew; within a kind, explicit <order> first, then file order.
        let mut people: Vec<_> = self.nfo.people.into_iter().enumerate().collect();
        people.sort_by_key(|(i, p)| (p.kind as u8, p.order.is_none(), p.order, *i));
        self.nfo.people = people.into_iter().map(|(_, p)| p).collect();
        self.nfo
    }
}

fn attribute(tag: &quick_xml::events::BytesStart<'_>, name: &str) -> Option<String> {
    tag.attributes()
        .flatten()
        .find(|a| a.key.local_name().as_ref().eq_ignore_ascii_case(name))
        .and_then(|a| {
            a.normalized_value(quick_xml::XmlVersion::Implicit1_0)
                .ok()
                .map(|v| v.into_owned())
        })
}

pub(super) fn parse(text: &str) -> Option<Nfo> {
    let mut reader = Reader::from_str(text.trim_start_matches('\u{feff}'));
    let mut state = State::default();
    let mut path: Vec<String> = Vec::new();
    let mut value = String::new();
    loop {
        match reader.read_event().ok()? {
            Event::Start(tag) => {
                if path.len() >= 64 {
                    return None;
                }
                let name = tag.local_name().as_ref().to_ascii_lowercase();
                if path.is_empty() {
                    if !state.nfo.root.is_empty() {
                        // Multi-episode files repeat the root; keep the first.
                        break;
                    }
                    if !matches!(name.as_str(), "movie" | "tvshow" | "episodedetails") {
                        return None;
                    }
                    state.nfo.root = name.clone();
                }
                path.push(name);
                value.clear();
                match path.iter().skip(1).map(String::as_str).collect::<Vec<_>>()[..] {
                    ["actor"] => {
                        state.actor = Some(Person {
                            name: String::new(),
                            role: None,
                            kind: VideoPersonKind::Actor,
                            thumb: None,
                            order: None,
                        });
                    }
                    ["ratings", "rating"] => {
                        state.rating = Some(Rating {
                            max: attribute(&tag, "max")
                                .and_then(|m| m.trim().parse().ok())
                                .unwrap_or(10.0),
                            default: attribute(&tag, "default")
                                .is_some_and(|d| d.eq_ignore_ascii_case("true")),
                            ..Rating::default()
                        });
                    }
                    _ => {}
                }
            }
            Event::End(_) => {
                if path.is_empty() {
                    return None;
                }
                let text = std::mem::take(&mut value);
                if path.len() == 3
                    && path[1] == "ratings"
                    && path[2] == "rating"
                    && let Some(r) = state.rating.as_mut()
                    && r.value.is_none()
                    && !text.trim().is_empty()
                {
                    r.raw = Some(text.clone());
                }
                state.end(&path, &text);
                path.pop();
            }
            Event::Text(event) if !path.is_empty() => value.push_str(&event.xml10_content()),
            Event::CData(event) if !path.is_empty() => value.push_str(&event.xml10_content()),
            Event::GeneralRef(event) if !path.is_empty() => {
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
            Event::Eof => {
                if !path.is_empty() || state.nfo.root.is_empty() {
                    return None;
                }
                break;
            }
            _ => {}
        }
    }
    Some(state.finish())
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

    #[test]
    fn reads_kodi_movie_fixture() {
        let n = parse(include_str!("fixtures/kodi-movie.nfo")).unwrap();
        assert_eq!(n.root, "movie");
        assert_eq!(n.title.as_deref(), Some("The Lighthouse Keeper"));
        assert_eq!(n.original_title.as_deref(), Some("Le Gardien du Phare"));
        assert_eq!(n.tagline.as_deref(), Some("Every light hides a shadow."));
        assert_eq!(n.genres, ["Drama", "Mystery", "Thriller"]);
        assert_eq!(n.studios, ["North Pier Films", "Atlantic Pictures"]);
        assert_eq!(n.runtime_ms, Some(118 * 60_000));
        assert_eq!(n.rating, Some(7.8));
        assert_eq!(n.official_rating.as_deref(), Some("PG-13"));
        assert_eq!(n.premiered.as_deref(), Some("2019-10-18"));
        assert_eq!(n.year, Some(2019));
        let actors: Vec<_> = n
            .people
            .iter()
            .filter(|p| p.kind == VideoPersonKind::Actor)
            .map(|p| (p.name.as_str(), p.role.as_deref(), p.thumb.as_deref()))
            .collect();
        assert_eq!(
            actors,
            [
                ("Ana Ruiz", Some("Marie"), Some(".actors/Ana_Ruiz.jpg")),
                ("Tom Weller", Some("Keeper"), None),
                ("Jo Park", None, Some("https://image.example/jo.jpg")),
            ]
        );
        let crew: Vec<_> = n
            .people
            .iter()
            .filter(|p| p.kind != VideoPersonKind::Actor)
            .map(|p| (p.kind, p.name.as_str()))
            .collect();
        assert_eq!(
            crew,
            [
                (VideoPersonKind::Director, "Lena Morel"),
                (VideoPersonKind::Writer, "Lena Morel"),
                (VideoPersonKind::Writer, "Paul Grant"),
                (VideoPersonKind::Producer, "Mia Holt"),
            ]
        );
    }

    #[test]
    fn reads_jellyfin_and_legacy_shapes() {
        let n = parse(include_str!("fixtures/jellyfin-tvshow.nfo")).unwrap();
        assert_eq!(n.root, "tvshow");
        assert_eq!(n.genres, ["Sci-Fi & Fantasy", "Animation", "Comedy"]);
        assert_eq!(n.rating, Some(8.5));
        assert_eq!(n.official_rating.as_deref(), Some("TV-14"));
        assert_eq!(n.premiered.as_deref(), Some("2014-04-06"));
        assert_eq!(n.people[0].name, "Rae Kim");
        assert_eq!(n.people[0].order, Some(0));
        assert!(
            n.people
                .iter()
                .any(|p| p.kind == VideoPersonKind::Director && p.name == "Sam Holt")
        );

        let ep = parse(include_str!("fixtures/kodi-episode.nfo")).unwrap();
        assert_eq!(ep.root, "episodedetails");
        assert_eq!((ep.season, ep.episode), (Some(1), Some(2)));
        assert_eq!(ep.title.as_deref(), Some("Second Light"));
        assert_eq!(ep.premiered.as_deref(), Some("2014-04-13"));
        assert_eq!(ep.runtime_ms, Some(1_412_000));
        assert_eq!(ep.rating, Some(7.0));
        assert_eq!(
            ep.people
                .iter()
                .map(|p| (p.kind, p.name.as_str()))
                .collect::<Vec<_>>(),
            [
                (VideoPersonKind::Director, "Ola Berg"),
                (VideoPersonKind::Writer, "Ina Lund")
            ]
        );
    }

    #[test]
    fn normalizes_values() {
        assert_eq!(certification("Rated R").as_deref(), Some("R"));
        assert_eq!(certification("Germany:12").as_deref(), Some("12"));
        assert_eq!(rating("86", 100.0), Some(8.6));
        assert_eq!(rating("11", 10.0), None);
        assert_eq!(date("2020-02-30").as_deref(), Some("2020-02-30"));
        assert!(date("2020-13-01").is_none());
        assert!(date("20200101").is_none());
        let n = parse("<movie><rating>6,4</rating><runtime>95 min</runtime><genre>Action / Comedy</genre><genre>action</genre></movie>\nhttps://www.themoviedb.org/movie/1").unwrap();
        assert_eq!(n.rating, Some(6.4));
        assert_eq!(n.runtime_ms, Some(95 * 60_000));
        assert_eq!(n.genres, ["Action", "Comedy"]);
        let multi = parse("<episodedetails><title>One</title><episode>1</episode></episodedetails><episodedetails><title>Two</title><episode>2</episode></episodedetails>").unwrap();
        assert_eq!(
            (multi.title.as_deref(), multi.episode),
            (Some("One"), Some(1))
        );
    }
}

use palveni_core::{
    Error, LibraryId, Result,
    music::{GlobalMusicSearchHit, MusicSearchKind},
};
use palveni_db::{Db, music::search::SearchLibrary};
use serde::{Deserialize, Serialize};
use tokio::task::spawn_blocking;

use super::{Entry, Index, MusicSearch, join_error};

#[derive(Debug, Deserialize, Serialize)]
pub struct GlobalCursor {
    stamp: String,
    library: LibraryId,
    position: usize,
}

pub struct GlobalMatches {
    pub entries: Vec<GlobalMusicSearchHit>,
    pub next: Option<GlobalCursor>,
}

struct RankedHit {
    title_key: String,
    position: usize,
    result: GlobalMusicSearchHit,
}

impl RankedHit {
    fn new(entry: &Entry, position: usize, source: &SearchLibrary) -> Self {
        Self {
            title_key: entry.sort_key().0.to_owned(),
            position,
            result: GlobalMusicSearchHit {
                library_id: source.id,
                library_name: source.name.clone(),
                hit: entry.hit.clone(),
            },
        }
    }

    fn key(&self) -> ((&str, Option<&str>, &str), LibraryId) {
        let hit = &self.result.hit;
        (
            (&self.title_key, hit.artist_id.as_deref(), &hit.id),
            self.result.library_id,
        )
    }
}

struct Page {
    kind: MusicSearchKind,
    terms: Vec<String>,
    limit: usize,
    after: Option<RankedHit>,
    hits: Vec<RankedHit>,
}

impl Page {
    fn merge(
        &mut self,
        index: &Index,
        source: &SearchLibrary,
        anchor: Option<usize>,
    ) -> Result<()> {
        let rows = index.rows(self.kind);
        if let Some(position) = anchor {
            let entry = rows
                .get(position)
                .filter(|e| e.contains_terms(&self.terms))
                .ok_or_else(|| Error::invalid("Invalid music search position"))?;
            self.after = Some(RankedHit::new(entry, position, source));
        }

        // Seek directly past the preceding page even when many libraries share a group key.
        let start = self.after.as_ref().map_or(0, |after| {
            rows.partition_point(|row| (row.sort_key(), source.id) <= after.key())
        });
        let end = if self.hits.len() > self.limit {
            let cutoff = self.hits[self.limit].key();
            rows.partition_point(|row| (row.sort_key(), source.id) < cutoff)
        } else {
            rows.len()
        };
        if start >= end {
            return Ok(());
        }
        self.hits.extend(
            rows[start..end]
                .iter()
                .enumerate()
                .filter(|(_, row)| row.contains_terms(&self.terms))
                .take(self.limit + 1)
                .map(|(i, row)| RankedHit::new(row, start + i, source)),
        );
        self.hits.sort_unstable_by(|a, b| a.key().cmp(&b.key()));
        self.hits.truncate(self.limit + 1);
        Ok(())
    }

    fn finish(mut self, stamp: String) -> GlobalMatches {
        let next = if self.hits.len() > self.limit {
            self.hits.truncate(self.limit);
            self.hits.last().map(|row| GlobalCursor {
                stamp,
                library: row.result.library_id,
                position: row.position,
            })
        } else {
            None
        };
        GlobalMatches {
            entries: self.hits.into_iter().map(|row| row.result).collect(),
            next,
        }
    }
}

impl MusicSearch {
    pub async fn search_all(
        &self,
        db: &Db,
        kind: MusicSearchKind,
        query: Vec<String>,
        limit: usize,
        cursor: Option<GlobalCursor>,
    ) -> Result<GlobalMatches> {
        let _permit = self
            .readers
            .acquire()
            .await
            .map_err(|e| Error::internal(e.to_string()))?;
        let mut catalog = db.music().search_catalog().await?;
        // Read hot libraries before a cold load can evict them.
        {
            let cached = self.cached.lock().await;
            catalog.libraries.sort_unstable_by_key(|source| {
                !cached.iter().any(|(index, _)| {
                    index.library == source.id && index.revision == source.revision
                })
            });
        }
        if let Some(cursor) = &cursor {
            if cursor.stamp.len() != 64 || !cursor.stamp.bytes().all(|c| c.is_ascii_hexdigit()) {
                return Err(Error::invalid("Invalid music search revision"));
            }
            if cursor.stamp != catalog.stamp {
                return Err(Error::conflict(
                    "Music libraries changed; restart the search",
                ));
            }
            let i = catalog
                .libraries
                .iter()
                .position(|l| l.id == cursor.library)
                .ok_or_else(|| Error::invalid("Invalid music search library"))?;
            // Recover the complete sort key from the index; even long tags keep a small cursor.
            catalog.libraries.swap(0, i);
        }
        let mut page = Page {
            kind,
            terms: query,
            limit: limit.clamp(1, 200),
            after: None,
            hits: Vec::new(),
        };
        for source in catalog.libraries {
            let index = self
                .index(db, source.id, source.revision)
                .await
                .map_err(|error| match error {
                    Error::NotFound(_) => {
                        Error::conflict("Music libraries changed; restart the search")
                    }
                    other => other,
                })?;
            let anchor = cursor
                .as_ref()
                .filter(|c| c.library == source.id)
                .map(|c| c.position);
            page = spawn_blocking(move || {
                page.merge(&index, &source, anchor)?;
                Ok::<_, Error>(page)
            })
            .await
            .map_err(join_error)??;
        }
        Ok(page.finish(catalog.stamp))
    }
}

#[cfg(test)]
mod tests {
    use super::super::{Builder, terms, tests::track};
    use super::*;

    #[test]
    fn global_pages_interleave_libraries_and_keep_duplicate_album_groups() {
        let mut sources = Vec::new();
        for (n, (name, albums)) in [
            ("A", ["Zinc", "海邊"]),
            ("B", ["海邊", "Amber"]),
            ("C", ["Leaf", "海邊"]),
        ]
        .into_iter()
        .enumerate()
        {
            let id = format!("00000000-0000-7000-8000-{n:012}").parse().unwrap();
            let source = SearchLibrary {
                id,
                name: name.into(),
                revision: 1,
            };
            let mut builder = Builder::default();
            builder.extend(
                albums
                    .into_iter()
                    .map(|album| track(album, "Cue", "Artist", album))
                    .collect(),
            );
            sources.push((builder.finish(source.id, 1), source));
        }
        let expected = [
            ("Amber", "B"),
            ("海邊", "A"),
            ("海邊", "B"),
            ("海邊", "C"),
            ("Leaf", "C"),
            ("Zinc", "A"),
        ]
        .map(|(album, name)| (album.to_owned(), name.to_owned()));
        for limit in 1..=6 {
            let mut cursor: Option<GlobalCursor> = None;
            let mut seen = Vec::new();
            loop {
                let mut page = Page {
                    kind: MusicSearchKind::Album,
                    terms: terms("artist").unwrap(),
                    limit,
                    after: None,
                    hits: Vec::new(),
                };
                // Start with the anchor as the real handler does; other shards may be processed in any order.
                sources.reverse();
                if let Some(c) = &cursor {
                    let i = sources.iter().position(|(_, s)| s.id == c.library).unwrap();
                    sources.swap(0, i);
                }
                for (index, source) in &sources {
                    page.merge(
                        index,
                        source,
                        cursor
                            .as_ref()
                            .filter(|c| c.library == source.id)
                            .map(|c| c.position),
                    )
                    .unwrap();
                }
                let result = page.finish("stamp".into());
                seen.extend(
                    result
                        .entries
                        .into_iter()
                        .map(|r| (r.hit.title, r.library_name)),
                );
                assert!(seen.len() <= expected.len());
                cursor = result.next;
                if cursor.is_none() {
                    break;
                }
            }
            assert_eq!(seen, expected, "limit {limit}");
        }
    }
}

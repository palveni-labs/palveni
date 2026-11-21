use palveni_core::{
    LibraryId, LibraryKind,
    music::MusicEntry,
    spoken::{SpokenEntry, SpokenPage, SpokenRecording},
};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QuerySelect};

use super::{Music, MusicCursor, sort_prefix};
use crate::{DbError, convert::parse_id, entities::libraries};

// A spoken library has one shelf, which occupies the existing album index's artist column.
pub(super) const SHELF: &str = "spoken";

pub(super) fn natural_sort(title: &str) -> String {
    let mut out = String::new();
    let lower = title.to_lowercase();
    let mut chars = lower.chars().peekable();
    while let Some(c) = chars.next() {
        if c.is_ascii_digit() {
            let mut digits = String::from(c);
            while chars.peek().is_some_and(char::is_ascii_digit) {
                digits.extend(chars.next());
            }
            let value = digits.trim_start_matches('0');
            out.push_str(&format!("0{:04}{value}", value.len()));
        } else {
            out.push(c);
        }
        if out.len() >= 96 {
            break;
        }
    }
    sort_prefix(&out)
}

impl MusicCursor {
    pub fn decode_spoken(
        value: &str,
        library: LibraryId,
        collection: Option<&str>,
    ) -> Option<Self> {
        Self::decode(value, library, Some(SHELF), collection)
    }
}

impl Music<'_> {
    pub async fn browse_spoken(
        &self,
        library: LibraryId,
        collection: Option<&str>,
        limit: u32,
        cursor: Option<MusicCursor>,
    ) -> Result<Option<SpokenPage>, DbError> {
        let kind = libraries::Entity::find_by_id(library.to_string())
            .select_only()
            .column(libraries::Column::Kind)
            .filter(libraries::Column::Kind.is_in(["audiobook", "podcast"]))
            .filter(libraries::Column::DeletedAt.is_null())
            .into_tuple::<String>()
            .one(self.db.main())
            .await?;
        let Some(kind) = kind else { return Ok(None) };
        let kind: LibraryKind = parse_id(&kind, "spoken library kind")?;
        let page = self
            .browse(library, Some(SHELF), collection, limit, cursor)
            .await?;
        let mut entries = Vec::with_capacity(page.entries.len());
        for entry in page.entries {
            entries.push(match entry {
                MusicEntry::Album { id, name, .. } => {
                    let title = name.ok_or_else(|| {
                        DbError::Corrupt("missing spoken collection title".into())
                    })?;
                    if kind == LibraryKind::Audiobook {
                        SpokenEntry::Book { id, title }
                    } else {
                        SpokenEntry::Show { id, title }
                    }
                }
                MusicEntry::Track {
                    id,
                    title,
                    artist,
                    disc_number,
                    track_number,
                    duration_ms,
                } => {
                    let audio = SpokenRecording {
                        id,
                        title,
                        artist,
                        disc_number,
                        number: track_number,
                        duration_ms,
                    };
                    if kind == LibraryKind::Audiobook {
                        SpokenEntry::Chapter(audio)
                    } else {
                        SpokenEntry::Episode(audio)
                    }
                }
                MusicEntry::Artist { .. } => {
                    return Err(DbError::Corrupt("unexpected artist on spoken shelf".into()));
                }
            });
        }
        Ok(Some(SpokenPage {
            entries,
            next_cursor: page.next_cursor,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chapter_sort_uses_numbers_wherever_they_appear() {
        assert!(natural_sort("第2章") < natural_sort("第12章"));
        assert_eq!(natural_sort("PART 002"), natural_sort("Part 2"));
    }
}

//! Album-artist, album and track views of the live music index.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use palveni_core::{
    ItemId, ItemKind, LibraryId,
    music::{MusicEntry, MusicPage, TrackTags},
};
use sea_orm::sea_query::OnConflict;
use sea_orm::{
    ColumnTrait, Condition, DatabaseTransaction, EntityTrait, JoinType, QueryFilter, QueryOrder,
    QuerySelect, Set,
};
use serde::{Deserialize, Serialize};

use crate::{
    Db, DbError,
    convert::{parse_id, sha256_hex},
    entities::{
        items,
        track_metadata::{self as tracks, Column as C},
    },
    items::ItemUpsert,
};

pub struct Music<'a> {
    pub(crate) db: &'a Db,
}

pub struct TrackLocation {
    pub root: PathBuf,
    pub relative_path: PathBuf,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MusicCursor {
    scope: String,
    sort: String,
    id: String,
}

impl MusicCursor {
    pub fn decode(
        value: &str,
        library: LibraryId,
        artist: Option<&str>,
        album: Option<&str>,
    ) -> Option<Self> {
        if value.len() > 1024 {
            return None;
        }
        let cursor: Self = serde_json::from_slice(&hex::decode(value).ok()?).ok()?;
        (cursor.scope == scope_key(library, artist, album)).then_some(cursor)
    }
}

fn scope_key(library: LibraryId, artist: Option<&str>, album: Option<&str>) -> String {
    sha256_hex(&format!(
        "{library}/{}/{}",
        artist.unwrap_or(""),
        album.unwrap_or("")
    ))
}

// ASCII sort prefixes give the same ordering under all supported DB collations.
// The complete name hash or the file ID breaks ties, including long common prefixes.
fn sort_prefix(lowercase: &str) -> String {
    let bytes = lowercase.as_bytes();
    hex::encode(&bytes[..bytes.len().min(96)])
}

fn group(name: Option<&str>) -> (String, String) {
    let normalized = name.unwrap_or("").to_lowercase();
    (sha256_hex(&normalized), sort_prefix(&normalized))
}

fn model(
    item_id: String,
    library_id: LibraryId,
    fallback: &str,
    tags: &TrackTags,
) -> tracks::ActiveModel {
    let album_artist = tags.album_artist.as_ref().or(tags.artist.as_ref()).cloned();
    let (artist_key, artist_sort) = group(album_artist.as_deref());
    let (album_key, album_sort) = group(tags.album.as_deref());
    let title = tags.title.as_deref().unwrap_or(fallback);
    let track_sort = format!(
        "{:010}/{:010}/{}",
        tags.disc_number.unwrap_or(1),
        tags.track_number.unwrap_or(u32::MAX),
        sort_prefix(&title.to_lowercase())
    );
    tracks::ActiveModel {
        item_id: Set(item_id),
        library_id: Set(library_id.to_string()),
        artist_key: Set(artist_key),
        artist_sort: Set(artist_sort),
        album_artist: Set(album_artist),
        album_key: Set(album_key),
        album_sort: Set(album_sort),
        album: Set(tags.album.clone()),
        track_sort: Set(track_sort),
        title: Set(title.to_owned()),
        artist: Set(tags.artist.clone()),
        disc_number: Set(tags.disc_number.map(i64::from)),
        track_number: Set(tags.track_number.map(i64::from)),
        duration_ms: Set(tags.duration_ms),
    }
}

pub(crate) async fn update(
    txn: &DatabaseTransaction,
    library: LibraryId,
    upserts: &[ItemUpsert],
    removed: &[ItemId],
) -> Result<(), DbError> {
    let obsolete: Vec<_> = removed
        .iter()
        .copied()
        .chain(
            upserts
                .iter()
                .filter(|u| u.kind != ItemKind::Track)
                .map(|u| u.id),
        )
        .map(|id| id.to_string())
        .collect();
    if !obsolete.is_empty() {
        tracks::Entity::delete_many()
            .filter(C::LibraryId.eq(library.to_string()))
            .filter(C::ItemId.is_in(obsolete))
            .exec(txn)
            .await?;
    }
    let tagged: Vec<_> = upserts
        .iter()
        .filter_map(|u| u.music.as_ref().map(|tags| (sha256_hex(&u.path), u, tags)))
        .collect();
    if tagged.is_empty() {
        return Ok(());
    }
    // A concurrent scan may already have inserted the same file under another UUID.
    let ids: HashMap<String, String> = items::Entity::find()
        .select_only()
        .column(items::Column::PathHash)
        .column(items::Column::Id)
        .filter(items::Column::LibraryId.eq(library.to_string()))
        .filter(items::Column::PathHash.is_in(tagged.iter().map(|(hash, _, _)| hash.clone())))
        .into_tuple()
        .all(txn)
        .await?
        .into_iter()
        .collect();
    let mut rows = Vec::with_capacity(tagged.len());
    for (path, item, tags) in tagged {
        let id = ids
            .get(&path)
            .ok_or_else(|| DbError::Corrupt("missing indexed track".into()))?;
        rows.push(model(id.clone(), library, &item.title, tags));
    }
    tracks::Entity::insert_many(rows)
        .on_conflict(
            OnConflict::column(C::ItemId)
                .update_columns([
                    C::ArtistKey,
                    C::ArtistSort,
                    C::AlbumArtist,
                    C::AlbumKey,
                    C::AlbumSort,
                    C::Album,
                    C::TrackSort,
                    C::Title,
                    C::Artist,
                    C::DiscNumber,
                    C::TrackNumber,
                    C::DurationMs,
                ])
                .to_owned(),
        )
        .exec_without_returning(txn)
        .await?;
    Ok(())
}

impl Music<'_> {
    pub async fn has_library(&self, id: LibraryId) -> Result<bool, DbError> {
        use crate::entities::libraries;
        let row = libraries::Entity::find_by_id(id.to_string())
            .select_only()
            .column(libraries::Column::Id)
            .filter(libraries::Column::Kind.eq("music"))
            .filter(libraries::Column::DeletedAt.is_null())
            .into_tuple::<String>()
            .one(self.db.main())
            .await?;
        Ok(row.is_some())
    }

    /// Resolves one live track without counting or loading the rest of its library.
    pub async fn track_location(&self, id: ItemId) -> Result<Option<TrackLocation>, DbError> {
        use crate::entities::{libraries, library_paths};
        let rows = items::Entity::find_by_id(id.to_string())
            .join(
                JoinType::InnerJoin,
                items::Entity::belongs_to(libraries::Entity)
                    .from(items::Column::LibraryId)
                    .to(libraries::Column::Id)
                    .into(),
            )
            .join_rev(
                JoinType::InnerJoin,
                library_paths::Entity::belongs_to(libraries::Entity)
                    .from(library_paths::Column::LibraryId)
                    .to(libraries::Column::Id)
                    .into(),
            )
            .filter(items::Column::Kind.eq(ItemKind::Track.as_str()))
            .filter(items::Column::DeletedAt.is_null())
            .filter(items::Column::Path.is_not_null())
            .filter(libraries::Column::Kind.eq("music"))
            .filter(libraries::Column::DeletedAt.is_null())
            .select_only()
            .column_as(items::Column::Path, "file_path")
            .column_as(library_paths::Column::Path, "root_path")
            .into_tuple::<(String, String)>()
            .all(self.db.main())
            .await?;
        Ok(rows.into_iter().find_map(|(file, root)| {
            let relative_path = Path::new(&file).strip_prefix(&root).ok()?.to_owned();
            Some(TrackLocation {
                root: root.into(),
                relative_path,
            })
        }))
    }

    pub async fn indexed_item_ids(&self, library: LibraryId) -> Result<HashSet<String>, DbError> {
        Ok(tracks::Entity::find()
            .select_only()
            .column(C::ItemId)
            .filter(C::LibraryId.eq(library.to_string()))
            .into_tuple::<String>()
            .all(self.db.main())
            .await?
            .into_iter()
            .collect())
    }

    pub async fn browse(
        &self,
        library: LibraryId,
        artist: Option<&str>,
        album: Option<&str>,
        limit: u32,
        cursor: Option<MusicCursor>,
    ) -> Result<MusicPage, DbError> {
        let limit = limit.clamp(1, 200) as usize;
        let (order, key) = match (artist, album) {
            (_, Some(_)) => (C::TrackSort, C::ItemId),
            (Some(_), None) => (C::AlbumSort, C::AlbumKey),
            (None, None) => (C::ArtistSort, C::ArtistKey),
        };
        let mut query = tracks::Entity::find().filter(C::LibraryId.eq(library.to_string()));
        if let Some(a) = artist {
            query = query.filter(C::ArtistKey.eq(a));
        }
        if let Some(a) = album {
            query = query.filter(C::AlbumKey.eq(a));
        }
        if let Some(cursor) = cursor {
            query = query.filter(
                Condition::any().add(order.gt(cursor.sort.clone())).add(
                    Condition::all()
                        .add(order.eq(cursor.sort))
                        .add(key.gt(cursor.id)),
                ),
            );
        }
        let query = query
            .order_by_asc(order)
            .order_by_asc(key)
            .limit(limit as u64 + 1);
        let mut entries = Vec::new();
        let mut position = None;
        if album.is_some() {
            for row in query.all(self.db.main()).await? {
                let id = parse_id(&row.item_id, "track id")?;
                if entries.len() == limit - 1 {
                    position = Some((row.track_sort, row.item_id));
                }
                entries.push(MusicEntry::Track {
                    id,
                    title: row.title,
                    artist: row.artist,
                    disc_number: row.disc_number.and_then(|n| u32::try_from(n).ok()),
                    track_number: row.track_number.and_then(|n| u32::try_from(n).ok()),
                    duration_ms: row.duration_ms,
                });
            }
        } else {
            let name = if artist.is_some() {
                C::Album
            } else {
                C::AlbumArtist
            };
            let groups = query
                .select_only()
                .column(order)
                .column(key)
                .column_as(name.min(), "name")
                .group_by(order)
                .group_by(key)
                .into_tuple::<(String, String, Option<String>)>()
                .all(self.db.main())
                .await?;
            for (sort, id, name) in groups {
                if entries.len() == limit - 1 {
                    position = Some((sort, id.clone()));
                }
                entries.push(if artist.is_some() {
                    MusicEntry::Album { id, name }
                } else {
                    MusicEntry::Artist { id, name }
                });
            }
        }
        let next_cursor = if entries.len() > limit {
            entries.truncate(limit);
            position
                .and_then(|(sort, id)| {
                    serde_json::to_vec(&MusicCursor {
                        scope: scope_key(library, artist, album),
                        sort,
                        id,
                    })
                    .ok()
                })
                .map(hex::encode)
        } else {
            None
        };
        Ok(MusicPage {
            entries,
            next_cursor,
        })
    }
}

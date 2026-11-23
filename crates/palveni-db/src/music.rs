//! Album-artist, album and track views of the live music index.

pub mod catalog;
pub mod library;
pub mod loudness;
pub mod search;
mod spoken;

use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use palveni_core::{
    ItemId, ItemKind, LibraryId, LibraryKind,
    music::{MusicEntry, MusicPage, TrackTags},
};
use sea_orm::sea_query::{Expr, ExprTrait, Func, OnConflict};
use sea_orm::{
    ColumnTrait, Condition, DatabaseTransaction, EntityTrait, FromQueryResult, JoinType,
    QueryFilter, QueryOrder, QuerySelect, RelationTrait, Set, TransactionTrait,
};
use serde::{Deserialize, Serialize};

use crate::{
    Db, DbError,
    convert::{parse_id, sha256_hex},
    entities::{
        items, music_album_pending as pending, music_albums as albums, track_genres as genres,
        track_metadata::{self as tracks, Column as C},
    },
    items::ItemUpsert,
};

/// Current tag schema generation stored in `track_metadata.tags_version`.
/// Tracks indexed before this version have `NULL` and are re-read on the next scan.
pub const TAGS_VERSION: i64 = 1;

/// Mersenne prime `2^31 - 1` used for deterministic seeded shuffle permutations.
pub(crate) const SHUFFLE_MODULUS: i64 = 2_147_483_647;

pub struct IndexedTrack {
    pub cue_stamp: Option<String>,
    pub current: bool,
}

pub fn is_group_key(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

pub fn album_id(artist_key: &str, album_key: &str) -> String {
    format!("{artist_key}.{album_key}")
}

pub fn parse_album_id(raw: &str) -> Option<(&str, &str)> {
    let (artist, album) = raw.split_once('.')?;
    (is_group_key(artist) && is_group_key(album)).then_some((artist, album))
}

pub(crate) fn shuffle_value(input: &str) -> i64 {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(input.as_bytes());
    let raw = u32::from_be_bytes([digest[0], digest[1], digest[2], digest[3]]);
    i64::from(raw & 0x7fff_ffff) % SHUFFLE_MODULUS
}

pub struct Music<'a> {
    pub(crate) db: &'a Db,
}

pub struct TrackLocation {
    pub cue_frames: Option<std::ops::Range<u64>>,
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

pub(crate) fn visible_recordings() -> Condition {
    Condition::any()
        .add(C::CueStamp.is_null())
        .add(C::CueSource.is_not_null())
}

pub fn cue_fingerprint(text: &str) -> String {
    sha256_hex(text)
}

/// Retain the library UUIDv7 timestamp and derive the track-specific entropy from its stable key.
pub fn cue_id(library: LibraryId, key: &str) -> ItemId {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(format!("palveni:cue:v1\0{library}\0{key}").as_bytes());
    let mut bytes = *library.as_uuid().as_bytes();
    bytes[6..].copy_from_slice(&digest[..10]);
    bytes[6] = (bytes[6] & 0x0f) | 0x70;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    ItemId::from_uuid_bytes(bytes)
}

fn model(
    item_id: String,
    library_id: LibraryId,
    fallback: &str,
    tags: &TrackTags,
) -> (tracks::ActiveModel, String, String) {
    let album_artist = tags.album_artist.as_ref().or(tags.artist.as_ref()).cloned();
    let title = tags.title.as_deref().unwrap_or(fallback);
    let (mut album_key, album_sort) = group(tags.album.as_deref());
    let (artist_key, artist_sort, title_sort) = if let Some(dir) = &tags.spoken_location {
        album_key = sha256_hex(&format!("{dir}\0{album_key}"));
        (
            spoken::SHELF.to_owned(),
            String::new(),
            spoken::natural_sort(title),
        )
    } else {
        let (key, sort) = group(album_artist.as_deref());
        (key, sort, sort_prefix(&title.to_lowercase()))
    };
    let track_artist = tags.artist.as_deref().or(album_artist.as_deref());
    let track_artist_sort = sort_prefix(&track_artist.unwrap_or("").to_lowercase());
    let track_sort = format!(
        "{:010}/{:010}/{}",
        tags.disc_number.unwrap_or(1),
        tags.track_number.unwrap_or(u32::MAX),
        title_sort
    );
    let shuffle = shuffle_value(&format!("track\0{library_id}\0{item_id}"));
    let active = tracks::ActiveModel {
        item_id: Set(item_id),
        library_id: Set(library_id.to_string()),
        artist_key: Set(artist_key.clone()),
        artist_sort: Set(artist_sort),
        album_artist: Set(album_artist),
        album_key: Set(album_key.clone()),
        album_sort: Set(album_sort),
        album: Set(tags.album.clone()),
        track_sort: Set(track_sort),
        title: Set(title.to_owned()),
        artist: Set(tags.artist.clone()),
        disc_number: Set(tags.disc_number.map(i64::from)),
        track_number: Set(tags.track_number.map(i64::from)),
        duration_ms: Set(tags.duration_ms),
        cue_source: Set(tags.cue.as_ref().map(|c| c.source.clone())),
        cue_start: Set(tags.cue.as_ref().map(|c| c.start_frame)),
        cue_end: Set(tags.cue.as_ref().map(|c| c.end_frame)),
        cue_stamp: Set(if tags.cue_backing {
            Some("backing".into())
        } else {
            tags.cue.as_ref().map(|c| c.stamp.clone())
        }),
        year: Set(tags.year.map(i64::from)),
        codec: Set(tags.audio.codec.clone()),
        bitrate_kbps: Set(tags.audio.bitrate_kbps.map(i64::from)),
        sample_rate_hz: Set(tags.audio.sample_rate_hz.map(i64::from)),
        bit_depth: Set(tags.audio.bit_depth.map(i64::from)),
        channels: Set(tags.audio.channels.map(i64::from)),
        title_sort: Set(Some(title_sort)),
        track_artist_sort: Set(Some(track_artist_sort)),
        shuffle: Set(Some(shuffle)),
        tags_version: Set(Some(TAGS_VERSION)),
    };
    (active, artist_key, album_key)
}

async fn existing_albums_for(
    txn: &DatabaseTransaction,
    library: &str,
    item_ids: &[String],
    affected: &mut BTreeSet<(String, String)>,
) -> Result<(), DbError> {
    for chunk in item_ids.chunks(200) {
        let rows = tracks::Entity::find()
            .select_only()
            .columns([C::ArtistKey, C::AlbumKey])
            .filter(C::LibraryId.eq(library))
            .filter(C::ItemId.is_in(chunk.iter().cloned()))
            .filter(C::ArtistKey.ne(spoken::SHELF))
            .into_tuple::<(String, String)>()
            .all(txn)
            .await?;
        affected.extend(rows);
    }
    Ok(())
}

pub(crate) async fn update(
    txn: &DatabaseTransaction,
    library: LibraryId,
    upserts: &[ItemUpsert],
    removed: &[ItemId],
    rev: i64,
) -> Result<(), DbError> {
    let lib_str = library.to_string();
    let mut affected: BTreeSet<(String, String)> = BTreeSet::new();
    let obsolete: Vec<String> = removed
        .iter()
        .copied()
        .chain(upserts.iter().filter(|u| !u.kind.is_audio()).map(|u| u.id))
        .map(|id| id.to_string())
        .collect();
    if !obsolete.is_empty() {
        existing_albums_for(txn, &lib_str, &obsolete, &mut affected).await?;
        for chunk in obsolete.chunks(200) {
            genres::Entity::delete_many()
                .filter(genres::Column::LibraryId.eq(&lib_str))
                .filter(genres::Column::ItemId.is_in(chunk.iter().cloned()))
                .exec(txn)
                .await?;
            tracks::Entity::delete_many()
                .filter(C::LibraryId.eq(&lib_str))
                .filter(C::ItemId.is_in(chunk.iter().cloned()))
                .exec(txn)
                .await?;
        }
    }
    let tagged: Vec<_> = upserts
        .iter()
        .filter_map(|u| u.music.as_ref().map(|tags| (sha256_hex(&u.path), u, tags)))
        .collect();
    if !tagged.is_empty() {
        // A concurrent scan may already have inserted the same file under another UUID.
        let mut ids: HashMap<String, String> = HashMap::with_capacity(tagged.len());
        for chunk in tagged.chunks(200) {
            let chunk_ids = items::Entity::find()
                .select_only()
                .column(items::Column::PathHash)
                .column(items::Column::Id)
                .filter(items::Column::LibraryId.eq(&lib_str))
                .filter(
                    items::Column::PathHash.is_in(chunk.iter().map(|(hash, _, _)| hash.clone())),
                )
                .into_tuple::<(String, String)>()
                .all(txn)
                .await?;
            ids.extend(chunk_ids);
        }
        let target_ids: Vec<String> = tagged
            .iter()
            .map(|(path, _, _)| {
                ids.get(path)
                    .cloned()
                    .ok_or_else(|| DbError::Corrupt("missing indexed track".into()))
            })
            .collect::<Result<_, _>>()?;
        existing_albums_for(txn, &lib_str, &target_ids, &mut affected).await?;
        for chunk in target_ids.chunks(200) {
            genres::Entity::delete_many()
                .filter(genres::Column::LibraryId.eq(&lib_str))
                .filter(genres::Column::ItemId.is_in(chunk.iter().cloned()))
                .exec(txn)
                .await?;
        }
        let mut rows = Vec::with_capacity(tagged.len());
        let mut genre_rows = Vec::new();
        for ((_, item, tags), id) in tagged.into_iter().zip(target_ids) {
            let (active, artist_key, album_key) = model(id.clone(), library, &item.title, tags);
            rows.push(active);
            if artist_key != spoken::SHELF {
                affected.insert((artist_key.clone(), album_key.clone()));
                if item.kind == ItemKind::Track && !tags.cue_backing && !tags.genres.is_empty() {
                    let pair_id = album_id(&artist_key, &album_key);
                    let mut seen = BTreeSet::new();
                    for name in &tags.genres {
                        let trimmed = name.trim();
                        if trimmed.is_empty() {
                            continue;
                        }
                        let lower = trimmed.to_lowercase();
                        let g_key = sha256_hex(&lower);
                        if !seen.insert(g_key.clone()) {
                            continue;
                        }
                        let display: String = trimmed.chars().take(128).collect();
                        genre_rows.push(genres::ActiveModel {
                            item_id: Set(id.clone()),
                            genre_key: Set(g_key),
                            genre: Set(display),
                            genre_sort: Set(sort_prefix(&lower)),
                            library_id: Set(lib_str.clone()),
                            artist_key: Set(artist_key.clone()),
                            album_key: Set(album_key.clone()),
                            album_id: Set(pair_id.clone()),
                        });
                    }
                }
            }
        }
        // Keep bind parameters below SQLite's 999 limit (28 columns * 30 rows = 840).
        for chunk in rows.chunks(30) {
            tracks::Entity::insert_many(chunk.to_vec())
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
                            C::CueSource,
                            C::CueStart,
                            C::CueEnd,
                            C::CueStamp,
                            C::Year,
                            C::Codec,
                            C::BitrateKbps,
                            C::SampleRateHz,
                            C::BitDepth,
                            C::Channels,
                            C::TitleSort,
                            C::TrackArtistSort,
                            C::Shuffle,
                            C::TagsVersion,
                        ])
                        .to_owned(),
                )
                .exec_without_returning(txn)
                .await?;
        }
        // 8 columns * 100 rows = 800 bind parameters.
        for chunk in genre_rows.chunks(100) {
            genres::Entity::insert_many(chunk.to_vec())
                .exec_without_returning(txn)
                .await?;
        }
    }
    if !affected.is_empty() {
        let pending_rows: Vec<pending::ActiveModel> = affected
            .into_iter()
            .map(|(artist_key, album_key)| pending::ActiveModel {
                library_id: Set(lib_str.clone()),
                artist_key: Set(artist_key),
                album_key: Set(album_key),
                stamp: Set(rev),
            })
            .collect();
        // 4 columns * 200 rows = 800 bind parameters.
        for chunk in pending_rows.chunks(200) {
            pending::Entity::insert_many(chunk.to_vec())
                .on_conflict(
                    OnConflict::columns([
                        pending::Column::LibraryId,
                        pending::Column::ArtistKey,
                        pending::Column::AlbumKey,
                    ])
                    .update_column(pending::Column::Stamp)
                    .to_owned(),
                )
                .exec_without_returning(txn)
                .await?;
        }
    }
    Ok(())
}

pub(crate) fn display_title() -> sea_orm::sea_query::Expr {
    use sea_orm::sea_query::{Expr, Func};
    Func::coalesce([
        Expr::col((tracks::Entity, C::Title)),
        Expr::col((items::Entity, items::Column::Title)),
    ])
    .into()
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
        self.location_in(id, &[LibraryKind::Music]).await
    }

    /// Candidate tracks for albums where artwork was embedded in only one file.
    pub async fn artwork_siblings(&self, id: ItemId) -> Result<Vec<ItemId>, DbError> {
        let Some(track) = tracks::Entity::find_by_id(id.to_string())
            .filter(C::Album.is_not_null())
            .one(self.db.main())
            .await?
        else {
            return Ok(Vec::new());
        };
        let ids = tracks::Entity::find()
            .select_only()
            .column(C::ItemId)
            .filter(visible_recordings())
            .filter(C::LibraryId.eq(track.library_id))
            .filter(C::ArtistKey.eq(track.artist_key))
            .filter(C::AlbumKey.eq(track.album_key))
            .filter(C::ItemId.ne(id.to_string()))
            .order_by_asc(C::TrackSort)
            .order_by_asc(C::ItemId)
            .limit(2)
            .into_tuple::<String>()
            .all(self.db.main())
            .await?;
        ids.iter().map(|id| parse_id(id, "track id")).collect()
    }

    pub async fn audio_location(&self, id: ItemId) -> Result<Option<TrackLocation>, DbError> {
        self.location_in(
            id,
            &[
                LibraryKind::Music,
                LibraryKind::Audiobook,
                LibraryKind::Podcast,
            ],
        )
        .await
    }

    async fn location_in(
        &self,
        id: ItemId,
        kinds: &[LibraryKind],
    ) -> Result<Option<TrackLocation>, DbError> {
        use crate::entities::{libraries, library_paths};
        let allowed = kinds.iter().fold(Condition::any(), |filter, kind| {
            filter.add(
                Condition::all()
                    .add(items::Column::Kind.eq(kind.leaf_item_kind().as_str()))
                    .add(libraries::Column::Kind.eq(kind.as_str())),
            )
        });
        let rows = items::Entity::find_by_id(id.to_string())
            .join_rev(
                JoinType::LeftJoin,
                sea_orm::RelationTrait::def(&tracks::Relation::Item),
            )
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
            .filter(allowed)
            .filter(items::Column::DeletedAt.is_null())
            .filter(items::Column::Path.is_not_null())
            .filter(libraries::Column::DeletedAt.is_null())
            .select_only()
            .column_as(items::Column::Path, "file_path")
            .column_as(library_paths::Column::Path, "root_path")
            .columns([C::CueSource, C::CueStart, C::CueEnd])
            .into_tuple::<(String, String, Option<String>, Option<i64>, Option<i64>)>()
            .all(self.db.main())
            .await?;
        Ok(rows
            .into_iter()
            .find_map(|(file, root, source, start, end)| {
                let cue_frames = match (source.as_ref(), start, end) {
                    (Some(_), Some(s), Some(e)) if s >= 0 && e > s => {
                        Some(u64::try_from(s).ok()?..u64::try_from(e).ok()?)
                    }
                    (None, None, None) => None,
                    _ => return None,
                };
                let file = source.unwrap_or(file);
                let relative_path = Path::new(&file).strip_prefix(&root).ok()?.to_owned();
                Some(TrackLocation {
                    cue_frames,
                    root: root.into(),
                    relative_path,
                })
            }))
    }

    pub async fn indexed_item_ids(
        &self,
        library: LibraryId,
    ) -> Result<HashMap<String, IndexedTrack>, DbError> {
        Ok(tracks::Entity::find()
            .select_only()
            .columns([C::ItemId, C::CueStamp, C::TagsVersion])
            .filter(C::LibraryId.eq(library.to_string()))
            .into_tuple::<(String, Option<String>, Option<i64>)>()
            .all(self.db.main())
            .await?
            .into_iter()
            .map(|(id, cue_stamp, version)| {
                (
                    id,
                    IndexedTrack {
                        cue_stamp,
                        current: version == Some(TAGS_VERSION),
                    },
                )
            })
            .collect())
    }

    /// Recomputes any album aggregates marked dirty in `music_album_pending` for this library.
    pub async fn refresh_albums(&self, library: LibraryId) -> Result<(), DbError> {
        #[derive(FromQueryResult)]
        struct AlbumAggregate {
            artist_key: String,
            album_key: String,
            name: Option<String>,
            name_sort: String,
            artist: Option<String>,
            artist_sort: String,
            year: Option<i64>,
            track_count: String,
            disc_count: String,
            duration_ms: Option<String>,
            artwork_item_id: String,
            added_at: i64,
        }

        let lib_str = library.to_string();
        loop {
            let batch = pending::Entity::find()
                .filter(pending::Column::LibraryId.eq(&lib_str))
                .order_by_asc(pending::Column::ArtistKey)
                .order_by_asc(pending::Column::AlbumKey)
                .limit(100)
                .all(self.db.main())
                .await?;
            if batch.is_empty() {
                return Ok(());
            }
            let txn = self.db.main().begin().await?;
            // Acquire the SQLite write lock up front via `busy_timeout` before reading,
            // avoiding SQLITE_BUSY on read-to-write upgrade during concurrent scans.
            crate::revision::next_revision(&txn).await?;
            let pair_cond = |artist_col: C, album_col: C, pairs: &[(String, String)]| {
                pairs.iter().fold(Condition::any(), |cond, (ak, bk)| {
                    cond.add(
                        Condition::all()
                            .add(artist_col.eq(ak.as_str()))
                            .add(album_col.eq(bk.as_str())),
                    )
                })
            };
            let current_pending = pending::Entity::find()
                .filter(pending::Column::LibraryId.eq(&lib_str))
                .filter(batch.iter().fold(Condition::any(), |cond, row| {
                    cond.add(
                        Condition::all()
                            .add(pending::Column::ArtistKey.eq(row.artist_key.as_str()))
                            .add(pending::Column::AlbumKey.eq(row.album_key.as_str())),
                    )
                }))
                .all(&txn)
                .await?;
            if current_pending.is_empty() {
                txn.commit().await?;
                continue;
            }
            let pairs: Vec<(String, String)> = current_pending
                .iter()
                .map(|row| (row.artist_key.clone(), row.album_key.clone()))
                .collect();
            let disc_expr =
                Func::coalesce([Expr::col((tracks::Entity, C::DiscNumber)), Expr::val(1i64)]);
            let aggregates = tracks::Entity::find()
                .select_only()
                .join(JoinType::InnerJoin, tracks::Relation::Item.def())
                .filter(visible_recordings())
                .filter(C::LibraryId.eq(&lib_str))
                .filter(items::Column::DeletedAt.is_null())
                .filter(items::Column::Kind.eq("track"))
                .filter(pair_cond(C::ArtistKey, C::AlbumKey, &pairs))
                .column(C::ArtistKey)
                .column(C::AlbumKey)
                .column_as(C::Album.min(), "name")
                .column_as(C::AlbumSort.min(), "name_sort")
                .column_as(C::AlbumArtist.min(), "artist")
                .column_as(C::ArtistSort.min(), "artist_sort")
                .column_as(C::Year.max(), "year")
                .column_as(C::ItemId.count().cast_as("CHAR(24)"), "track_count")
                .column_as(
                    Expr::expr(disc_expr).count_distinct().cast_as("CHAR(24)"),
                    "disc_count",
                )
                .column_as(C::DurationMs.sum().cast_as("CHAR(24)"), "duration_ms")
                .column_as(C::ItemId.min(), "artwork_item_id")
                .column_as(items::Column::CreatedAt.min(), "added_at")
                .group_by(C::ArtistKey)
                .group_by(C::AlbumKey)
                .into_model::<AlbumAggregate>()
                .all(&txn)
                .await?;

            let mut present = BTreeSet::new();
            let mut upserts = Vec::with_capacity(aggregates.len());
            for agg in aggregates {
                present.insert((agg.artist_key.clone(), agg.album_key.clone()));
                let track_count = agg.track_count.trim().parse::<i64>().unwrap_or(0);
                let disc_count =
                    std::cmp::max(agg.disc_count.trim().parse::<i64>().unwrap_or(1), 1);
                let duration_ms = agg
                    .duration_ms
                    .as_deref()
                    .and_then(|s| s.trim().parse::<i64>().ok())
                    .unwrap_or(0);
                let shuffle = shuffle_value(&format!(
                    "album\0{lib_str}\0{}\0{}",
                    agg.artist_key, agg.album_key
                ));
                upserts.push(albums::ActiveModel {
                    library_id: Set(lib_str.clone()),
                    artist_key: Set(agg.artist_key),
                    album_key: Set(agg.album_key),
                    name: Set(agg.name),
                    name_sort: Set(agg.name_sort),
                    artist: Set(agg.artist),
                    artist_sort: Set(agg.artist_sort),
                    year: Set(agg.year),
                    track_count: Set(track_count),
                    disc_count: Set(disc_count),
                    duration_ms: Set(duration_ms),
                    artwork_item_id: Set(agg.artwork_item_id),
                    added_at: Set(agg.added_at),
                    shuffle: Set(shuffle),
                });
            }
            // 14 columns * 50 rows = 700 bind parameters.
            for chunk in upserts.chunks(50) {
                albums::Entity::insert_many(chunk.to_vec())
                    .on_conflict(
                        OnConflict::columns([
                            albums::Column::LibraryId,
                            albums::Column::ArtistKey,
                            albums::Column::AlbumKey,
                        ])
                        .update_columns([
                            albums::Column::Name,
                            albums::Column::NameSort,
                            albums::Column::Artist,
                            albums::Column::ArtistSort,
                            albums::Column::Year,
                            albums::Column::TrackCount,
                            albums::Column::DiscCount,
                            albums::Column::DurationMs,
                            albums::Column::ArtworkItemId,
                            albums::Column::AddedAt,
                            albums::Column::Shuffle,
                        ])
                        .to_owned(),
                    )
                    .exec_without_returning(&txn)
                    .await?;
            }
            let gone: Vec<&(String, String)> = pairs
                .iter()
                .filter(|pair| !present.contains(*pair))
                .collect();
            if !gone.is_empty() {
                let delete_cond = gone.iter().fold(Condition::any(), |cond, (ak, bk)| {
                    cond.add(
                        Condition::all()
                            .add(albums::Column::ArtistKey.eq(ak.as_str()))
                            .add(albums::Column::AlbumKey.eq(bk.as_str())),
                    )
                });
                albums::Entity::delete_many()
                    .filter(albums::Column::LibraryId.eq(&lib_str))
                    .filter(delete_cond)
                    .exec(&txn)
                    .await?;
            }
            let clear_cond = current_pending.iter().fold(Condition::any(), |cond, row| {
                cond.add(
                    Condition::all()
                        .add(pending::Column::ArtistKey.eq(row.artist_key.as_str()))
                        .add(pending::Column::AlbumKey.eq(row.album_key.as_str()))
                        .add(pending::Column::Stamp.eq(row.stamp)),
                )
            });
            pending::Entity::delete_many()
                .filter(pending::Column::LibraryId.eq(&lib_str))
                .filter(clear_cond)
                .exec(&txn)
                .await?;
            txn.commit().await?;
        }
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
        let mut query = tracks::Entity::find()
            .filter(visible_recordings())
            .filter(C::LibraryId.eq(library.to_string()));
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
                .column_as(C::ItemId.min(), "representative")
                .group_by(order)
                .group_by(key)
                .into_tuple::<(String, String, Option<String>, String)>()
                .all(self.db.main())
                .await?;
            for (sort, id, name, representative) in groups {
                if entries.len() == limit - 1 {
                    position = Some((sort, id.clone()));
                }
                entries.push(if artist.is_some() {
                    MusicEntry::Album {
                        id,
                        name,
                        artwork_item_id: parse_id(&representative, "track id")?,
                    }
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

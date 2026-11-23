//! Cross-library overview, storage accounting and catalog covers.

use std::collections::{BTreeMap, HashMap};

use palveni_core::overview::{Catalog, CatalogWork, CatalogWorkKind, LibraryBytes};
use sea_orm::sea_query::{Expr, ExprTrait};
use sea_orm::{
    ColumnTrait, Condition, EntityTrait, JoinType, PaginatorTrait, QueryFilter, QueryOrder,
    QuerySelect, RelationTrait,
};

use crate::convert::{now_ms, sha256_hex, ts};
use crate::entities::{
    items, libraries, library_mounts, music_albums, sessions, track_metadata, video_metadata,
};
use crate::items::SORT_GROUP_LEAF;
use crate::{Db, DbError};

#[derive(sea_orm::FromQueryResult)]
struct TrackAlbumRow {
    item_id: String,
    library_id: String,
    artist_key: String,
    album_key: String,
    album: Option<String>,
    album_artist: Option<String>,
    artist: Option<String>,
    created_at: i64,
}

pub struct Overview<'a> {
    pub(crate) db: &'a Db,
}

#[derive(Clone, Debug)]
pub struct PinnedMountInfo {
    pub id: String,
    pub path: String,
    pub source_hash: String,
    pub affected_libraries: usize,
}

fn parse_sum_u64(raw: Option<String>) -> u64 {
    raw.as_deref()
        .map(str::trim)
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0)
}

impl Overview<'_> {
    /// Per-library summed file sizes across live leaf items.
    pub async fn library_storage_bytes(&self) -> Result<Vec<LibraryBytes>, DbError> {
        let libs = libraries::Entity::find()
            .filter(libraries::Column::DeletedAt.is_null())
            .order_by_asc(libraries::Column::NameKey)
            .order_by_asc(libraries::Column::Id)
            .all(self.db.main())
            .await?;
        if libs.is_empty() {
            return Ok(Vec::new());
        }

        let sums = items::Entity::find()
            .filter(items::Column::DeletedAt.is_null())
            .filter(items::Column::SortGroup.eq(SORT_GROUP_LEAF))
            .filter(items::Column::SizeBytes.is_not_null())
            .select_only()
            .column(items::Column::LibraryId)
            .column_as(items::Column::SizeBytes.sum().cast_as("CHAR(24)"), "bytes")
            .group_by(items::Column::LibraryId)
            .into_tuple::<(String, Option<String>)>()
            .all(self.db.main())
            .await?;

        let map: HashMap<String, u64> = sums
            .into_iter()
            .map(|(lib_id, raw)| (lib_id, parse_sum_u64(raw)))
            .collect();

        let mut out: Vec<LibraryBytes> = libs
            .into_iter()
            .map(|l| LibraryBytes {
                bytes: map.get(&l.id).copied().unwrap_or(0),
                library_id: l.id,
                name: l.name,
            })
            .collect();
        out.sort_by(|a, b| b.bytes.cmp(&a.bytes).then_with(|| a.name.cmp(&b.name)));
        Ok(out)
    }

    pub async fn pinned_mounts(&self) -> Result<Vec<PinnedMountInfo>, DbError> {
        let rows = library_mounts::Entity::find()
            .join(
                JoinType::InnerJoin,
                library_mounts::Entity::belongs_to(libraries::Entity)
                    .from(library_mounts::Column::LibraryId)
                    .to(libraries::Column::Id)
                    .into(),
            )
            .filter(libraries::Column::DeletedAt.is_null())
            .all(self.db.main())
            .await?;

        let mut grouped: BTreeMap<String, (String, usize)> = BTreeMap::new();
        for row in rows {
            let entry = grouped
                .entry(row.path)
                .or_insert_with(|| (row.source_hash, 0));
            entry.1 = entry.1.saturating_add(1);
        }

        Ok(grouped
            .into_iter()
            .map(|(path, (source_hash, count))| PinnedMountInfo {
                id: format!("mnt-{}", &sha256_hex(&path)[..12]),
                path,
                source_hash,
                affected_libraries: count,
            })
            .collect())
    }

    pub async fn refresh_mount_source(
        &self,
        path: &str,
        new_identity: &str,
    ) -> Result<bool, DbError> {
        let hash = sha256_hex(new_identity);
        let res = library_mounts::Entity::update_many()
            .col_expr(library_mounts::Column::SourceHash, Expr::val(hash))
            .filter(library_mounts::Column::Path.eq(path.to_owned()))
            .exec(self.db.main())
            .await?;
        Ok(res.rows_affected > 0)
    }

    pub async fn active_sessions_count(&self, window_ms: i64) -> Result<usize, DbError> {
        let now = now_ms();
        let cutoff = now.saturating_sub(window_ms);
        let count = sessions::Entity::find()
            .filter(sessions::Column::RevokedAt.is_null())
            .filter(
                Condition::any()
                    .add(sessions::Column::ExpiresAt.is_null())
                    .add(sessions::Column::ExpiresAt.gt(now)),
            )
            .filter(sessions::Column::LastUsedAt.gte(cutoff))
            .count(self.db.main())
            .await?;
        Ok(usize::try_from(count).unwrap_or(usize::MAX))
    }

    /// Builds the `Catalog` response (`recent` works + per-library `libraryCovers`)
    /// using prefixed artwork IDs (`vp_<itemId>`, `ma_<itemId>`, `ph_<itemId>`).
    pub async fn catalog(&self, recent_limit: usize) -> Result<Catalog, DbError> {
        let libs = libraries::Entity::find()
            .filter(libraries::Column::DeletedAt.is_null())
            .all(self.db.main())
            .await?;
        if libs.is_empty() {
            return Ok(Catalog {
                recent: Vec::new(),
                library_covers: BTreeMap::new(),
            });
        }
        let live_lib_kinds: HashMap<String, String> =
            libs.into_iter().map(|l| (l.id, l.kind)).collect();

        let mut candidates: Vec<(i64, CatalogWork)> = Vec::new();
        let mut library_covers: BTreeMap<String, Vec<String>> = BTreeMap::new();

        // 1. Video works (deduplicated per library + work_key)
        let video_rows = video_metadata::Entity::find()
            .join(JoinType::InnerJoin, video_metadata::Relation::Item.def())
            .filter(items::Column::DeletedAt.is_null())
            .order_by_desc(items::Column::CreatedAt)
            .order_by_desc(video_metadata::Column::ItemId)
            .limit(200)
            .select_only()
            .columns([
                video_metadata::Column::ItemId,
                video_metadata::Column::LibraryId,
                video_metadata::Column::WorkKey,
                video_metadata::Column::WorkTitle,
                video_metadata::Column::WorkYear,
                video_metadata::Column::Season,
            ])
            .column(items::Column::CreatedAt)
            .into_tuple::<(
                String,
                String,
                String,
                String,
                Option<i32>,
                Option<i32>,
                i64,
            )>()
            .all(self.db.main())
            .await?;

        let mut seen_video_works = std::collections::HashSet::new();
        for (item_id, lib_id, work_key, work_title, work_year, season, created_at) in video_rows {
            if !live_lib_kinds.contains_key(&lib_id) {
                continue;
            }
            if !seen_video_works.insert((lib_id.clone(), work_key.clone())) {
                continue;
            }
            let artwork_id = format!("vp_{item_id}");
            let covers = library_covers.entry(lib_id.clone()).or_default();
            if covers.len() < 4 {
                covers.push(artwork_id.clone());
            }
            let is_series = season.is_some();
            let meta = if let Some(s) = season {
                Some(format!("第 {s} 季"))
            } else {
                work_year.map(|y| y.to_string())
            };
            candidates.push((
                created_at,
                CatalogWork {
                    id: work_key,
                    title: work_title,
                    kind: if is_series {
                        CatalogWorkKind::Series
                    } else {
                        CatalogWorkKind::Movie
                    },
                    meta,
                    artwork_id: Some(artwork_id),
                    library_id: Some(lib_id),
                    added_at: ts(created_at)?,
                },
            ));
        }

        // 2. Music albums (from rolled-up `music_albums` + live fallback to `track_metadata`)
        let album_rows = music_albums::Entity::find()
            .order_by_desc(music_albums::Column::AddedAt)
            .order_by_asc(music_albums::Column::AlbumKey)
            .limit(100)
            .all(self.db.main())
            .await?;

        let mut seen_music_albums = std::collections::HashSet::new();
        for album in album_rows {
            if !live_lib_kinds.contains_key(&album.library_id) {
                continue;
            }
            seen_music_albums.insert((
                album.library_id.clone(),
                album.artist_key.clone(),
                album.album_key.clone(),
            ));
            let artwork_id = (!album.artwork_item_id.is_empty())
                .then(|| format!("ma_{}", album.artwork_item_id));
            if let Some(ref art) = artwork_id {
                let covers = library_covers.entry(album.library_id.clone()).or_default();
                if covers.len() < 4 {
                    covers.push(art.clone());
                }
            }
            let title = album
                .name
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| "未知专辑".into());
            candidates.push((
                album.added_at,
                CatalogWork {
                    id: format!("{}.{}", album.artist_key, album.album_key),
                    title,
                    kind: CatalogWorkKind::Album,
                    meta: album.artist.filter(|s| !s.trim().is_empty()),
                    artwork_id,
                    library_id: Some(album.library_id),
                    added_at: ts(album.added_at)?,
                },
            ));
        }

        let track_album_rows = track_metadata::Entity::find()
            .join(JoinType::InnerJoin, track_metadata::Relation::Item.def())
            .filter(items::Column::DeletedAt.is_null())
            .order_by_desc(items::Column::CreatedAt)
            .order_by_asc(track_metadata::Column::ItemId)
            .limit(120)
            .select_only()
            .column_as(track_metadata::Column::ItemId, "item_id")
            .column_as(track_metadata::Column::LibraryId, "library_id")
            .column_as(track_metadata::Column::ArtistKey, "artist_key")
            .column_as(track_metadata::Column::AlbumKey, "album_key")
            .column_as(track_metadata::Column::Album, "album")
            .column_as(track_metadata::Column::AlbumArtist, "album_artist")
            .column_as(track_metadata::Column::Artist, "artist")
            .column_as(items::Column::CreatedAt, "created_at")
            .into_model::<TrackAlbumRow>()
            .all(self.db.main())
            .await?;

        for row in track_album_rows {
            if !live_lib_kinds.contains_key(&row.library_id) {
                continue;
            }
            if !seen_music_albums.insert((
                row.library_id.clone(),
                row.artist_key.clone(),
                row.album_key.clone(),
            )) {
                continue;
            }
            let art = format!("ma_{}", row.item_id);
            let covers = library_covers.entry(row.library_id.clone()).or_default();
            if covers.len() < 4 {
                covers.push(art.clone());
            }
            let title = row
                .album
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| "未知专辑".into());
            let meta = row
                .album_artist
                .filter(|s| !s.trim().is_empty())
                .or_else(|| row.artist.filter(|s| !s.trim().is_empty()));
            candidates.push((
                row.created_at,
                CatalogWork {
                    id: format!("{}.{}", row.artist_key, row.album_key),
                    title,
                    kind: CatalogWorkKind::Album,
                    meta,
                    artwork_id: Some(art),
                    library_id: Some(row.library_id),
                    added_at: ts(row.created_at)?,
                },
            ));
        }

        // 3. Books, comics, and photos
        let other_rows = items::Entity::find()
            .filter(items::Column::DeletedAt.is_null())
            .filter(items::Column::SortGroup.eq(SORT_GROUP_LEAF))
            .filter(items::Column::Kind.is_in(["book", "comic", "photo"]))
            .order_by_desc(items::Column::CreatedAt)
            .order_by_desc(items::Column::Id)
            .limit(60)
            .all(self.db.main())
            .await?;

        for row in other_rows {
            if !live_lib_kinds.contains_key(&row.library_id) {
                continue;
            }
            let (kind, artwork_id) = match row.kind.as_str() {
                "photo" => {
                    let art = format!("ph_{}", row.id);
                    let covers = library_covers.entry(row.library_id.clone()).or_default();
                    if covers.len() < 4 {
                        covers.push(art.clone());
                    }
                    (CatalogWorkKind::Photo, Some(art))
                }
                _ => {
                    let art = format!("rd_{}", row.id);
                    let covers = library_covers.entry(row.library_id.clone()).or_default();
                    if covers.len() < 4 {
                        covers.push(art.clone());
                    }
                    (CatalogWorkKind::Book, Some(art))
                }
            };
            candidates.push((
                row.created_at,
                CatalogWork {
                    id: row.id,
                    title: row.title,
                    kind,
                    meta: None,
                    artwork_id,
                    library_id: Some(row.library_id),
                    added_at: ts(row.created_at)?,
                },
            ));
        }

        candidates.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.id.cmp(&b.1.id)));
        let recent = candidates
            .into_iter()
            .take(recent_limit.clamp(1, 60))
            .map(|(_, w)| w)
            .collect();

        Ok(Catalog {
            recent,
            library_covers,
        })
    }
}

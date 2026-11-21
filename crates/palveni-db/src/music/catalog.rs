//! Offset-paged, library-scoped catalog for clients using indexed music tags.

use super::Music;
use crate::{
    DbError,
    entities::{items, libraries, subsonic_folders as folders, track_metadata as track},
};
use sea_orm::sea_query::{Expr, ExprTrait, Func, LikeExpr};
use sea_orm::{
    ColumnTrait, EntityTrait, FromQueryResult, JoinType, QueryFilter, QueryOrder, QuerySelect,
    RelationTrait, Select,
};

type C = track::Column;
type I = items::Column;
type F = folders::Column;

#[derive(Clone, FromQueryResult)]
pub struct CatalogSong {
    pub id: String,
    pub folder: i32,
    pub artist_key: String,
    pub album_key: String,
    pub title: String,
    pub artist: Option<String>,
    pub album_artist: Option<String>,
    pub album: Option<String>,
    pub disc: Option<i64>,
    pub track: Option<i64>,
    pub duration_ms: Option<i64>,
    pub bytes: Option<i64>,
    pub created: i64,
    pub path: Option<String>,
}

#[derive(FromQueryResult)]
pub struct CatalogAlbum {
    pub artwork_id: String,
    pub folder: i32,
    pub artist_key: String,
    pub album_key: String,
    pub artist: Option<String>,
    pub name: Option<String>,
    pub count: String,
    pub duration_ms: Option<String>,
    pub created: i64,
}

#[derive(FromQueryResult)]
pub struct CatalogArtist {
    pub folder: i32,
    pub artist_key: String,
    pub name: Option<String>,
    pub album_count: String,
}

pub const MAX_ALBUM_TRACKS: usize = 10_000;

pub struct AlbumKey<'a> {
    pub folder: i32,
    pub artist: &'a str,
    pub album: &'a str,
}

#[derive(Clone, Copy)]
pub struct Page {
    pub count: u64,
    pub offset: u64,
}

fn live(scope: &[i32]) -> Select<track::Entity> {
    let mut query = track::Entity::find()
        .select_only()
        .join(JoinType::InnerJoin, track::Relation::Item.def())
        .join(
            JoinType::InnerJoin,
            track::Entity::belongs_to(folders::Entity)
                .from(C::LibraryId)
                .to(F::LibraryId)
                .into(),
        )
        .join(JoinType::InnerJoin, folders::Relation::Library.def())
        .filter(libraries::Column::Kind.eq("music"))
        .filter(libraries::Column::DeletedAt.is_null())
        .filter(I::DeletedAt.is_null())
        .filter(I::Kind.eq("track"));
    if !scope.is_empty() {
        query = query.filter(F::Id.is_in(scope.iter().copied()));
    }
    query
}

fn criteria(mut query: Select<track::Entity>, term: &str, fields: &[C]) -> Select<track::Entity> {
    if !term.is_empty() {
        let literal = term
            .to_lowercase()
            .replace('!', "!!")
            .replace('%', "!%")
            .replace('_', "!_");
        let pattern = format!("%{literal}%");
        let filter = fields.iter().fold(sea_orm::Condition::any(), |acc, field| {
            acc.add(
                Expr::expr(Func::lower(Expr::col((track::Entity, *field))))
                    .like(LikeExpr::new(&pattern).escape('!')),
            )
        });
        query = query.filter(filter);
    }
    query
}

fn album_filter(query: Select<track::Entity>, key: &AlbumKey<'_>) -> Select<track::Entity> {
    query
        .filter(F::Id.eq(key.folder))
        .filter(C::ArtistKey.eq(key.artist))
        .filter(C::AlbumKey.eq(key.album))
}

fn song_columns(q: Select<track::Entity>) -> Select<track::Entity> {
    q.column_as(C::ItemId, "id")
        .column_as(F::Id, "folder")
        .column(C::ArtistKey)
        .column(C::AlbumKey)
        .column(C::Title)
        .column(C::Artist)
        .column(C::AlbumArtist)
        .column(C::Album)
        .column_as(C::DiscNumber, "disc")
        .column_as(C::TrackNumber, "track")
        .column(C::DurationMs)
        .column_as(I::SizeBytes, "bytes")
        .column_as(I::CreatedAt, "created")
        .column(I::Path)
}

fn album_columns(q: Select<track::Entity>) -> Select<track::Entity> {
    // PostgreSQL sums BIGINT as NUMERIC. A fixed-width decimal text representation also
    // decodes exactly on SQLite and MySQL without a driver-specific numeric type.
    q.column_as(F::Id, "folder")
        .column(C::ArtistKey)
        .column(C::AlbumKey)
        .column_as(C::ItemId.min(), "artwork_id")
        .column_as(C::AlbumArtist.min(), "artist")
        .column_as(C::Album.min(), "name")
        .column_as(C::ItemId.count().cast_as("CHAR(24)"), "count")
        .column_as(C::DurationMs.sum().cast_as("CHAR(24)"), "duration_ms")
        .column_as(I::CreatedAt.min(), "created")
        .group_by(F::Id)
        .group_by(C::ArtistKey)
        .group_by(C::AlbumKey)
}

fn artist_columns(q: Select<track::Entity>) -> Select<track::Entity> {
    q.column_as(F::Id, "folder")
        .column(C::ArtistKey)
        .column_as(C::AlbumArtist.min(), "name")
        .column_as(
            Expr::col((track::Entity, C::AlbumKey))
                .count_distinct()
                .cast_as("CHAR(24)"),
            "album_count",
        )
        .group_by(F::Id)
        .group_by(C::ArtistKey)
}

mod favorites;

impl Music<'_> {
    pub async fn catalog_scan_status(&self) -> Result<(bool, u64, Option<i64>), DbError> {
        use crate::entities::jobs;
        use sea_orm::PaginatorTrait;
        let libraries = libraries::Entity::find()
            .select_only()
            .column(libraries::Column::Id)
            .column(libraries::Column::LastScanAt)
            .filter(libraries::Column::Kind.eq("music"))
            .filter(libraries::Column::DeletedAt.is_null())
            .into_tuple::<(String, Option<i64>)>()
            .all(self.db.main())
            .await?;
        let last = libraries.iter().filter_map(|(_, at)| *at).max();
        let mut scanning = false;
        for chunk in libraries.chunks(200) {
            let job = jobs::Entity::find()
                .select_only()
                .column(jobs::Column::Id)
                .filter(jobs::Column::Kind.eq("library.scan"))
                .filter(jobs::Column::State.is_in(["queued", "running"]))
                .filter(jobs::Column::SubjectId.is_in(chunk.iter().map(|(id, _)| id.clone())))
                .into_tuple::<String>()
                .one(self.db.ops())
                .await?;
            if job.is_some() {
                scanning = true;
                break;
            }
        }
        let count = live(&[]).column(C::ItemId).count(self.db.main()).await?;
        Ok((scanning, count, last))
    }

    pub async fn catalog_folders(&self) -> Result<Vec<(i32, String)>, DbError> {
        Ok(folders::Entity::find()
            .select_only()
            .column(F::Id)
            .column(libraries::Column::Name)
            .join(JoinType::InnerJoin, folders::Relation::Library.def())
            .filter(libraries::Column::Kind.eq("music"))
            .filter(libraries::Column::DeletedAt.is_null())
            .order_by_asc(F::Id)
            .into_tuple()
            .all(self.db.main())
            .await?)
    }

    pub async fn catalog_songs(
        &self,
        scope: &[i32],
        term: &str,
        page: Page,
    ) -> Result<Vec<CatalogSong>, DbError> {
        Ok(song_columns(criteria(
            live(scope),
            term,
            &[C::Title, C::Artist, C::AlbumArtist, C::Album],
        ))
        .order_by_asc(C::ItemId)
        .offset(page.offset)
        .limit(page.count)
        .into_model()
        .all(self.db.main())
        .await?)
    }

    pub async fn catalog_by_ids(
        &self,
        ids: &[palveni_core::ItemId],
    ) -> Result<Vec<CatalogSong>, DbError> {
        let mut found = std::collections::HashMap::new();
        for chunk in ids.chunks(200) {
            let rows = song_columns(
                live(&[]).filter(C::ItemId.is_in(chunk.iter().map(ToString::to_string))),
            )
            .into_model::<CatalogSong>()
            .all(self.db.main())
            .await?;
            for row in rows {
                found.insert(row.id.clone(), row);
            }
        }
        Ok(ids
            .iter()
            .filter_map(|id| found.get(&id.to_string()).cloned())
            .collect())
    }

    pub async fn catalog_song(&self, id: &str) -> Result<Option<CatalogSong>, DbError> {
        Ok(song_columns(live(&[]).filter(C::ItemId.eq(id)))
            .into_model()
            .one(self.db.main())
            .await?)
    }

    pub async fn catalog_albums(
        &self,
        scope: &[i32],
        term: &str,
        page: Page,
        by_artist: bool,
    ) -> Result<Vec<CatalogAlbum>, DbError> {
        let mut q = album_columns(criteria(live(scope), term, &[C::Album, C::AlbumArtist]));
        if by_artist {
            q = q.order_by_asc(C::ArtistSort.min());
        }
        Ok(q.order_by_asc(C::AlbumSort.min())
            .order_by_asc(F::Id)
            .order_by_asc(C::ArtistKey)
            .order_by_asc(C::AlbumKey)
            .offset(page.offset)
            .limit(page.count)
            .into_model()
            .all(self.db.main())
            .await?)
    }

    pub async fn catalog_artists(
        &self,
        scope: &[i32],
        term: &str,
        page: Page,
    ) -> Result<Vec<CatalogArtist>, DbError> {
        Ok(
            artist_columns(criteria(live(scope), term, &[C::AlbumArtist]))
                .order_by_asc(C::ArtistSort.min())
                .order_by_asc(F::Id)
                .order_by_asc(C::ArtistKey)
                .offset(page.offset)
                .limit(page.count)
                .into_model()
                .all(self.db.main())
                .await?,
        )
    }

    pub async fn catalog_album(&self, key: &AlbumKey<'_>) -> Result<Option<CatalogAlbum>, DbError> {
        Ok(album_columns(album_filter(live(&[]), key))
            .into_model()
            .one(self.db.main())
            .await?)
    }

    pub async fn catalog_album_songs(
        &self,
        key: &AlbumKey<'_>,
    ) -> Result<Vec<CatalogSong>, DbError> {
        Ok(song_columns(album_filter(live(&[]), key))
            .order_by_asc(C::TrackSort)
            .order_by_asc(C::ItemId)
            .limit((MAX_ALBUM_TRACKS + 1) as u64)
            .into_model()
            .all(self.db.main())
            .await?)
    }
}

use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect};

use super::{C, Music, items, tracks};
use crate::DbError;
use palveni_core::LibraryId;

/// Keyset batches keep search indexing independent of driver-specific text collations.
#[derive(sea_orm::FromQueryResult)]
pub struct SearchTrack {
    pub id: String,
    pub title: String,
    pub artist: Option<String>,
    pub artist_key: String,
    pub album_artist: Option<String>,
    pub album_key: String,
    pub album: Option<String>,
}

pub struct SearchChanges {
    pub ids: Vec<String>,
    pub tracks: Vec<SearchTrack>,
}

fn project(
    query: sea_orm::Select<tracks::Entity>,
) -> sea_orm::Selector<sea_orm::SelectModel<SearchTrack>> {
    query
        .select_only()
        .column_as(C::ItemId, "id")
        .columns([
            C::Title,
            C::Artist,
            C::ArtistKey,
            C::AlbumArtist,
            C::AlbumKey,
            C::Album,
        ])
        .into_model::<SearchTrack>()
}

impl Music<'_> {
    /// Includes tombstones so deleting the last track also invalidates its index.
    pub async fn search_revision(&self, id: LibraryId) -> Result<i64, DbError> {
        if !self.has_library(id).await? {
            return Err(DbError::NotFound("music library"));
        }
        let latest = items::Entity::find()
            .filter(items::Column::LibraryId.eq(id.to_string()))
            .order_by_desc(items::Column::Revision)
            .select_only()
            .column(items::Column::Revision)
            .into_tuple::<i64>()
            .one(self.db.main())
            .await?;
        Ok(latest.unwrap_or_default())
    }

    pub async fn search_tracks(
        &self,
        id: LibraryId,
        after: &str,
    ) -> Result<Vec<SearchTrack>, DbError> {
        Ok(project(
            tracks::Entity::find()
                .filter(super::visible_recordings())
                .filter(C::LibraryId.eq(id.to_string()))
                .filter(C::ItemId.gt(after))
                .order_by_asc(C::ItemId)
                .limit(512),
        )
        .all(self.db.main())
        .await?)
    }

    pub async fn search_changes(
        &self,
        library: LibraryId,
        after: i64,
    ) -> Result<Option<SearchChanges>, DbError> {
        // A large backlog is cheaper to rebuild in primary-key order.
        const MAX_DELTA: usize = 32;
        let ids = items::Entity::find()
            .filter(items::Column::LibraryId.eq(library.to_string()))
            .filter(items::Column::Revision.gt(after))
            .limit((MAX_DELTA + 1) as u64)
            .select_only()
            .column(items::Column::Id)
            .into_tuple::<String>()
            .all(self.db.main())
            .await?;
        if ids.len() > MAX_DELTA {
            return Ok(None);
        }
        let tracks = project(
            tracks::Entity::find()
                .filter(super::visible_recordings())
                .filter(C::LibraryId.eq(library.to_string()))
                .filter(C::ItemId.is_in(ids.iter().cloned()))
                .order_by_asc(C::ItemId),
        )
        .all(self.db.main())
        .await?;
        Ok(Some(SearchChanges { ids, tracks }))
    }

    pub async fn search_group_sample(
        &self,
        library: LibraryId,
        artist: &str,
        album: Option<&str>,
    ) -> Result<Option<SearchTrack>, DbError> {
        let mut query = tracks::Entity::find()
            .filter(super::visible_recordings())
            .filter(C::LibraryId.eq(library.to_string()))
            .filter(C::ArtistKey.eq(artist));
        if let Some(album) = album {
            query = query.filter(C::AlbumKey.eq(album));
        }
        Ok(project(query.order_by_asc(C::ItemId).limit(1))
            .one(self.db.main())
            .await?)
    }
}

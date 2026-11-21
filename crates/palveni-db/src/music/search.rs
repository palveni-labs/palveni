use sea_orm::sea_query::{Expr, ExprTrait, Order, Query};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
use sha2::{Digest, Sha256};

use super::{C, Music, items, tracks};
use crate::{DbError, convert::parse_id, entities::libraries};
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

pub struct SearchLibrary {
    pub id: LibraryId,
    pub name: String,
    pub revision: i64,
}

pub struct SearchCatalog {
    pub libraries: Vec<SearchLibrary>,
    pub stamp: String,
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
    async fn search_libraries(&self, id: Option<LibraryId>) -> Result<Vec<SearchLibrary>, DbError> {
        use items::Column as I;
        use libraries::Column as L;

        // One seek in the (library_id, revision) index per live music library, including tombstones.
        let latest = Query::select()
            .column(I::Revision)
            .from(items::Entity)
            .and_where(Expr::col((items::Entity, I::LibraryId)).equals((libraries::Entity, L::Id)))
            .order_by(I::Revision, Order::Desc)
            .limit(1)
            .to_owned();
        let mut query = libraries::Entity::find()
            .select_only()
            .columns([L::Id, L::Name])
            .expr_as(latest, "latest")
            .filter(L::Kind.eq("music"))
            .filter(L::DeletedAt.is_null())
            .order_by_asc(L::Id);
        if let Some(id) = id {
            query = query.filter(L::Id.eq(id.to_string()));
        }
        query
            .into_tuple::<(String, String, Option<i64>)>()
            .all(self.db.main())
            .await?
            .into_iter()
            .map(|(id, name, revision)| {
                Ok(SearchLibrary {
                    id: parse_id(&id, "library id")?,
                    name,
                    revision: revision.unwrap_or_default(),
                })
            })
            .collect()
    }

    pub async fn search_revision(&self, id: LibraryId) -> Result<i64, DbError> {
        self.search_libraries(Some(id))
            .await?
            .first()
            .map(|l| l.revision)
            .ok_or(DbError::NotFound("music library"))
    }

    pub async fn search_catalog(&self) -> Result<SearchCatalog, DbError> {
        let libraries = self.search_libraries(None).await?;
        let mut hash = Sha256::new();
        for library in &libraries {
            hash.update(library.id.as_uuid().as_bytes());
            hash.update(library.revision.to_be_bytes());
        }
        Ok(SearchCatalog {
            libraries,
            stamp: hex::encode(hash.finalize()),
        })
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

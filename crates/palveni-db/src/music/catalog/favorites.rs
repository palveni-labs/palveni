use super::{
    C, CatalogAlbum, CatalogArtist, F, Music, album_columns, artist_columns, folders, live, track,
};
use crate::{DbError, entities::music_collection_favorites as favorites, revision::next_revision};
use favorites::Column as Fav;
use palveni_core::UserId;
use sea_orm::{
    ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect, Set, TransactionTrait,
    sea_query::{Expr, ExprTrait, Query},
};

fn favorited(user: UserId, albums: bool) -> Expr {
    let album = if albums {
        Expr::col((favorites::Entity, Fav::AlbumKey)).eq(Expr::col((track::Entity, C::AlbumKey)))
    } else {
        Fav::AlbumKey.eq("")
    };
    let subquery = Query::select()
        .column(Fav::FolderId)
        .from(favorites::Entity)
        .and_where(Fav::UserId.eq(user.to_string()))
        .and_where(
            Expr::col((favorites::Entity, Fav::FolderId)).eq(Expr::col((folders::Entity, F::Id))),
        )
        .and_where(
            Expr::col((favorites::Entity, Fav::ArtistKey))
                .eq(Expr::col((track::Entity, C::ArtistKey))),
        )
        .and_where(album)
        .to_owned();
    Expr::exists(subquery)
}

impl Music<'_> {
    pub async fn favorite_collections(
        &self,
        user: UserId,
        scope: &[i32],
    ) -> Result<(Vec<CatalogArtist>, Vec<CatalogAlbum>), DbError> {
        let artists = artist_columns(live(scope).filter(favorited(user, false)))
            .order_by_asc(C::ArtistSort.min())
            .order_by_asc(F::Id)
            .order_by_asc(C::ArtistKey)
            .into_model()
            .all(self.db.main())
            .await?;
        let albums = album_columns(live(scope).filter(favorited(user, true)))
            .order_by_asc(C::AlbumSort.min())
            .order_by_asc(F::Id)
            .order_by_asc(C::ArtistKey)
            .order_by_asc(C::AlbumKey)
            .into_model()
            .all(self.db.main())
            .await?;
        Ok((artists, albums))
    }

    pub async fn set_collection_favorite(
        &self,
        user: UserId,
        folder: i32,
        artist: &str,
        album: Option<&str>,
        favorite: bool,
    ) -> Result<(), DbError> {
        let db = self.db.main().begin().await?;
        next_revision(&db).await?;
        let mut target = live(&[folder]).filter(C::ArtistKey.eq(artist));
        if let Some(album) = album {
            target = target.filter(C::AlbumKey.eq(album));
        }
        if target
            .column(C::ItemId)
            .into_tuple::<String>()
            .one(&db)
            .await?
            .is_none()
        {
            return Err(DbError::NotFound("Music collection"));
        }
        let key = (
            user.to_string(),
            folder,
            artist.to_owned(),
            album.unwrap_or_default().to_owned(),
        );
        if favorite {
            favorites::Entity::insert(favorites::ActiveModel {
                user_id: Set(key.0),
                folder_id: Set(key.1),
                artist_key: Set(key.2),
                album_key: Set(key.3),
            })
            .on_conflict(
                sea_orm::sea_query::OnConflict::columns([
                    Fav::UserId,
                    Fav::FolderId,
                    Fav::ArtistKey,
                    Fav::AlbumKey,
                ])
                .update_column(Fav::AlbumKey)
                .to_owned(),
            )
            .exec_without_returning(&db)
            .await?;
        } else {
            favorites::Entity::delete_by_id(key).exec(&db).await?;
        }
        db.commit().await?;
        Ok(())
    }
}

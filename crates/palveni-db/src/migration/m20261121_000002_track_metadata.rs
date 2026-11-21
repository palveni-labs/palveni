use sea_orm_migration::prelude::*;

use super::{bigint_null, id_pk, id_ref, text, text_null, varchar};
use crate::entities::{
    items,
    track_metadata::{Column as C, Entity as Tracks},
};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[allow(elided_lifetimes_in_paths)]
#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Tracks)
                    .col(id_pk(C::ItemId))
                    .col(id_ref(C::LibraryId))
                    .col(varchar(C::ArtistKey, 64))
                    .col(varchar(C::ArtistSort, 192))
                    .col(text_null(C::AlbumArtist))
                    .col(varchar(C::AlbumKey, 64))
                    .col(varchar(C::AlbumSort, 192))
                    .col(text_null(C::Album))
                    .col(varchar(C::TrackSort, 216))
                    .col(text(C::Title))
                    .col(text_null(C::Artist))
                    .col(bigint_null(C::DiscNumber))
                    .col(bigint_null(C::TrackNumber))
                    .col(bigint_null(C::DurationMs))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_track_metadata_item")
                            .from(Tracks, C::ItemId)
                            .to(items::Entity, items::Column::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;
        for (name, columns) in [
            (
                "idx_track_artist",
                vec![C::LibraryId, C::ArtistSort, C::ArtistKey],
            ),
            (
                "idx_track_album",
                vec![C::LibraryId, C::ArtistKey, C::AlbumSort, C::AlbumKey],
            ),
            (
                "idx_track_order",
                vec![
                    C::LibraryId,
                    C::ArtistKey,
                    C::AlbumKey,
                    C::TrackSort,
                    C::ItemId,
                ],
            ),
        ] {
            let mut index = Index::create().name(name).table(Tracks).to_owned();
            for column in columns {
                index.col(column);
            }
            manager.create_index(index).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Tracks).to_owned())
            .await
    }
}

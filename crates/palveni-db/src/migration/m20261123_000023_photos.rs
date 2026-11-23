//! Photos metadata, personal photo albums and album item memberships.
use sea_orm::EntityName;
use sea_orm_migration::prelude::*;

use super::{
    SHA256_HEX_LEN, bigint, id_pk, id_ref, id_ref_null, int, text_null, varchar, varchar_null,
};
use crate::entities::{
    items, libraries,
    photo_album_items::{Column as AI, Entity as AlbumItems},
    photo_albums::{Column as A, Entity as Albums},
    photo_metadata::{Column as M, Entity as Metadata},
    users,
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
                    .table(Metadata)
                    .col(id_pk(M::ItemId))
                    .col(id_ref(M::LibraryId))
                    .col(varchar(M::Stamp, 64))
                    .col(bigint(M::TakenAt))
                    .col(varchar(M::DayBucket, 10))
                    .col(varchar(M::MonthBucket, 7))
                    .col(ColumnDef::new(M::Width).integer().null())
                    .col(ColumnDef::new(M::Height).integer().null())
                    .col(int(M::Orientation))
                    .col(varchar_null(M::CameraMake, 128))
                    .col(varchar_null(M::CameraModel, 128))
                    .col(varchar_null(M::LensModel, 128))
                    .col(ColumnDef::new(M::Iso).integer().null())
                    .col(ColumnDef::new(M::FNumber).double().null())
                    .col(varchar_null(M::ExposureTime, 32))
                    .col(ColumnDef::new(M::FocalLength).double().null())
                    .col(ColumnDef::new(M::Latitude).double().null())
                    .col(ColumnDef::new(M::Longitude).double().null())
                    .col(varchar_null(M::ContentHash, SHA256_HEX_LEN))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_photo_metadata_item")
                            .from(Metadata, M::ItemId)
                            .to(items::Entity, items::Column::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_photo_metadata_library")
                            .from(Metadata, M::LibraryId)
                            .to(libraries::Entity, libraries::Column::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Albums)
                    .col(id_pk(A::Id))
                    .col(id_ref(A::UserId))
                    .col(varchar(A::Title, 128))
                    .col(text_null(A::Description))
                    .col(id_ref_null(A::CoverItemId))
                    .col(bigint(A::CreatedAt))
                    .col(bigint(A::UpdatedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_photo_albums_user")
                            .from(Albums, A::UserId)
                            .to(users::Entity, users::Column::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(AlbumItems)
                    .col(id_ref(AI::AlbumId))
                    .col(id_ref(AI::ItemId))
                    .col(int(AI::Position))
                    .col(bigint(AI::AddedAt))
                    .primary_key(Index::create().col(AI::AlbumId).col(AI::ItemId))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_photo_album_items_album")
                            .from(AlbumItems, AI::AlbumId)
                            .to(Albums, A::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_photo_album_items_item")
                            .from(AlbumItems, AI::ItemId)
                            .to(items::Entity, items::Column::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_photo_metadata_timeline")
                    .table(Metadata)
                    .col(M::TakenAt)
                    .col(M::ItemId)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_photo_metadata_library_timeline")
                    .table(Metadata)
                    .col(M::LibraryId)
                    .col(M::TakenAt)
                    .col(M::ItemId)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_photo_metadata_month")
                    .table(Metadata)
                    .col(M::MonthBucket)
                    .col(M::TakenAt)
                    .col(M::ItemId)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_photo_metadata_hash")
                    .table(Metadata)
                    .col(M::ContentHash)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_photo_albums_user_updated")
                    .table(Albums)
                    .col(A::UserId)
                    .col(A::UpdatedAt)
                    .col(A::Id)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_photo_album_items_order")
                    .table(AlbumItems)
                    .col(AI::AlbumId)
                    .col(AI::Position)
                    .col(AI::ItemId)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_photo_album_items_item")
                    .table(AlbumItems)
                    .col(AI::ItemId)
                    .col(AI::AlbumId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for table in [
            AlbumItems.table_name(),
            Albums.table_name(),
            Metadata.table_name(),
        ] {
            manager
                .drop_table(
                    Table::drop()
                        .table(Alias::new(table))
                        .if_exists()
                        .to_owned(),
                )
                .await?;
        }
        Ok(())
    }
}

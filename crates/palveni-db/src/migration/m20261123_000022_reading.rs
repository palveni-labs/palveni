//! Reading catalog metadata (ebooks and comics) and personal bookmarks.
use sea_orm::EntityName;
use sea_orm_migration::prelude::*;

use super::{bigint, id_pk, id_ref, int, text, text_null, varchar, varchar_null};
use crate::entities::{
    items, libraries,
    reading_bookmarks::{Column as B, Entity as Bookmarks},
    reading_metadata::{Column as M, Entity as Metadata},
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
                    .col(varchar(M::Stamp, 128))
                    .col(varchar(M::Kind, 16))
                    .col(varchar(M::Format, 16))
                    .col(text(M::Title))
                    .col(varchar(M::SortTitle, 255))
                    .col(text(M::AuthorsJson))
                    .col(varchar_null(M::SeriesName, 255))
                    .col(varchar_null(M::SeriesKey, 255))
                    .col(ColumnDef::new(M::SeriesIndex).double().null())
                    .col(text_null(M::Summary))
                    .col(varchar_null(M::Publisher, 255))
                    .col(varchar_null(M::Language, 32))
                    .col(ColumnDef::new(M::PageCount).integer().null())
                    .col(ColumnDef::new(M::HasCover).boolean().not_null())
                    .col(text(M::TagsJson))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_reading_metadata_item")
                            .from(Metadata, M::ItemId)
                            .to(items::Entity, items::Column::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_reading_metadata_library")
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
                    .table(Bookmarks)
                    .col(id_pk(B::Id))
                    .col(id_ref(B::UserId))
                    .col(id_ref(B::ItemId))
                    .col(int(B::Page))
                    .col(varchar_null(B::Locator, 512))
                    .col(varchar(B::Label, 255))
                    .col(text_null(B::Note))
                    .col(bigint(B::CreatedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_reading_bookmarks_user")
                            .from(Bookmarks, B::UserId)
                            .to(users::Entity, users::Column::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_reading_bookmarks_item")
                            .from(Bookmarks, B::ItemId)
                            .to(items::Entity, items::Column::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_reading_meta_lib_sort")
                    .table(Metadata)
                    .col(M::LibraryId)
                    .col(M::SortTitle)
                    .col(M::ItemId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_reading_meta_lib_series")
                    .table(Metadata)
                    .col(M::LibraryId)
                    .col(M::SeriesKey)
                    .col(M::SeriesIndex)
                    .col(M::ItemId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_reading_meta_kind")
                    .table(Metadata)
                    .col(M::Kind)
                    .col(M::LibraryId)
                    .col(M::ItemId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_reading_bookmarks_user_item")
                    .table(Bookmarks)
                    .col(B::UserId)
                    .col(B::ItemId)
                    .col(B::CreatedAt)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for table in [Bookmarks.table_name(), Metadata.table_name()] {
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

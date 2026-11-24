use sea_orm_migration::prelude::*;

use super::{bigint, id_pk, text_null, varchar, varchar_null};
use crate::entities::{
    items,
    metadata_override::{Column as MO, Entity as MetadataOverrides},
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
                    .table(MetadataOverrides)
                    .col(id_pk(MO::ItemId))
                    .col(varchar_null(MO::WorkKey, 128))
                    .col(varchar(MO::Provider, 32))
                    .col(varchar_null(MO::ExternalId, 128))
                    .col(varchar_null(MO::Title, 255))
                    .col(varchar_null(MO::OriginalTitle, 255))
                    .col(ColumnDef::new(MO::Year).integer().null().take())
                    .col(text_null(MO::Overview))
                    .col(ColumnDef::new(MO::Rating).double().null().take())
                    .col(text_null(MO::GenresJson))
                    .col(text_null(MO::PosterUrl))
                    .col(text_null(MO::BackdropUrl))
                    .col(bigint(MO::UpdatedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_metadata_overrides_item")
                            .from(MetadataOverrides, MO::ItemId)
                            .to(items::Entity, items::Column::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_metadata_overrides_work_key")
                    .table(MetadataOverrides)
                    .col(MO::WorkKey)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_metadata_overrides_updated")
                    .table(MetadataOverrides)
                    .col(MO::UpdatedAt)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(MetadataOverrides).to_owned())
            .await
    }
}

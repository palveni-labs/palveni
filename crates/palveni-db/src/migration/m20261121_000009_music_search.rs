use crate::entities::{items, track_metadata};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[allow(elided_lifetimes_in_paths)]
#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_index(
                Index::create()
                    .name("idx_items_library_revision")
                    .table(items::Entity)
                    .col(items::Column::LibraryId)
                    .col(items::Column::Revision)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_tracks_library_id")
                    .table(track_metadata::Entity)
                    .col(track_metadata::Column::LibraryId)
                    .col(track_metadata::Column::ItemId)
                    .to_owned(),
            )
            .await
    }
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name("idx_tracks_library_id")
                    .table(track_metadata::Entity)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .name("idx_items_library_revision")
                    .table(items::Entity)
                    .to_owned(),
            )
            .await
    }
}

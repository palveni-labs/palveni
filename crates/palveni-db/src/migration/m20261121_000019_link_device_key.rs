use crate::entities::devices::{Column, Entity};
use sea_orm::EntityName;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const NAME: &str = "uq_devices_link_key";

#[allow(elided_lifetimes_in_paths)]
#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.has_index(Entity.table_name(), NAME).await? {
            return Ok(());
        }
        manager
            .create_index(
                Index::create()
                    .name(NAME)
                    .table(Entity)
                    .col(Column::PublicKey)
                    .unique()
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if !manager.has_index(Entity.table_name(), NAME).await? {
            return Ok(());
        }
        manager
            .drop_index(Index::drop().name(NAME).table(Entity).to_owned())
            .await
    }
}

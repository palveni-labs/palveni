use sea_orm_migration::prelude::*;

use super::{id_pk, text, varchar};
use crate::entities::media_connections::{Column as C, Entity as Connections};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[allow(elided_lifetimes_in_paths)]
#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Connections)
                    .col(id_pk(C::Id))
                    .col(varchar(C::Name, 100))
                    .col(varchar(C::Kind, 24))
                    .col(text(C::Address))
                    .col(text(C::Credentials))
                    .to_owned(),
            )
            .await
    }
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Connections).to_owned())
            .await
    }
}

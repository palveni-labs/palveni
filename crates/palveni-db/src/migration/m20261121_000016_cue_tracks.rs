use super::{bigint_null, text_null};
use crate::entities::track_metadata::{Column as C, Entity as Tracks};
use sea_orm::EntityName;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[allow(elided_lifetimes_in_paths)]
#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for col in [
            text_null(C::CueSource),
            bigint_null(C::CueStart),
            bigint_null(C::CueEnd),
            ColumnDef::new(C::CueStamp).string_len(64).null().take(),
        ] {
            if manager
                .has_column(Tracks.table_name(), col.get_column_name())
                .await?
            {
                continue;
            }
            manager
                .alter_table(Table::alter().table(Tracks).add_column(col).to_owned())
                .await?;
        }
        Ok(())
    }
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for col in [C::CueStamp, C::CueEnd, C::CueStart, C::CueSource] {
            if !manager
                .has_column(Tracks.table_name(), col.to_string())
                .await?
            {
                continue;
            }
            manager
                .alter_table(Table::alter().table(Tracks).drop_column(col).to_owned())
                .await?;
        }
        Ok(())
    }
}

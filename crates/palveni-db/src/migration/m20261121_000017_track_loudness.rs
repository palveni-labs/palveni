use super::{bigint, id_pk};
use crate::entities::{
    items,
    track_loudness::{Column as C, Entity as Loudness},
};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[allow(elided_lifetimes_in_paths)]
#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.create_table(
            Table::create()
                .table(Loudness)
                .col(id_pk(C::ItemId))
                .col(bigint(C::Revision))
                .col(bigint(C::MeasuredAt))
                .col(ColumnDef::new(C::IntegratedLufs).double().null())
                .col(ColumnDef::new(C::RangeLu).double().null())
                .col(ColumnDef::new(C::TruePeakDbtp).double().null())
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_track_loudness_item")
                        .from(Loudness, C::ItemId)
                        .to(items::Entity, items::Column::Id)
                        .on_delete(ForeignKeyAction::Cascade),
                )
                .to_owned(),
        )
        .await
    }
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.drop_table(Table::drop().table(Loudness).to_owned()).await
    }
}

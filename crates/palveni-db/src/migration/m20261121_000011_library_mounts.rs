use super::{SHA256_HEX_LEN, id_ref, text, varchar};
use crate::entities::{
    libraries,
    library_mounts::{Column as C, Entity as Mounts},
};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[allow(elided_lifetimes_in_paths)]
#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Mounts)
                    .col(id_ref(C::LibraryId))
                    .col(varchar(C::PathHash, SHA256_HEX_LEN))
                    .col(text(C::Path))
                    .col(varchar(C::SourceHash, SHA256_HEX_LEN))
                    .primary_key(Index::create().col(C::LibraryId).col(C::PathHash))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_library_mount_library")
                            .from(Mounts, C::LibraryId)
                            .to(libraries::Entity, libraries::Column::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Mounts).to_owned())
            .await
    }
}

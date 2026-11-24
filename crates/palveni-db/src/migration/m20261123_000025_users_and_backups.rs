use sea_orm_migration::prelude::*;

use super::{SHA256_HEX_LEN, bigint, id_pk, id_ref, id_ref_null, int, text, varchar};
use crate::entities::{
    libraries,
    system_backups::{Column as SB, Entity as Backups},
    user_library_access::{Column as ULA, Entity as LibraryAccess},
    user_policies::{Column as UP, Entity as Policies},
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
                    .table(Policies)
                    .col(id_pk(UP::UserId))
                    .col(int(UP::AllLibraries))
                    .col(int(UP::AllowDownload))
                    .col(int(UP::AllowTranscode))
                    .col(int(UP::DailyAiLimit))
                    .col(bigint(UP::UpdatedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_user_policies_user")
                            .from(Policies, UP::UserId)
                            .to(users::Entity, users::Column::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(LibraryAccess)
                    .col(id_ref(ULA::UserId))
                    .col(id_ref(ULA::LibraryId))
                    .primary_key(Index::create().col(ULA::UserId).col(ULA::LibraryId))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_user_library_access_user")
                            .from(LibraryAccess, ULA::UserId)
                            .to(users::Entity, users::Column::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_user_library_access_library")
                            .from(LibraryAccess, ULA::LibraryId)
                            .to(libraries::Entity, libraries::Column::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_user_library_access_library")
                    .table(LibraryAccess)
                    .col(ULA::LibraryId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Backups)
                    .col(id_pk(SB::Id))
                    .col(varchar(SB::Name, 128))
                    .col(id_ref_null(SB::CreatorId))
                    .col(bigint(SB::SizeBytes))
                    .col(varchar(SB::Sha256, SHA256_HEX_LEN))
                    .col(text(SB::SummaryJson))
                    .col(text(SB::PayloadJson))
                    .col(bigint(SB::CreatedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_system_backups_creator")
                            .from(Backups, SB::CreatorId)
                            .to(users::Entity, users::Column::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_system_backups_created_at")
                    .table(Backups)
                    .col(SB::CreatedAt)
                    .col(SB::Id)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_system_backups_creator")
                    .table(Backups)
                    .col(SB::CreatorId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Backups).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(LibraryAccess).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Policies).to_owned())
            .await
    }
}

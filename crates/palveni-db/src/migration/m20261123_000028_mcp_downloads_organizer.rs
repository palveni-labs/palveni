use sea_orm_migration::prelude::*;

use super::{
    bigint, bigint_null, id_pk, id_ref, id_ref_null, int, text, text_null, varchar, varchar_null,
};
use crate::entities::{
    downloader_clients::{Column as DC, Entity as Clients},
    libraries,
    media_subscriptions::{Column as MS, Entity as Subscriptions},
    organize_batches::{Column as OB, Entity as Batches},
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
                    .table(Clients)
                    .col(id_pk(DC::Id))
                    .col(varchar(DC::Name, 128))
                    .col(varchar(DC::Kind, 32))
                    .col(varchar(DC::BaseUrl, 512))
                    .col(text_null(DC::CredentialCipher))
                    .col(varchar_null(DC::SavePath, 512))
                    .col(int(DC::Enabled))
                    .col(bigint(DC::CreatedAt))
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_downloader_clients_created")
                    .table(Clients)
                    .col(DC::CreatedAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Subscriptions)
                    .col(id_pk(MS::Id))
                    .col(id_ref(MS::OwnerId))
                    .col(varchar(MS::Title, 255))
                    .col(varchar(MS::MediaKind, 32))
                    .col(varchar_null(MS::QualityRule, 128))
                    .col(varchar_null(MS::FeedUrl, 512))
                    .col(id_ref_null(MS::TargetLibraryId))
                    .col(int(MS::Enabled))
                    .col(bigint_null(MS::LastCheckedAt))
                    .col(bigint(MS::CreatedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_media_subscriptions_owner")
                            .from(Subscriptions, MS::OwnerId)
                            .to(users::Entity, users::Column::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_media_subscriptions_owner_created")
                    .table(Subscriptions)
                    .col(MS::OwnerId)
                    .col(MS::CreatedAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Batches)
                    .col(id_pk(OB::Id))
                    .col(id_ref(OB::ActorId))
                    .col(id_ref(OB::LibraryId))
                    .col(varchar(OB::Mode, 32))
                    .col(varchar(OB::Template, 255))
                    .col(text(OB::OperationsJson))
                    .col(bigint_null(OB::UndoneAt))
                    .col(bigint(OB::CreatedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_organize_batches_actor")
                            .from(Batches, OB::ActorId)
                            .to(users::Entity, users::Column::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_organize_batches_library")
                            .from(Batches, OB::LibraryId)
                            .to(libraries::Entity, libraries::Column::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_organize_batches_library_created")
                    .table(Batches)
                    .col(OB::LibraryId)
                    .col(OB::CreatedAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_organize_batches_created")
                    .table(Batches)
                    .col(OB::CreatedAt)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Batches).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Subscriptions).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Clients).to_owned())
            .await
    }
}

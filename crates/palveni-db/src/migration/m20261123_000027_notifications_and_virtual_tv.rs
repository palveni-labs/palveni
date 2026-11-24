use sea_orm_migration::prelude::*;

use super::{
    SHA256_HEX_LEN, bigint, bigint_null, id_pk, id_ref, id_ref_null, int, text, text_null, varchar,
    varchar_null,
};
use crate::entities::{
    inbound_webhooks::{Column as IW, Entity as Webhooks},
    libraries,
    notification_channels::{Column as NC, Entity as Channels},
    notification_deliveries::{Column as ND, Entity as Deliveries},
    scheduled_routines::{Column as SR, Entity as Routines},
    users,
    virtual_channels::{Column as VC, Entity as VirtualChannels},
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
                    .table(Channels)
                    .col(id_pk(NC::Id))
                    .col(id_ref(NC::OwnerId))
                    .col(varchar(NC::Name, 128))
                    .col(varchar(NC::Kind, 32))
                    .col(text(NC::ConfigCipher))
                    .col(text(NC::EventsJson))
                    .col(varchar_null(NC::QuietHours, 32))
                    .col(int(NC::Enabled))
                    .col(bigint(NC::CreatedAt))
                    .col(bigint(NC::UpdatedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_notif_channels_owner")
                            .from(Channels, NC::OwnerId)
                            .to(users::Entity, users::Column::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_notif_channels_owner_created")
                    .table(Channels)
                    .col(NC::OwnerId)
                    .col(NC::CreatedAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_notif_channels_enabled")
                    .table(Channels)
                    .col(NC::Enabled)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Deliveries)
                    .col(id_pk(ND::Id))
                    .col(id_ref_null(ND::OwnerId))
                    .col(id_ref_null(ND::ChannelId))
                    .col(varchar(ND::EventType, 64))
                    .col(varchar(ND::Title, 255))
                    .col(text(ND::Body))
                    .col(varchar(ND::Status, 32))
                    .col(text_null(ND::Error))
                    .col(bigint_null(ND::ReadAt))
                    .col(bigint(ND::CreatedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_notif_deliveries_owner")
                            .from(Deliveries, ND::OwnerId)
                            .to(users::Entity, users::Column::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_notif_deliveries_owner_created")
                    .table(Deliveries)
                    .col(ND::OwnerId)
                    .col(ND::CreatedAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_notif_deliveries_owner_read")
                    .table(Deliveries)
                    .col(ND::OwnerId)
                    .col(ND::Status)
                    .col(ND::ReadAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Routines)
                    .col(id_pk(SR::Id))
                    .col(id_ref(SR::OwnerId))
                    .col(varchar(SR::Name, 128))
                    .col(varchar(SR::ScheduleKind, 32))
                    .col(varchar(SR::ScheduleExpr, 64))
                    .col(varchar(SR::ActionKind, 32))
                    .col(text(SR::ActionPayload))
                    .col(int(SR::Enabled))
                    .col(bigint_null(SR::LastRunAt))
                    .col(varchar_null(SR::LastStatus, 64))
                    .col(bigint(SR::CreatedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_sched_routines_owner")
                            .from(Routines, SR::OwnerId)
                            .to(users::Entity, users::Column::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_sched_routines_owner_created")
                    .table(Routines)
                    .col(SR::OwnerId)
                    .col(SR::CreatedAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_sched_routines_enabled_kind")
                    .table(Routines)
                    .col(SR::Enabled)
                    .col(SR::ScheduleKind)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(VirtualChannels)
                    .col(id_pk(VC::Id))
                    .col(int(VC::Number))
                    .col(varchar(VC::Name, 128))
                    .col(id_ref(VC::LibraryId))
                    .col(varchar_null(VC::GenreFilter, 128))
                    .col(int(VC::Enabled))
                    .col(bigint(VC::CreatedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_virtual_channels_library")
                            .from(VirtualChannels, VC::LibraryId)
                            .to(libraries::Entity, libraries::Column::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_virtual_channels_number")
                    .table(VirtualChannels)
                    .col(VC::Number)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_virtual_channels_enabled_num")
                    .table(VirtualChannels)
                    .col(VC::Enabled)
                    .col(VC::Number)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Webhooks)
                    .col(id_pk(IW::Id))
                    .col(varchar(IW::Name, 128))
                    .col(varchar(IW::SourceKind, 32))
                    .col(varchar(IW::TokenHash, SHA256_HEX_LEN))
                    .col(id_ref_null(IW::LibraryId))
                    .col(int(IW::Enabled))
                    .col(bigint_null(IW::LastTriggeredAt))
                    .col(bigint(IW::CreatedAt))
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_inbound_webhooks_token_hash")
                    .table(Webhooks)
                    .col(IW::TokenHash)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_inbound_webhooks_created")
                    .table(Webhooks)
                    .col(IW::CreatedAt)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Webhooks).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(VirtualChannels).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Routines).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Deliveries).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Channels).to_owned())
            .await
    }
}

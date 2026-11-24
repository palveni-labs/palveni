use sea_orm_migration::prelude::*;

use super::{
    bigint, bigint_null, id_pk, id_ref, id_ref_null, int, text, text_null, varchar, varchar_null,
};
use crate::entities::{
    assistant_conversations::{Column as AC, Entity as Conversations},
    assistant_memories::{Column as AMem, Entity as Memories},
    assistant_messages::{Column as AMsg, Entity as Messages},
    assistant_runs::{Column as AR, Entity as Runs},
    home_bridges::{Column as HB, Entity as Bridges},
    home_devices::{Column as HD, Entity as Devices},
    home_rooms::{Column as HR, Entity as Rooms},
    home_rules::{Column as HRul, Entity as Rules},
    home_scenes::{Column as HS, Entity as Scenes},
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
                    .table(Bridges)
                    .col(id_pk(HB::Id))
                    .col(varchar(HB::Name, 100))
                    .col(text(HB::BaseUrl))
                    .col(text(HB::EncryptedToken))
                    .col(ColumnDef::new(HB::Enabled).boolean().not_null())
                    .col(bigint_null(HB::LastSyncAt))
                    .col(bigint(HB::CreatedAt))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Rooms)
                    .col(varchar(HR::Id, 64).primary_key())
                    .col(varchar(HR::Name, 100))
                    .col(int(HR::SortOrder))
                    .col(bigint(HR::CreatedAt))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Devices)
                    .col(varchar(HD::Id, 128).primary_key())
                    .col(varchar(HD::RoomId, 64))
                    .col(id_ref_null(HD::BridgeId))
                    .col(varchar_null(HD::ExternalEntityId, 255))
                    .col(varchar(HD::Name, 128))
                    .col(varchar(HD::Kind, 32))
                    .col(ColumnDef::new(HD::Online).boolean().not_null())
                    .col(text(HD::StateJson))
                    .col(int(HD::SortOrder))
                    .col(bigint(HD::UpdatedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_home_devices_room")
                            .from(Devices, HD::RoomId)
                            .to(Rooms, HR::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_home_devices_bridge")
                            .from(Devices, HD::BridgeId)
                            .to(Bridges, HB::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_home_devices_room_order")
                    .table(Devices)
                    .col(HD::RoomId)
                    .col(HD::SortOrder)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_home_devices_bridge_entity")
                    .table(Devices)
                    .col(HD::BridgeId)
                    .col(HD::ExternalEntityId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Scenes)
                    .col(varchar(HS::Id, 128).primary_key())
                    .col(id_ref_null(HS::BridgeId))
                    .col(varchar_null(HS::ExternalEntityId, 255))
                    .col(varchar(HS::Name, 100))
                    .col(varchar(HS::Icon, 32))
                    .col(ColumnDef::new(HS::Active).boolean().not_null())
                    .col(text(HS::ActionsJson))
                    .col(int(HS::SortOrder))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_home_scenes_bridge")
                            .from(Scenes, HS::BridgeId)
                            .to(Bridges, HB::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Rules)
                    .col(varchar(HRul::Id, 64).primary_key())
                    .col(varchar(HRul::EventKind, 64))
                    .col(varchar(HRul::TriggerText, 255))
                    .col(varchar(HRul::ActionText, 255))
                    .col(text(HRul::ActionsJson))
                    .col(ColumnDef::new(HRul::Enabled).boolean().not_null())
                    .col(bigint_null(HRul::LastRunAt))
                    .col(int(HRul::SortOrder))
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_home_rules_event_enabled")
                    .table(Rules)
                    .col(HRul::EventKind)
                    .col(HRul::Enabled)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Conversations)
                    .col(id_pk(AC::Id))
                    .col(id_ref(AC::UserId))
                    .col(ColumnDef::new(AC::Archived).boolean().not_null())
                    .col(bigint(AC::CreatedAt))
                    .col(bigint(AC::UpdatedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_assistant_conv_user")
                            .from(Conversations, AC::UserId)
                            .to(users::Entity, users::Column::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_assistant_conv_user_active")
                    .table(Conversations)
                    .col(AC::UserId)
                    .col(AC::Archived)
                    .col(AC::UpdatedAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Runs)
                    .col(id_pk(AR::Id))
                    .col(id_ref(AR::ConversationId))
                    .col(id_ref(AR::UserId))
                    .col(varchar_null(AR::Title, 255))
                    .col(text_null(AR::SubjectJson))
                    .col(text_null(AR::RequestText))
                    .col(text_null(AR::PreviewsJson))
                    .col(varchar(AR::State, 32))
                    .col(text(AR::StepsJson))
                    .col(text(AR::BlocksJson))
                    .col(text_null(AR::PendingActionJson))
                    .col(id_ref_null(AR::TaskId))
                    .col(bigint(AR::CreatedAt))
                    .col(bigint(AR::UpdatedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_assistant_runs_conv")
                            .from(Runs, AR::ConversationId)
                            .to(Conversations, AC::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_assistant_runs_user")
                            .from(Runs, AR::UserId)
                            .to(users::Entity, users::Column::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_assistant_runs_conv_created")
                    .table(Runs)
                    .col(AR::ConversationId)
                    .col(AR::CreatedAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_assistant_runs_user_state")
                    .table(Runs)
                    .col(AR::UserId)
                    .col(AR::State)
                    .col(AR::CreatedAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_assistant_runs_task_id")
                    .table(Runs)
                    .col(AR::TaskId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Messages)
                    .col(id_pk(AMsg::Id))
                    .col(id_ref(AMsg::ConversationId))
                    .col(varchar(AMsg::Role, 16))
                    .col(text(AMsg::Text))
                    .col(id_ref_null(AMsg::RunId))
                    .col(bigint(AMsg::CreatedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_assistant_msgs_conv")
                            .from(Messages, AMsg::ConversationId)
                            .to(Conversations, AC::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_assistant_msgs_run")
                            .from(Messages, AMsg::RunId)
                            .to(Runs, AR::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_assistant_msgs_conv_created")
                    .table(Messages)
                    .col(AMsg::ConversationId)
                    .col(AMsg::CreatedAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Memories)
                    .col(id_pk(AMem::Id))
                    .col(id_ref(AMem::UserId))
                    .col(text(AMem::Content))
                    .col(bigint(AMem::CreatedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_assistant_memories_user")
                            .from(Memories, AMem::UserId)
                            .to(users::Entity, users::Column::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_assistant_memories_user_created")
                    .table(Memories)
                    .col(AMem::UserId)
                    .col(AMem::CreatedAt)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Memories).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Messages).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Runs).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Conversations).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Rules).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Scenes).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Devices).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Rooms).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Bridges).to_owned())
            .await
    }
}

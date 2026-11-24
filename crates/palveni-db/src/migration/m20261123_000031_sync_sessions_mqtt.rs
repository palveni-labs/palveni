use sea_orm_migration::prelude::*;

use super::{
    bigint, bigint_null, id_pk, id_ref, id_ref_null, int, text_null, varchar, varchar_null,
};
use crate::entities::{
    mqtt_bridge::{Column as MB, Entity as MqttBridges},
    playback_session::{Column as PS, Entity as PlaybackSessions},
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
                    .table(PlaybackSessions)
                    .col(
                        ColumnDef::new(PS::Id)
                            .string_len(64)
                            .not_null()
                            .primary_key()
                            .take(),
                    )
                    .col(id_ref(PS::UserId))
                    .col(varchar(PS::DeviceName, 128))
                    .col(varchar(PS::ClientKind, 32))
                    .col(varchar(PS::MediaKind, 32))
                    .col(id_ref_null(PS::ItemId))
                    .col(id_ref_null(PS::LibraryId))
                    .col(varchar_null(PS::Title, 255))
                    .col(varchar_null(PS::Subtitle, 255))
                    .col(varchar_null(PS::ArtworkId, 80))
                    .col(varchar(PS::State, 32))
                    .col(bigint(PS::PositionMs))
                    .col(bigint(PS::DurationMs))
                    .col(int(PS::Volume))
                    .col(varchar_null(PS::AmbientHex, 16))
                    .col(bigint(PS::UpdatedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_playback_sessions_user")
                            .from(PlaybackSessions, PS::UserId)
                            .to(users::Entity, users::Column::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_playback_sessions_user_updated")
                    .table(PlaybackSessions)
                    .col(PS::UserId)
                    .col(PS::UpdatedAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(MqttBridges)
                    .col(id_pk(MB::Id))
                    .col(varchar(MB::Name, 128))
                    .col(varchar(MB::Host, 255))
                    .col(int(MB::Port))
                    .col(varchar_null(MB::Username, 128))
                    .col(text_null(MB::PasswordCipher))
                    .col(varchar(MB::TopicPrefix, 128))
                    .col(varchar_null(MB::AmbientLightDeviceId, 64))
                    .col(int(MB::PublishPlayback))
                    .col(int(MB::Enabled))
                    .col(bigint_null(MB::LastPublishedAt))
                    .col(bigint(MB::CreatedAt))
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_mqtt_bridges_created")
                    .table(MqttBridges)
                    .col(MB::CreatedAt)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(MqttBridges).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(PlaybackSessions).to_owned())
            .await
    }
}

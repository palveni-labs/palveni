use super::{bigint, bigint_null, id_ref, text_null};
use crate::entities::playlists::{Column as P, Entity as Playlists};
use crate::entities::{
    items,
    subsonic_plays::{Column as C, Entity as Plays},
    users,
};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[allow(elided_lifetimes_in_paths)]
#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Playlists)
                    .add_column(text_null(P::Comment))
                    .to_owned(),
            )
            .await?;
        for column in [P::CreatedAt, P::ChangedAt] {
            manager
                .alter_table(
                    Table::alter()
                        .table(Playlists)
                        .add_column(bigint_null(column))
                        .to_owned(),
                )
                .await?;
        }
        manager
            .create_table(
                Table::create()
                    .table(Plays)
                    .col(id_ref(C::UserId))
                    .col(id_ref(C::ItemId))
                    .col(bigint(C::PlayedAt))
                    .primary_key(
                        Index::create()
                            .col(C::UserId)
                            .col(C::ItemId)
                            .col(C::PlayedAt),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_music_listen_user")
                            .from(Plays, C::UserId)
                            .to(users::Entity, users::Column::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_music_listen_item")
                            .from(Plays, C::ItemId)
                            .to(items::Entity, items::Column::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Plays).to_owned())
            .await?;
        for column in [P::ChangedAt, P::CreatedAt, P::Comment] {
            manager
                .alter_table(
                    Table::alter()
                        .table(Playlists)
                        .drop_column(column)
                        .to_owned(),
                )
                .await?;
        }
        Ok(())
    }
}

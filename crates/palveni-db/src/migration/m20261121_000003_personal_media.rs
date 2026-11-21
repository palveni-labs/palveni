use sea_orm_migration::prelude::*;

use super::{bigint, bigint_null, id_pk, id_ref, int, varchar};
use crate::entities::{
    items,
    playlist_entries::{Column as E, Entity as Entries},
    playlists::{Column as P, Entity as Playlists},
    user_item_state::{Column as S, Entity as States},
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
                    .table(States)
                    .col(id_ref(S::UserId))
                    .col(id_ref(S::ItemId))
                    .primary_key(Index::create().col(S::UserId).col(S::ItemId))
                    .col(bigint_null(S::FavoriteOrder))
                    .col(int(S::Stars))
                    .col(bigint(S::PositionMs))
                    .col(ColumnDef::new(S::Completed).boolean().not_null())
                    .col(bigint(S::PlayCount))
                    .col(bigint_null(S::LastPlayedAt))
                    .col(bigint(S::PlaybackVersion))
                    .col(bigint_null(S::PreviousVersion))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_state_user")
                            .from(States, S::UserId)
                            .to(users::Entity, users::Column::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_state_item")
                            .from(States, S::ItemId)
                            .to(items::Entity, items::Column::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;
        for (name, column) in [
            ("idx_state_favorites", S::FavoriteOrder),
            ("idx_state_recent", S::PlaybackVersion),
        ] {
            manager
                .create_index(
                    Index::create()
                        .name(name)
                        .table(States)
                        .col(S::UserId)
                        .col(column)
                        .to_owned(),
                )
                .await?;
        }
        manager
            .create_table(
                Table::create()
                    .table(Playlists)
                    .col(id_pk(P::Id))
                    .col(id_ref(P::UserId))
                    .col(varchar(P::Name, 128))
                    .col(bigint(P::Version))
                    .col(ColumnDef::new(P::Deleted).boolean().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_playlist_user")
                            .from(Playlists, P::UserId)
                            .to(users::Entity, users::Column::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_playlist_owner")
                    .table(Playlists)
                    .col(P::UserId)
                    .col(P::Deleted)
                    .col(P::Version)
                    .to_owned(),
            )
            .await?;
        manager
            .create_table(
                Table::create()
                    .table(Entries)
                    .col(id_ref(E::PlaylistId))
                    .col(int(E::Position))
                    .col(id_ref(E::ItemId))
                    .primary_key(Index::create().col(E::PlaylistId).col(E::Position))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_entry_playlist")
                            .from(Entries, E::PlaylistId)
                            .to(Playlists, P::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_entry_item")
                            .from(Entries, E::ItemId)
                            .to(items::Entity, items::Column::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_playlist_item")
                    .table(Entries)
                    .col(E::ItemId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Entries).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Playlists).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(States).to_owned())
            .await
    }
}

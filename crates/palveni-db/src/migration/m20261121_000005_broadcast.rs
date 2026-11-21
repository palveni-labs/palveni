use sea_orm_migration::prelude::*;

use super::{bigint, id_pk, id_ref, int, varchar};
use crate::entities::{
    broadcast_entries::{Column as E, Entity as Entries},
    broadcast_favorites::{Column as F, Entity as Favorites},
    broadcast_sources::{Column as S, Entity as Sources},
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
                    .table(Sources)
                    .col(id_pk(S::Id))
                    .col(varchar(S::Kind, 12))
                    .col(varchar(S::Name, 128))
                    .col(bigint(S::Version))
                    .col(int(S::EntryCount))
                    .col(ColumnDef::new(S::FeedUrl).text().null())
                    .col(ColumnDef::new(S::GuideUrl).text().null())
                    .to_owned(),
            )
            .await?;
        manager
            .create_table(
                Table::create()
                    .table(Entries)
                    .col(id_ref(E::SourceId))
                    .col(varchar(E::EntryId, 64))
                    .col(int(E::Position))
                    .col(ColumnDef::new(E::Payload).text().not_null())
                    .primary_key(Index::create().col(E::SourceId).col(E::EntryId))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_bcast_entry_source")
                            .from(Entries, E::SourceId)
                            .to(Sources, S::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_bcast_entry_order")
                    .table(Entries)
                    .col(E::SourceId)
                    .col(E::Position)
                    .unique()
                    .to_owned(),
            )
            .await?;
        manager
            .create_table(
                Table::create()
                    .table(Favorites)
                    .col(id_ref(F::UserId))
                    .col(id_ref(F::SourceId))
                    .col(varchar(F::EntryId, 64))
                    .primary_key(
                        Index::create()
                            .col(F::UserId)
                            .col(F::SourceId)
                            .col(F::EntryId),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_bcast_favorite_user")
                            .from(Favorites, F::UserId)
                            .to(users::Entity, users::Column::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_bcast_favorite_source")
                            .from(Favorites, F::SourceId)
                            .to(Sources, S::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Favorites).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Entries).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Sources).to_owned())
            .await
    }
}

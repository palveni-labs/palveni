//! Library-level film and series metadata: details, genres, people and
//! favorite works. Existing rows gain the columns empty and are refilled by
//! the next scan, whose sidecar signature changes with this release.
use sea_orm::EntityName;
use sea_orm_migration::prelude::*;

use super::{bigint, bigint_null, id_ref, int, text_null, varchar, varchar_null};
use crate::entities::{
    items, libraries, users,
    video_favorites::{Column as F, Entity as Favorites},
    video_genres::{Column as G, Entity as Genres},
    video_metadata::{Column as C, Entity as Videos},
    video_people::{Column as P, Entity as People},
};

#[derive(DeriveMigrationName)]
pub struct Migration;

fn columns() -> Vec<ColumnDef> {
    vec![
        text_null(C::WorkOriginalTitle),
        text_null(C::WorkTagline),
        ColumnDef::new(C::WorkRating).integer().null().take(),
        varchar_null(C::WorkOfficialRating, 32),
        varchar_null(C::WorkPremiered, 10),
        text_null(C::WorkStudios),
        text_null(C::WorkBackdrop),
        bigint_null(C::RuntimeMs),
        ColumnDef::new(C::Rating).integer().null().take(),
        varchar_null(C::Premiered, 10),
        text_null(C::Thumb),
    ]
}

fn item_fk(
    name: &str,
    table: impl IntoTableRef,
    column: impl IntoIden,
) -> ForeignKeyCreateStatement {
    ForeignKey::create()
        .name(name)
        .from(table, column)
        .to(items::Entity, items::Column::Id)
        .on_delete(ForeignKeyAction::Cascade)
        .to_owned()
}

#[allow(elided_lifetimes_in_paths)]
#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for col in columns() {
            if !manager
                .has_column(Videos.table_name(), col.get_column_name())
                .await?
            {
                manager
                    .alter_table(Table::alter().table(Videos).add_column(col).to_owned())
                    .await?;
            }
        }
        manager
            .create_table(
                Table::create()
                    .table(Genres)
                    .col(id_ref(G::ItemId))
                    .col(varchar(G::Name, 128))
                    .primary_key(Index::create().col(G::ItemId).col(G::Name))
                    .foreign_key(&mut item_fk("fk_video_genres_item", Genres, G::ItemId))
                    .to_owned(),
            )
            .await?;
        manager
            .create_table(
                Table::create()
                    .table(People)
                    .col(id_ref(P::ItemId))
                    .col(int(P::Position))
                    .col(varchar(P::Kind, 16))
                    .col(varchar(P::Name, 256))
                    .col(varchar(P::PersonKey, 64))
                    .col(text_null(P::Role))
                    .col(text_null(P::Image))
                    .primary_key(Index::create().col(P::ItemId).col(P::Position))
                    .foreign_key(&mut item_fk("fk_video_people_item", People, P::ItemId))
                    .to_owned(),
            )
            .await?;
        manager
            .create_table(
                Table::create()
                    .table(Favorites)
                    .col(id_ref(F::UserId))
                    .col(id_ref(F::LibraryId))
                    .col(varchar(F::WorkKey, 64))
                    .col(bigint(F::CreatedAt))
                    .primary_key(
                        Index::create()
                            .col(F::UserId)
                            .col(F::LibraryId)
                            .col(F::WorkKey),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_video_favorites_user")
                            .from(Favorites, F::UserId)
                            .to(users::Entity, users::Column::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_video_favorites_library")
                            .from(Favorites, F::LibraryId)
                            .to(libraries::Entity, libraries::Column::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_video_genres_name")
                    .table(Genres)
                    .col(G::Name)
                    .col(G::ItemId)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_video_people_key")
                    .table(People)
                    .col(P::PersonKey)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for table in [
            Favorites.table_name(),
            People.table_name(),
            Genres.table_name(),
        ] {
            manager
                .drop_table(
                    Table::drop()
                        .table(Alias::new(table))
                        .if_exists()
                        .to_owned(),
                )
                .await?;
        }
        for col in columns() {
            let name = col.get_column_name();
            if manager.has_column(Videos.table_name(), &name).await? {
                manager
                    .alter_table(
                        Table::alter()
                            .table(Videos)
                            .drop_column(Alias::new(name))
                            .to_owned(),
                    )
                    .await?;
            }
        }
        Ok(())
    }
}

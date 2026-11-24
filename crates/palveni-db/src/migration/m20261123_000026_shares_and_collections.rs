//! Public/password-protected share links and cross-media collections.

use sea_orm::EntityName;
use sea_orm_migration::prelude::*;

use super::{
    bigint, bigint_null, id_pk, id_ref, id_ref_null, int, text_null, varchar, varchar_null,
};
use crate::entities::{
    media_collection_items::{Column as CI, Entity as CollectionItems},
    media_collections::{Column as C, Entity as Collections},
    share_links::{Column as S, Entity as Shares},
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
                    .table(Shares)
                    .col(id_pk(S::Id))
                    .col(varchar(S::Token, 64).unique_key())
                    .col(id_ref(S::CreatorId))
                    .col(varchar(S::TargetKind, 32))
                    .col(varchar(S::TargetId, 128))
                    .col(id_ref_null(S::LibraryId))
                    .col(varchar(S::Title, 255))
                    .col(text_null(S::PasswordHash))
                    .col(int(S::AllowDownload))
                    .col(int(S::StripGps))
                    .col(bigint_null(S::ExpiresAt))
                    .col(ColumnDef::new(S::MaxViews).integer().null())
                    .col(bigint(S::ViewCount))
                    .col(bigint_null(S::RevokedAt))
                    .col(bigint(S::CreatedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_share_links_creator")
                            .from(Shares, S::CreatorId)
                            .to(users::Entity, users::Column::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Collections)
                    .col(id_pk(C::Id))
                    .col(id_ref(C::OwnerId))
                    .col(varchar(C::Title, 255))
                    .col(text_null(C::Description))
                    .col(varchar_null(C::CoverArtworkId, 80))
                    .col(bigint(C::CreatedAt))
                    .col(bigint(C::UpdatedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_media_collections_owner")
                            .from(Collections, C::OwnerId)
                            .to(users::Entity, users::Column::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(CollectionItems)
                    .col(id_pk(CI::Id))
                    .col(id_ref(CI::CollectionId))
                    .col(varchar(CI::EntryKind, 32))
                    .col(varchar(CI::TargetId, 128))
                    .col(id_ref_null(CI::LibraryId))
                    .col(varchar(CI::Title, 255))
                    .col(varchar_null(CI::Subtitle, 255))
                    .col(varchar_null(CI::ArtworkId, 80))
                    .col(int(CI::Position))
                    .col(bigint(CI::AddedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_media_collection_items_collection")
                            .from(CollectionItems, CI::CollectionId)
                            .to(Collections, C::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_share_links_creator_created")
                    .table(Shares)
                    .col(S::CreatorId)
                    .col(S::CreatedAt)
                    .col(S::Id)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_share_links_target")
                    .table(Shares)
                    .col(S::TargetKind)
                    .col(S::TargetId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_media_collections_owner_updated")
                    .table(Collections)
                    .col(C::OwnerId)
                    .col(C::UpdatedAt)
                    .col(C::Id)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_media_collection_items_order")
                    .table(CollectionItems)
                    .col(CI::CollectionId)
                    .col(CI::Position)
                    .col(CI::Id)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_media_collection_items_target")
                    .table(CollectionItems)
                    .col(CI::CollectionId)
                    .col(CI::EntryKind)
                    .col(CI::TargetId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for table in [
            CollectionItems.table_name(),
            Collections.table_name(),
            Shares.table_name(),
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
        Ok(())
    }
}

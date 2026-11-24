use sea_orm_migration::prelude::*;

use super::{SHA256_HEX_LEN, bigint, bigint_null, id_pk, id_ref, varchar};
use crate::entities::{
    client_api_key::{Column as K, Entity as Keys},
    items, users,
    video_segment::{Column as S, Entity as Segments},
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
                    .table(Keys)
                    .col(id_pk(K::Id))
                    .col(id_ref(K::UserId))
                    .col(varchar(K::Name, 128))
                    .col(varchar(K::ClientKind, 32))
                    .col(varchar(K::KeyPrefix, 16))
                    .col(varchar(K::KeyHash, SHA256_HEX_LEN))
                    .col(bigint_null(K::LastUsedAt))
                    .col(bigint_null(K::RevokedAt))
                    .col(bigint(K::CreatedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_client_api_keys_user")
                            .from(Keys, K::UserId)
                            .to(users::Entity, users::Column::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_client_api_keys_key_hash")
                    .table(Keys)
                    .col(K::KeyHash)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_client_api_keys_user_revoked")
                    .table(Keys)
                    .col(K::UserId)
                    .col(K::RevokedAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Segments)
                    .col(id_pk(S::Id))
                    .col(id_ref(S::ItemId))
                    .col(varchar(S::Kind, 32))
                    .col(bigint(S::StartMs))
                    .col(bigint(S::EndMs))
                    .col(varchar(S::Source, 32))
                    .col(bigint(S::UpdatedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_video_segments_item")
                            .from(Segments, S::ItemId)
                            .to(items::Entity, items::Column::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_video_segments_item_kind")
                    .table(Segments)
                    .col(S::ItemId)
                    .col(S::Kind)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Segments).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Keys).to_owned())
            .await
    }
}

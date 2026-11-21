use super::{SHA256_HEX_LEN, bigint, id_ref, text, varchar};
use crate::entities::{libraries, scan_review_items as entries, scan_reviews as reviews};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[allow(elided_lifetimes_in_paths)]
#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        use entries::Column as E;
        use reviews::Column as R;
        manager
            .create_table(
                Table::create()
                    .table(reviews::Entity)
                    .if_not_exists()
                    .col(varchar(R::Revision, SHA256_HEX_LEN).primary_key())
                    .col(id_ref(R::LibraryId))
                    .col(varchar(R::ScopeHash, SHA256_HEX_LEN))
                    .col(text(R::Scope))
                    .col(varchar(R::SourceHash, SHA256_HEX_LEN))
                    .col(varchar(R::Digest, SHA256_HEX_LEN))
                    .col(bigint(R::MissingCount))
                    .index(
                        Index::create()
                            .name("uq_scan_review_scope")
                            .unique()
                            .col(R::LibraryId)
                            .col(R::ScopeHash),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_scan_review_library")
                            .from(reviews::Entity, R::LibraryId)
                            .to(libraries::Entity, libraries::Column::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_table(
                Table::create()
                    .if_not_exists()
                    .table(entries::Entity)
                    .col(varchar(E::Revision, SHA256_HEX_LEN))
                    .col(bigint(E::Ordinal))
                    .col(id_ref(E::ItemId))
                    .col(text(E::Path))
                    .primary_key(Index::create().col(E::Revision).col(E::Ordinal))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_scan_review_item_revision")
                            .from(entries::Entity, E::Revision)
                            .to(reviews::Entity, R::Revision)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(entries::Entity).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(reviews::Entity).to_owned())
            .await
    }
}

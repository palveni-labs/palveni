use sea_orm_migration::prelude::*;

use super::{id_pk, id_ref, text, text_null, varchar};
use crate::entities::{
    items,
    video_metadata::{Column as C, Entity as Videos},
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
                    .table(Videos)
                    .col(id_pk(C::ItemId))
                    .col(id_ref(C::LibraryId))
                    .col(varchar(C::Stamp, 64))
                    .col(varchar(C::WorkKey, 64))
                    .col(varchar(C::WorkSort, 160))
                    .col(text(C::WorkTitle))
                    .col(ColumnDef::new(C::WorkYear).integer().null())
                    .col(text_null(C::WorkOverview))
                    .col(text_null(C::WorkPoster))
                    .col(ColumnDef::new(C::Season).integer().null())
                    .col(text_null(C::SeasonPoster))
                    .col(ColumnDef::new(C::Episode).integer().null())
                    .col(varchar(C::EpisodeSort, 180))
                    .col(text(C::Title))
                    .col(text_null(C::Overview))
                    .col(text_null(C::EpisodePoster))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_video_metadata_item")
                            .from(Videos, C::ItemId)
                            .to(items::Entity, items::Column::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;
        for (name, fields) in [
            (
                "idx_video_work",
                vec![C::LibraryId, C::WorkSort, C::WorkKey],
            ),
            (
                "idx_video_episode",
                vec![
                    C::LibraryId,
                    C::WorkKey,
                    C::Season,
                    C::EpisodeSort,
                    C::ItemId,
                ],
            ),
        ] {
            let mut index = Index::create().name(name).table(Videos).to_owned();
            for field in fields {
                index.col(field);
            }
            manager.create_index(index).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Videos).to_owned())
            .await
    }
}

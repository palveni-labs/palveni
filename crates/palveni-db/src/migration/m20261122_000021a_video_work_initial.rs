use sea_orm::{ColumnTrait, EntityName, EntityTrait, QueryFilter, QuerySelect, Set};
use sea_orm_migration::prelude::*;

use crate::entities::video_metadata::{self, Column as C, Entity as Videos};
use crate::video::work_initial;

#[derive(DeriveMigrationName)]
pub struct Migration;

const INDEX: &str = "idx_video_work_initial";

#[allow(elided_lifetimes_in_paths)]
#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if !manager
            .has_column(Videos.table_name(), "work_initial")
            .await?
        {
            manager
                .alter_table(
                    Table::alter()
                        .table(Videos)
                        .add_column(
                            ColumnDef::new(C::WorkInitial)
                                .string_len(1)
                                .not_null()
                                .default("#"),
                        )
                        .to_owned(),
                )
                .await?;
        }
        let conn = manager.get_connection();
        let works: Vec<(String, String, String)> = Videos::find()
            .select_only()
            .columns([C::LibraryId, C::WorkKey, C::WorkTitle])
            .distinct()
            .into_tuple()
            .all(conn)
            .await?;
        for (library, key, title) in works {
            let initial = work_initial(&title);
            Videos::update_many()
                .set(video_metadata::ActiveModel {
                    work_initial: Set(initial.clone()),
                    ..Default::default()
                })
                .filter(C::LibraryId.eq(library))
                .filter(C::WorkKey.eq(key))
                .filter(C::WorkTitle.eq(title))
                .filter(C::WorkInitial.ne(initial))
                .exec(conn)
                .await?;
        }
        if !manager.has_index(Videos.table_name(), INDEX).await? {
            manager
                .create_index(
                    Index::create()
                        .name(INDEX)
                        .table(Videos)
                        .col(C::LibraryId)
                        .col(C::WorkInitial)
                        .col(C::WorkSort)
                        .col(C::WorkKey)
                        .to_owned(),
                )
                .await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.has_index(Videos.table_name(), INDEX).await? {
            manager
                .drop_index(Index::drop().name(INDEX).table(Videos).to_owned())
                .await?;
        }
        if manager
            .has_column(Videos.table_name(), "work_initial")
            .await?
        {
            manager
                .alter_table(
                    Table::alter()
                        .table(Videos)
                        .drop_column(C::WorkInitial)
                        .to_owned(),
                )
                .await?;
        }
        Ok(())
    }
}

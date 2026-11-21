use super::{id_ref, varchar_null};
use crate::entities::{libraries, subsonic_folders as folders, users};
use sea_orm::sea_query::Expr;
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
                    .table(users::Entity)
                    .add_column(varchar_null(users::Column::SubsonicGeneration, 64))
                    .to_owned(),
            )
            .await?;
        manager
            .create_table(
                Table::create()
                    .table(folders::Entity)
                    .col(
                        ColumnDef::new(folders::Column::Id)
                            .integer()
                            .not_null()
                            .primary_key()
                            .auto_increment(),
                    )
                    .col(id_ref(folders::Column::LibraryId).unique_key())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_subsonic_folder_library")
                            .from(folders::Entity, folders::Column::LibraryId)
                            .to(libraries::Entity, libraries::Column::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;
        let existing = Query::select()
            .column(libraries::Column::Id)
            .from(libraries::Entity)
            .and_where(Expr::col(libraries::Column::Kind).eq("music"))
            .and_where(Expr::col(libraries::Column::DeletedAt).is_null())
            .order_by(libraries::Column::Id, Order::Asc)
            .to_owned();
        let insert = Query::insert()
            .into_table(folders::Entity)
            .columns([folders::Column::LibraryId])
            .select_from(existing)
            .map_err(|err| DbErr::Custom(err.to_string()))?
            .to_owned();
        manager.exec_stmt(insert).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(folders::Entity).to_owned())
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(users::Entity)
                    .drop_column(users::Column::SubsonicGeneration)
                    .to_owned(),
            )
            .await
    }
}

use super::{id_ref, int, varchar};
use crate::entities::{music_collection_favorites as favorites, subsonic_folders, users};
use favorites::Column as C;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[allow(elided_lifetimes_in_paths)]
#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(favorites::Entity)
                    .col(id_ref(C::UserId))
                    .col(int(C::FolderId))
                    .col(varchar(C::ArtistKey, 64))
                    .col(varchar(C::AlbumKey, 64))
                    .primary_key(
                        Index::create()
                            .col(C::UserId)
                            .col(C::FolderId)
                            .col(C::ArtistKey)
                            .col(C::AlbumKey),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_music_collection_favorite_user")
                            .from(favorites::Entity, C::UserId)
                            .to(users::Entity, users::Column::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_music_collection_favorite_folder")
                            .from(favorites::Entity, C::FolderId)
                            .to(subsonic_folders::Entity, subsonic_folders::Column::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(favorites::Entity).to_owned())
            .await
    }
}

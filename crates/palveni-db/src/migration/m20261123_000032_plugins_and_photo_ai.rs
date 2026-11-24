//! Recipe/widget plugin installations, local photo AI tags, people profiles and person tags.
use sea_orm::EntityName;
use sea_orm_migration::prelude::*;

use super::{
    bigint, bigint_null, id_pk, id_ref, id_ref_null, int, text, text_null, varchar, varchar_null,
};
use crate::entities::{
    items,
    photo_ai_tag::{Column as PAT, Entity as PhotoAiTags},
    photo_person::{Column as PP, Entity as PhotoPeople},
    photo_person_tag::{Column as PPT, Entity as PhotoPersonTags},
    plugin_installation::{Column as PI, Entity as PluginInstallations},
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
                    .table(PluginInstallations)
                    .col(
                        ColumnDef::new(PI::PluginId)
                            .string_len(128)
                            .not_null()
                            .primary_key(),
                    )
                    .col(varchar(PI::Name, 128))
                    .col(varchar(PI::Version, 32))
                    .col(varchar_null(PI::PreviousVersion, 32))
                    .col(varchar(PI::Runtime, 32))
                    .col(text(PI::ProvidesJson))
                    .col(text(PI::PermissionsJson))
                    .col(text(PI::GrantedPermissionsJson))
                    .col(text(PI::SettingsSchemaJson))
                    .col(text(PI::SettingsJson))
                    .col(int(PI::Enabled))
                    .col(varchar(PI::Source, 32))
                    .col(bigint(PI::InstalledAt))
                    .col(bigint(PI::UpdatedAt))
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_plugin_installations_updated")
                    .table(PluginInstallations)
                    .col(PI::UpdatedAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(PhotoAiTags)
                    .col(id_pk(PAT::ItemId))
                    .col(bigint_null(PAT::Dhash))
                    .col(varchar_null(PAT::DominantColor, 32))
                    .col(varchar_null(PAT::DominantHex, 16))
                    .col(text(PAT::SceneTagsJson))
                    .col(varchar_null(PAT::TimeOfDay, 32))
                    .col(varchar_null(PAT::Season, 32))
                    .col(bigint(PAT::AnalyzedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_photo_ai_tags_item")
                            .from(PhotoAiTags, PAT::ItemId)
                            .to(items::Entity, items::Column::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_photo_ai_tags_dhash")
                    .table(PhotoAiTags)
                    .col(PAT::Dhash)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_photo_ai_tags_color")
                    .table(PhotoAiTags)
                    .col(PAT::DominantColor)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_photo_ai_tags_time_season")
                    .table(PhotoAiTags)
                    .col(PAT::TimeOfDay)
                    .col(PAT::Season)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(PhotoPeople)
                    .col(id_pk(PP::Id))
                    .col(id_ref(PP::OwnerId))
                    .col(varchar(PP::Name, 128))
                    .col(id_ref_null(PP::CoverItemId))
                    .col(bigint(PP::CreatedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_photo_people_owner")
                            .from(PhotoPeople, PP::OwnerId)
                            .to(users::Entity, users::Column::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_photo_people_owner_name")
                    .table(PhotoPeople)
                    .col(PP::OwnerId)
                    .col(PP::Name)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(PhotoPersonTags)
                    .col(id_ref(PPT::PersonId))
                    .col(id_ref(PPT::ItemId))
                    .col(text_null(PPT::BboxJson))
                    .col(bigint(PPT::CreatedAt))
                    .primary_key(Index::create().col(PPT::PersonId).col(PPT::ItemId))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_photo_person_tags_person")
                            .from(PhotoPersonTags, PPT::PersonId)
                            .to(PhotoPeople, PP::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_photo_person_tags_item")
                            .from(PhotoPersonTags, PPT::ItemId)
                            .to(items::Entity, items::Column::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_photo_person_tags_item")
                    .table(PhotoPersonTags)
                    .col(PPT::ItemId)
                    .col(PPT::PersonId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for table in [
            PhotoPersonTags.table_name(),
            PhotoPeople.table_name(),
            PhotoAiTags.table_name(),
            PluginInstallations.table_name(),
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

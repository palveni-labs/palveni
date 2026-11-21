//! Main store, initial schema.

use sea_orm_migration::prelude::*;

use super::{
    SHA256_HEX_LEN, bigint, bigint_null, id_pk, id_ref, id_ref_null, int, text, text_null, varchar,
    varchar_null,
};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
    Username,
    UsernameKey,
    PasswordHash,
    Role,
    CreatedAt,
    UpdatedAt,
    DisabledAt,
}

#[derive(DeriveIden)]
enum Devices {
    Table,
    Id,
    UserId,
    Name,
    PublicKey,
    CreatedAt,
    LastSeenAt,
    RevokedAt,
}

#[derive(DeriveIden)]
enum Sessions {
    Table,
    Id,
    UserId,
    DeviceId,
    TokenHash,
    CreatedAt,
    LastUsedAt,
    ExpiresAt,
    RevokedAt,
}

#[derive(DeriveIden)]
enum Libraries {
    Table,
    Id,
    Name,
    NameKey,
    Kind,
    LastScanAt,
    CreatedAt,
    UpdatedAt,
    Revision,
    DeletedAt,
}

#[derive(DeriveIden)]
enum LibraryPaths {
    Table,
    Id,
    LibraryId,
    Path,
    SortOrder,
    CreatedAt,
}

#[derive(DeriveIden)]
enum Items {
    Table,
    Id,
    LibraryId,
    ParentId,
    Kind,
    Title,
    SortTitle,
    SearchText,
    SortGroup,
    Path,
    PathHash,
    SizeBytes,
    ModifiedAt,
    CreatedAt,
    UpdatedAt,
    Revision,
    DeletedAt,
}

#[derive(DeriveIden)]
enum Settings {
    Table,
    SettingKey,
    SettingValue,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum SyncState {
    Table,
    Id,
    Revision,
}

// The trait is declared with an elided `SchemaManager` lifetime; naming it
// here would not match the async-trait desugaring.
#[allow(elided_lifetimes_in_paths)]
#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // users
        manager
            .create_table(
                Table::create()
                    .table(Users::Table)
                    .col(id_pk(Users::Id))
                    .col(varchar(Users::Username, 64))
                    .col(varchar(Users::UsernameKey, 64))
                    .col(text(Users::PasswordHash))
                    .col(varchar(Users::Role, 16))
                    .col(bigint(Users::CreatedAt))
                    .col(bigint(Users::UpdatedAt))
                    .col(bigint_null(Users::DisabledAt))
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_users_username_key")
                    .table(Users::Table)
                    .col(Users::UsernameKey)
                    .unique()
                    .to_owned(),
            )
            .await?;

        // devices
        manager
            .create_table(
                Table::create()
                    .table(Devices::Table)
                    .col(id_pk(Devices::Id))
                    .col(id_ref(Devices::UserId))
                    .col(varchar(Devices::Name, 128))
                    .col(varchar_null(Devices::PublicKey, 128))
                    .col(bigint(Devices::CreatedAt))
                    .col(bigint(Devices::LastSeenAt))
                    .col(bigint_null(Devices::RevokedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_devices_user")
                            .from(Devices::Table, Devices::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_devices_user")
                    .table(Devices::Table)
                    .col(Devices::UserId)
                    .to_owned(),
            )
            .await?;

        // sessions
        manager
            .create_table(
                Table::create()
                    .table(Sessions::Table)
                    .col(id_pk(Sessions::Id))
                    .col(id_ref(Sessions::UserId))
                    .col(id_ref_null(Sessions::DeviceId))
                    .col(varchar(Sessions::TokenHash, SHA256_HEX_LEN))
                    .col(bigint(Sessions::CreatedAt))
                    .col(bigint(Sessions::LastUsedAt))
                    .col(bigint_null(Sessions::ExpiresAt))
                    .col(bigint_null(Sessions::RevokedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_sessions_user")
                            .from(Sessions::Table, Sessions::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_sessions_device")
                            .from(Sessions::Table, Sessions::DeviceId)
                            .to(Devices::Table, Devices::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_sessions_token_hash")
                    .table(Sessions::Table)
                    .col(Sessions::TokenHash)
                    .unique()
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_sessions_user")
                    .table(Sessions::Table)
                    .col(Sessions::UserId)
                    .to_owned(),
            )
            .await?;

        // libraries
        manager
            .create_table(
                Table::create()
                    .table(Libraries::Table)
                    .col(id_pk(Libraries::Id))
                    .col(varchar(Libraries::Name, 100))
                    .col(varchar(Libraries::NameKey, 100))
                    .col(varchar(Libraries::Kind, 16))
                    .col(bigint_null(Libraries::LastScanAt))
                    .col(bigint(Libraries::CreatedAt))
                    .col(bigint(Libraries::UpdatedAt))
                    .col(bigint(Libraries::Revision))
                    .col(bigint_null(Libraries::DeletedAt))
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_libraries_name_key")
                    .table(Libraries::Table)
                    .col(Libraries::NameKey)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_libraries_revision")
                    .table(Libraries::Table)
                    .col(Libraries::Revision)
                    .to_owned(),
            )
            .await?;

        // library_paths
        manager
            .create_table(
                Table::create()
                    .table(LibraryPaths::Table)
                    .col(id_pk(LibraryPaths::Id))
                    .col(id_ref(LibraryPaths::LibraryId))
                    .col(text(LibraryPaths::Path))
                    .col(int(LibraryPaths::SortOrder))
                    .col(bigint(LibraryPaths::CreatedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_library_paths_library")
                            .from(LibraryPaths::Table, LibraryPaths::LibraryId)
                            .to(Libraries::Table, Libraries::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_library_paths_library")
                    .table(LibraryPaths::Table)
                    .col(LibraryPaths::LibraryId)
                    .to_owned(),
            )
            .await?;

        // items
        //
        // `parent_id` deliberately has no foreign key: MySQL/InnoDB limits
        // cascading actions to 15 levels, which a deep directory tree can
        // exceed, and the scanner already maintains parent integrity.
        manager
            .create_table(
                Table::create()
                    .table(Items::Table)
                    .col(id_pk(Items::Id))
                    .col(id_ref(Items::LibraryId))
                    .col(id_ref_null(Items::ParentId))
                    .col(varchar(Items::Kind, 16))
                    .col(text(Items::Title))
                    .col(varchar(Items::SortTitle, 255))
                    .col(text(Items::SearchText))
                    .col(int(Items::SortGroup))
                    .col(text_null(Items::Path))
                    .col(varchar_null(Items::PathHash, SHA256_HEX_LEN))
                    .col(bigint_null(Items::SizeBytes))
                    .col(bigint_null(Items::ModifiedAt))
                    .col(bigint(Items::CreatedAt))
                    .col(bigint(Items::UpdatedAt))
                    .col(bigint(Items::Revision))
                    .col(bigint_null(Items::DeletedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_items_library")
                            .from(Items::Table, Items::LibraryId)
                            .to(Libraries::Table, Libraries::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_items_library_path_hash")
                    .table(Items::Table)
                    .col(Items::LibraryId)
                    .col(Items::PathHash)
                    .unique()
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_items_browse")
                    .table(Items::Table)
                    .col(Items::LibraryId)
                    .col(Items::ParentId)
                    .col(Items::SortGroup)
                    .col(Items::SortTitle)
                    .col(Items::Id)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_items_library_group")
                    .table(Items::Table)
                    .col(Items::LibraryId)
                    .col(Items::SortGroup)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_items_revision")
                    .table(Items::Table)
                    .col(Items::Revision)
                    .to_owned(),
            )
            .await?;

        // settings
        manager
            .create_table(
                Table::create()
                    .table(Settings::Table)
                    .col(
                        ColumnDef::new(Settings::SettingKey)
                            .string_len(100)
                            .not_null()
                            .primary_key(),
                    )
                    .col(text(Settings::SettingValue))
                    .col(bigint(Settings::UpdatedAt))
                    .to_owned(),
            )
            .await?;

        // sync_state: the row itself is inserted at startup by `Db::open`.
        manager
            .create_table(
                Table::create()
                    .table(SyncState::Table)
                    .col(
                        ColumnDef::new(SyncState::Id)
                            .integer()
                            .not_null()
                            .primary_key(),
                    )
                    .col(bigint(SyncState::Revision))
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for table in [
            SyncState::Table.into_iden(),
            Settings::Table.into_iden(),
            Items::Table.into_iden(),
            LibraryPaths::Table.into_iden(),
            Libraries::Table.into_iden(),
            Sessions::Table.into_iden(),
            Devices::Table.into_iden(),
            Users::Table.into_iden(),
        ] {
            manager
                .drop_table(Table::drop().table(table).to_owned())
                .await?;
        }
        Ok(())
    }
}

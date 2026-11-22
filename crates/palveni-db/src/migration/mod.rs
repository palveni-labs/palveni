//! Programmatic, backend-agnostic migrations.
//!
//! Each store has its own migrator and its own bookkeeping table, because on
//! PostgreSQL/MySQL both stores live in the same database and SeaORM refuses
//! to run when the bookkeeping table lists migrations it does not know.
//!
//! Rules for migrations in this crate:
//! - no raw SQL, no backend-specific column types or index features
//!   (no partial indexes, no FTS, no `IF NOT EXISTS` on indexes because
//!   MySQL does not support it);
//! - index names are globally unique (PostgreSQL scopes them per schema,
//!   and both stores may share one schema).

use sea_orm_migration::prelude::*;

mod m20261120_000001_main_init;
mod m20261120_000001_ops_init;
mod m20261121_000002_track_metadata;
mod m20261121_000003_personal_media;
mod m20261121_000004_video_metadata;
mod m20261121_000005_broadcast;
mod m20261121_000007_media_connections;
mod m20261121_000009_music_search;
mod m20261121_000010_subsonic;
mod m20261121_000011_library_mounts;
mod m20261121_000014_music_listens;
mod m20261121_000015_music_collection_favorites;
mod m20261121_000016_cue_tracks;

mod m20261121_000017_track_loudness;
mod m20261121_000018_scan_reviews;
mod m20261121_000019_link_device_key;
mod m20261122_000020_music_catalog;
mod m20261122_000021_video_library;

pub struct MainMigrator;

#[async_trait::async_trait]
impl MigratorTrait for MainMigrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20261120_000001_main_init::Migration),
            Box::new(m20261121_000002_track_metadata::Migration),
            Box::new(m20261121_000003_personal_media::Migration),
            Box::new(m20261121_000004_video_metadata::Migration),
            Box::new(m20261121_000005_broadcast::Migration),
            Box::new(m20261121_000007_media_connections::Migration),
            Box::new(m20261121_000009_music_search::Migration),
            Box::new(m20261121_000010_subsonic::Migration),
            Box::new(m20261121_000011_library_mounts::Migration),
            Box::new(m20261121_000014_music_listens::Migration),
            Box::new(m20261121_000015_music_collection_favorites::Migration),
            Box::new(m20261121_000016_cue_tracks::Migration),
            Box::new(m20261121_000017_track_loudness::Migration),
            Box::new(m20261121_000018_scan_reviews::Migration),
            Box::new(m20261121_000019_link_device_key::Migration),
            Box::new(m20261122_000020_music_catalog::Migration),
            Box::new(m20261122_000021_video_library::Migration),
        ]
    }

    fn migration_table_name() -> DynIden {
        Alias::new("palveni_main_migrations").into_iden()
    }
}

pub struct OpsMigrator;

#[async_trait::async_trait]
impl MigratorTrait for OpsMigrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![Box::new(m20261120_000001_ops_init::Migration)]
    }

    fn migration_table_name() -> DynIden {
        Alias::new("palveni_ops_migrations").into_iden()
    }
}

// Column helpers shared by the migrations of both stores.

const ID_LEN: u32 = 36;
const SHA256_HEX_LEN: u32 = 64;

/// `VARCHAR(36) NOT NULL PRIMARY KEY`.
fn id_pk<T: IntoIden>(name: T) -> ColumnDef {
    ColumnDef::new(name)
        .string_len(ID_LEN)
        .not_null()
        .primary_key()
        .take()
}

/// `VARCHAR(36) NOT NULL`.
fn id_ref<T: IntoIden>(name: T) -> ColumnDef {
    ColumnDef::new(name).string_len(ID_LEN).not_null().take()
}

/// `VARCHAR(36) NULL`.
fn id_ref_null<T: IntoIden>(name: T) -> ColumnDef {
    ColumnDef::new(name).string_len(ID_LEN).null().take()
}

/// `BIGINT NOT NULL`: timestamps (Unix milliseconds), revisions, sizes.
fn bigint<T: IntoIden>(name: T) -> ColumnDef {
    ColumnDef::new(name).big_integer().not_null().take()
}

/// `BIGINT NULL`.
fn bigint_null<T: IntoIden>(name: T) -> ColumnDef {
    ColumnDef::new(name).big_integer().null().take()
}

/// `INTEGER NOT NULL`.
fn int<T: IntoIden>(name: T) -> ColumnDef {
    ColumnDef::new(name).integer().not_null().take()
}

/// `VARCHAR(len) NOT NULL`; use for anything that is indexed.
fn varchar<T: IntoIden>(name: T, len: u32) -> ColumnDef {
    ColumnDef::new(name).string_len(len).not_null().take()
}

/// `VARCHAR(len) NULL`.
fn varchar_null<T: IntoIden>(name: T, len: u32) -> ColumnDef {
    ColumnDef::new(name).string_len(len).null().take()
}

/// `TEXT NOT NULL`; never indexed.
fn text<T: IntoIden>(name: T) -> ColumnDef {
    ColumnDef::new(name).text().not_null().take()
}

/// `TEXT NULL`.
fn text_null<T: IntoIden>(name: T) -> ColumnDef {
    ColumnDef::new(name).text().null().take()
}

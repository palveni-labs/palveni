//! Ops store, initial schema.

use sea_orm_migration::prelude::*;

use super::{bigint, bigint_null, id_pk, id_ref_null, int, text, text_null, varchar, varchar_null};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum Jobs {
    Table,
    Id,
    Kind,
    Payload,
    RequiredCapabilities,
    State,
    SubjectType,
    SubjectId,
    SubjectName,
    Progress,
    Step,
    Message,
    LastError,
    Attempts,
    DedupeKey,
    ActiveKey,
    CreatedAt,
    StartedAt,
    FinishedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum AuditLog {
    Table,
    Id,
    OccurredAt,
    ActorId,
    Action,
    Target,
    Detail,
}

#[derive(DeriveIden)]
enum PluginKv {
    Table,
    PluginId,
    KvKey,
    KvValue,
    UpdatedAt,
}

// The trait is declared with an elided `SchemaManager` lifetime; naming it
// here would not match the async-trait desugaring.
#[allow(elided_lifetimes_in_paths)]
#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // jobs
        manager
            .create_table(
                Table::create()
                    .table(Jobs::Table)
                    .col(id_pk(Jobs::Id))
                    .col(varchar(Jobs::Kind, 64))
                    .col(text(Jobs::Payload))
                    .col(text(Jobs::RequiredCapabilities))
                    .col(varchar(Jobs::State, 16))
                    .col(varchar_null(Jobs::SubjectType, 32))
                    .col(varchar_null(Jobs::SubjectId, 64))
                    .col(text_null(Jobs::SubjectName))
                    .col(ColumnDef::new(Jobs::Progress).double().null())
                    .col(text_null(Jobs::Step))
                    .col(text_null(Jobs::Message))
                    .col(text_null(Jobs::LastError))
                    .col(int(Jobs::Attempts))
                    .col(varchar_null(Jobs::DedupeKey, 255))
                    .col(varchar_null(Jobs::ActiveKey, 255))
                    .col(bigint(Jobs::CreatedAt))
                    .col(bigint_null(Jobs::StartedAt))
                    .col(bigint_null(Jobs::FinishedAt))
                    .col(bigint(Jobs::UpdatedAt))
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_jobs_state_created")
                    .table(Jobs::Table)
                    .col(Jobs::State)
                    .col(Jobs::CreatedAt)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_jobs_created")
                    .table(Jobs::Table)
                    .col(Jobs::CreatedAt)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_jobs_active_key")
                    .table(Jobs::Table)
                    .col(Jobs::ActiveKey)
                    .unique()
                    .to_owned(),
            )
            .await?;

        // audit_log
        manager
            .create_table(
                Table::create()
                    .table(AuditLog::Table)
                    .col(id_pk(AuditLog::Id))
                    .col(bigint(AuditLog::OccurredAt))
                    .col(id_ref_null(AuditLog::ActorId))
                    .col(varchar(AuditLog::Action, 64))
                    .col(text_null(AuditLog::Target))
                    .col(text_null(AuditLog::Detail))
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_audit_log_occurred")
                    .table(AuditLog::Table)
                    .col(AuditLog::OccurredAt)
                    .to_owned(),
            )
            .await?;

        // plugin_kv
        //
        // `blob` is BLOB on SQLite/MySQL and BYTEA on PostgreSQL. On MySQL a
        // BLOB holds at most 64 KiB per value; the plugin host enforces that
        // limit for every backend so plugins behave identically.
        manager
            .create_table(
                Table::create()
                    .table(PluginKv::Table)
                    .col(varchar(PluginKv::PluginId, 128))
                    .col(varchar(PluginKv::KvKey, 255))
                    .col(ColumnDef::new(PluginKv::KvValue).blob().not_null())
                    .col(bigint(PluginKv::UpdatedAt))
                    .primary_key(
                        Index::create()
                            .name("pk_plugin_kv")
                            .col(PluginKv::PluginId)
                            .col(PluginKv::KvKey),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for table in [
            PluginKv::Table.into_iden(),
            AuditLog::Table.into_iden(),
            Jobs::Table.into_iden(),
        ] {
            manager
                .drop_table(Table::drop().table(table).to_owned())
                .await?;
        }
        Ok(())
    }
}

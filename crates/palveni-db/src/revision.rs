//! Global, monotonically increasing revision for incremental sync.
//!
//! The counter is a single row bumped with `UPDATE … SET revision =
//! revision + 1` inside the writing transaction, then read back. The UPDATE
//! takes the row lock (PostgreSQL/MySQL) or the database write lock
//! (SQLite) and holds it until commit, so revisions are handed out in commit
//! order on every backend.
//!
//! Catalog write transactions call [`next_revision`] as their *first*
//! statement. On SQLite this makes a deferred transaction acquire the write
//! lock immediately (waiting via `busy_timeout`) instead of failing with
//! `SQLITE_BUSY` when it later tries to upgrade a read lock. It also
//! serialises read-check-write sequences such as the duplicate-name check.

use sea_orm::sea_query::Expr;
use sea_orm::{
    ActiveValue, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, TransactionTrait,
    UpdateMany,
};

use crate::entities::sync_state;
use crate::error::{DbError, is_unique_violation};

pub(crate) const SYNC_ROW_ID: i32 = 1;

/// `UPDATE sync_state SET revision = revision + 1 WHERE id = 1`.
fn bump_statement() -> UpdateMany<sync_state::Entity> {
    // Fully qualified so that `ExprTrait` is not in scope here; its blanket
    // methods (`eq`, `gt`, …) would otherwise be ambiguous with
    // `ColumnTrait`'s.
    let bump = sea_orm::sea_query::ExprTrait::add(Expr::col(sync_state::Column::Revision), 1i64);
    sync_state::Entity::update_many()
        .col_expr(sync_state::Column::Revision, bump)
        .filter(sync_state::Column::Id.eq(SYNC_ROW_ID))
}

/// Allocates the next revision. Must run inside a transaction.
pub(crate) async fn next_revision<C: ConnectionTrait>(conn: &C) -> Result<i64, DbError> {
    let result = bump_statement().exec(conn).await?;
    if result.rows_affected != 1 {
        return Err(DbError::Corrupt("sync_state row is missing".to_owned()));
    }
    let row = sync_state::Entity::find_by_id(SYNC_ROW_ID)
        .one(conn)
        .await?
        .ok_or_else(|| DbError::Corrupt("sync_state row is missing".to_owned()))?;
    Ok(row.revision)
}

/// Creates the counter row if it does not exist yet.
pub(crate) async fn ensure_counter<C: ConnectionTrait>(conn: &C) -> Result<(), DbError> {
    if sync_state::Entity::find_by_id(SYNC_ROW_ID)
        .one(conn)
        .await?
        .is_some()
    {
        return Ok(());
    }
    let row = sync_state::ActiveModel {
        id: ActiveValue::Set(SYNC_ROW_ID),
        revision: ActiveValue::Set(0),
    };
    match sync_state::Entity::insert(row)
        .exec_without_returning(conn)
        .await
    {
        Ok(_) => Ok(()),
        // Another process created it in the meantime.
        Err(err) if is_unique_violation(&err) => Ok(()),
        Err(err) => Err(err.into()),
    }
}

/// Current revision without allocating a new one.
pub async fn current_revision<C: ConnectionTrait>(conn: &C) -> Result<i64, DbError> {
    Ok(sync_state::Entity::find_by_id(SYNC_ROW_ID)
        .one(conn)
        .await?
        .map(|row| row.revision)
        .unwrap_or(0))
}

impl crate::Db {
    async fn event_revision(&self) -> Result<i64, DbError> {
        let txn = self.main().begin().await?;
        let revision = next_revision(&txn).await?;
        txn.commit().await?;
        Ok(revision)
    }

    /// Reserve a fresh boundary on every start, including after an unclean shutdown.
    pub async fn event_bus(&self, capacity: usize) -> Result<palveni_core::EventBus, DbError> {
        let initial = self.event_revision().await?;
        let db = self.clone();
        Ok(palveni_core::EventBus::new(capacity, initial, move || {
            let db = db.clone();
            async move { db.event_revision().await }
        }))
    }
}

#[cfg(test)]
mod tests {
    use sea_orm::{DbBackend, QueryTrait};

    use super::*;

    #[cfg(feature = "sqlite")]
    #[tokio::test]
    async fn event_cursor_uses_durable_counter_and_resets_on_restart() {
        use crate::{Db, DbOptions};
        use palveni_core::{LibraryId, ServerEvent};
        let dir = tempfile::tempdir().unwrap();
        let opts = DbOptions {
            main_url: format!("sqlite://{}/main.db?mode=rwc", dir.path().display()),
            ops_url: format!("sqlite://{}/ops.db?mode=rwc", dir.path().display()),
            min_connections: None,
            max_connections: None,
        };
        let db = Db::open(&opts).await.unwrap();
        let bus = db.event_bus(256).await.unwrap();
        let mut subscriber = bus.subscribe();
        let baseline = subscriber.after(None).latest;
        bus.publish(ServerEvent::LibraryDeleted {
            id: LibraryId::generate(),
        });
        assert!(subscriber.changed().await);
        let delivered = subscriber.after(Some(baseline)).entries[0].revision;
        assert_eq!(db.current_revision().await.unwrap(), delivered);
        drop(subscriber);
        drop(bus);
        db.close().await.unwrap();

        let restarted = Db::open(&opts).await.unwrap();
        let fresh = restarted.event_bus(256).await.unwrap();
        let reset = fresh.subscribe().after(Some(delivered));
        assert!(reset.reset);
        assert!(reset.latest > delivered);
    }

    /// The counter must be a plain, portable arithmetic update on every
    /// backend (no RETURNING, no dialect functions).
    #[test]
    fn bump_is_portable() {
        let sqlite = bump_statement().build(DbBackend::Sqlite).to_string();
        let postgres = bump_statement().build(DbBackend::Postgres).to_string();
        let mysql = bump_statement().build(DbBackend::MySql).to_string();
        assert_eq!(
            sqlite,
            r#"UPDATE "sync_state" SET "revision" = "revision" + 1 WHERE "sync_state"."id" = 1"#
        );
        assert_eq!(
            postgres,
            r#"UPDATE "sync_state" SET "revision" = "revision" + 1 WHERE "sync_state"."id" = 1"#
        );
        assert_eq!(
            mysql,
            "UPDATE `sync_state` SET `revision` = `revision` + 1 WHERE `sync_state`.`id` = 1"
        );
    }
}

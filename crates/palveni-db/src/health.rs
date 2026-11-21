//! Read-mostly inspection for `palveni doctor`. Never runs migrations and
//! never creates a SQLite file that does not exist yet.

use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter};
use sea_orm_migration::MigratorTrait;

use crate::entities::{items, libraries, library_paths, users};
use crate::migration::{MainMigrator, OpsMigrator};
use crate::{Backend, DbOptions, connect, redact_url, sqlite_file_path};

#[derive(Clone, Debug, Default)]
pub struct StoreHealth {
    /// URL with any password removed.
    pub url: String,
    pub backend: Option<Backend>,
    pub reachable: bool,
    pub pending_migrations: Option<usize>,
    pub error: Option<String>,
}

#[derive(Clone, Debug)]
pub struct LibraryLocations {
    pub name: String,
    pub paths: Vec<String>,
}

#[derive(Clone, Debug, Default)]
pub struct DbHealth {
    pub main: StoreHealth,
    pub ops: StoreHealth,
    pub users: Option<u64>,
    pub live_items: Option<u64>,
    pub libraries: Vec<LibraryLocations>,
}

#[derive(Clone, Copy)]
enum Store {
    Main,
    Ops,
}

pub async fn inspect(options: &DbOptions) -> DbHealth {
    let (main, main_conn) = open_store(&options.main_url, options, Store::Main).await;
    let (ops, _ops_conn) = open_store(&options.ops_url, options, Store::Ops).await;
    let mut health = DbHealth {
        main,
        ops,
        ..DbHealth::default()
    };
    if let Some(conn) = main_conn {
        // Counts fail harmlessly (and are left as None) before the first
        // migration has created the tables.
        health.users = users::Entity::find().count(&conn).await.ok();
        health.live_items = items::Entity::find()
            .filter(items::Column::DeletedAt.is_null())
            .count(&conn)
            .await
            .ok();
        if let Ok(rows) = libraries::Entity::find()
            .filter(libraries::Column::DeletedAt.is_null())
            .all(&conn)
            .await
        {
            for row in rows {
                let paths = library_paths::Entity::find()
                    .filter(library_paths::Column::LibraryId.eq(row.id.clone()))
                    .all(&conn)
                    .await
                    .map(|ps| ps.into_iter().map(|p| p.path).collect())
                    .unwrap_or_default();
                health.libraries.push(LibraryLocations {
                    name: row.name,
                    paths,
                });
            }
        }
    }
    health
}

async fn open_store(
    url: &str,
    options: &DbOptions,
    store: Store,
) -> (StoreHealth, Option<DatabaseConnection>) {
    let mut health = StoreHealth {
        url: redact_url(url),
        backend: Backend::from_url(url),
        ..StoreHealth::default()
    };
    if health.backend == Some(Backend::Sqlite)
        && let Some(path) = sqlite_file_path(url)
        && !path.exists()
    {
        health.error = Some(format!(
            "{} does not exist yet (created on first `palveni serve`)",
            path.display()
        ));
        return (health, None);
    }
    // A doctor run needs one connection, not a pool.
    let single = DbOptions {
        max_connections: Some(1),
        min_connections: Some(1),
        ..options.clone()
    };
    let conn = match connect(url, &single).await {
        Ok(conn) => conn,
        Err(err) => {
            health.error = Some(err.to_string());
            return (health, None);
        }
    };
    health.reachable = true;
    let pending = match store {
        Store::Main => MainMigrator::get_pending_migrations(&conn).await,
        Store::Ops => OpsMigrator::get_pending_migrations(&conn).await,
    };
    match pending {
        Ok(list) => health.pending_migrations = Some(list.len()),
        Err(err) => health.error = Some(format!("cannot read migration state: {err}")),
    }
    (health, Some(conn))
}

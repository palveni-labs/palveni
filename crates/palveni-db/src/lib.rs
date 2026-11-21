//! Persistence for Palveni Server.
//!
//! Backends: SQLite (default, zero configuration), PostgreSQL and
//! MySQL/MariaDB, selected by URL and compiled in via the `sqlite`,
//! `postgres` and `mysql` features. All SQL is built by SeaORM /
//! sea-query; repositories contain no dialect-specific SQL.
//!
//! Two logical stores, each with its own connection pool:
//! - **main**: users, sessions/devices, libraries, items, settings and the
//!   sync revision counter — everything that is joined with the catalog;
//! - **ops**: jobs, audit log, plugin key/value — high-churn data.
//!
//! With SQLite they are two files so scan writes and job/log churn never
//! wait on the same write lock; with PostgreSQL/MySQL both live in one
//! database. Nothing joins or spans a transaction across the two stores.
//!
//! Business code never writes SQL; it goes through the repositories exposed
//! by [`Db`].

#[cfg(test)]
mod conformance;
mod convert;
mod entities;
mod error;
mod migration;
mod revision;

pub mod audit;
pub mod broadcast;
pub mod connections;
pub mod health;
pub mod items;
pub mod jobs;
pub mod libraries;
pub mod music;
pub mod personal;
pub mod playlists;
pub mod plugin_kv;
pub mod scan_reviews;
pub mod sessions;
pub mod settings;
pub mod users;
pub mod video;

use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use sea_orm_migration::MigratorTrait;

pub use error::DbError;
pub use migration::{MainMigrator, OpsMigrator};

/// Pool sizing when the configuration does not say otherwise.
const DEFAULT_MAX_CONNECTIONS_SQLITE: u32 = 8;
const DEFAULT_MAX_CONNECTIONS_NETWORK: u32 = 10;
const ACQUIRE_TIMEOUT: Duration = Duration::from_secs(30);
/// SQLite waits this long for a competing writer before reporting "busy".
#[cfg(feature = "sqlite")]
const SQLITE_BUSY_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Backend {
    Sqlite,
    Postgres,
    MySql,
}

impl Backend {
    pub fn from_url(url: &str) -> Option<Backend> {
        let scheme = url.split_once(':')?.0.to_ascii_lowercase();
        match scheme.as_str() {
            "sqlite" => Some(Backend::Sqlite),
            "postgres" | "postgresql" => Some(Backend::Postgres),
            "mysql" | "mariadb" => Some(Backend::MySql),
            _ => None,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Backend::Sqlite => "sqlite",
            Backend::Postgres => "postgres",
            Backend::MySql => "mysql",
        }
    }

    /// Whether support for this backend was compiled into this build.
    pub const fn is_compiled_in(self) -> bool {
        match self {
            Backend::Sqlite => cfg!(feature = "sqlite"),
            Backend::Postgres => cfg!(feature = "postgres"),
            Backend::MySql => cfg!(feature = "mysql"),
        }
    }
}

impl fmt::Display for Backend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Connection settings for both stores.
#[derive(Clone, Debug)]
pub struct DbOptions {
    pub main_url: String,
    pub ops_url: String,
    pub max_connections: Option<u32>,
    pub min_connections: Option<u32>,
}

impl DbOptions {
    pub fn from_config(config: &palveni_core::config::DatabaseConfig) -> Self {
        DbOptions {
            main_url: config.url.clone(),
            ops_url: config.ops_url(),
            max_connections: config.max_connections,
            min_connections: config.min_connections,
        }
    }
}

/// Handle to both stores. Cheap to clone.
#[derive(Clone)]
pub struct Db {
    main: Arc<DatabaseConnection>,
    ops: Arc<DatabaseConnection>,
    backend: Backend,
}

impl Db {
    /// Connects to both stores and applies pending migrations.
    pub async fn open(options: &DbOptions) -> Result<Db, DbError> {
        let backend = backend_for(&options.main_url)?;
        backend_for(&options.ops_url)?;

        let main = connect(&options.main_url, options).await?;
        MainMigrator::up(&main, None).await?;
        revision::ensure_counter(&main).await?;

        let ops = connect(&options.ops_url, options).await?;
        OpsMigrator::up(&ops, None).await?;

        Ok(Db {
            main: Arc::new(main),
            ops: Arc::new(ops),
            backend,
        })
    }

    pub fn backend(&self) -> Backend {
        self.backend
    }

    pub(crate) fn main(&self) -> &DatabaseConnection {
        &self.main
    }

    pub(crate) fn ops(&self) -> &DatabaseConnection {
        &self.ops
    }

    /// Closes both pools. With SQLite the last connection to close
    /// checkpoints the WAL, so the database is a single file again.
    pub async fn close(&self) -> Result<(), DbError> {
        let ops = self.ops.close_by_ref().await;
        let main = self.main.close_by_ref().await;
        ops?;
        main?;
        Ok(())
    }

    pub async fn current_revision(&self) -> Result<i64, DbError> {
        revision::current_revision(self.main()).await
    }

    pub fn personal(&self) -> personal::PersonalMedia<'_> {
        personal::PersonalMedia { db: self }
    }

    pub fn playlists(&self) -> playlists::Playlists<'_> {
        playlists::Playlists { db: self }
    }

    pub fn users(&self) -> users::Users<'_> {
        users::Users { db: self }
    }

    pub fn sessions(&self) -> sessions::Sessions<'_> {
        sessions::Sessions { db: self }
    }

    pub fn libraries(&self) -> libraries::Libraries<'_> {
        libraries::Libraries { db: self }
    }

    pub fn music(&self) -> music::Music<'_> {
        music::Music { db: self }
    }

    pub fn video(&self) -> video::Videos<'_> {
        video::Videos { db: self }
    }

    pub fn scan_reviews(&self) -> scan_reviews::ScanReviews<'_> {
        scan_reviews::ScanReviews { db: self }
    }

    pub fn items(&self) -> items::Items<'_> {
        items::Items { db: self }
    }

    pub fn settings(&self) -> settings::Settings<'_> {
        settings::Settings { db: self }
    }

    pub fn jobs(&self) -> jobs::Jobs<'_> {
        jobs::Jobs { db: self }
    }

    pub fn audit(&self) -> audit::Audit<'_> {
        audit::Audit { db: self }
    }

    pub fn plugin_kv(&self) -> plugin_kv::PluginKv<'_> {
        plugin_kv::PluginKv { db: self }
    }
}

impl fmt::Debug for Db {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Db")
            .field("backend", &self.backend)
            .finish()
    }
}

fn backend_for(url: &str) -> Result<Backend, DbError> {
    let backend = Backend::from_url(url).ok_or_else(|| {
        DbError::Config(format!(
            "unsupported database URL `{}` (expected sqlite://, postgres:// or mysql://)",
            redact_url(url)
        ))
    })?;
    if !backend.is_compiled_in() {
        return Err(DbError::Config(format!(
            "this build does not include the {backend} backend"
        )));
    }
    Ok(backend)
}

pub(crate) async fn connect(url: &str, options: &DbOptions) -> Result<DatabaseConnection, DbError> {
    let backend = backend_for(url)?;
    let max = options.max_connections.unwrap_or(match backend {
        Backend::Sqlite => DEFAULT_MAX_CONNECTIONS_SQLITE,
        Backend::Postgres | Backend::MySql => DEFAULT_MAX_CONNECTIONS_NETWORK,
    });
    let max = max.max(1);
    let min = options.min_connections.unwrap_or(1).min(max);

    let mut connect_options = ConnectOptions::new(driver_url(url));
    connect_options
        .max_connections(max)
        .min_connections(min)
        .acquire_timeout(ACQUIRE_TIMEOUT)
        // Per-statement logging would flood the log during scans.
        .sqlx_logging(false);
    if backend == Backend::Sqlite {
        prepare_sqlite(url, &mut connect_options)?;
    }
    tracing::debug!(backend = %backend, url = %redact_url(url), max, min, "connecting to database");
    Ok(Database::connect(connect_options).await?)
}

/// Applies the per-connection SQLite tuning and makes sure the parent
/// directory of the database file exists.
#[cfg(feature = "sqlite")]
fn prepare_sqlite(url: &str, connect_options: &mut ConnectOptions) -> Result<(), DbError> {
    if let Some(path) = sqlite_file_path(url)
        && let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent)
            .map_err(|e| DbError::Config(format!("cannot create {}: {e}", parent.display())))?;
    }
    connect_options.map_sqlx_sqlite_opts(tune_sqlite);
    Ok(())
}

#[cfg(not(feature = "sqlite"))]
fn prepare_sqlite(_url: &str, _connect_options: &mut ConnectOptions) -> Result<(), DbError> {
    Err(DbError::Config(
        "this build does not include the sqlite backend".to_owned(),
    ))
}

/// Applied by sqlx to every connection the pool opens.
///
/// WAL lets any number of readers run concurrently with one writer;
/// `synchronous=NORMAL` is durable across application crashes and only
/// risks the last transactions on power loss, which is the usual trade-off
/// for WAL; the busy timeout makes writers queue instead of failing.
#[cfg(feature = "sqlite")]
fn tune_sqlite(options: sqlx::sqlite::SqliteConnectOptions) -> sqlx::sqlite::SqliteConnectOptions {
    options
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
        .synchronous(sqlx::sqlite::SqliteSynchronous::Normal)
        .foreign_keys(true)
        .busy_timeout(SQLITE_BUSY_TIMEOUT)
}

/// Extracts the file path from a `sqlite:` URL; `None` for in-memory URLs.
pub(crate) fn sqlite_file_path(url: &str) -> Option<PathBuf> {
    let rest = url
        .strip_prefix("sqlite://")
        .or_else(|| url.strip_prefix("sqlite:"))?;
    let path = rest.split_once('?').map_or(rest, |(p, _)| p);
    if path.is_empty() || path.contains(":memory:") {
        return None;
    }
    let decoded = path
        .replace("%3F", "?")
        .replace("%3f", "?")
        .replace("%23", "#")
        .replace("%25", "%");
    Some(PathBuf::from(decoded))
}

/// The driver only understands `mysql://`; `mariadb://` is accepted in
/// configuration because people write it, and rewritten here.
fn driver_url(url: &str) -> String {
    match url.split_once("://") {
        Some((scheme, rest)) if scheme.eq_ignore_ascii_case("mariadb") => format!("mysql://{rest}"),
        _ => url.to_owned(),
    }
}

/// Hides the password in `scheme://user:password@host/...` URLs.
pub fn redact_url(url: &str) -> String {
    let Some((scheme, rest)) = url.split_once("://") else {
        return url.to_owned();
    };
    let Some((credentials, host)) = rest.split_once('@') else {
        return url.to_owned();
    };
    match credentials.split_once(':') {
        Some((user, _password)) => format!("{scheme}://{user}:***@{host}"),
        None => url.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backend_detection() {
        assert_eq!(
            Backend::from_url("sqlite://data/palveni.db?mode=rwc"),
            Some(Backend::Sqlite)
        );
        assert_eq!(
            Backend::from_url("postgresql://h/db"),
            Some(Backend::Postgres)
        );
        assert_eq!(Backend::from_url("MySQL://h/db"), Some(Backend::MySql));
        assert_eq!(Backend::from_url("mariadb://h/db"), Some(Backend::MySql));
        assert_eq!(Backend::from_url("redis://h"), None);
        assert_eq!(Backend::from_url("no-scheme"), None);
    }

    #[test]
    fn sqlite_paths() {
        assert_eq!(
            sqlite_file_path("sqlite://data/palveni.db?mode=rwc"),
            Some(PathBuf::from("data/palveni.db"))
        );
        assert_eq!(
            sqlite_file_path("sqlite:///srv/a%3Fb/palveni.db"),
            Some(PathBuf::from("/srv/a?b/palveni.db"))
        );
        assert_eq!(sqlite_file_path("sqlite::memory:"), None);
        assert_eq!(sqlite_file_path("postgres://h/db"), None);
    }

    #[test]
    fn mariadb_urls_use_the_mysql_driver() {
        assert_eq!(driver_url("mariadb://u:p@h/db"), "mysql://u:p@h/db");
        assert_eq!(driver_url("MariaDB://h/db"), "mysql://h/db");
        assert_eq!(driver_url("mysql://h/db"), "mysql://h/db");
        assert_eq!(
            driver_url("sqlite://data/x.db?mode=rwc"),
            "sqlite://data/x.db?mode=rwc"
        );
    }

    #[test]
    fn redaction() {
        assert_eq!(
            redact_url("postgres://palveni:hunter2@db:5432/palveni"),
            "postgres://palveni:***@db:5432/palveni"
        );
        assert_eq!(
            redact_url("sqlite://data/palveni.db"),
            "sqlite://data/palveni.db"
        );
        assert_eq!(redact_url("mysql://root@db/x"), "mysql://root@db/x");
    }
}

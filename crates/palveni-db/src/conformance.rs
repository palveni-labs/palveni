//! Data-layer conformance suite: the same tests for every backend.
//!
//! By default every test runs against SQLite in its own temporary directory.
//! To run against PostgreSQL or MySQL/MariaDB, build with the matching
//! feature and point the suite at an **empty database created for the
//! purpose**:
//!
//! ```text
//! PALVENI_TEST_DATABASE_URL=postgres://user:pass@host/palveni_test \
//!     cargo test -p palveni-db --features postgres conformance
//! ```
//!
//! `PALVENI_TEST_OPS_DATABASE_URL` optionally points the ops store at a
//! different database; by default both stores share the one database (as
//! in production for network backends).
//!
//! What the suite does to that database:
//! - It requires empty stores before each test, including when all existing
//!   tables happen to belong to Palveni.
//! - After each passing test it drops all of Palveni's tables again, so a
//!   successful run leaves the database empty. A failing test leaves its
//!   tables in place for inspection. Use a fresh test database to retry.
//! - It never creates or drops databases, schemas, roles or users, and
//!   tests run one at a time because they share the database.

use std::collections::{BTreeSet, HashSet};

use palveni_core::{ItemId, ItemKind, JobState, JobStep, JobSubject, LibraryKind, Role, clock};
use sea_orm::sea_query::{Alias, Table};
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement};
use sea_orm_migration::MigratorTrait;
use tokio::sync::{Mutex, MutexGuard};

use crate::items::{ItemCursor, ItemQuery, ItemUpsert};
use crate::jobs::NewJobRecord;
use crate::libraries::NewLibrary;
use crate::migration::{MainMigrator, OpsMigrator};
use crate::{Db, DbError, DbOptions, connect, redact_url};

pub const ENV_DATABASE_URL: &str = "PALVENI_TEST_DATABASE_URL";
pub const ENV_OPS_DATABASE_URL: &str = "PALVENI_TEST_OPS_DATABASE_URL";

/// Every table the migrations create, children before parents so they can
/// be dropped in this order on backends that enforce foreign keys.
const PALVENI_TABLES: [&str; 14] = [
    // ops store
    "plugin_kv",
    "audit_log",
    "jobs",
    "palveni_ops_migrations",
    // main store
    "sync_state",
    "settings",
    "track_metadata",
    "items",
    "library_paths",
    "libraries",
    "sessions",
    "devices",
    "users",
    "palveni_main_migrations",
];

static LOCK: Mutex<()> = Mutex::const_new(());

struct Harness {
    db: Db,
    options: DbOptions,
    external: bool,
    _dir: Option<tempfile::TempDir>,
    _guard: MutexGuard<'static, ()>,
}

fn options(main_url: String, ops_url: String) -> DbOptions {
    DbOptions {
        main_url,
        ops_url,
        max_connections: Some(8),
        min_connections: Some(1),
    }
}

fn external_options() -> Option<DbOptions> {
    let main = std::env::var(ENV_DATABASE_URL)
        .ok()
        .filter(|v| !v.trim().is_empty())?;
    let ops = std::env::var(ENV_OPS_DATABASE_URL)
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| palveni_core::config::derive_ops_url(&main));
    Some(options(main, ops))
}

impl Harness {
    async fn new() -> Harness {
        let guard = LOCK.lock().await;
        let (options, dir, external) = match external_options() {
            Some(options) => (options, None, true),
            None => {
                let dir = tempfile::tempdir().expect("temporary directory");
                let base = dir.path().display().to_string();
                let options = options(
                    format!("sqlite://{base}/main.db?mode=rwc"),
                    format!("sqlite://{base}/ops.db?mode=rwc"),
                );
                (options, Some(dir), false)
            }
        };
        if external {
            check_external(&options).await.expect("empty test stores");
        }
        let db = Db::open(&options)
            .await
            .expect("migrations must apply to an empty database");
        Harness {
            db,
            options,
            external,
            _dir: dir,
            _guard: guard,
        }
    }

    async fn finish(self) {
        if self.external {
            drop_palveni_tables(self.db.ops()).await;
            drop_palveni_tables(self.db.main()).await;
        }
        self.db.close().await.expect("close pools");
    }
}

fn distinct_urls(options: &DbOptions) -> Vec<String> {
    let mut urls = vec![options.main_url.clone()];
    if options.ops_url != options.main_url {
        urls.push(options.ops_url.clone());
    }
    urls
}

async fn check_external(options: &DbOptions) -> Result<(), String> {
    for url in distinct_urls(options) {
        let single = DbOptions {
            max_connections: Some(1),
            min_connections: Some(1),
            ..options.clone()
        };
        let conn = connect(&url, &single)
            .await
            .expect("connect to test database");
        let tables = list_tables(&conn).await;
        conn.close_by_ref().await.expect("close");
        if !tables.is_empty() {
            return Err(format!(
                "refusing to run: {} already contains tables {tables:?}. \
                 Use an empty database created for the tests.",
                redact_url(&url)
            ));
        }
    }
    Ok(())
}

/// The only backend-specific SQL in the crate: catalogue queries, used by
/// the test safety check.
async fn list_tables(conn: &DatabaseConnection) -> BTreeSet<String> {
    let backend = conn.get_database_backend();
    let sql = match backend {
        DbBackend::Sqlite => {
            "SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%'"
        }
        DbBackend::Postgres => {
            "SELECT tablename::text FROM pg_catalog.pg_tables WHERE schemaname = current_schema()"
        }
        DbBackend::MySql => {
            "SELECT CAST(table_name AS CHAR) FROM information_schema.tables \
             WHERE table_schema = DATABASE()"
        }
        _ => "",
    };
    assert!(!sql.is_empty(), "unsupported backend {backend:?}");
    let rows = conn
        .query_all_raw(Statement::from_string(backend, sql))
        .await
        .expect("list tables");
    rows.iter()
        .map(|row| row.try_get_by_index::<String>(0).expect("table name"))
        .collect()
}

async fn drop_palveni_tables(conn: &DatabaseConnection) {
    for table in PALVENI_TABLES {
        let stmt = Table::drop()
            .table(Alias::new(table))
            .if_exists()
            .to_owned();
        conn.execute(&stmt).await.expect("drop table");
    }
}

fn track(name: &str, path: &str, parent: Option<ItemId>, size: i64) -> ItemUpsert {
    ItemUpsert {
        music: None,
        id: ItemId::generate(),
        parent_id: parent,
        kind: ItemKind::Track,
        title: name.to_owned(),
        sort_title: name.to_lowercase(),
        search_text: name.to_lowercase(),
        path: path.to_owned(),
        size_bytes: Some(size),
        modified_at: Some(1_700_000_000_000),
    }
}

fn folder(name: &str, path: &str, parent: Option<ItemId>) -> ItemUpsert {
    ItemUpsert {
        music: None,
        id: ItemId::generate(),
        parent_id: parent,
        kind: ItemKind::Folder,
        title: name.to_owned(),
        sort_title: name.to_lowercase(),
        search_text: name.to_lowercase(),
        path: path.to_owned(),
        size_bytes: None,
        modified_at: None,
    }
}

async fn new_library(db: &Db, name: &str) -> palveni_core::Library {
    db.libraries()
        .create(NewLibrary {
            name: name.to_owned(),
            kind: LibraryKind::Music,
            paths: vec![format!("/media/{name}")],
        })
        .await
        .expect("create library")
}

/// Pages through a listing and returns titles in order.
async fn all_titles(db: &Db, mut query: ItemQuery) -> Vec<String> {
    let mut titles = Vec::new();
    loop {
        let page = db.items().list(&query).await.expect("list items");
        titles.extend(page.items.into_iter().map(|i| i.title));
        match page.next_cursor {
            Some(cursor) => {
                // Round-trip through the opaque client form.
                query.cursor = ItemCursor::decode(&cursor.encode());
                assert!(query.cursor.is_some());
            }
            None => return titles,
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn conformance_migrations_apply_from_empty_and_reopen() {
    let h = Harness::new().await;
    for conn in [h.db.main(), h.db.ops()] {
        let tables = list_tables(conn).await;
        assert!(!tables.is_empty());
    }
    let mut all: BTreeSet<String> = list_tables(h.db.main()).await;
    all.extend(list_tables(h.db.ops()).await);
    let expected: BTreeSet<String> = PALVENI_TABLES.iter().map(|t| (*t).to_owned()).collect();
    assert_eq!(all, expected);
    assert!(
        MainMigrator::get_pending_migrations(h.db.main())
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        OpsMigrator::get_pending_migrations(h.db.ops())
            .await
            .unwrap()
            .is_empty()
    );

    // Re-opening applies nothing and keeps data.
    h.db.settings().set("probe", "kept").await.unwrap();
    assert!(check_external(&h.options).await.is_err());
    let again = Db::open(&h.options).await.expect("reopen");
    assert_eq!(
        again.settings().get("probe").await.unwrap().as_deref(),
        Some("kept")
    );
    again.close().await.unwrap();

    let health = crate::health::inspect(&h.options).await;
    assert!(health.main.reachable && health.ops.reachable, "{health:?}");
    assert_eq!(health.main.pending_migrations, Some(0));
    assert_eq!(health.ops.pending_migrations, Some(0));
    assert_eq!(health.users, Some(0));
    h.finish().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn conformance_users() {
    let h = Harness::new().await;
    let users = h.db.users();
    assert_eq!(users.count().await.unwrap(), 0);
    let admin = users
        .create_first_admin("Admin", "hash-a")
        .await
        .unwrap()
        .expect("first admin");
    assert_eq!(admin.role, Role::Admin);
    assert!(
        users
            .create_first_admin("second", "x")
            .await
            .unwrap()
            .is_none()
    );

    let bob = users.create("bob", "hash-b", Role::User).await.unwrap();
    assert!(matches!(
        users.create("BOB", "x", Role::User).await,
        Err(DbError::Conflict(_))
    ));
    let creds = users
        .find_credentials("ADMIN")
        .await
        .unwrap()
        .expect("case-insensitive");
    assert_eq!(creds.user.id, admin.id);
    assert_eq!(creds.password_hash, "hash-a");
    assert!(!creds.disabled);
    assert!(users.find_credentials("nobody").await.unwrap().is_none());
    assert_eq!(
        users.get(bob.id).await.unwrap().map(|u| u.username),
        Some("bob".into())
    );
    assert_eq!(users.count().await.unwrap(), 2);
    h.finish().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn conformance_sessions_and_devices() {
    let h = Harness::new().await;
    let user = h.db.users().create("u", "h", Role::User).await.unwrap();
    let sessions = h.db.sessions();

    let issued = sessions.create(user.id, "Phone", None).await.unwrap();
    let auth = sessions
        .authenticate(&issued.token)
        .await
        .unwrap()
        .expect("valid token");
    assert_eq!(auth.user.id, user.id);
    assert_eq!(auth.device_id, Some(issued.device_id));
    assert!(
        sessions
            .authenticate("pvt_not-a-token")
            .await
            .unwrap()
            .is_none()
    );
    sessions
        .touch(issued.session_id, Some(issued.device_id))
        .await
        .unwrap();
    sessions.revoke(issued.session_id).await.unwrap();
    sessions.revoke(issued.session_id).await.unwrap(); // idempotent
    assert!(
        sessions
            .authenticate(&issued.token)
            .await
            .unwrap()
            .is_none()
    );

    let other = sessions.create(user.id, "Laptop", None).await.unwrap();
    sessions.revoke_device(other.device_id).await.unwrap();
    assert!(sessions.authenticate(&other.token).await.unwrap().is_none());

    let past = clock::now() - chrono::Duration::hours(1);
    let expired = sessions.create(user.id, "Old", Some(past)).await.unwrap();
    assert!(
        sessions
            .authenticate(&expired.token)
            .await
            .unwrap()
            .is_none()
    );
    h.finish().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn conformance_libraries_and_revisions() {
    let h = Harness::new().await;
    let libraries = h.db.libraries();
    let start = h.db.current_revision().await.unwrap();

    let movies = libraries
        .create(NewLibrary {
            name: "Movies".into(),
            kind: LibraryKind::Video,
            paths: vec!["/media/b".into(), "/media/a".into()],
        })
        .await
        .unwrap();
    assert_eq!(
        movies.paths,
        vec!["/media/b", "/media/a"],
        "path order is kept"
    );
    assert_eq!(h.db.current_revision().await.unwrap(), start + 1);
    assert!(matches!(
        libraries
            .create(NewLibrary {
                name: "movies".into(),
                kind: LibraryKind::Video,
                paths: vec!["/x".into()],
            })
            .await,
        Err(DbError::Conflict(_))
    ));
    let music = new_library(&h.db, "Music").await;

    let names: Vec<String> = libraries
        .list()
        .await
        .unwrap()
        .into_iter()
        .map(|l| l.name)
        .collect();
    assert_eq!(names, vec!["Movies", "Music"]);
    assert_eq!(
        libraries.get(music.id).await.unwrap().map(|l| l.kind),
        Some(LibraryKind::Music)
    );

    let before = h.db.current_revision().await.unwrap();
    assert!(
        libraries
            .mark_scanned(music.id, clock::now())
            .await
            .unwrap()
    );
    assert!(h.db.current_revision().await.unwrap() > before);
    assert!(
        libraries
            .get(music.id)
            .await
            .unwrap()
            .unwrap()
            .last_scan_at
            .is_some()
    );

    assert!(libraries.delete(movies.id).await.unwrap());
    assert!(!libraries.delete(movies.id).await.unwrap());
    assert!(libraries.get(movies.id).await.unwrap().is_none());
    assert!(
        !libraries
            .mark_scanned(movies.id, clock::now())
            .await
            .unwrap()
    );
    // A deleted library's name can be reused.
    new_library(&h.db, "Movies").await;
    assert_eq!(libraries.list().await.unwrap().len(), 2);
    h.finish().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn conformance_items_upsert_paging_and_tombstones() {
    let h = Harness::new().await;
    let library = new_library(&h.db, "Music").await;
    let items = h.db.items();

    // One batch of the scanner's maximum size (500 rows) exercises the
    // bind-parameter limits of every backend.
    let album = folder("Album", "/media/Music/Album", None);
    let singles = folder("Singles", "/media/Music/Singles", None);
    let mut batch = vec![album.clone(), singles.clone()];
    for i in 0..497 {
        batch.push(track(
            &format!("Track {i:03}"),
            &format!("/media/Music/Album/{i:03}.flac"),
            Some(album.id),
            100 + i64::from(i),
        ));
    }
    batch.push(track("Loose", "/media/Music/loose.mp3", None, 5));
    assert_eq!(batch.len(), 500);
    let rev_before = h.db.current_revision().await.unwrap();
    items
        .apply_scan_batch(library.id, &batch, &[])
        .await
        .unwrap();
    assert_eq!(h.db.current_revision().await.unwrap(), rev_before + 1);

    let query = |parent: Option<ItemId>, kind: Option<ItemKind>, limit: u32| ItemQuery {
        library_id: library.id,
        parent_id: parent,
        kind,
        limit,
        cursor: None,
    };
    // Folders first, then titles (ASCII, so the order is the same on
    // every backend's default collation).
    assert_eq!(
        all_titles(&h.db, query(None, None, 2)).await,
        vec!["Album", "Singles", "Loose"]
    );
    assert_eq!(
        all_titles(&h.db, query(None, Some(ItemKind::Track), 50)).await,
        vec!["Loose"]
    );
    let album_titles = all_titles(&h.db, query(Some(album.id), None, 37)).await;
    let expected: Vec<String> = (0..497).map(|i| format!("Track {i:03}")).collect();
    assert_eq!(album_titles, expected);

    let lib = h.db.libraries().get(library.id).await.unwrap().unwrap();
    assert_eq!(lib.item_count, 498, "folders are not counted");

    // Changing a file keeps its id and creation time.
    let original = items.get(batch[5].id).await.unwrap().unwrap();
    let mut changed = batch[5].clone();
    changed.id = ItemId::generate(); // the path, not the id, is the key
    changed.size_bytes = Some(999_999);
    items
        .apply_scan_batch(library.id, &[changed], &[])
        .await
        .unwrap();
    let after = items.get(batch[5].id).await.unwrap().unwrap();
    assert_eq!(after.size_bytes, Some(999_999));
    assert_eq!(after.created_at, original.created_at);

    // Tombstones hide items but keep them in the scan index.
    let gone: Vec<ItemId> = batch[10..20].iter().map(|u| u.id).collect();
    items
        .apply_scan_batch(library.id, &[], &gone)
        .await
        .unwrap();
    assert!(items.get(gone[0]).await.unwrap().is_none());
    let lib = h.db.libraries().get(library.id).await.unwrap().unwrap();
    assert_eq!(lib.item_count, 488);
    let index = items.scan_index(library.id).await.unwrap();
    assert_eq!(index.len(), 500);
    assert_eq!(index.iter().filter(|e| e.deleted).count(), 10);

    // A reappearing file is revived under its old id.
    items
        .apply_scan_batch(library.id, &batch[10..11], &[])
        .await
        .unwrap();
    assert!(items.get(gone[0]).await.unwrap().is_some());

    // Non-ASCII titles: the order is backend-defined, but paging must
    // return every item exactly once.
    let singles_batch: Vec<ItemUpsert> = ["张学友", "Ångström", "ábc", "Zebra", "äpfel"]
        .iter()
        .enumerate()
        .map(|(i, t)| {
            track(
                t,
                &format!("/media/Music/Singles/{i}.mp3"),
                Some(singles.id),
                1,
            )
        })
        .collect();
    items
        .apply_scan_batch(library.id, &singles_batch, &[])
        .await
        .unwrap();
    let seen = all_titles(&h.db, query(Some(singles.id), None, 1)).await;
    assert_eq!(seen.len(), 5);
    assert_eq!(seen.iter().collect::<HashSet<_>>().len(), 5);

    // Writes to a deleted library are refused.
    assert!(h.db.libraries().delete(library.id).await.unwrap());
    assert!(matches!(
        items.apply_scan_batch(library.id, &batch[..1], &[]).await,
        Err(DbError::NotFound("library"))
    ));
    assert!(items.scan_index(library.id).await.unwrap().is_empty());
    h.finish().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn conformance_settings() {
    let h = Harness::new().await;
    let settings = h.db.settings();
    assert!(settings.get("k").await.unwrap().is_none());
    settings.set("k", "one").await.unwrap();
    settings.set("k", "two").await.unwrap();
    assert_eq!(settings.get("k").await.unwrap().as_deref(), Some("two"));
    let id = settings.server_id().await.unwrap();
    assert_eq!(settings.server_id().await.unwrap(), id);
    h.finish().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn conformance_jobs() {
    let h = Harness::new().await;
    let jobs = h.db.jobs();
    let library = palveni_core::LibraryId::generate();
    let new = |key: Option<&str>| NewJobRecord {
        kind: "library.scan".into(),
        subject: Some(JobSubject::library(library, "Music")),
        payload: serde_json::json!({ "libraryId": library }),
        required_capabilities: vec!["local".into()],
        dedupe_key: key.map(str::to_owned),
    };

    let first = jobs.enqueue(new(Some("scan:1"))).await.unwrap();
    assert!(first.created);
    let job_id = first.record.job.id;
    let dup = jobs.enqueue(new(Some("scan:1"))).await.unwrap();
    assert!(!dup.created);
    assert_eq!(dup.record.job.id, job_id);
    assert_eq!(dup.record.job.subject, first.record.job.subject);

    let queued = jobs.list_queued(10).await.unwrap();
    assert_eq!(queued.len(), 1);
    assert_eq!(queued[0].required_capabilities, vec!["local"]);
    assert!(jobs.try_claim(job_id, clock::now()).await.unwrap());
    assert!(!jobs.try_claim(job_id, clock::now()).await.unwrap());

    // Interrupted jobs go back to the queue without their step.
    assert_eq!(jobs.requeue_interrupted().await.unwrap(), 1);
    let record = jobs.get(job_id).await.unwrap().unwrap();
    assert_eq!(record.job.state, JobState::Queued);
    assert!(record.job.started_at.is_none());
    assert!(jobs.try_claim(job_id, clock::now()).await.unwrap());
    assert_eq!(jobs.get(job_id).await.unwrap().unwrap().attempts, 2);

    // Finishing persists the snapshot and releases the dedupe key.
    let mut done = record.job.clone();
    done.state = JobState::Succeeded;
    done.progress = Some(1.0);
    done.step = Some(
        JobStep::new("scan.summary")
            .with("added", 3u64)
            .with("updated", 0u64),
    );
    done.message = Some("3 added".into());
    done.finished_at = Some(clock::now());
    jobs.finish(&done).await.unwrap();
    let stored = jobs.get(job_id).await.unwrap().unwrap().job;
    assert_eq!(stored.state, JobState::Succeeded);
    assert_eq!(stored.step, done.step);
    assert_eq!(stored.message, done.message);
    assert_eq!(
        stored.subject.as_ref().and_then(|s| s.name.as_deref()),
        Some("Music")
    );
    assert!(jobs.enqueue(new(Some("scan:1"))).await.unwrap().created);

    // A failed job keeps only its error.
    let failing = jobs.enqueue(new(None)).await.unwrap().record.job;
    let mut failed = failing.clone();
    failed.state = JobState::Failed;
    failed.error = Some("disk gone".into());
    failed.finished_at = Some(clock::now());
    jobs.finish(&failed).await.unwrap();
    let stored = jobs.get(failing.id).await.unwrap().unwrap().job;
    assert_eq!(stored.error.as_deref(), Some("disk gone"));
    assert!(stored.step.is_none() && stored.message.is_none());

    let recent = jobs.list_recent(2).await.unwrap();
    assert_eq!(recent.len(), 2);
    assert!(recent[0].created_at >= recent[1].created_at);
    h.finish().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn conformance_audit_and_plugin_kv() {
    let h = Harness::new().await;
    h.db.audit()
        .record(crate::audit::AuditEntry {
            action: crate::audit::actions::LOGIN_SUCCEEDED,
            actor: None,
            target: Some("device".into()),
            detail: Some(serde_json::json!({ "deviceName": "x" })),
        })
        .await
        .unwrap();
    let recent = h.db.audit().recent(10).await.unwrap();
    assert_eq!(recent.len(), 1);
    assert_eq!(recent[0].action, "auth.login");

    let kv = h.db.plugin_kv();
    let bytes = vec![0u8, 1, 255, 0, 42];
    kv.set("org.example.p", "k", bytes.clone()).await.unwrap();
    assert_eq!(kv.get("org.example.p", "k").await.unwrap(), Some(bytes));
    kv.set("org.example.p", "k", b"new".to_vec()).await.unwrap();
    assert_eq!(
        kv.get("org.example.p", "k").await.unwrap(),
        Some(b"new".to_vec())
    );
    kv.set("org.example.p", "other", b"x".to_vec())
        .await
        .unwrap();
    assert!(kv.delete("org.example.p", "k").await.unwrap());
    assert!(!kv.delete("org.example.p", "k").await.unwrap());
    assert!(matches!(
        kv.set(
            "org.example.p",
            "big",
            vec![0; crate::plugin_kv::MAX_VALUE_BYTES + 1]
        )
        .await,
        Err(DbError::Conflict(_))
    ));
    // The largest allowed value must fit on every backend.
    kv.set(
        "org.example.p",
        "max",
        vec![7; crate::plugin_kv::MAX_VALUE_BYTES],
    )
    .await
    .unwrap();
    assert_eq!(kv.clear("org.example.p").await.unwrap(), 2);
    h.finish().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn conformance_concurrent_writers_all_succeed() {
    let h = Harness::new().await;
    let admin =
        h.db.users()
            .create_first_admin("admin", "hash")
            .await
            .unwrap()
            .unwrap();
    let start = h.db.current_revision().await.unwrap();

    let mut tasks = Vec::new();
    for i in 0..24 {
        let library_db = h.db.clone();
        tasks.push(tokio::spawn(async move {
            library_db
                .libraries()
                .create(NewLibrary {
                    name: format!("Library {i}"),
                    kind: LibraryKind::Music,
                    paths: vec![format!("/media/{i}")],
                })
                .await
                .map(|_| ())
        }));
        let session_db = h.db.clone();
        tasks.push(tokio::spawn(async move {
            session_db
                .sessions()
                .create(admin.id, &format!("device {i}"), None)
                .await
                .map(|_| ())
        }));
        let job_db = h.db.clone();
        tasks.push(tokio::spawn(async move {
            job_db
                .jobs()
                .enqueue(NewJobRecord {
                    kind: "test".into(),
                    subject: None,
                    payload: serde_json::json!({ "i": i }),
                    required_capabilities: Vec::new(),
                    dedupe_key: None,
                })
                .await
                .map(|_| ())
        }));
        let reader_db = h.db.clone();
        tasks.push(tokio::spawn(async move {
            reader_db.libraries().list().await.map(|_| ())
        }));
    }
    for task in tasks {
        task.await.unwrap().expect("concurrent operation failed");
    }
    assert_eq!(h.db.libraries().list().await.unwrap().len(), 24);
    // One revision per library transaction, none lost or duplicated.
    assert_eq!(h.db.current_revision().await.unwrap(), start + 24);
    assert_eq!(h.db.jobs().list_recent(100).await.unwrap().len(), 24);
    h.finish().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn conformance_duplicate_name_race_has_one_winner() {
    let h = Harness::new().await;
    let mut tasks = Vec::new();
    for i in 0..8 {
        let db = h.db.clone();
        tasks.push(tokio::spawn(async move {
            db.libraries()
                .create(NewLibrary {
                    // Differs only in case: the check is case-insensitive.
                    name: if i % 2 == 0 {
                        "Music".into()
                    } else {
                        "MUSIC".into()
                    },
                    kind: LibraryKind::Music,
                    paths: vec![format!("/media/{i}")],
                })
                .await
        }));
    }
    let mut created = 0;
    let mut conflicts = 0;
    let mut unexpected = Vec::new();
    for task in tasks {
        match task.await.unwrap() {
            Ok(_) => created += 1,
            Err(DbError::Conflict(_)) => conflicts += 1,
            Err(other) => unexpected.push(other.to_string()),
        }
    }
    assert!(unexpected.is_empty(), "unexpected errors: {unexpected:?}");
    assert_eq!((created, conflicts), (1, 7));
    h.finish().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn conformance_setup_and_dedupe_races() {
    let h = Harness::new().await;
    let mut tasks = Vec::new();
    for i in 0..6 {
        let db = h.db.clone();
        tasks.push(tokio::spawn(async move {
            db.users()
                .create_first_admin(&format!("admin{i}"), "hash")
                .await
        }));
    }
    let mut winners = HashSet::new();
    for task in tasks {
        if let Some(user) = task.await.unwrap().unwrap() {
            winners.insert(user.id);
        }
    }
    assert_eq!(winners.len(), 1);
    assert_eq!(h.db.users().count().await.unwrap(), 1);

    let mut tasks = Vec::new();
    for _ in 0..6 {
        let db = h.db.clone();
        tasks.push(tokio::spawn(async move {
            db.jobs()
                .enqueue(NewJobRecord {
                    kind: "library.scan".into(),
                    subject: None,
                    payload: serde_json::json!({}),
                    required_capabilities: Vec::new(),
                    dedupe_key: Some("library.scan:x".into()),
                })
                .await
        }));
    }
    let mut ids = HashSet::new();
    for task in tasks {
        ids.insert(task.await.unwrap().unwrap().record.job.id);
    }
    assert_eq!(ids.len(), 1);
    h.finish().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn conformance_music_backfill_pagination_and_removal() {
    use crate::music::MusicCursor;
    use palveni_core::music::{MusicEntry, TrackTags};

    let h = Harness::new().await;
    let db = &h.db;
    let library = new_library(db, "Recordings").await.id;
    let other = new_library(db, "Other recordings").await.id;
    let mut second = track("second", "/media/02.flac", None, 16);
    db.items()
        .apply_scan_batch(library, &[second.clone()], &[])
        .await
        .unwrap();
    assert!(!db.items().scan_index(library).await.unwrap()[0].music_indexed);
    let mut first = track("first", "/media/01.flac", None, 20);
    first.music = Some(TrackTags {
        artist: Some("Guest".into()),
        album_artist: Some("Ensemble".into()),
        album: Some("Sessions".into()),
        title: Some("Overture".into()),
        track_number: Some(1),
        disc_number: Some(1),
        ..Default::default()
    });
    second.music = Some(TrackTags {
        track_number: Some(2),
        ..first.music.clone().unwrap()
    });
    db.items()
        .apply_scan_batch(library, &[second.clone(), first.clone()], &[])
        .await
        .unwrap();
    assert!(
        db.items()
            .scan_index(library)
            .await
            .unwrap()
            .iter()
            .all(|i| i.music_indexed)
    );
    let artists = db
        .music()
        .browse(library, None, None, 10, None)
        .await
        .unwrap();
    let artists = serde_json::to_value(artists).unwrap();
    let groups = artists["entries"].as_array().unwrap();
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0]["kind"], "artist");
    assert_eq!(groups[0]["name"], "Ensemble");
    let artist = groups[0]["id"].as_str().unwrap();
    let albums = db
        .music()
        .browse(library, Some(artist), None, 10, None)
        .await
        .unwrap();
    let albums = serde_json::to_value(albums).unwrap();
    let groups = albums["entries"].as_array().unwrap();
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0]["kind"], "album");
    assert_eq!(groups[0]["name"], "Sessions");
    let album = groups[0]["id"].as_str().unwrap();
    let page = db
        .music()
        .browse(library, Some(artist), Some(album), 1, None)
        .await
        .unwrap();
    assert!(
        matches!(&page.entries[..], [MusicEntry::Track{id,artist:Some(artist),..}] if *id == first.id && artist == "Guest")
    );
    let raw = page.next_cursor.unwrap();
    assert!(MusicCursor::decode(&raw, other, Some(artist), Some(album)).is_none());
    let cursor = MusicCursor::decode(&raw, library, Some(artist), Some(album)).unwrap();
    let page = db
        .music()
        .browse(library, Some(artist), Some(album), 1, Some(cursor))
        .await
        .unwrap();
    assert!(matches!(page.entries[..], [MusicEntry::Track{id,..}] if id == second.id));
    assert!(page.next_cursor.is_none());
    second.music.as_mut().unwrap().album = Some("Encore".into());
    db.items()
        .apply_scan_batch(library, &[second], &[first.id])
        .await
        .unwrap();
    assert!(
        db.music()
            .browse(library, Some(artist), Some(album), 10, None)
            .await
            .unwrap()
            .entries
            .is_empty()
    );
    assert!(
        db.music()
            .browse(other, None, None, 10, None)
            .await
            .unwrap()
            .entries
            .is_empty()
    );
    db.libraries().delete(library).await.unwrap();
    assert!(
        db.music()
            .indexed_item_ids(library)
            .await
            .unwrap()
            .is_empty()
    );
    h.finish().await;
}

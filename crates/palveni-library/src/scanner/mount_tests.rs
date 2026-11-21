use super::*;
use crate::mounts::linux_mounts;
use palveni_db::{DbOptions, libraries::NewLibrary};
use std::{cell::Cell, future::ready};

struct Quiet;
impl ScanObserver for Quiet {
    fn progress(&self, _: Option<f64>, _: &ScanStep) {}
}

#[tokio::test]
async fn disconnected_share_preserves_its_index_while_siblings_update() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().canonicalize().unwrap().join("media");
    let share = root.join("share");
    std::fs::create_dir_all(&share).unwrap();
    std::fs::write(share.join("film.mp4"), b"film").unwrap();
    let opts = DbOptions {
        main_url: format!("sqlite://{}/main.db?mode=rwc", tmp.path().display()),
        ops_url: format!("sqlite://{}/ops.db?mode=rwc", tmp.path().display()),
        max_connections: Some(4),
        min_connections: None,
    };
    let db = Db::open(&opts).await.unwrap();
    let library = db
        .libraries()
        .create(NewLibrary {
            name: "Movies".into(),
            kind: LibraryKind::Video,
            paths: vec![root.to_str().unwrap().into()],
        })
        .await
        .unwrap();
    let scanner = Scanner::new(
        db.clone(),
        EventBus::new(32, 0, || async { Ok::<_, std::io::Error>(1) }),
    );
    let online = format!("12 1 0:32 / {} ro - cifs //test/films ro", share.display());
    let scan = |info: &str| ready(Ok(linux_mounts(info)));
    scanner
        .scan_with_mounts(library.id, &Quiet, || scan(&online))
        .await
        .unwrap();
    let original = db
        .items()
        .scan_index(library.id)
        .await
        .unwrap()
        .into_iter()
        .find(|i| i.path.ends_with("film.mp4"))
        .unwrap()
        .id;
    let parked = tmp.path().join("parked");
    std::fs::rename(&share, &parked).unwrap();
    std::fs::create_dir(&share).unwrap();
    std::fs::write(share.join("replacement.mp4"), b"unrelated").unwrap();
    std::fs::write(root.join("local.mp4"), b"local").unwrap();

    let read = Cell::new(0);
    let err = scanner
        .scan_with_mounts(library.id, &Quiet, || {
            read.set(read.get() + 1);
            scan(if read.get() == 1 { &online } else { "" })
        })
        .await
        .unwrap_err();
    assert!(matches!(err, ScanError::MountChanged));
    let reopened = Db::open(&opts).await.unwrap();
    let restarted = Scanner::new(
        reopened,
        EventBus::new(32, 0, || async { Ok::<_, std::io::Error>(1) }),
    );
    let partial = restarted
        .scan_with_mounts(library.id, &Quiet, || scan(""))
        .await
        .unwrap();
    assert_eq!(partial.unreadable_dirs, 1);
    assert_eq!(partial.removed, 0);
    let rows = db.items().scan_index(library.id).await.unwrap();
    assert!(rows.iter().any(|r| r.id == original));
    assert!(rows.iter().any(|r| r.path.ends_with("/local.mp4")));
    assert!(!rows.iter().any(|r| r.path.ends_with("/replacement.mp4")));

    std::fs::rename(&share, tmp.path().join("underlying")).unwrap();
    std::fs::rename(&parked, &share).unwrap();
    let restored = restarted
        .scan_with_mounts(library.id, &Quiet, || scan(&online))
        .await
        .unwrap();
    assert_eq!(restored.unreadable_dirs, 0);
    assert!(db.items().get(original).await.unwrap().is_some());
    std::fs::remove_file(share.join("film.mp4")).unwrap();
    let updated = restarted
        .scan_with_mounts(library.id, &Quiet, || scan(&online))
        .await
        .unwrap();
    assert_eq!(updated.removed, 1);
    assert!(db.items().get(original).await.unwrap().is_none());
}

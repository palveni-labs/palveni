use super::*;
use crate::mounts::linux_mounts;
use palveni_db::{DbOptions, libraries::NewLibrary};
use std::{cell::Cell, future::ready};

async fn only_review(db: &Db, library: LibraryId) -> palveni_db::scan_reviews::MissingReview {
    let mut rows = db.scan_reviews().list(library).await.unwrap();
    assert_eq!(rows.len(), 1);
    rows.pop().unwrap()
}

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
    std::fs::write(share.join("other.mp4"), b"other").unwrap();
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
        .scan_with_mounts(library.id, &Quiet, None, || scan(&online))
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
        .scan_with_mounts(library.id, &Quiet, None, || {
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
        .scan_with_mounts(library.id, &Quiet, None, || scan(""))
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
        .scan_with_mounts(library.id, &Quiet, None, || scan(&online))
        .await
        .unwrap();
    assert_eq!(restored.unreadable_dirs, 0);
    assert!(db.items().get(original).await.unwrap().is_some());
    std::fs::remove_file(share.join("film.mp4")).unwrap();
    let updated = restarted
        .scan_with_mounts(library.id, &Quiet, None, || scan(&online))
        .await
        .unwrap();
    assert_eq!(updated.removed, 1);
    assert!(db.items().get(original).await.unwrap().is_none());
}

#[tokio::test]
async fn missing_music_survives_restart_partial_recovery_and_versioned_confirmation() {
    use palveni_core::{Role, personal::PlaybackUpdate};
    use palveni_db::personal::MediaChange;
    use std::fs;

    let tmp = tempfile::tempdir().unwrap();
    let base = tmp.path().canonicalize().unwrap();
    let suspect = base.join("library");
    let share = suspect.join("Collections/discs");
    let local = suspect.join("online");
    fs::create_dir_all(&share).unwrap();
    fs::create_dir_all(&local).unwrap();
    let opts = DbOptions {
        main_url: format!("sqlite://{}/main.db?mode=rwc", base.display()),
        ops_url: format!("sqlite://{}/ops.db?mode=rwc", base.display()),
        max_connections: Some(4),
        min_connections: None,
    };
    let db = Db::open(&opts).await.unwrap();
    let library = db
        .libraries()
        .create(NewLibrary {
            name: "Music".into(),
            kind: LibraryKind::Music,
            paths: vec![suspect.to_str().unwrap().into()],
        })
        .await
        .unwrap()
        .id;
    let user = db
        .users()
        .create("listener", "unused", Role::User)
        .await
        .unwrap()
        .id;
    let scanner = |db: Db| {
        Scanner::new(
            db,
            EventBus::new(32, 0, || async { Ok::<_, std::io::Error>(1) }),
        )
    };
    let initial = scanner(db.clone());
    let online = format!("12 1 0:32 / {} ro - cifs //test/discs ro", share.display());
    let mounts = || ready(Ok(linux_mounts(&online)));
    initial
        .scan_with_mounts(library, &Quiet, None, mounts)
        .await
        .unwrap();
    assert!(db.scan_reviews().list(library).await.unwrap().is_empty());

    let mut wav = b"RIFF\0\0\0\0WAVEfmt \x10\0\0\0\x01\0\x01\0\x40\x1f\0\0\x80\x3e\0\0\x02\0\x10\0data\0\0\0\0".to_vec();
    wav[4..8].copy_from_slice(&16036u32.to_le_bytes());
    wav[40..44].copy_from_slice(&16000u32.to_le_bytes());
    wav.resize(16044, 0);
    fs::write(share.join("concert.wav"), wav).unwrap();
    fs::write(share.join("concert.cue"), "FILE \"concert.wav\" WAVE\nTRACK 01 AUDIO\nINDEX 01 00:00:00\nTRACK 02 AUDIO\nINDEX 01 00:00:35").unwrap();
    fs::write(share.join("return.mp3"), b"song").unwrap();
    initial
        .scan_with_mounts(library, &Quiet, None, mounts)
        .await
        .unwrap();
    let rows = db.items().scan_index(library).await.unwrap();
    let id = |suffix| rows.iter().find(|r| r.path.ends_with(suffix)).unwrap().id;
    let movement = id("/concert.cue/02");
    let restored = id("/return.mp3");
    let parent = id("/Collections");
    let child = id("/discs");
    db.personal()
        .update(user, movement, MediaChange::Favorite(true))
        .await
        .unwrap();
    let state = db
        .personal()
        .update(
            user,
            movement,
            MediaChange::Progress(PlaybackUpdate {
                expected_version: 0,
                position_ms: 200,
                completed: false,
            }),
        )
        .await
        .unwrap();

    let parked = base.join("saved");
    fs::rename(&share, &parked).unwrap();
    fs::create_dir(&share).unwrap();
    fs::write(share.join("unrelated.mp3"), b"other").unwrap();
    fs::write(local.join("new.mp3"), b"sibling").unwrap();
    let summary = initial
        .scan_with_mounts(library, &Quiet, None, mounts)
        .await
        .unwrap();
    assert_eq!((summary.added, summary.removed), (1, 0));
    let first = only_review(&db, library).await;
    assert_eq!(first.missing_count, 2);
    assert!(
        first
            .sample_paths
            .iter()
            .all(|p| p.ends_with("/concert.wav") || p.ends_with("/return.mp3"))
    );
    assert_eq!(db.personal().get(user, movement).await.unwrap(), state);
    assert!(
        db.music()
            .track_location(movement)
            .await
            .unwrap()
            .unwrap()
            .cue_frames
            .is_some()
    );
    for item in [parent, child] {
        assert!(db.items().get(item).await.unwrap().is_some());
    }
    let restarted = scanner(Db::open(&opts).await.unwrap());
    let offline = || ready(Ok(linux_mounts("")));
    restarted
        .scan_with_mounts(library, &Quiet, Some(&first.revision), offline)
        .await
        .unwrap();
    assert_eq!(only_review(&db, library).await.revision, first.revision);
    assert_eq!(db.personal().get(user, movement).await.unwrap(), state);
    fs::rename(parked.join("return.mp3"), share.join("return.mp3")).unwrap();
    restarted
        .scan_with_mounts(library, &Quiet, Some(&first.revision), mounts)
        .await
        .unwrap();
    let second = only_review(&db, library).await;
    assert_ne!(second.revision, first.revision);
    assert_eq!(second.missing_count, 1);
    assert_eq!(db.personal().get(user, movement).await.unwrap(), state);
    assert!(db.items().get(restored).await.unwrap().is_some());

    fs::rename(share.join("return.mp3"), parked.join("return.mp3")).unwrap();
    restarted
        .scan_with_mounts(library, &Quiet, Some(&second.revision), mounts)
        .await
        .unwrap();
    let changed = only_review(&db, library).await;
    assert_eq!(changed.missing_count, 2);
    assert_ne!(changed.revision, second.revision);
    assert!(db.items().get(restored).await.unwrap().is_some());
    fs::rename(parked.join("return.mp3"), share.join("return.mp3")).unwrap();
    restarted
        .scan_with_mounts(library, &Quiet, Some(&changed.revision), mounts)
        .await
        .unwrap();
    let current = only_review(&db, library).await;
    assert_ne!(current.revision, second.revision);
    let changing = Cell::new(0);
    let err = restarted
        .scan_with_mounts(library, &Quiet, Some(&current.revision), || {
            changing.set(changing.get() + 1);
            ready(Ok(linux_mounts(if changing.get() == 3 {
                ""
            } else {
                &online
            })))
        })
        .await
        .unwrap_err();
    assert!(matches!(err, ScanError::MountChanged));
    assert_eq!(db.personal().get(user, movement).await.unwrap(), state);

    restarted
        .scan_with_mounts(library, &Quiet, Some(&current.revision), mounts)
        .await
        .unwrap();
    assert!(db.scan_reviews().list(library).await.unwrap().is_empty());
    assert!(db.items().get(movement).await.unwrap().is_none());
    assert!(db.items().get(restored).await.unwrap().is_some());
    assert!(db.items().get(parent).await.unwrap().is_some());
    // A later outage follows a new review even if the previously restored song disappears again.
    fs::rename(share.join("return.mp3"), parked.join("return.mp3")).unwrap();
    fs::rename(share.join("unrelated.mp3"), parked.join("unrelated.mp3")).unwrap();
    restarted
        .scan_with_mounts(library, &Quiet, Some(&current.revision), mounts)
        .await
        .unwrap();
    let later = only_review(&db, library).await;
    assert_ne!(later.revision, current.revision);
    assert!(db.items().get(restored).await.unwrap().is_some());
    fs::rename(parked.join("return.mp3"), share.join("return.mp3")).unwrap();
    fs::rename(parked.join("unrelated.mp3"), share.join("unrelated.mp3")).unwrap();
    restarted
        .scan_with_mounts(library, &Quiet, None, mounts)
        .await
        .unwrap();
    assert!(db.scan_reviews().list(library).await.unwrap().is_empty());
    assert!(db.items().get(restored).await.unwrap().is_some());
}

#[tokio::test]
async fn unpinned_root_reviews_bulk_disappearance_and_keeps_the_last_items_pending() {
    use std::fs;

    let tmp = tempfile::tempdir().unwrap();
    let base = tmp.path().canonicalize().unwrap();
    let root = base.join("books");
    let parked = base.join("spare");
    fs::create_dir(&root).unwrap();
    fs::create_dir(&parked).unwrap();
    let db = Db::open(&DbOptions {
        main_url: format!("sqlite://{}/main.db?mode=rwc", base.display()),
        ops_url: format!("sqlite://{}/ops.db?mode=rwc", base.display()),
        max_connections: Some(4),
        min_connections: None,
    })
    .await
    .unwrap();
    let library = db
        .libraries()
        .create(NewLibrary {
            name: "Ebooks".into(),
            kind: LibraryKind::Book,
            paths: vec![root.to_str().unwrap().into()],
        })
        .await
        .unwrap()
        .id;
    let scanner = Scanner::new(
        db.clone(),
        EventBus::new(32, 0, || async { Ok::<_, std::io::Error>(1) }),
    );
    let scan = || scanner.scan_with_mounts(library, &Quiet, None, || ready(Ok(linux_mounts(""))));
    for n in 0..111 {
        fs::write(root.join(format!("{n:03}.epub")), b"book").unwrap();
    }
    assert_eq!(scan().await.unwrap().added, 111);
    for n in 0..100 {
        let name = format!("{n:03}.epub");
        fs::rename(root.join(&name), parked.join(&name)).unwrap();
    }
    assert_eq!(scan().await.unwrap().removed, 0);
    let first = only_review(&db, library).await;
    assert_eq!(first.missing_count, 100);
    assert_eq!(first.sample_paths.len(), 10);

    // All filenames reappearing on a different covering filesystem cannot clear a review.
    for n in 0..100 {
        fs::write(root.join(format!("{n:03}.epub")), b"different media").unwrap();
    }
    let replacement = format!("9 1 2:1 / {} ro - nfs //other/books ro", root.display());
    let err = scanner
        .scan_with_mounts(library, &Quiet, Some(&first.revision), || {
            ready(Ok(linux_mounts(&replacement)))
        })
        .await
        .unwrap_err();
    assert!(matches!(err, ScanError::NoReadableRoot));
    assert_eq!(only_review(&db, library).await.revision, first.revision);
    for n in 0..100 {
        fs::remove_file(root.join(format!("{n:03}.epub"))).unwrap();
    }

    // A single unreadable child makes the root inventory insufficient for confirmation.
    let scope = root.to_str().unwrap().to_owned();
    let (mut scopes, _) = reviews::ReviewScopes::load(
        &db,
        library,
        std::slice::from_ref(&scope),
        &linux_mounts(""),
    )
    .await
    .unwrap();
    let mut partial = WalkOutcome::default();
    partial
        .protected
        .push(root.join("temporarily unreadable").to_str().unwrap().into());
    let index = db.items().scan_index(library).await.unwrap();
    assert!(
        scopes
            .inspect(&db, library, Some(&first.revision), &index, &partial)
            .await
            .unwrap()
            .is_none()
    );
    assert!(scopes.held(root.join("000.epub").to_str().unwrap()));
    assert_eq!(only_review(&db, library).await.revision, first.revision);
    fs::rename(parked.join("000.epub"), root.join("000.epub")).unwrap();
    assert_eq!(scan().await.unwrap().removed, 0);
    assert_eq!(only_review(&db, library).await.missing_count, 99);
    for n in 1..100 {
        let name = format!("{n:03}.epub");
        fs::rename(parked.join(&name), root.join(&name)).unwrap();
    }
    assert_eq!(scan().await.unwrap().removed, 0);
    assert!(db.scan_reviews().list(library).await.unwrap().is_empty());
    for n in 1..100 {
        let name = format!("{n:03}.epub");
        fs::rename(root.join(&name), parked.join(&name)).unwrap();
    }
    assert_eq!(scan().await.unwrap().removed, 99);
    assert!(db.scan_reviews().list(library).await.unwrap().is_empty());
}

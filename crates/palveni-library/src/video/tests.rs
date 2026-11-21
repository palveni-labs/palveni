use super::*;
use palveni_core::{LibraryKind, events::EventBus, video::VideoKind};
use palveni_db::{Db, DbOptions, libraries::NewLibrary, video::PosterLevel};
use std::fs;
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

struct Observer;
impl ScanObserver for Observer {
    fn progress(&self, _: Option<f64>, _: &ScanStep) {}
}

#[tokio::test]
async fn imports_sidecars_into_existing_file_tree_and_reconciles_changes() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("media");
    let film = root.join("A (2023)");
    let show = root.join("Drama");
    let season = show.join("Season 02");
    fs::create_dir_all(&film).unwrap();
    fs::create_dir_all(&season).unwrap();
    fs::write(film.join("release.mkv"), b"sample").unwrap();
    fs::write(
        film.join("movie.nfo"),
        b"<movie><title>Amber</title><plot>First plot</plot><year>2023</year></movie>",
    )
    .unwrap();
    let jpeg = &[0xff, 0xd8, 0xff, 0xe0];
    fs::write(film.join("poster.jpg"), jpeg).unwrap();
    fs::write(
        show.join("tvshow.nfo"),
        b"<tvshow><title>Blue</title></tvshow>",
    )
    .unwrap();
    fs::write(show.join("poster.jpg"), jpeg).unwrap();
    fs::write(season.join("Drama.S02E03.mp4"), b"episode").unwrap();
    fs::write(season.join("Drama.S02E03.nfo"),b"<episodedetails><title>The Creek</title><season>2</season><episode>3</episode></episodedetails>").unwrap();
    let opts = DbOptions {
        main_url: format!("sqlite://{}?mode=rwc", tmp.path().join("m.db").display()),
        ops_url: format!("sqlite://{}?mode=rwc", tmp.path().join("o.db").display()),
        max_connections: Some(4),
        min_connections: None,
    };
    let db = Db::open(&opts).await.unwrap();
    let id = db
        .libraries()
        .create(NewLibrary {
            name: "Video".into(),
            kind: LibraryKind::Video,
            paths: vec![fs::canonicalize(&root).unwrap().to_str().unwrap().into()],
        })
        .await
        .unwrap()
        .id;
    let seq = Arc::new(AtomicI64::new(0));
    let events = EventBus::new(32, 0, move || {
        let n = seq.fetch_add(1, Ordering::Relaxed) + 1;
        async move { Ok::<_, std::io::Error>(n) }
    });
    let scanner = crate::Scanner::new(db.clone(), events);
    scanner.scan(id, &Observer).await.unwrap();
    let page = db.video().browse(id, None, None, 8, None).await.unwrap();
    assert_eq!(
        page.entries
            .iter()
            .map(|v| v.title.as_str())
            .collect::<Vec<_>>(),
        ["Amber", "Blue"]
    );
    assert_eq!(page.entries[0].kind, VideoKind::Movie);
    let movie = page.entries[0].item_id.unwrap();
    let loc = db
        .video()
        .poster_location(movie, PosterLevel::Work)
        .await
        .unwrap()
        .unwrap();
    let (mime, size, mut stream) = open_poster(loc).await.unwrap();
    let mut bytes = Vec::new();
    tokio::io::AsyncReadExt::read_to_end(&mut stream, &mut bytes)
        .await
        .unwrap();
    assert_eq!(
        (mime, size, bytes.as_slice()),
        ("image/jpeg", 4, jpeg.as_slice())
    );
    let key = &page.entries[1].id;
    let page = db
        .video()
        .browse(id, Some(key), Some(2), 8, None)
        .await
        .unwrap();
    assert_eq!(page.entries[0].title, "The Creek");
    assert_eq!(page.entries[0].episode_number, Some(3));
    let ep = page.entries[0].item_id.unwrap();
    assert!(
        db.video()
            .poster_location(ep, PosterLevel::Episode)
            .await
            .unwrap()
            .is_some()
    );
    fs::write(
        film.join("movie.nfo"),
        b"<movie><title>After</title></movie>",
    )
    .unwrap();
    fs::remove_file(film.join("poster.jpg")).unwrap();
    #[cfg(unix)]
    {
        let outside = tmp.path().join("external.jpg");
        fs::write(&outside, jpeg).unwrap();
        std::os::unix::fs::symlink(&outside, film.join("poster.jpg")).unwrap();
        let indexed = db
            .video()
            .poster_location(movie, PosterLevel::Work)
            .await
            .unwrap()
            .unwrap();
        assert!(matches!(open_poster(indexed).await, Err(Error::PathEscape)));
    }
    scanner.scan(id, &Observer).await.unwrap();
    let page = db.video().browse(id, None, None, 8, None).await.unwrap();
    assert_eq!(page.entries[0].title, "After");
    assert_eq!(page.entries[0].item_id, Some(movie));
    assert!(page.entries[0].poster_item_id.is_none());
    assert!(
        db.video()
            .poster_location(movie, PosterLevel::Work)
            .await
            .unwrap()
            .is_none()
    );
}

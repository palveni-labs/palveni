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

#[tokio::test]
async fn indexes_extended_nfo_backdrops_thumbs_and_local_actors() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("media");
    let film = root.join("The Lighthouse Keeper (2019)");
    let film_actors = film.join(".actors");
    let show = root.join("Orbit Shift");
    let show_actors = show.join(".actors");
    let season = show.join("Season 01");
    fs::create_dir_all(&film_actors).unwrap();
    fs::create_dir_all(&show_actors).unwrap();
    fs::create_dir_all(&season).unwrap();

    let jpeg = &[0xff, 0xd8, 0xff, 0xe0, 0x00, 0x10];
    fs::write(film.join("lighthouse.mkv"), b"movie").unwrap();
    fs::write(
        film.join("movie.nfo"),
        include_str!("fixtures/kodi-movie.nfo"),
    )
    .unwrap();
    fs::write(film.join("poster.jpg"), jpeg).unwrap();
    fs::write(film.join("fanart.jpg"), jpeg).unwrap();
    fs::write(film_actors.join("Ana_Ruiz.jpg"), jpeg).unwrap();
    fs::write(film_actors.join("Tom Weller.jpg"), jpeg).unwrap();

    fs::write(
        show.join("tvshow.nfo"),
        include_str!("fixtures/jellyfin-tvshow.nfo"),
    )
    .unwrap();
    fs::write(show.join("poster.jpg"), jpeg).unwrap();
    fs::write(show.join("backdrop.jpg"), jpeg).unwrap();
    fs::write(show_actors.join("Rae_Kim.jpg"), jpeg).unwrap();
    fs::write(season.join("Orbit.Shift.S01E02.mkv"), b"ep").unwrap();
    fs::write(
        season.join("Orbit.Shift.S01E02.nfo"),
        include_str!("fixtures/kodi-episode.nfo"),
    )
    .unwrap();
    fs::write(season.join("Orbit.Shift.S01E02-thumb.jpg"), jpeg).unwrap();

    let opts = DbOptions {
        main_url: format!("sqlite://{}?mode=rwc", tmp.path().join("m.db").display()),
        ops_url: format!("sqlite://{}?mode=rwc", tmp.path().join("o.db").display()),
        max_connections: Some(4),
        min_connections: None,
    };
    let db = Db::open(&opts).await.unwrap();
    let user = db
        .users()
        .create("viewer", "hash", palveni_core::Role::Admin)
        .await
        .unwrap()
        .id;
    let id = db
        .libraries()
        .create(NewLibrary {
            name: "Cinema".into(),
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

    // `.actors` must not appear as an indexed folder item.
    let items = db.items().scan_index(id).await.unwrap();
    assert!(!items.iter().any(|i| i.path.contains(".actors")));

    let page = db.video().browse(id, None, None, 10, None).await.unwrap();
    assert_eq!(page.total, 2);
    let show_entry = page
        .entries
        .iter()
        .find(|e| e.kind == VideoKind::Series)
        .unwrap();
    let movie_entry = page
        .entries
        .iter()
        .find(|e| e.kind == VideoKind::Movie)
        .unwrap();
    assert_eq!(movie_entry.title, "The Lighthouse Keeper");
    assert_eq!(
        movie_entry.original_title.as_deref(),
        Some("Le Gardien du Phare")
    );
    assert_eq!(
        movie_entry.tagline.as_deref(),
        Some("Every light hides a shadow.")
    );
    assert_eq!(movie_entry.community_rating, Some(7.8));
    assert_eq!(movie_entry.official_rating.as_deref(), Some("PG-13"));
    assert_eq!(movie_entry.premiered.as_deref(), Some("2019-10-18"));
    assert_eq!(movie_entry.runtime_ms, Some(118 * 60_000));
    assert!(movie_entry.backdrop_item_id.is_some());
    assert_eq!(movie_entry.genres, ["Drama", "Mystery", "Thriller"]);

    let detail = db
        .video()
        .work_detail(user, id, &movie_entry.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(detail.studios, ["North Pier Films", "Atlantic Pictures"]);
    // Ana Ruiz via NFO relative thumb, Tom Weller via `.actors/Tom Weller.jpg`, Jo Park remote URL ignored!
    let ana = detail.people.iter().find(|p| p.name == "Ana Ruiz").unwrap();
    let tom = detail
        .people
        .iter()
        .find(|p| p.name == "Tom Weller")
        .unwrap();
    let jo = detail.people.iter().find(|p| p.name == "Jo Park").unwrap();
    assert!(ana.image_url.is_some());
    assert!(tom.image_url.is_some());
    assert!(jo.image_url.is_none());

    let person_loc = db
        .video()
        .person_image_location(id, &palveni_db::video::person_key("Ana Ruiz"))
        .await
        .unwrap()
        .unwrap();
    let (mime, size, _) = open_poster(person_loc).await.unwrap();
    assert_eq!((mime, size), ("image/jpeg", jpeg.len() as u64));

    let show_detail = db
        .video()
        .work_detail(user, id, &show_entry.id)
        .await
        .unwrap()
        .unwrap();
    assert!(show_detail.work.backdrop_item_id.is_some());
    let rae = show_detail
        .people
        .iter()
        .find(|p| p.name == "Rae Kim")
        .unwrap();
    assert!(rae.image_url.is_some());
    let ep = show_detail.next_up.as_ref().unwrap();
    assert_eq!(ep.title, "Second Light");
    assert_eq!(ep.runtime_ms, Some(1_412_000));
    assert_eq!(ep.series_id.as_deref(), Some(show_entry.id.as_str()));
    assert_eq!(ep.series_title.as_deref(), Some("Orbit Shift"));
    let thumb_loc = db
        .video()
        .thumb_location(ep.item_id.unwrap())
        .await
        .unwrap()
        .unwrap();
    assert!(thumb_loc.local_thumb.is_some());
}

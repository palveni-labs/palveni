use palveni_core::metadata::MetadataProvider;
use palveni_core::reading::ReadingKind;
use palveni_core::{ItemId, ItemKind, LibraryKind, Role};

use super::{Harness, ItemUpsert, NewLibrary};
use crate::DbError;
use crate::convert::sha256_hex;
use crate::metadata_overrides::MetadataOverrideUpsert;
use crate::reading::ReadingMetadataUpsert;
use crate::video::VideoMetadata;

fn make_item(kind: ItemKind, title: &str, path: &str) -> ItemUpsert {
    ItemUpsert {
        music: None,
        id: ItemId::generate(),
        parent_id: None,
        kind,
        title: title.to_owned(),
        sort_title: title.to_lowercase(),
        search_text: title.to_lowercase(),
        path: path.to_owned(),
        size_bytes: Some(2048),
        modified_at: Some(1_730_000_000_000),
    }
}

#[tokio::test]
async fn conformance_metadata_overrides_sync_video_and_reading_and_cascade() {
    let h = Harness::new().await;
    let user =
        h.db.users()
            .create("admin", "pw", Role::Admin)
            .await
            .expect("create user");

    let video_lib =
        h.db.libraries()
            .create(NewLibrary {
                name: "Cinema".into(),
                kind: LibraryKind::Video,
                paths: vec!["/srv/cinema".into()],
            })
            .await
            .expect("create video library");

    let book_lib =
        h.db.libraries()
            .create(NewLibrary {
                name: "Books".into(),
                kind: LibraryKind::Book,
                paths: vec!["/srv/books".into()],
            })
            .await
            .expect("create book library");

    let movie_item = make_item(
        ItemKind::Video,
        "raw_file_01",
        "/srv/cinema/raw_file_01.mkv",
    );
    let movie_id = movie_item.id;
    h.db.items()
        .apply_scan_batch(video_lib.id, &[movie_item], &[])
        .await
        .expect("insert video item");

    let work_group = "cinema/raw_file_01";
    let work_key = sha256_hex(work_group);
    h.db.video()
        .save_batch(
            video_lib.id,
            vec![VideoMetadata {
                item_id: movie_id,
                stamp: "v1".into(),
                work_group: work_group.into(),
                work_title: "Raw File 01".into(),
                year: Some(2000),
                work_overview: Some("Placeholder".into()),
                work_poster: None,
                work_original_title: None,
                work_tagline: None,
                work_rating: Some(50),
                work_official_rating: None,
                work_premiered: None,
                work_studios: Vec::new(),
                work_backdrop: None,
                season: None,
                season_poster: None,
                episode: None,
                title: "Raw File 01".into(),
                overview: Some("Placeholder".into()),
                episode_poster: None,
                runtime_ms: Some(7_200_000),
                rating: Some(50),
                premiered: None,
                thumb: None,
                genres: vec!["OldGenre".into()],
                people: Vec::new(),
            }],
        )
        .await
        .expect("upsert video metadata");

    let book_item = make_item(ItemKind::Book, "untitled.epub", "/srv/books/untitled.epub");
    let book_id = book_item.id;
    h.db.items()
        .apply_scan_batch(book_lib.id, &[book_item], &[])
        .await
        .expect("insert book item");

    h.db.reading()
        .upsert_batch(&[ReadingMetadataUpsert {
            item_id: book_id,
            library_id: book_lib.id,
            stamp: "r1".into(),
            kind: ReadingKind::Book,
            format: "epub".into(),
            title: "Untitled".into(),
            sort_title: "untitled".into(),
            authors: vec!["Author A".into()],
            series_name: None,
            series_key: None,
            series_index: None,
            summary: Some("Old summary".into()),
            publisher: None,
            language: Some("en".into()),
            page_count: Some(200),
            has_cover: false,
            tags: vec!["Draft".into()],
        }])
        .await
        .expect("upsert reading metadata");

    // 1. Missing item returns NotFound.
    let missing_err =
        h.db.metadata_overrides()
            .upsert(MetadataOverrideUpsert {
                item_id: ItemId::generate(),
                work_key: None,
                provider: MetadataProvider::Manual,
                external_id: None,
                title: Some("Nope".into()),
                original_title: None,
                year: None,
                overview: None,
                rating: None,
                genres: Vec::new(),
                poster_url: None,
                backdrop_url: None,
            })
            .await
            .expect_err("missing item should fail");
    assert!(matches!(missing_err, DbError::NotFound(_)));

    // 2. Upsert override on video item syncs video_metadata + video_genres + items.
    let saved_movie =
        h.db.metadata_overrides()
            .upsert(MetadataOverrideUpsert {
                item_id: movie_id,
                work_key: None,
                provider: MetadataProvider::Tmdb,
                external_id: Some("438631".into()),
                title: Some("沙丘".into()),
                original_title: Some("Dune".into()),
                year: Some(2021),
                overview: Some("Paul Atreides travels to Arrakis.".into()),
                rating: Some(8.4),
                genres: vec!["科幻".into(), "冒险".into()],
                poster_url: Some("https://image.tmdb.org/t/p/w500/dune.jpg".into()),
                backdrop_url: Some("https://image.tmdb.org/t/p/w1280/dune-bg.jpg".into()),
            })
            .await
            .expect("upsert movie override");
    assert_eq!(saved_movie.item_id, movie_id);
    assert_eq!(saved_movie.work_key.as_deref(), Some(work_key.as_str()));
    assert_eq!(saved_movie.provider, MetadataProvider::Tmdb);
    assert_eq!(saved_movie.title.as_deref(), Some("沙丘"));
    assert_eq!(saved_movie.genres, vec!["科幻", "冒险"]);

    let fetched =
        h.db.metadata_overrides()
            .get(movie_id)
            .await
            .expect("get movie override")
            .expect("override exists");
    assert_eq!(fetched.external_id.as_deref(), Some("438631"));
    assert_eq!(fetched.year, Some(2021));

    let detail =
        h.db.video()
            .work_detail(user.id, video_lib.id, &work_key)
            .await
            .expect("work detail")
            .expect("work exists");
    assert_eq!(detail.work.title, "沙丘");
    assert_eq!(detail.work.original_title.as_deref(), Some("Dune"));
    assert_eq!(detail.work.year, Some(2021));
    assert_eq!(
        detail.work.overview.as_deref(),
        Some("Paul Atreides travels to Arrakis.")
    );
    assert_eq!(detail.work.community_rating, Some(8.4));
    assert_eq!(detail.work.genres, vec!["冒险", "科幻"]);

    // 3. Re-upsert replaces fields cleanly.
    let updated_movie =
        h.db.metadata_overrides()
            .upsert(MetadataOverrideUpsert {
                item_id: movie_id,
                work_key: Some(work_key.clone()),
                provider: MetadataProvider::Manual,
                external_id: None,
                title: Some("沙丘：第一部".into()),
                original_title: Some("Dune: Part One".into()),
                year: Some(2021),
                overview: Some("Updated overview".into()),
                rating: Some(8.7),
                genres: vec!["史诗".into()],
                poster_url: None,
                backdrop_url: None,
            })
            .await
            .expect("update movie override");
    assert_eq!(updated_movie.provider, MetadataProvider::Manual);
    assert_eq!(updated_movie.title.as_deref(), Some("沙丘：第一部"));

    // 4. Upsert override on reading item syncs reading_metadata.
    let _saved_book =
        h.db.metadata_overrides()
            .upsert(MetadataOverrideUpsert {
                item_id: book_id,
                work_key: None,
                provider: MetadataProvider::Openlibrary,
                external_id: Some("/works/OL123W".into()),
                title: Some("三体".into()),
                original_title: Some("The Three-Body Problem".into()),
                year: Some(2008),
                overview: Some("文明与宇宙社会学。".into()),
                rating: Some(9.2),
                genres: vec!["科幻".into(), "硬科幻".into()],
                poster_url: Some("https://covers.openlibrary.org/b/id/1-L.jpg".into()),
                backdrop_url: None,
            })
            .await
            .expect("upsert book override");

    let reading_entry =
        h.db.reading()
            .entry(user.id, book_id)
            .await
            .expect("get reading entry")
            .expect("reading entry exists");
    assert_eq!(reading_entry.title, "三体");
    assert_eq!(reading_entry.summary.as_deref(), Some("文明与宇宙社会学。"));
    assert_eq!(reading_entry.tags, vec!["科幻", "硬科幻"]);

    // 5. Delete override returns true then false.
    assert!(
        h.db.metadata_overrides()
            .delete(movie_id)
            .await
            .expect("delete override")
    );
    assert!(
        !h.db
            .metadata_overrides()
            .delete(movie_id)
            .await
            .expect("delete again")
    );
    assert!(
        h.db.metadata_overrides()
            .get(movie_id)
            .await
            .expect("get after delete")
            .is_none()
    );

    // 6. Live count & WebDAV listing work cleanly.
    assert_eq!(h.db.items().count_live().await.expect("count live"), 2);
    assert_eq!(
        h.db.items()
            .list_webdav_items(video_lib.id)
            .await
            .expect("webdav list")
            .len(),
        1
    );

    // 7. Deleting book library cascades cleanly to metadata_overrides.
    assert!(
        h.db.libraries()
            .delete(book_lib.id)
            .await
            .expect("delete book library")
    );
    assert!(
        h.db.metadata_overrides()
            .get(book_id)
            .await
            .expect("get cascaded override")
            .is_none()
    );

    h.finish().await;
}

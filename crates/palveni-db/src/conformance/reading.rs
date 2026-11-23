use super::*;
use crate::personal::MediaChange;
use crate::reading::{ReadingCursor, ReadingFilters, ReadingMetadataUpsert, series_key_for};
use palveni_core::reading::{
    CreateBookmarkRequest, ReadingChapterSummary, ReadingKind, ReadingProgressUpdate,
    ReadingReaderMode, ReadingSort, ReadingSortDir,
};

fn make_item(kind: ItemKind, title: &str, path: &str, size: i64) -> ItemUpsert {
    ItemUpsert {
        music: None,
        id: ItemId::generate(),
        parent_id: None,
        kind,
        title: title.to_owned(),
        sort_title: title.to_lowercase(),
        search_text: title.to_lowercase(),
        path: path.to_owned(),
        size_bytes: Some(size),
        modified_at: Some(1_730_000_000_000),
    }
}

#[tokio::test]
async fn conformance_reading_metadata_filters_progress_bookmarks_and_opds() {
    let h = Harness::new().await;
    let alice =
        h.db.users()
            .create("alice", "pw", Role::User)
            .await
            .unwrap()
            .id;
    let bob =
        h.db.users()
            .create("bob", "pw", Role::User)
            .await
            .unwrap()
            .id;

    let book_lib =
        h.db.libraries()
            .create(NewLibrary {
                name: "电子书".into(),
                kind: LibraryKind::Book,
                paths: vec!["/srv/books".into()],
            })
            .await
            .unwrap()
            .id;
    let comic_lib =
        h.db.libraries()
            .create(NewLibrary {
                name: "漫画".into(),
                kind: LibraryKind::Comic,
                paths: vec!["/srv/comics".into()],
            })
            .await
            .unwrap()
            .id;

    let b1 = make_item(
        ItemKind::Book,
        "雾灯 I",
        "/srv/books/three-body-1.epub",
        900_000,
    );
    let b2 = make_item(
        ItemKind::Book,
        "雾灯 II：白礁航线",
        "/srv/books/three-body-2.epub",
        1_200_000,
    );
    let b3 = make_item(
        ItemKind::Book,
        "设计系统手册",
        "/srv/books/design-handbook.pdf",
        4_500_000,
    );
    let c1 = make_item(
        ItemKind::Comic,
        "云梯集市 第01卷",
        "/srv/comics/dungeon-meshi-01.cbz",
        45_000_000,
    );
    let c2 = make_item(
        ItemKind::Comic,
        "云梯集市 第02卷",
        "/srv/comics/dungeon-meshi-02.cbz",
        48_000_000,
    );

    h.db.items()
        .apply_scan_batch(book_lib, &[b1.clone(), b2.clone(), b3.clone()], &[])
        .await
        .unwrap();
    h.db.items()
        .apply_scan_batch(comic_lib, &[c1.clone(), c2.clone()], &[])
        .await
        .unwrap();

    let repo = h.db.reading();

    // Unindexed pre-existing items already appear as candidates and fallback entries.
    let candidates = repo.library_candidates(book_lib).await.unwrap();
    assert_eq!(candidates.len(), 3);
    assert!(candidates.iter().all(|c| c.existing_stamp.is_none()));

    repo.upsert_batch(&[
        ReadingMetadataUpsert {
            item_id: b1.id,
            library_id: book_lib,
            stamp: "v1:900000:1730000000000".into(),
            kind: ReadingKind::Book,
            format: "epub".into(),
            title: "雾灯".into(),
            sort_title: "san ti 01".into(),
            authors: vec!["岑砚禾".into()],
            series_name: Some("北湾手记".into()),
            series_key: series_key_for("北湾手记"),
            series_index: Some(1.0),
            summary: Some("守塔人记录海雾里第一次出现的蓝色灯号。".into()),
            publisher: Some("纸鸥书社".into()),
            language: Some("zh-CN".into()),
            page_count: Some(36),
            has_cover: true,
            tags: vec!["科幻".into(), "雨果奖".into()],
        },
        ReadingMetadataUpsert {
            item_id: b2.id,
            library_id: book_lib,
            stamp: "v1:1200000:1730000000000".into(),
            kind: ReadingKind::Book,
            format: "epub".into(),
            title: "白礁航线".into(),
            sort_title: "san ti 02".into(),
            authors: vec!["岑砚禾".into()],
            series_name: Some("北湾手记".into()),
            series_key: series_key_for("北湾手记"),
            series_index: Some(2.0),
            summary: Some("巡岸员沿着旧航标寻找遗失的测绘日志。".into()),
            publisher: Some("纸鸥书社".into()),
            language: Some("zh-CN".into()),
            page_count: Some(42),
            has_cover: true,
            tags: vec!["科幻".into()],
        },
        ReadingMetadataUpsert {
            item_id: b3.id,
            library_id: book_lib,
            stamp: "v1:4500000:1730000000000".into(),
            kind: ReadingKind::Book,
            format: "pdf".into(),
            title: "设计系统手册".into(),
            sort_title: "she ji xi tong".into(),
            authors: vec!["陈默".into()],
            series_name: None,
            series_key: None,
            series_index: None,
            summary: Some("排版、网格与可访问性。".into()),
            publisher: None,
            language: Some("zh-CN".into()),
            page_count: None,
            has_cover: false,
            tags: vec!["设计".into()],
        },
        ReadingMetadataUpsert {
            item_id: c1.id,
            library_id: comic_lib,
            stamp: "v1:45000000:1730000000000".into(),
            kind: ReadingKind::Comic,
            format: "cbz".into(),
            title: "云梯集市 第1卷".into(),
            sort_title: "mi gong fan 01".into(),
            authors: vec!["谷芦".into()],
            series_name: Some("云梯集市".into()),
            series_key: series_key_for("云梯集市"),
            series_index: Some(1.0),
            summary: Some("在地下城烹饪魔物的冒险故事。".into()),
            publisher: None,
            language: Some("zh-CN".into()),
            page_count: Some(192),
            has_cover: true,
            tags: vec!["奇幻".into(), "美食".into()],
        },
        ReadingMetadataUpsert {
            item_id: c2.id,
            library_id: comic_lib,
            stamp: "v1:48000000:1730000000000".into(),
            kind: ReadingKind::Comic,
            format: "cbz".into(),
            title: "云梯集市 第2卷".into(),
            sort_title: "mi gong fan 02".into(),
            authors: vec!["谷芦".into()],
            series_name: Some("云梯集市".into()),
            series_key: series_key_for("云梯集市"),
            series_index: Some(2.0),
            summary: None,
            publisher: None,
            language: Some("zh-CN".into()),
            page_count: Some(198),
            has_cover: true,
            tags: vec!["奇幻".into()],
        },
    ])
    .await
    .unwrap();

    // Stamps recorded accurately
    let stamps = repo.stamps(book_lib).await.unwrap();
    assert_eq!(stamps.len(), 3);

    // Filter by series + sort by series ascending
    let by_series = repo
        .list(
            alice,
            book_lib,
            &ReadingFilters {
                sort: ReadingSort::Series,
                dir: Some(ReadingSortDir::Asc),
                ..ReadingFilters {
                    limit: 50,
                    ..Default::default()
                }
            },
        )
        .await
        .unwrap()
        .unwrap();
    assert_eq!(by_series.total, 3);
    assert_eq!(by_series.entries[0].item_id, b1.id);
    assert_eq!(by_series.entries[1].item_id, b2.id);
    assert_eq!(by_series.entries[2].item_id, b3.id);

    // Filter by author, format and query
    let author_page = repo
        .list(
            alice,
            book_lib,
            &ReadingFilters {
                author: Some("岑砚禾"),
                limit: 1,
                sort: ReadingSort::Title,
                dir: Some(ReadingSortDir::Asc),
                ..Default::default()
            },
        )
        .await
        .unwrap()
        .unwrap();
    assert_eq!(author_page.total, 2);
    assert_eq!(author_page.entries.len(), 1);
    assert_eq!(author_page.entries[0].item_id, b1.id);
    let cursor = ReadingCursor::decode(author_page.next_cursor.as_deref().unwrap()).unwrap();
    let author_page_2 = repo
        .list(
            alice,
            book_lib,
            &ReadingFilters {
                author: Some("岑砚禾"),
                limit: 1,
                cursor: Some(cursor),
                sort: ReadingSort::Title,
                dir: Some(ReadingSortDir::Asc),
                ..Default::default()
            },
        )
        .await
        .unwrap()
        .unwrap();
    assert_eq!(author_page_2.entries[0].item_id, b2.id);
    assert!(author_page_2.next_cursor.is_none());

    // Favorite via personal() works on books & comics!
    h.db.personal()
        .update(alice, b1.id, MediaChange::Favorite(true))
        .await
        .unwrap();
    h.db.personal()
        .update(alice, c1.id, MediaChange::Favorite(true))
        .await
        .unwrap();

    let fav_books = repo
        .list(
            alice,
            book_lib,
            &ReadingFilters {
                favorite: Some(true),
                limit: 20,
                ..Default::default()
            },
        )
        .await
        .unwrap()
        .unwrap();
    assert_eq!(fav_books.total, 1);
    assert_eq!(fav_books.entries[0].item_id, b1.id);
    assert!(fav_books.entries[0].state.favorite);

    // Save reading progress with locator (and Bob stays isolated)
    let state1 = repo
        .save_progress(
            alice,
            b1.id,
            ReadingProgressUpdate {
                page: 12,
                total_pages: Some(36),
                locator: Some("chapter:12#p4".into()),
                completed: false,
            },
        )
        .await
        .unwrap();
    assert_eq!(state1.page, 12);
    assert_eq!(state1.locator.as_deref(), Some("chapter:12#p4"));
    assert!(state1.favorite);
    assert!(!state1.completed);

    // Comic completed for c1, in-progress for c2
    repo.save_progress(
        alice,
        c1.id,
        ReadingProgressUpdate {
            page: 191,
            total_pages: Some(192),
            locator: Some("page:191".into()),
            completed: true,
        },
    )
    .await
    .unwrap();
    repo.save_progress(
        alice,
        c2.id,
        ReadingProgressUpdate {
            page: 35,
            total_pages: Some(198),
            locator: Some("page:35".into()),
            completed: false,
        },
    )
    .await
    .unwrap();

    // Home shelf check
    let home = repo.home(alice, None).await.unwrap().unwrap();
    assert_eq!(home.continue_reading.len(), 2); // b1 and c2 (c1 is completed)
    assert_eq!(home.recent_books.len(), 3);
    assert_eq!(home.recent_comics.len(), 2);
    assert_eq!(home.series.len(), 2);
    let dm_series = home
        .series
        .iter()
        .find(|s| s.series_name == "云梯集市")
        .unwrap();
    assert_eq!(dm_series.item_count, 2);
    assert_eq!(dm_series.completed_count, 1);

    // Bookmarks create / manifest / user isolation / delete
    let bm1 = repo
        .create_bookmark(
            alice,
            b1.id,
            CreateBookmarkRequest {
                page: 5,
                locator: Some("chapter:5".into()),
                label: "守塔人登上信号塔".into(),
                note: Some("关键转折".into()),
            },
        )
        .await
        .unwrap();
    let bm2 = repo
        .create_bookmark(
            alice,
            b1.id,
            CreateBookmarkRequest {
                page: 18,
                locator: Some("chapter:18".into()),
                label: "灯塔谜题".into(),
                note: None,
            },
        )
        .await
        .unwrap();

    let manifest = repo
        .manifest(
            alice,
            b1.id,
            vec![ReadingChapterSummary {
                index: 0,
                title: "第一章 夜灯初亮".into(),
            }],
        )
        .await
        .unwrap()
        .unwrap();
    assert_eq!(manifest.reader_mode, ReadingReaderMode::Chapters);
    assert_eq!(manifest.entry.state.page, 12);
    assert_eq!(
        manifest.entry.state.locator.as_deref(),
        Some("chapter:12#p4")
    );
    // Internal locator row never leaks into bookmarks!
    assert_eq!(manifest.bookmarks.len(), 2);
    assert_eq!(manifest.bookmarks[0].id, bm1.id);
    assert_eq!(manifest.bookmarks[1].id, bm2.id);

    // Bob sees zero progress and zero bookmarks, and cannot delete Alice's bookmark
    let bob_manifest = repo
        .manifest(bob, b1.id, Vec::new())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(bob_manifest.entry.state.page, 0);
    assert!(bob_manifest.bookmarks.is_empty());
    assert!(!repo.delete_bookmark(bob, bm1.id).await.unwrap());
    assert!(repo.delete_bookmark(alice, bm1.id).await.unwrap());
    assert_eq!(repo.bookmarks(alice, b1.id).await.unwrap().len(), 1);

    // OPDS shelves expose both Book and Comic libraries
    let shelves = repo.opds_shelves(None).await.unwrap();
    assert_eq!(shelves.len(), 2);
    let opds_comics = repo.opds_catalog(comic_lib, 10, None).await.unwrap();
    assert_eq!(opds_comics.items.len(), 2);
    assert_eq!(opds_comics.items[0].authors, vec!["谷芦"]);
    assert_eq!(opds_comics.items[0].page_count, Some(192));

    h.finish().await;
}

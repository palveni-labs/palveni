use palveni_core::collections::MediaCollectionEntryKind;
use palveni_core::music::{AudioProperties, TrackTags};
use palveni_core::reading::ReadingKind;
use palveni_core::search::GlobalSearchKind;
use palveni_core::shares::{ShareMediaKind, ShareTargetKind};
use palveni_core::{ItemId, ItemKind, LibraryKind, Role};

use super::Harness;
use crate::collections::{AddCollectionItemInput, CollectionPatch};
use crate::items::ItemUpsert;
use crate::libraries::NewLibrary;
use crate::photos::PhotoMetadataUpsert;
use crate::reading::ReadingMetadataUpsert;
use crate::shares::{NewShareLink, RevokeOutcome, ShareUnavailableReason};
use crate::video::VideoMetadata;

fn make_item(kind: ItemKind, title: &str, path: &str, music: Option<TrackTags>) -> ItemUpsert {
    ItemUpsert {
        music,
        id: ItemId::generate(),
        parent_id: None,
        kind,
        title: title.to_owned(),
        sort_title: title.to_lowercase(),
        search_text: title.to_lowercase(),
        path: path.to_owned(),
        size_bytes: Some(1_024_000),
        modified_at: Some(1_760_000_000_000),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn conformance_shares_collections_related_and_global_search() {
    let h = Harness::new().await;
    let alice =
        h.db.users()
            .create("alice", "pw", Role::User)
            .await
            .unwrap()
            .id;
    let admin =
        h.db.users()
            .create("admin", "pw", Role::Admin)
            .await
            .unwrap()
            .id;

    let video_lib =
        h.db.libraries()
            .create(NewLibrary {
                name: "电影库".into(),
                kind: LibraryKind::Video,
                paths: vec!["/media/movies".into()],
            })
            .await
            .unwrap();
    let music_lib =
        h.db.libraries()
            .create(NewLibrary {
                name: "原声音乐".into(),
                kind: LibraryKind::Music,
                paths: vec!["/media/music".into()],
            })
            .await
            .unwrap();
    let book_lib =
        h.db.libraries()
            .create(NewLibrary {
                name: "电子书架".into(),
                kind: LibraryKind::Book,
                paths: vec!["/media/books".into()],
            })
            .await
            .unwrap();
    let photo_lib =
        h.db.libraries()
            .create(NewLibrary {
                name: "家庭相册".into(),
                kind: LibraryKind::Photo,
                paths: vec!["/media/photos".into()],
            })
            .await
            .unwrap();

    // Seed Video Work ("铃芽之旅")
    let movie_item = make_item(
        ItemKind::Video,
        "铃芽之旅",
        "/media/movies/Suzume/Suzume.mkv",
        None,
    );
    let movie_id = movie_item.id;
    h.db.items()
        .apply_scan_batch(video_lib.id, &[movie_item], &[])
        .await
        .unwrap();
    h.db.video()
        .save_batch(
            video_lib.id,
            vec![VideoMetadata {
                item_id: movie_id,
                stamp: "vid-v1".into(),
                work_group: "movie:/media/movies/Suzume".into(),
                work_title: "铃芽之旅".into(),
                year: Some(2022),
                work_overview: Some("门的那一边，存在着所有的时间。".into()),
                work_poster: None,
                work_original_title: Some("すずめの戸締まり".into()),
                work_tagline: None,
                work_rating: Some(82),
                work_official_rating: None,
                work_premiered: Some("2022-11-11".into()),
                work_studios: vec![],
                work_backdrop: None,
                season: None,
                season_poster: None,
                episode: None,
                title: "铃芽之旅".into(),
                overview: None,
                episode_poster: None,
                runtime_ms: Some(7_320_000),
                rating: None,
                premiered: None,
                thumb: None,
                genres: vec!["动画".into()],
                people: vec![],
            }],
        )
        .await
        .unwrap();

    // Seed OST Album ("铃芽之旅 原声带")
    let ost_track = make_item(
        ItemKind::Track,
        "すずめ",
        "/media/music/RADWIMPS/Suzume OST/01.flac",
        Some(TrackTags {
            title: Some("すずめ".into()),
            artist: Some("RADWIMPS".into()),
            album_artist: Some("RADWIMPS".into()),
            album: Some("铃芽之旅 原声带".into()),
            disc_number: Some(1),
            track_number: Some(1),
            duration_ms: Some(238_000),
            year: Some(2022),
            genres: vec!["Soundtrack".into()],
            audio: AudioProperties {
                codec: Some("flac".into()),
                bitrate_kbps: Some(980),
                sample_rate_hz: Some(48_000),
                ..Default::default()
            },
            ..Default::default()
        }),
    );
    let ost_track_id = ost_track.id;
    h.db.items()
        .apply_scan_batch(music_lib.id, &[ost_track], &[])
        .await
        .unwrap();

    // Seed Novel ("铃芽之旅（官方小说）")
    let novel_item = make_item(
        ItemKind::Book,
        "铃芽之旅（官方小说）",
        "/media/books/Suzume.epub",
        None,
    );
    let novel_id = novel_item.id;
    h.db.items()
        .apply_scan_batch(book_lib.id, &[novel_item], &[])
        .await
        .unwrap();
    h.db.reading()
        .upsert_batch(&[ReadingMetadataUpsert {
            item_id: novel_id,
            library_id: book_lib.id,
            stamp: "rd-v1".into(),
            kind: ReadingKind::Book,
            format: "epub".into(),
            title: "铃芽之旅（官方小说）".into(),
            sort_title: "铃芽之旅".into(),
            authors: vec!["新海诚".into()],
            series_name: None,
            series_key: None,
            series_index: None,
            summary: Some("同名动画长篇原著小说".into()),
            publisher: None,
            language: Some("zh".into()),
            page_count: Some(320),
            has_cover: true,
            tags: vec![],
        }])
        .await
        .unwrap();

    // Seed Photo with GPS coordinates + Photo Album
    let photo_item = make_item(
        ItemKind::Photo,
        "九州圣地巡礼",
        "/media/photos/kyushu_01.jpg",
        None,
    );
    let photo_id = photo_item.id;
    h.db.items()
        .apply_scan_batch(photo_lib.id, &[photo_item], &[])
        .await
        .unwrap();
    h.db.photos()
        .upsert_batch(
            photo_lib.id,
            &[PhotoMetadataUpsert {
                item_id: photo_id,
                stamp: "ph-v1".into(),
                taken_at: 1_760_000_000_000,
                day_bucket: "2025-10-09".into(),
                month_bucket: "2025-10".into(),
                width: Some(4032),
                height: Some(3024),
                orientation: 1,
                camera_make: Some("SONY".into()),
                camera_model: Some("ILCE-7C".into()),
                lens_model: None,
                iso: Some(100),
                f_number: Some(2.8),
                exposure_time: Some("1/500s".into()),
                focal_length: Some(35.0),
                latitude: Some(33.2794),
                longitude: Some(131.5006),
                content_hash: Some("d".repeat(64)),
            }],
        )
        .await
        .unwrap();
    let photo_album =
        h.db.photos()
            .create_album(alice, "巡礼相册", None, None)
            .await
            .unwrap();
    h.db.photos()
        .add_album_items(alice, photo_album.id, &[photo_id])
        .await
        .unwrap();

    // Initialize smart home snapshot so global search also hits devices/scenes
    let _home = h.db.home().snapshot().await.unwrap();

    // 1. Cross-Media Related Discovery (`related_across_media`)
    let related =
        h.db.collections()
            .related_across_media("铃芽之旅", Some(MediaCollectionEntryKind::Video), 10)
            .await
            .unwrap();
    assert_eq!(related.len(), 2);
    assert!(
        related
            .iter()
            .any(|w| w.entry_kind == MediaCollectionEntryKind::MusicAlbum
                && w.title == "铃芽之旅 原声带")
    );
    assert!(related.iter().any(
        |w| w.entry_kind == MediaCollectionEntryKind::Book && w.title == "铃芽之旅（官方小说）"
    ));

    // 2. Cross-Media Collections CRUD + autoRelated
    let coll =
        h.db.collections()
            .create(
                alice,
                "铃芽之旅企划",
                Some("电影 + 原声 + 小说 + 巡礼照片"),
                None,
            )
            .await
            .unwrap();
    let e_photo =
        h.db.collections()
            .add_item(
                alice,
                coll.id,
                false,
                AddCollectionItemInput {
                    entry_kind: MediaCollectionEntryKind::Photo,
                    target_id: &photo_id.to_string(),
                    library_id: Some(photo_lib.id),
                    title: None,
                    subtitle: None,
                    artwork_id: None,
                },
            )
            .await
            .unwrap();
    let e_track =
        h.db.collections()
            .add_item(
                alice,
                coll.id,
                false,
                AddCollectionItemInput {
                    entry_kind: MediaCollectionEntryKind::Track,
                    target_id: &ost_track_id.to_string(),
                    library_id: Some(music_lib.id),
                    title: None,
                    subtitle: None,
                    artwork_id: None,
                },
            )
            .await
            .unwrap();

    let detail =
        h.db.collections()
            .get_detail(alice, coll.id, false)
            .await
            .unwrap()
            .unwrap();
    assert_eq!(detail.items.len(), 2);
    // `铃芽之旅企划` / `铃芽之旅` auto-discovers the film, OST album, and novel!
    let e_book =
        h.db.collections()
            .add_item(
                alice,
                coll.id,
                false,
                AddCollectionItemInput {
                    entry_kind: MediaCollectionEntryKind::Book,
                    target_id: &novel_id.to_string(),
                    library_id: Some(book_lib.id),
                    title: None,
                    subtitle: None,
                    artwork_id: None,
                },
            )
            .await
            .unwrap();
    let detail2 =
        h.db.collections()
            .get_detail(alice, coll.id, false)
            .await
            .unwrap()
            .unwrap();
    assert_eq!(detail2.items.len(), 3);
    assert!(
        detail2
            .auto_related
            .iter()
            .any(|w| w.entry_kind == MediaCollectionEntryKind::Video && w.title == "铃芽之旅")
    );

    // Reorder items via `update`
    let updated_coll =
        h.db.collections()
            .update(
                alice,
                coll.id,
                false,
                CollectionPatch {
                    title: Some("铃芽之旅全集"),
                    description: None,
                    cover_artwork_id: None,
                    item_order: Some(&[e_book.id, e_track.id, e_photo.id]),
                },
            )
            .await
            .unwrap();
    assert_eq!(updated_coll.title, "铃芽之旅全集");

    // 3. Share Links: GPS stripping, max_views, revocation & permission checks
    let stripped_share =
        h.db.shares()
            .create(NewShareLink {
                token: "tok_stripped_1234567890ab".into(),
                creator_id: alice,
                target_kind: ShareTargetKind::PhotoAlbum,
                target_id: photo_album.id.to_string(),
                library_id: None,
                title: None,
                password_hash: None,
                allow_download: false,
                strip_gps: true,
                expires_at: None,
                max_views: Some(1),
            })
            .await
            .unwrap();
    assert_eq!(stripped_share.title, "巡礼相册");
    assert!(stripped_share.strip_gps);

    let rec =
        h.db.shares()
            .find_by_token("tok_stripped_1234567890ab")
            .await
            .unwrap()
            .unwrap();
    let resolved = h.db.shares().resolve_items(&rec).await.unwrap();
    assert_eq!(resolved.len(), 1);
    assert_eq!(resolved[0].media_kind, ShareMediaKind::Photo);
    assert_eq!(resolved[0].latitude, None);
    assert_eq!(resolved[0].longitude, None);

    // Atomic max_views = 1 enforcement
    let now = 1_760_100_000_000;
    assert!(
        h.db.shares()
            .consume_view(stripped_share.id, now)
            .await
            .unwrap()
    );
    assert!(
        !h.db
            .shares()
            .consume_view(stripped_share.id, now)
            .await
            .unwrap()
    );
    let after_one =
        h.db.shares()
            .find_by_token("tok_stripped_1234567890ab")
            .await
            .unwrap()
            .unwrap();
    assert_eq!(
        after_one.check_availability(now, true),
        Err(ShareUnavailableReason::MaxViewsReached)
    );
    // Media streaming inside the already-opened session still passes `enforce_max_views = false`
    assert_eq!(after_one.check_availability(now, false), Ok(()));

    // Share with `strip_gps = false` preserves coordinates
    let raw_gps_share =
        h.db.shares()
            .create(NewShareLink {
                token: "tok_with_gps_1234567890ab".into(),
                creator_id: alice,
                target_kind: ShareTargetKind::PhotoAlbum,
                target_id: photo_album.id.to_string(),
                library_id: None,
                title: Some("含定位相册".into()),
                password_hash: Some("$argon2id$dummy".into()),
                allow_download: true,
                strip_gps: false,
                expires_at: None,
                max_views: None,
            })
            .await
            .unwrap();
    let raw_rec =
        h.db.shares()
            .find_by_token("tok_with_gps_1234567890ab")
            .await
            .unwrap()
            .unwrap();
    let raw_items = h.db.shares().resolve_items(&raw_rec).await.unwrap();
    assert_eq!(raw_items[0].latitude, Some(33.2794));
    assert_eq!(raw_items[0].longitude, Some(131.5006));

    // Admin can revoke Alice's share
    assert_eq!(
        h.db.shares()
            .revoke(raw_gps_share.id, admin, true)
            .await
            .unwrap(),
        RevokeOutcome::Revoked
    );

    // 4. Global Cross-Domain Search (`search_all`)
    let hits =
        h.db.global_search()
            .search_all(alice, "铃芽", 20)
            .await
            .unwrap();
    assert!(hits.iter().any(|h| h.kind == GlobalSearchKind::VideoWork));
    assert!(hits.iter().any(|h| h.kind == GlobalSearchKind::MusicAlbum));
    assert!(hits.iter().any(|h| h.kind == GlobalSearchKind::Book));
    assert!(hits.iter().any(|h| h.kind == GlobalSearchKind::Collection));

    let photo_hits =
        h.db.global_search()
            .search_all(alice, "ILCE-7C", 10)
            .await
            .unwrap();
    assert!(photo_hits.iter().any(|h| h.kind == GlobalSearchKind::Photo));

    h.finish().await;
}

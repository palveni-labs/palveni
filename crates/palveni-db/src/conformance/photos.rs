use chrono::TimeZone;
use palveni_core::{ItemId, ItemKind, LibraryKind, Role};

use super::Harness;
use crate::DbError;
use crate::items::ItemUpsert;
use crate::libraries::NewLibrary;
use crate::personal::MediaChange;
use crate::photos::{
    PhotoCursor, PhotoMetadataUpsert, PhotoTimelineQuery, parse_place_key, timeline_filter_hash,
};

fn make_photo_item(title: &str, path: &str, size: i64, modified: i64) -> ItemUpsert {
    ItemUpsert {
        music: None,
        id: ItemId::generate(),
        parent_id: None,
        kind: ItemKind::Photo,
        title: title.to_owned(),
        sort_title: title.to_lowercase(),
        search_text: title.to_lowercase(),
        path: path.to_owned(),
        size_bytes: Some(size),
        modified_at: Some(modified),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn conformance_photos_timeline_memories_info_and_albums() {
    let h = Harness::new().await;
    let alice =
        h.db.users()
            .create("alice", "pw", Role::User)
            .await
            .unwrap()
            .id;
    let bob =
        h.db.users()
            .create("bob", "pw", Role::Admin)
            .await
            .unwrap()
            .id;

    let lib =
        h.db.libraries()
            .create(NewLibrary {
                name: "Family Photos".into(),
                kind: LibraryKind::Photo,
                paths: vec!["/photos".into()],
            })
            .await
            .unwrap();

    let p1 = make_photo_item(
        "IMG_0001",
        "/photos/2025/IMG_0001.JPG",
        4_200_000,
        1_700_000_000_000,
    );
    let p2 = make_photo_item(
        "IMG_0002",
        "/photos/2025/IMG_0002.JPG",
        3_800_000,
        1_700_100_000_000,
    );
    let p3 = make_photo_item(
        "DSC_9001",
        "/photos/2026/DSC_9001.JPG",
        8_100_000,
        1_750_000_000_000,
    );
    let (id1, id2, id3) = (p1.id, p2.id, p3.id);

    h.db.items()
        .apply_scan_batch(lib.id, &[p1, p2, p3], &[])
        .await
        .unwrap();

    let candidates = h.db.photos().scan_index(lib.id).await.unwrap();
    assert_eq!(candidates.len(), 3);
    assert!(candidates.iter().all(|c| c.stamp.is_none()));

    let t1 = chrono::Utc
        .with_ymd_and_hms(2025, 11, 23, 9, 15, 0)
        .single()
        .unwrap()
        .timestamp_millis();
    let t2 = chrono::Utc
        .with_ymd_and_hms(2025, 11, 23, 18, 30, 0)
        .single()
        .unwrap()
        .timestamp_millis();
    let t3 = chrono::Utc
        .with_ymd_and_hms(2026, 6, 12, 14, 0, 0)
        .single()
        .unwrap()
        .timestamp_millis();

    h.db.photos()
        .upsert_batch(
            lib.id,
            &[
                PhotoMetadataUpsert {
                    item_id: id1,
                    stamp: "photo-v1:4200000:1700000000000".into(),
                    taken_at: t1,
                    day_bucket: "2025-11-23".into(),
                    month_bucket: "2025-11".into(),
                    width: Some(4032),
                    height: Some(3024),
                    orientation: 1,
                    camera_make: Some("Apple".into()),
                    camera_model: Some("iPhone 15 Pro".into()),
                    lens_model: Some("24mm f/1.78".into()),
                    iso: Some(100),
                    f_number: Some(1.78),
                    exposure_time: Some("1/500s".into()),
                    focal_length: Some(6.765),
                    latitude: Some(31.2304),
                    longitude: Some(121.4737),
                    content_hash: Some("a".repeat(64)),
                },
                PhotoMetadataUpsert {
                    item_id: id2,
                    stamp: "photo-v1:3800000:1700100000000".into(),
                    taken_at: t2,
                    day_bucket: "2025-11-23".into(),
                    month_bucket: "2025-11".into(),
                    width: Some(4032),
                    height: Some(3024),
                    orientation: 6,
                    camera_make: Some("Apple".into()),
                    camera_model: Some("iPhone 15 Pro".into()),
                    lens_model: None,
                    iso: Some(400),
                    f_number: Some(1.78),
                    exposure_time: Some("1/60s".into()),
                    focal_length: Some(6.765),
                    latitude: Some(31.2412),
                    longitude: Some(121.4891),
                    content_hash: Some("b".repeat(64)),
                },
                PhotoMetadataUpsert {
                    item_id: id3,
                    stamp: "photo-v1:8100000:1750000000000".into(),
                    taken_at: t3,
                    day_bucket: "2026-06-12".into(),
                    month_bucket: "2026-06".into(),
                    width: Some(7008),
                    height: Some(4672),
                    orientation: 1,
                    camera_make: Some("SONY".into()),
                    camera_model: Some("ILCE-7M4".into()),
                    lens_model: Some("FE 35mm F1.4 GM".into()),
                    iso: Some(200),
                    f_number: Some(2.0),
                    exposure_time: Some("1/1000s".into()),
                    focal_length: Some(35.0),
                    latitude: None,
                    longitude: None,
                    content_hash: Some("c".repeat(64)),
                },
            ],
        )
        .await
        .unwrap();

    // Mark id2 as Alice's favorite via personal()
    h.db.personal()
        .update(alice, id2, MediaChange::Favorite(true))
        .await
        .unwrap();

    // Cursor pagination in chronological DESC order
    let q1 = PhotoTimelineQuery {
        limit: 2,
        ..Default::default()
    };
    let page1 = h.db.photos().timeline(alice, &q1).await.unwrap();
    assert_eq!(page1.total, 3);
    assert_eq!(page1.months.len(), 2);
    assert_eq!(page1.months[0].month, "2026-06");
    assert_eq!(page1.months[0].count, 1);
    assert_eq!(page1.months[1].month, "2025-11");
    assert_eq!(page1.months[1].count, 2);
    assert_eq!(page1.items.len(), 2);
    assert_eq!(page1.items[0].id, id3);
    assert!(!page1.items[0].favorite);
    assert_eq!(page1.items[1].id, id2);
    assert!(page1.items[1].favorite);

    let cursor_str = page1.next_cursor.as_deref().unwrap();
    let fhash = timeline_filter_hash(&q1);
    let cursor = PhotoCursor::decode(cursor_str, &fhash).unwrap();
    let page2 =
        h.db.photos()
            .timeline(
                alice,
                &PhotoTimelineQuery {
                    limit: 2,
                    cursor: Some(cursor),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
    assert_eq!(page2.items.len(), 1);
    assert_eq!(page2.items[0].id, id1);
    assert!(page2.next_cursor.is_none());

    // Filter by camera, favorite, hasLocation, month
    let sony =
        h.db.photos()
            .timeline(
                alice,
                &PhotoTimelineQuery {
                    camera: Some("SONY ILCE-7M4"),
                    limit: 10,
                    ..Default::default()
                },
            )
            .await
            .unwrap();
    assert_eq!(sony.items.len(), 1);
    assert_eq!(sony.items[0].id, id3);

    let favs =
        h.db.photos()
            .timeline(
                alice,
                &PhotoTimelineQuery {
                    favorite: Some(true),
                    limit: 10,
                    ..Default::default()
                },
            )
            .await
            .unwrap();
    assert_eq!(favs.items.len(), 1);
    assert_eq!(favs.items[0].id, id2);
    assert_eq!(
        h.db.photos()
            .timeline(
                bob,
                &PhotoTimelineQuery {
                    favorite: Some(true),
                    limit: 10,
                    ..Default::default()
                },
            )
            .await
            .unwrap()
            .items
            .len(),
        0
    );

    // Memories: onThisDay (Nov 23 in 2026 matches 2025-11-23!), places (~0.1 deg cell merges id1 & id2!), cameras
    let now = chrono::Utc
        .with_ymd_and_hms(2026, 11, 23, 12, 0, 0)
        .single()
        .unwrap();
    let mem = h.db.photos().memories(alice, now).await.unwrap();
    assert_eq!(mem.total_photos, 3);
    assert_eq!(mem.geotagged_count, 2);
    assert_eq!(mem.on_this_day.len(), 2);
    assert_eq!(mem.places.len(), 1);
    assert_eq!(mem.places[0].count, 2);
    assert_eq!(mem.places[0].cover_item_id, id2);
    assert_eq!(mem.cameras.len(), 2);
    assert_eq!(mem.cameras[0].name, "Apple iPhone 15 Pro");
    assert_eq!(mem.cameras[0].count, 2);

    // Filter timeline by place cluster key
    let place_cell = parse_place_key(&mem.places[0].key).unwrap();
    let by_place =
        h.db.photos()
            .timeline(
                alice,
                &PhotoTimelineQuery {
                    place: Some(place_cell),
                    limit: 10,
                    ..Default::default()
                },
            )
            .await
            .unwrap();
    assert_eq!(by_place.items.len(), 2);

    // Albums CRUD + user isolation
    let album =
        h.db.photos()
            .create_album(alice, "Shanghai Trip", Some("Autumn walk"), None)
            .await
            .unwrap();
    assert_eq!(album.item_count, 0);
    assert!(
        h.db.photos()
            .get_album(bob, album.id)
            .await
            .unwrap()
            .is_none()
    );

    let album =
        h.db.photos()
            .add_album_items(alice, album.id, &[id1, id2, id1])
            .await
            .unwrap();
    assert_eq!(album.item_count, 2);
    assert_eq!(album.cover_item_id, Some(id1));
    assert_eq!(album.preview_item_ids, vec![id1, id2]);

    // Photo info includes user's favorite and albums, and masks host path for non-admin
    let info_alice =
        h.db.photos()
            .photo_info(alice, id2, false)
            .await
            .unwrap()
            .unwrap();
    assert!(info_alice.favorite);
    assert_eq!(info_alice.path, "2025/IMG_0002.JPG");
    assert_eq!(info_alice.albums.len(), 1);
    assert_eq!(info_alice.albums[0].title, "Shanghai Trip");

    let info_bob =
        h.db.photos()
            .photo_info(bob, id2, true)
            .await
            .unwrap()
            .unwrap();
    assert!(!info_bob.favorite);
    assert_eq!(info_bob.path, "/photos/2025/IMG_0002.JPG");
    assert!(info_bob.albums.is_empty());

    // Update & remove album items + delete album
    let renamed =
        h.db.photos()
            .update_album(alice, album.id, Some("Shanghai 2025"), Some(""), Some(id2))
            .await
            .unwrap();
    assert_eq!(renamed.title, "Shanghai 2025");
    assert_eq!(renamed.description, None);
    assert_eq!(renamed.cover_item_id, Some(id2));

    let after_rm =
        h.db.photos()
            .remove_album_items(alice, album.id, &[id2])
            .await
            .unwrap();
    assert_eq!(after_rm.item_count, 1);
    assert_eq!(after_rm.cover_item_id, Some(id1));

    assert!(matches!(
        h.db.photos().delete_album(bob, album.id).await,
        Ok(false)
    ));
    assert!(h.db.photos().delete_album(alice, album.id).await.unwrap());
    assert!(matches!(
        h.db.photos()
            .timeline(
                alice,
                &PhotoTimelineQuery {
                    album_id: Some(album.id),
                    limit: 10,
                    ..Default::default()
                }
            )
            .await,
        Err(DbError::NotFound("photo album"))
    ));

    h.finish().await;
}

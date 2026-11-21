use super::*;
use crate::convert::sha256_hex;
use crate::video::{VideoCursor, VideoMetadata};
use palveni_core::video::VideoOrder;

#[tokio::test]
async fn conformance_video_poster_order_and_cursor_boundaries() {
    let h = Harness::new().await;
    let lib =
        h.db.libraries()
            .create(NewLibrary {
                name: "Wall".into(),
                kind: LibraryKind::Video,
                paths: vec!["/video".into()],
            })
            .await
            .unwrap()
            .id;
    let mut items = Vec::new();
    let mut data = Vec::new();
    for (name, group, title, poster, season) in [
        ("plain", "early", "Amber", false, None),
        ("first episode", "series", "Winter", false, Some(1)),
        ("second episode", "series", "Winter", true, Some(2)),
        (
            "older series title",
            "series",
            "Winter archive",
            false,
            Some(3),
        ),
        ("twin cover one", "cover1", "Twins", true, None),
        ("twin cover two", "cover2", "Twins", true, None),
        ("twin plain one", "plain1", "Twins", false, None),
        ("twin plain two", "plain2", "Twins", false, None),
        ("last", "last", "Zebra", true, None),
    ] {
        let item = ItemUpsert {
            kind: ItemKind::Video,
            ..track(name, &format!("/video/{name}.mp4"), None, 10)
        };
        data.push(VideoMetadata {
            item_id: item.id,
            stamp: "fixture".into(),
            work_group: group.into(),
            work_title: title.into(),
            year: None,
            work_overview: None,
            work_poster: poster.then(|| "/video/poster.jpg".into()),
            season,
            season_poster: None,
            episode: season.map(|_| 1),
            title: name.into(),
            overview: None,
            episode_poster: None,
        });
        items.push(item);
    }
    h.db.items()
        .apply_scan_batch(lib, &items, &[])
        .await
        .unwrap();
    let repo = h.db.video();
    repo.save_batch(lib, data).await.unwrap();

    let mut covered_twins = ["cover1", "cover2"].map(sha256_hex);
    covered_twins.sort();
    let mut plain_twins = ["plain1", "plain2"].map(sha256_hex);
    plain_twins.sort();
    let series = sha256_hex("series");
    let expected = covered_twins
        .into_iter()
        .chain([series.clone(), sha256_hex("last"), sha256_hex("early")])
        .chain(plain_twins)
        .collect::<Vec<_>>();

    // A legacy cursor contains exactly scope, sort and id, with no order field.
    let legacy = hex::encode(
        serde_json::to_vec(&serde_json::json!({
            "scope": format!("{lib}//None"), "sort": hex::encode("amber"), "id": sha256_hex("early")
        }))
        .unwrap(),
    );
    let first_title = repo.browse(lib, None, None, 1, None).await.unwrap();
    assert_eq!(first_title.entries[0].title, "Amber");
    let wire: serde_json::Value =
        serde_json::from_slice(&hex::decode(first_title.next_cursor.unwrap()).unwrap()).unwrap();
    assert_eq!(wire.as_object().unwrap().len(), 3);
    let old = VideoCursor::decode_for_order(&legacy, lib, None, None, VideoOrder::Title).unwrap();
    assert_eq!(
        repo.browse(lib, None, None, 1, Some(old))
            .await
            .unwrap()
            .entries[0]
            .title,
        "Twins"
    );
    assert!(VideoCursor::decode_for_order(&legacy, lib, None, None, VideoOrder::Poster).is_none());

    for limit in [1, 3, 4] {
        let mut cursor = None;
        let mut ids = Vec::new();
        loop {
            let page = repo
                .browse_ordered(lib, None, None, VideoOrder::Poster, limit, cursor)
                .await
                .unwrap();
            assert!(!page.entries.is_empty());
            for entry in page.entries {
                assert_eq!(entry.poster_item_id.is_some(), ids.len() < 4);
                if entry.id == series {
                    assert_eq!(
                        (entry.file_count, entry.poster_item_id),
                        (3, Some(items[2].id))
                    );
                }
                ids.push(entry.id);
            }
            assert!(ids.len() <= expected.len());
            let Some(raw) = page.next_cursor else { break };
            assert!(VideoCursor::decode(&raw, lib, None, None).is_none());
            cursor = Some(
                VideoCursor::decode_for_order(&raw, lib, None, None, VideoOrder::Poster).unwrap(),
            );
        }
        assert_eq!(ids, expected);
    }
    for season in [None, Some(1)] {
        assert!(matches!(
            repo.browse_ordered(lib, Some(&series), season, VideoOrder::Poster, 1, None)
                .await,
            Err(DbError::Invalid(_))
        ));
    }
    h.db.items()
        .apply_scan_batch(lib, &[], &[items[2].id])
        .await
        .unwrap();
    let after_removal = repo
        .browse_ordered(lib, None, None, VideoOrder::Poster, 20, None)
        .await
        .unwrap();
    let show = after_removal
        .entries
        .iter()
        .find(|e| e.id == series)
        .unwrap();
    assert_eq!((show.file_count, show.poster_item_id), (2, None));
    assert_eq!(
        after_removal
            .entries
            .iter()
            .take_while(|e| e.poster_item_id.is_some())
            .count(),
        3
    );
    h.finish().await;
}

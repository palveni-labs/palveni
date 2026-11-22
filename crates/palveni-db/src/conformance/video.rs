use super::*;
use crate::convert::sha256_hex;
use crate::personal::MediaChange;
use crate::video::{StartsWith, VideoCursor, VideoFilters, VideoMetadata, VideoPersonInput};
use palveni_core::personal::PlaybackUpdate;
use palveni_core::video::{SortDir, VideoKind, VideoOrder, VideoPersonKind, WorkKind};

fn minimal_meta(item_id: ItemId, group: &str, title: &str) -> VideoMetadata {
    VideoMetadata {
        item_id,
        stamp: "fixture".into(),
        work_group: group.into(),
        work_title: title.into(),
        year: None,
        work_overview: None,
        work_poster: None,
        work_original_title: None,
        work_tagline: None,
        work_rating: None,
        work_official_rating: None,
        work_premiered: None,
        work_studios: Vec::new(),
        work_backdrop: None,
        season: None,
        season_poster: None,
        episode: None,
        title: title.into(),
        overview: None,
        episode_poster: None,
        runtime_ms: None,
        rating: None,
        premiered: None,
        thumb: None,
        genres: Vec::new(),
        people: Vec::new(),
    }
}

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
            "Winter archive 电影100%_!",
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
            work_poster: poster.then(|| "/video/poster.jpg".into()),
            season,
            episode: season.map(|_| 1),
            title: name.into(),
            ..minimal_meta(item.id, group, title)
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
    assert_eq!(first_title.total, 7);
    assert!(first_title.entries[0].series_id.is_none());
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
            assert_eq!(page.total, expected.len() as u64);
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
    for order in [VideoOrder::Title, VideoOrder::Poster] {
        let filter = VideoFilters {
            order,
            kind: Some(WorkKind::Series),
            title: "ARCHIVE 电影100%_!",
            ..Default::default()
        };
        let page = repo
            .browse_filtered(lib, None, None, filter, 1, None)
            .await
            .unwrap();
        assert_eq!(page.entries.len(), 1);
        assert_eq!(page.total, 1);
        assert_eq!(page.entries[0].id, series);
        assert_eq!(
            (page.entries[0].file_count, page.entries[0].poster_item_id),
            (3, Some(items[2].id))
        );
        assert!(page.next_cursor.is_none());
        for term in ["ARCHIVE 电影100__", "100%%", "%_", "!"] {
            let result = repo
                .browse_filtered(
                    lib,
                    None,
                    None,
                    VideoFilters {
                        title: term,
                        ..filter
                    },
                    10,
                    None,
                )
                .await
                .unwrap();
            assert_eq!(
                result.entries.len(),
                usize::from(["%_", "!"].contains(&term))
            );
        }

        let filter = VideoFilters {
            order,
            kind: Some(WorkKind::Movie),
            title: "WiN",
            ..Default::default()
        };
        let mut seen = Vec::new();
        let mut cursor = None;
        loop {
            let page = repo
                .browse_filtered(lib, None, None, filter, 1, cursor)
                .await
                .unwrap();
            assert_eq!(page.entries.len(), 1);
            assert_eq!(page.total, 4);
            assert_eq!(page.entries[0].title, "Twins");
            assert!(!seen.contains(&page.entries[0].id));
            seen.push(page.entries[0].id.clone());
            let Some(raw) = page.next_cursor else { break };
            assert!(VideoCursor::decode_for_order(&raw, lib, None, None, order).is_none());
            assert!(
                VideoCursor::decode_filtered(
                    &raw,
                    lib,
                    None,
                    None,
                    VideoFilters {
                        kind: None,
                        ..filter
                    }
                )
                .is_none()
            );
            assert!(
                VideoCursor::decode_filtered(
                    &raw,
                    lib,
                    None,
                    None,
                    VideoFilters {
                        title: "Tw",
                        ..filter
                    }
                )
                .is_none()
            );
            cursor = Some(VideoCursor::decode_filtered(&raw, lib, None, None, filter).unwrap());
        }
        assert_eq!(seen.len(), 4);
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

#[tokio::test]
async fn conformance_video_filters_orders_detail_and_home() {
    let h = Harness::new().await;
    let alice =
        h.db.users()
            .create("alice", "hash", Role::Admin)
            .await
            .unwrap()
            .id;
    let bob =
        h.db.users()
            .create("bob", "hash", Role::User)
            .await
            .unwrap()
            .id;
    let lib =
        h.db.libraries()
            .create(NewLibrary {
                name: "Cinema".into(),
                kind: LibraryKind::Video,
                paths: vec!["/cinema".into()],
            })
            .await
            .unwrap()
            .id;

    let mut items = Vec::new();
    let mut metas = Vec::new();

    // 1) Movie "Alpha" (2019, rating 7.8, runtime 118m, Drama + Mystery)
    let m_alpha = ItemUpsert {
        kind: ItemKind::Video,
        ..track("alpha", "/cinema/alpha/alpha.mkv", None, 1_000)
    };
    let alpha_id = m_alpha.id;
    items.push(m_alpha);
    metas.push(VideoMetadata {
        year: Some(2019),
        work_overview: Some("A lighthouse story.".into()),
        work_poster: Some("/cinema/alpha/poster.jpg".into()),
        work_original_title: Some("Le Phare".into()),
        work_tagline: Some("Watch the light.".into()),
        work_rating: Some(78),
        work_official_rating: Some("PG-13".into()),
        work_premiered: Some("2019-10-18".into()),
        work_studios: vec!["North Pier".into()],
        work_backdrop: Some("/cinema/alpha/fanart.jpg".into()),
        runtime_ms: Some(118 * 60_000),
        genres: vec!["Drama".into(), "Mystery".into()],
        people: vec![
            VideoPersonInput {
                kind: VideoPersonKind::Actor,
                name: "Ana Ruiz".into(),
                role: Some("Marie".into()),
                image: Some("/cinema/alpha/.actors/Ana_Ruiz.jpg".into()),
            },
            VideoPersonInput {
                kind: VideoPersonKind::Director,
                name: "Lena Morel".into(),
                role: None,
                image: None,
            },
        ],
        ..minimal_meta(alpha_id, "movie/alpha", "Alpha")
    });

    // 2) Movie "2001 Odyssey" (1968, rating 8.9, runtime 149m, Sci-Fi + Mystery) -> startsWith=#
    let m_2001 = ItemUpsert {
        kind: ItemKind::Video,
        ..track("2001", "/cinema/2001/film.mkv", None, 2_000)
    };
    let odyssey_id = m_2001.id;
    items.push(m_2001);
    metas.push(VideoMetadata {
        year: Some(1968),
        work_rating: Some(89),
        runtime_ms: Some(149 * 60_000),
        genres: vec!["Sci-Fi".into(), "Mystery".into()],
        ..minimal_meta(odyssey_id, "movie/2001", "2001 Odyssey")
    });

    // 3) Movie "流浪地球" (2019, no rating, runtime 125m, Sci-Fi) -> startsWith=#
    let m_cjk = ItemUpsert {
        kind: ItemKind::Video,
        ..track("wandering", "/cinema/cjk/film.mkv", None, 3_000)
    };
    let cjk_id = m_cjk.id;
    items.push(m_cjk);
    metas.push(VideoMetadata {
        year: Some(2019),
        runtime_ms: Some(125 * 60_000),
        genres: vec!["Sci-Fi".into()],
        ..minimal_meta(cjk_id, "movie/cjk", "流浪地球")
    });

    // 4) Series "Bravo" (2022, rating 8.5, Drama + Sci-Fi): S00E01 special, S01E01, S01E02, S02E01
    let mut ep_ids = Vec::new();
    for (season, ep, ep_title, thumb) in [
        (0, 1, "Behind the Scenes", false),
        (1, 1, "Pilot", true),
        (1, 2, "Second Light", false),
        (2, 1, "New Orbit", false),
    ] {
        let item = ItemUpsert {
            kind: ItemKind::Video,
            ..track(
                &format!("bravo-s{season}e{ep}"),
                &format!("/cinema/bravo/s{season}e{ep}.mkv"),
                None,
                500,
            )
        };
        ep_ids.push(item.id);
        metas.push(VideoMetadata {
            year: Some(2022),
            work_overview: Some("Space engineers.".into()),
            work_poster: Some("/cinema/bravo/poster.jpg".into()),
            work_backdrop: Some("/cinema/bravo/fanart.jpg".into()),
            work_rating: Some(85),
            work_official_rating: Some("TV-14".into()),
            work_premiered: Some("2022-04-06".into()),
            work_studios: vec!["Lowpoly".into()],
            season: Some(season),
            season_poster: Some("/cinema/bravo/season01.jpg".into()),
            episode: Some(ep),
            title: ep_title.into(),
            overview: Some(format!("Overview of {ep_title}")),
            runtime_ms: Some(45 * 60_000),
            rating: Some(75),
            premiered: Some("2022-04-13".into()),
            thumb: thumb.then(|| "/cinema/bravo/pilot-thumb.jpg".into()),
            genres: vec!["Drama".into(), "Sci-Fi".into()],
            people: vec![VideoPersonInput {
                kind: VideoPersonKind::Actor,
                name: "Rae Kim".into(),
                role: Some("Captain".into()),
                image: None,
            }],
            ..minimal_meta(item.id, "series/bravo", "Bravo")
        });
        items.push(item);
    }

    h.db.items()
        .apply_scan_batch(lib, &items, &[])
        .await
        .unwrap();
    let repo = h.db.video();
    repo.save_batch(lib, metas).await.unwrap();

    // Genres count distinct works (Bravo has 4 episodes, still counts as 1 work).
    let genres = repo.genres(lib).await.unwrap();
    let counts: Vec<(&str, u64)> = genres
        .genres
        .iter()
        .map(|g| (g.name.as_str(), g.count))
        .collect();
    assert_eq!(counts, [("Drama", 2), ("Mystery", 2), ("Sci-Fi", 3)]);

    // startsWith A, B, # (index range on work_sort).
    let sw_a = repo
        .browse_for(
            Some(alice),
            lib,
            None,
            None,
            VideoFilters {
                starts_with: StartsWith::parse("a"),
                ..Default::default()
            },
            20,
            None,
        )
        .await
        .unwrap();
    assert_eq!(sw_a.total, 1);
    assert_eq!(sw_a.entries[0].title, "Alpha");

    let sw_hash = repo
        .browse_for(
            Some(alice),
            lib,
            None,
            None,
            VideoFilters {
                starts_with: StartsWith::parse("#"),
                ..Default::default()
            },
            20,
            None,
        )
        .await
        .unwrap();
    assert_eq!(sw_hash.total, 2);
    let hash_titles: Vec<&str> = sw_hash.entries.iter().map(|e| e.title.as_str()).collect();
    assert_eq!(hash_titles, ["2001 Odyssey", "流浪地球"]);

    // Year range & genre filter.
    let filtered = repo
        .browse_for(
            Some(alice),
            lib,
            None,
            None,
            VideoFilters {
                genre: Some("drama"),
                year_from: Some(2019),
                year_to: Some(2020),
                ..Default::default()
            },
            20,
            None,
        )
        .await
        .unwrap();
    assert_eq!(filtered.total, 1);
    assert_eq!(filtered.entries[0].title, "Alpha");

    // Rating DESC (nulls last!): 2001 (8.9), Bravo (8.5), Alpha (7.8), 流浪地球 (null).
    let by_rating = repo
        .browse_for(
            Some(alice),
            lib,
            None,
            None,
            VideoFilters {
                order: VideoOrder::Rating,
                ..Default::default()
            },
            20,
            None,
        )
        .await
        .unwrap();
    assert_eq!(
        by_rating
            .entries
            .iter()
            .map(|e| e.title.as_str())
            .collect::<Vec<_>>(),
        ["2001 Odyssey", "Bravo", "Alpha", "流浪地球"]
    );

    // Runtime ASC (nulls last): Bravo (45m), Alpha (118m), 流浪地球 (125m), 2001 (149m).
    let by_runtime = repo
        .browse_for(
            Some(alice),
            lib,
            None,
            None,
            VideoFilters {
                order: VideoOrder::Runtime,
                dir: Some(SortDir::Asc),
                ..Default::default()
            },
            20,
            None,
        )
        .await
        .unwrap();
    assert_eq!(
        by_runtime
            .entries
            .iter()
            .map(|e| e.title.as_str())
            .collect::<Vec<_>>(),
        ["Bravo", "Alpha", "流浪地球", "2001 Odyssey"]
    );

    // Random order with seed pagination stability.
    let rand_filter = VideoFilters {
        order: VideoOrder::Random,
        seed: Some("seed-42"),
        ..Default::default()
    };
    let full_rand = repo
        .browse_for(Some(alice), lib, None, None, rand_filter, 20, None)
        .await
        .unwrap();
    let mut paged_ids = Vec::new();
    let mut cursor = None;
    loop {
        let page = repo
            .browse_for(Some(alice), lib, None, None, rand_filter, 2, cursor)
            .await
            .unwrap();
        assert_eq!(page.total, 4);
        paged_ids.extend(page.entries.into_iter().map(|e| e.id));
        let Some(raw) = page.next_cursor else { break };
        assert!(
            VideoCursor::decode_filtered(
                &raw,
                lib,
                None,
                None,
                VideoFilters {
                    seed: Some("other-seed"),
                    ..rand_filter
                }
            )
            .is_none()
        );
        cursor = Some(VideoCursor::decode_filtered(&raw, lib, None, None, rand_filter).unwrap());
    }
    assert_eq!(
        paged_ids,
        full_rand
            .entries
            .iter()
            .map(|e| e.id.clone())
            .collect::<Vec<_>>()
    );

    // Favorites and played filtering (isolated per user!).
    let alpha_key = sha256_hex("movie/alpha");
    let bravo_key = sha256_hex("series/bravo");
    assert!(
        repo.set_work_favorite(alice, lib, &alpha_key, true)
            .await
            .unwrap()
    );
    assert!(
        repo.set_work_played(alice, lib, &alpha_key, true)
            .await
            .unwrap()
    );
    let fav_alice = repo
        .browse_for(
            Some(alice),
            lib,
            None,
            None,
            VideoFilters {
                favorite: Some(true),
                ..Default::default()
            },
            20,
            None,
        )
        .await
        .unwrap();
    assert_eq!(fav_alice.total, 1);
    assert_eq!(fav_alice.entries[0].id, alpha_key);
    assert!(fav_alice.entries[0].state.as_ref().unwrap().favorite);
    assert!(fav_alice.entries[0].state.as_ref().unwrap().played);

    let fav_bob = repo
        .browse_for(
            Some(bob),
            lib,
            None,
            None,
            VideoFilters {
                favorite: Some(true),
                ..Default::default()
            },
            20,
            None,
        )
        .await
        .unwrap();
    assert_eq!(fav_bob.total, 0);

    let unplayed_alice = repo
        .browse_for(
            Some(alice),
            lib,
            None,
            None,
            VideoFilters {
                played: Some(false),
                ..Default::default()
            },
            20,
            None,
        )
        .await
        .unwrap();
    assert_eq!(unplayed_alice.total, 3);

    // Work detail for movie: studios, people imageUrl, files, similar.
    let alpha_detail = repo
        .work_detail(alice, lib, &alpha_key)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(alpha_detail.work.kind, VideoKind::Movie);
    assert!(alpha_detail.work.series_id.is_none());
    assert_eq!(alpha_detail.studios, ["North Pier"]);
    assert_eq!(alpha_detail.people.len(), 2);
    assert!(
        alpha_detail.people[0]
            .image_url
            .as_deref()
            .unwrap()
            .starts_with(&format!("/api/v1/libraries/{lib}/videos/people/"))
    );
    assert!(alpha_detail.people[1].image_url.is_none());
    assert_eq!(alpha_detail.files.len(), 1);
    assert_eq!(alpha_detail.files[0].item_id, alpha_id);
    assert_eq!(alpha_detail.files[0].size_bytes, Some(1_000));
    assert!(alpha_detail.next_up.is_none());
    assert!(alpha_detail.seasons.is_empty());
    // Similar works: Bravo (Drama) and 2001 (Mystery).
    assert_eq!(alpha_detail.similar.len(), 2);

    // Work detail for series: seasons have series_id/series_title/unplayedCount; nextUp starts at S01E01 (skips S00).
    let bravo_detail = repo
        .work_detail(alice, lib, &bravo_key)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(bravo_detail.work.kind, VideoKind::Series);
    assert_eq!(
        bravo_detail.work.state.as_ref().unwrap().unplayed_count,
        Some(4)
    );
    assert_eq!(bravo_detail.seasons.len(), 3);
    assert_eq!(
        bravo_detail.seasons[1].series_id.as_deref(),
        Some(bravo_key.as_str())
    );
    assert_eq!(
        bravo_detail.seasons[1].series_title.as_deref(),
        Some("Bravo")
    );
    assert_eq!(
        bravo_detail.seasons[1]
            .state
            .as_ref()
            .unwrap()
            .unplayed_count,
        Some(2)
    );
    let first_next = bravo_detail.next_up.as_ref().unwrap();
    assert_eq!(first_next.item_id, Some(ep_ids[1])); // S01E01, skipping S00E01
    assert_eq!(first_next.series_id.as_deref(), Some(bravo_key.as_str()));
    assert_eq!(first_next.series_title.as_deref(), Some("Bravo"));
    assert_eq!(first_next.thumb_item_id, Some(ep_ids[1]));

    // Home before watching episodes: nextUp is empty, latestMovies has 3, latestEpisodes collapses Bravo to 1 with fileCount=4.
    let home0 = repo.home(alice, Some(lib)).await.unwrap().unwrap();
    assert!(home0.continue_watching.is_empty());
    assert!(home0.next_up.is_empty());
    assert_eq!(home0.latest_movies.len(), 3);
    assert_eq!(home0.latest_episodes.len(), 1);
    assert_eq!(home0.latest_episodes[0].file_count, 4);
    assert_eq!(
        home0.latest_episodes[0].series_id.as_deref(),
        Some(bravo_key.as_str())
    );
    assert_eq!(
        home0.latest_episodes[0].series_title.as_deref(),
        Some("Bravo")
    );

    // Mark S01E01 played via MediaChange::Played(true) -> nextUp on home becomes S01E02!
    h.db.personal()
        .update(alice, ep_ids[1], MediaChange::Played(true))
        .await
        .unwrap();
    // Start watching movie 2001 halfway -> appears in continueWatching.
    h.db.personal()
        .update(
            alice,
            odyssey_id,
            MediaChange::Progress(PlaybackUpdate {
                expected_version: 0,
                position_ms: 600_000,
                completed: false,
            }),
        )
        .await
        .unwrap();

    let home1 = repo.home(alice, None).await.unwrap().unwrap();
    assert_eq!(home1.continue_watching.len(), 1);
    assert_eq!(home1.continue_watching[0].item_id, Some(odyssey_id));
    assert_eq!(
        home1.continue_watching[0]
            .state
            .as_ref()
            .unwrap()
            .position_ms,
        600_000
    );
    assert_eq!(home1.next_up.len(), 1);
    assert_eq!(home1.next_up[0].item_id, Some(ep_ids[2])); // S01E02!
    assert_eq!(
        home1.next_up[0].series_id.as_deref(),
        Some(bravo_key.as_str())
    );

    // Start watching S01E02 halfway -> moves to continueWatching and disappears from home.nextUp!
    h.db.personal()
        .update(
            alice,
            ep_ids[2],
            MediaChange::Progress(PlaybackUpdate {
                expected_version: 0,
                position_ms: 120_000,
                completed: false,
            }),
        )
        .await
        .unwrap();
    let home2 = repo.home(alice, Some(lib)).await.unwrap().unwrap();
    assert_eq!(home2.continue_watching.len(), 2);
    assert_eq!(home2.continue_watching[0].item_id, Some(ep_ids[2]));
    assert!(home2.next_up.is_empty());

    // Mark whole series played -> nextUp is None, played filter includes Bravo.
    assert!(
        repo.set_work_played(alice, lib, &bravo_key, true)
            .await
            .unwrap()
    );
    let bravo_done = repo
        .work_detail(alice, lib, &bravo_key)
        .await
        .unwrap()
        .unwrap();
    assert!(bravo_done.next_up.is_none());
    assert!(bravo_done.work.state.as_ref().unwrap().played);
    assert_eq!(
        bravo_done.work.state.as_ref().unwrap().unplayed_count,
        Some(0)
    );

    // Backdrop, thumb, and person image locations resolve cleanly inside root.
    assert!(repo.backdrop_location(alpha_id).await.unwrap().is_some());
    let thumb = repo.thumb_location(ep_ids[1]).await.unwrap().unwrap();
    assert!(thumb.local_thumb.is_some());
    let thumb_no_local = repo.thumb_location(ep_ids[2]).await.unwrap().unwrap();
    assert!(thumb_no_local.local_thumb.is_none());
    assert_eq!(thumb_no_local.runtime_ms, Some(45 * 60_000));
    assert!(
        repo.person_image_location(lib, &crate::video::person_key("Ana Ruiz"))
            .await
            .unwrap()
            .is_some()
    );

    h.finish().await;
}

use super::*;
use crate::music::library::{AlbumListQuery, ArtistListQuery, CatalogCursor, TrackListQuery};
use crate::personal::MediaChange;
use palveni_core::music::{
    AudioProperties, MusicAlbumSort, MusicArtistSort, MusicSortOrder, MusicTrackSort, TrackTags,
};
use palveni_core::personal::PlaybackUpdate;

fn tagged_track(name: &str, path: &str, tags: TrackTags, size: i64) -> ItemUpsert {
    let mut item = track(name, path, None, size);
    item.music = Some(tags);
    item
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn conformance_music_catalog_queries_sorts_filters_and_user_state() {
    let h = Harness::new().await;
    let db = &h.db;
    let lib = new_library(db, "Catalog").await.id;
    let other_lib = new_library(db, "Other").await.id;
    let alice = db
        .users()
        .create("alice", "x", Role::User)
        .await
        .unwrap()
        .id;
    let bob = db.users().create("bob", "x", Role::User).await.unwrap().id;

    // Two distinct artists sharing the exact same album name ("Echoes"),
    // plus a second album for Alpha, a multi-disc album for Beta, and a yearless album for Gamma.
    let alpha_echoes_1 = tagged_track(
        "Dawn",
        "/media/Catalog/alpha/echoes/01.flac",
        TrackTags {
            title: Some("Dawn".into()),
            artist: Some("Alpha feat. Guest".into()),
            album_artist: Some("Alpha".into()),
            album: Some("Echoes".into()),
            disc_number: Some(1),
            track_number: Some(1),
            duration_ms: Some(180_000),
            year: Some(2020),
            genres: vec!["Rock".into(), "Indie".into()],
            audio: AudioProperties {
                codec: Some("flac".into()),
                bitrate_kbps: Some(950),
                sample_rate_hz: Some(44_100),
                bit_depth: Some(16),
                channels: Some(2),
            },
            ..Default::default()
        },
        10_000,
    );
    let alpha_echoes_2 = tagged_track(
        "Dusk",
        "/media/Catalog/alpha/echoes/02.flac",
        TrackTags {
            title: Some("Dusk".into()),
            artist: Some("Alpha".into()),
            album_artist: Some("Alpha".into()),
            album: Some("Echoes".into()),
            disc_number: Some(1),
            track_number: Some(2),
            duration_ms: Some(240_000),
            year: Some(2020),
            genres: vec!["rock".into(), "Ambient".into()],
            audio: AudioProperties {
                codec: Some("flac".into()),
                bitrate_kbps: Some(980),
                sample_rate_hz: Some(96_000),
                bit_depth: Some(24),
                channels: Some(2),
            },
            ..Default::default()
        },
        20_000,
    );
    let alpha_older = tagged_track(
        "Roots",
        "/media/Catalog/alpha/roots/01.mp3",
        TrackTags {
            title: Some("Roots".into()),
            artist: Some("Alpha".into()),
            album_artist: Some("Alpha".into()),
            album: Some("Beginnings".into()),
            disc_number: Some(1),
            track_number: Some(1),
            duration_ms: Some(150_000),
            year: Some(2015),
            genres: vec!["Folk".into()],
            audio: AudioProperties {
                codec: Some("mp3".into()),
                bitrate_kbps: Some(320),
                sample_rate_hz: Some(44_100),
                bit_depth: None,
                channels: Some(2),
            },
            ..Default::default()
        },
        5_000,
    );
    let beta_echoes_d1 = tagged_track(
        "Pulse",
        "/media/Catalog/beta/echoes/1-01.m4a",
        TrackTags {
            title: Some("Pulse".into()),
            artist: Some("Beta".into()),
            album_artist: Some("Beta".into()),
            album: Some("Echoes".into()),
            disc_number: Some(1),
            track_number: Some(1),
            duration_ms: Some(300_000),
            year: Some(2023),
            genres: vec!["Rock".into(), "Electronic".into()],
            audio: AudioProperties {
                codec: Some("aac".into()),
                bitrate_kbps: Some(256),
                sample_rate_hz: Some(48_000),
                bit_depth: None,
                channels: Some(2),
            },
            ..Default::default()
        },
        7_000,
    );
    let beta_echoes_d2 = tagged_track(
        "Drift",
        "/media/Catalog/beta/echoes/2-01.m4a",
        TrackTags {
            title: Some("Drift".into()),
            artist: Some("Beta".into()),
            album_artist: Some("Beta".into()),
            album: Some("Echoes".into()),
            disc_number: Some(2),
            track_number: Some(1),
            duration_ms: Some(210_000),
            year: Some(2023),
            genres: vec!["Electronic".into()],
            audio: AudioProperties {
                codec: Some("alac".into()),
                bitrate_kbps: Some(850),
                sample_rate_hz: Some(44_100),
                bit_depth: Some(16),
                channels: Some(2),
            },
            ..Default::default()
        },
        12_000,
    );
    let gamma_unyear = tagged_track(
        "NoDate",
        "/media/Catalog/gamma/nodate/01.opus",
        TrackTags {
            title: Some("NoDate".into()),
            artist: Some("Gamma".into()),
            album_artist: Some("Gamma".into()),
            album: Some("Untitled".into()),
            disc_number: None,
            track_number: None,
            duration_ms: None,
            year: None,
            genres: vec!["Jazz".into()],
            audio: AudioProperties {
                codec: Some("opus".into()),
                bitrate_kbps: Some(128),
                sample_rate_hz: Some(48_000),
                bit_depth: None,
                channels: Some(2),
            },
            ..Default::default()
        },
        3_000,
    );
    let other_track = tagged_track(
        "External",
        "/media/Other/ext.flac",
        TrackTags {
            title: Some("External".into()),
            artist: Some("Omega".into()),
            album_artist: Some("Omega".into()),
            album: Some("Far".into()),
            year: Some(2024),
            genres: vec!["Classical".into()],
            ..Default::default()
        },
        4_000,
    );

    // Insert in two batches so `added_at` differs deterministically between older and newer tracks.
    db.items()
        .apply_scan_batch(lib, &[alpha_older.clone(), gamma_unyear.clone()], &[])
        .await
        .unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    db.items()
        .apply_scan_batch(
            lib,
            &[
                alpha_echoes_1.clone(),
                alpha_echoes_2.clone(),
                beta_echoes_d1.clone(),
                beta_echoes_d2.clone(),
            ],
            &[],
        )
        .await
        .unwrap();
    db.items()
        .apply_scan_batch(other_lib, std::slice::from_ref(&other_track), &[])
        .await
        .unwrap();

    // Genres: Rock merges "Rock" and "rock" across 2 distinct albums (Alpha/Echoes & Beta/Echoes) and 3 tracks.
    let genres = db.music().list_genres(lib).await.unwrap().unwrap().genres;
    let rock = genres
        .iter()
        .find(|g| g.name.eq_ignore_ascii_case("rock"))
        .expect("rock genre");
    assert_eq!((rock.album_count, rock.track_count), (2, 3));
    assert_eq!(
        genres
            .iter()
            .map(|g| g.name.to_lowercase())
            .collect::<Vec<_>>(),
        ["ambient", "electronic", "folk", "indie", "jazz", "rock"]
    );

    // Albums: two artists with the same album name ("Echoes") MUST have distinct album IDs.
    let all_albums = db
        .music()
        .list_albums(
            alice,
            &AlbumListQuery {
                library_id: lib,
                sort: MusicAlbumSort::Title,
                order: MusicSortOrder::Asc,
                genre: None,
                year_from: None,
                year_to: None,
                artist_id: None,
                favorite_only: false,
                q: None,
                seed: None,
                cursor: None,
                limit: 60,
            },
        )
        .await
        .unwrap()
        .unwrap();
    assert_eq!(all_albums.total, 4);
    assert_eq!(all_albums.albums.len(), 4);
    let echoes: Vec<_> = all_albums
        .albums
        .iter()
        .filter(|a| a.name.as_deref() == Some("Echoes"))
        .collect();
    assert_eq!(echoes.len(), 2);
    assert_ne!(echoes[0].id, echoes[1].id);
    let alpha_echoes = echoes
        .iter()
        .find(|a| a.artist.as_deref() == Some("Alpha"))
        .unwrap();
    let beta_echoes = echoes
        .iter()
        .find(|a| a.artist.as_deref() == Some("Beta"))
        .unwrap();
    assert_eq!((alpha_echoes.track_count, alpha_echoes.disc_count), (2, 1));
    assert_eq!((beta_echoes.track_count, beta_echoes.disc_count), (2, 2));
    assert_eq!(beta_echoes.duration_ms, 510_000);
    assert_eq!(alpha_echoes.year, Some(2020));
    assert_eq!(
        alpha_echoes
            .genres
            .iter()
            .map(|g| g.to_lowercase())
            .collect::<Vec<_>>(),
        ["ambient", "indie", "rock"]
    );

    // Personal state: ratings, favorites, and play counting via completed: true.
    let personal = db.personal();
    // Rate Alpha/Echoes tracks 5 and 4 -> rounded average = 5.
    personal
        .update(alice, alpha_echoes_1.id, MediaChange::Rating(5))
        .await
        .unwrap();
    personal
        .update(alice, alpha_echoes_2.id, MediaChange::Rating(4))
        .await
        .unwrap();
    // Rate only 1 of 2 tracks on Beta/Echoes as 3 -> average of rated tracks = 3 (unrated ignored).
    personal
        .update(alice, beta_echoes_d1.id, MediaChange::Rating(3))
        .await
        .unwrap();
    // Favorite tracks and collections for alice (not bob).
    personal
        .update(alice, alpha_echoes_1.id, MediaChange::Favorite(true))
        .await
        .unwrap();
    let (alpha_ak, alpha_bk) = crate::music::parse_album_id(&alpha_echoes.id).unwrap();
    db.music()
        .set_library_collection_favorite(alice, lib, alpha_ak, Some(alpha_bk), true)
        .await
        .unwrap();
    db.music()
        .set_library_collection_favorite(alice, lib, alpha_ak, None, true)
        .await
        .unwrap();

    // Play counting rule: reporting completed=true increments playCount and updates lastPlayedAt.
    let p1 = personal
        .update(
            alice,
            alpha_echoes_1.id,
            MediaChange::Progress(PlaybackUpdate {
                expected_version: 0,
                position_ms: 95_000,
                completed: true,
            }),
        )
        .await
        .unwrap();
    assert_eq!(p1.play_count, 1);
    assert!(p1.last_played_at.is_some());
    // Retrying the exact same request is idempotent.
    let p1_retry = personal
        .update(
            alice,
            alpha_echoes_1.id,
            MediaChange::Progress(PlaybackUpdate {
                expected_version: 0,
                position_ms: 95_000,
                completed: true,
            }),
        )
        .await
        .unwrap();
    assert_eq!(p1_retry.play_count, 1);
    // Playing again with updated expected_version increments playCount to 2.
    let p2 = personal
        .update(
            alice,
            alpha_echoes_1.id,
            MediaChange::Progress(PlaybackUpdate {
                expected_version: p1.playback_version,
                position_ms: 100_000,
                completed: true,
            }),
        )
        .await
        .unwrap();
    assert_eq!(p2.play_count, 2);
    tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    // Play Beta/Echoes track once (more recently played than Alpha/Echoes).
    personal
        .update(
            alice,
            beta_echoes_d2.id,
            MediaChange::Progress(PlaybackUpdate {
                expected_version: 0,
                position_ms: 110_000,
                completed: true,
            }),
        )
        .await
        .unwrap();

    // Album detail: check disc/track sort, rounded stars, playCount, favorite, per-user isolation.
    let detail = db
        .music()
        .get_album_detail(alice, lib, alpha_ak, alpha_bk)
        .await
        .unwrap()
        .unwrap();
    assert!(detail.album.favorite);
    assert_eq!(detail.album.stars, 5);
    assert_eq!(detail.album.play_count, 2);
    assert!(detail.album.last_played_at.is_some());
    assert_eq!(
        detail
            .tracks
            .iter()
            .map(|t| t.title.as_str())
            .collect::<Vec<_>>(),
        ["Dawn", "Dusk"]
    );
    assert_eq!(detail.tracks[0].codec.as_deref(), Some("flac"));
    assert_eq!(detail.tracks[0].bitrate_kbps, Some(950));
    assert_eq!(detail.tracks[0].sample_rate_hz, Some(44_100));
    assert_eq!(detail.tracks[0].bit_depth, Some(16));
    assert_eq!(detail.tracks[0].channels, Some(2));
    assert_eq!(detail.tracks[0].size_bytes, Some(10_000));
    assert!(detail.tracks[0].state.favorite);
    assert_eq!(detail.tracks[0].state.stars, 5);
    assert_eq!(detail.tracks[0].state.play_count, 2);

    let bob_detail = db
        .music()
        .get_album_detail(bob, lib, alpha_ak, alpha_bk)
        .await
        .unwrap()
        .unwrap();
    assert!(!bob_detail.album.favorite);
    assert_eq!(
        (bob_detail.album.stars, bob_detail.album.play_count),
        (0, 0)
    );
    assert!(bob_detail.album.last_played_at.is_none());

    // Multi-disc album detail keeps disc 1 before disc 2 and averages only rated tracks.
    let (beta_ak, beta_bk) = crate::music::parse_album_id(&beta_echoes.id).unwrap();
    let beta_detail = db
        .music()
        .get_album_detail(alice, lib, beta_ak, beta_bk)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(beta_detail.album.stars, 3);
    assert_eq!(
        beta_detail
            .tracks
            .iter()
            .map(|t| t.title.as_str())
            .collect::<Vec<_>>(),
        ["Pulse", "Drift"]
    );

    // Exhaustive album paging across every sort mode (limit=1 verifies keyset continuity & total).
    for sort in [
        MusicAlbumSort::Title,
        MusicAlbumSort::Artist,
        MusicAlbumSort::Year,
        MusicAlbumSort::Added,
        MusicAlbumSort::Played,
        MusicAlbumSort::Plays,
        MusicAlbumSort::Rating,
        MusicAlbumSort::Random,
    ] {
        for order in [MusicSortOrder::Asc, MusicSortOrder::Desc] {
            let mut seen = Vec::new();
            let mut cursor = None;
            loop {
                let query = AlbumListQuery {
                    library_id: lib,
                    sort,
                    order,
                    genre: None,
                    year_from: None,
                    year_to: None,
                    artist_id: None,
                    favorite_only: false,
                    q: None,
                    seed: (sort == MusicAlbumSort::Random).then(|| "stable-seed".into()),
                    cursor: cursor.clone(),
                    limit: 1,
                };
                let page = db
                    .music()
                    .list_albums(alice, &query)
                    .await
                    .unwrap()
                    .unwrap();
                assert_eq!(page.total, 4, "sort={sort:?} order={order:?}");
                seen.extend(page.albums.into_iter().map(|a| a.id));
                match page.next_cursor {
                    Some(raw) => {
                        cursor = Some(
                            CatalogCursor::decode_albums(&raw, &query).expect("valid album cursor"),
                        );
                    }
                    None => break,
                }
            }
            assert_eq!(seen.len(), 4, "sort={sort:?} order={order:?}");
            assert_eq!(
                seen.iter().collect::<HashSet<_>>().len(),
                4,
                "duplicate album in sort={sort:?} order={order:?}"
            );
            // Verify NULL-last on Year (Untitled has no year -> always last in Asc AND Desc)
            // and Played (2 played albums come before 2 unplayed albums in Asc AND Desc).
            if sort == MusicAlbumSort::Year {
                let last = all_albums
                    .albums
                    .iter()
                    .find(|a| a.name.as_deref() == Some("Untitled"))
                    .unwrap();
                assert_eq!(seen.last().unwrap(), &last.id);
            }
            if sort == MusicAlbumSort::Played {
                assert_eq!(
                    seen[..2].iter().cloned().collect::<HashSet<_>>(),
                    HashSet::from([alpha_echoes.id.clone(), beta_echoes.id.clone()])
                );
                if order == MusicSortOrder::Desc {
                    assert_eq!(seen[0], beta_echoes.id);
                }
            }
            if sort == MusicAlbumSort::Plays && order == MusicSortOrder::Desc {
                assert_eq!(seen[0], alpha_echoes.id);
                assert_eq!(seen[1], beta_echoes.id);
            }
            if sort == MusicAlbumSort::Rating && order == MusicSortOrder::Desc {
                assert_eq!(seen[0], alpha_echoes.id);
                assert_eq!(seen[1], beta_echoes.id);
            }
        }
    }

    // Album filters: genre (case-insensitive), yearFrom/yearTo, artistId, favorite, q.
    let filtered = db
        .music()
        .list_albums(
            alice,
            &AlbumListQuery {
                library_id: lib,
                sort: MusicAlbumSort::Title,
                order: MusicSortOrder::Asc,
                genre: Some("ROCK".into()),
                year_from: Some(2019),
                year_to: Some(2021),
                artist_id: Some(alpha_ak.to_owned()),
                favorite_only: true,
                q: Some("ech".into()),
                seed: None,
                cursor: None,
                limit: 60,
            },
        )
        .await
        .unwrap()
        .unwrap();
    assert_eq!(filtered.total, 1);
    assert_eq!(filtered.albums[0].id, alpha_echoes.id);

    // Exhaustive artist paging across all 5 sort modes.
    for sort in [
        MusicArtistSort::Name,
        MusicArtistSort::Albums,
        MusicArtistSort::Tracks,
        MusicArtistSort::Plays,
        MusicArtistSort::Random,
    ] {
        let mut seen = Vec::new();
        let mut cursor = None;
        loop {
            let query = ArtistListQuery {
                library_id: lib,
                sort,
                order: sort.default_order(),
                favorite_only: false,
                q: None,
                seed: (sort == MusicArtistSort::Random).then(|| "artist-seed".into()),
                cursor: cursor.clone(),
                limit: 1,
            };
            let page = db
                .music()
                .list_artists(alice, &query)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(page.total, 3);
            seen.extend(page.artists.into_iter().map(|a| a.id));
            match page.next_cursor {
                Some(raw) => {
                    cursor = Some(
                        CatalogCursor::decode_artists(&raw, &query).expect("valid artist cursor"),
                    );
                }
                None => break,
            }
        }
        assert_eq!(seen.len(), 3);
        assert_eq!(seen.iter().collect::<HashSet<_>>().len(), 3);
        if matches!(
            sort,
            MusicArtistSort::Albums | MusicArtistSort::Tracks | MusicArtistSort::Plays
        ) {
            assert_eq!(seen[0], alpha_ak);
        }
    }

    // Artist detail: albums sorted year DESC, topTracks ordered by playCount DESC.
    let artist_detail = db
        .music()
        .get_artist_detail(alice, lib, alpha_ak)
        .await
        .unwrap()
        .unwrap();
    assert!(artist_detail.artist.favorite);
    assert_eq!(
        (
            artist_detail.artist.album_count,
            artist_detail.artist.track_count,
            artist_detail.artist.play_count
        ),
        (2, 3, 2)
    );
    assert_eq!(
        artist_detail
            .albums
            .iter()
            .map(|a| a.year)
            .collect::<Vec<_>>(),
        [Some(2020), Some(2015)]
    );
    assert_eq!(artist_detail.top_tracks[0].id, alpha_echoes_1.id);
    assert_eq!(artist_detail.top_tracks.len(), 3);
    // For bob (all playCounts = 0), topTracks still returns the 3 tracks deterministically.
    let bob_artist = db
        .music()
        .get_artist_detail(bob, lib, alpha_ak)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(bob_artist.top_tracks.len(), 3);

    // Exhaustive track paging across all 10 sort modes.
    for sort in [
        MusicTrackSort::Title,
        MusicTrackSort::Artist,
        MusicTrackSort::Album,
        MusicTrackSort::Year,
        MusicTrackSort::Added,
        MusicTrackSort::Duration,
        MusicTrackSort::Plays,
        MusicTrackSort::Played,
        MusicTrackSort::Rating,
        MusicTrackSort::Random,
    ] {
        let mut seen = Vec::new();
        let mut cursor = None;
        loop {
            let query = TrackListQuery {
                library_id: lib,
                sort,
                order: sort.default_order(),
                genre: None,
                album: None,
                artist_id: None,
                favorite_only: false,
                q: None,
                seed: (sort == MusicTrackSort::Random).then(|| "track-seed".into()),
                cursor: cursor.clone(),
                limit: 2,
            };
            let page = db
                .music()
                .list_tracks(alice, &query)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(page.total, 6, "track sort={sort:?}");
            seen.extend(page.tracks.into_iter().map(|t| t.id));
            match page.next_cursor {
                Some(raw) => {
                    cursor = Some(
                        CatalogCursor::decode_tracks(&raw, &query).expect("valid track cursor"),
                    );
                }
                None => break,
            }
        }
        assert_eq!(seen.len(), 6, "track sort={sort:?}");
        assert_eq!(seen.iter().collect::<HashSet<_>>().len(), 6);
        if sort == MusicTrackSort::Duration || sort == MusicTrackSort::Year {
            // NoDate has NULL year and NULL duration -> must sort last.
            assert_eq!(seen.last().unwrap(), &gamma_unyear.id);
        }
        if sort == MusicTrackSort::Plays {
            assert_eq!(seen[0], alpha_echoes_1.id);
        }
        if sort == MusicTrackSort::Played {
            assert_eq!(seen[0], beta_echoes_d2.id);
            assert_eq!(seen[1], alpha_echoes_1.id);
        }
        if sort == MusicTrackSort::Rating {
            assert_eq!(seen[0], alpha_echoes_1.id);
            assert_eq!(seen[1], alpha_echoes_2.id);
            assert_eq!(seen[2], beta_echoes_d1.id);
        }
    }

    // Track filters: favorite, albumId, artistId, genre, q (including literal %).
    let fav_tracks = db
        .music()
        .list_tracks(
            alice,
            &TrackListQuery {
                library_id: lib,
                sort: MusicTrackSort::Title,
                order: MusicSortOrder::Asc,
                genre: None,
                album: Some((alpha_ak.to_owned(), alpha_bk.to_owned())),
                artist_id: Some(alpha_ak.to_owned()),
                favorite_only: true,
                q: Some("daw".into()),
                seed: None,
                cursor: None,
                limit: 60,
            },
        )
        .await
        .unwrap()
        .unwrap();
    assert_eq!(fav_tracks.total, 1);
    assert_eq!(fav_tracks.tracks[0].id, alpha_echoes_1.id);
    assert_eq!(
        db.music()
            .list_tracks(
                alice,
                &TrackListQuery {
                    library_id: lib,
                    sort: MusicTrackSort::Title,
                    order: MusicSortOrder::Asc,
                    genre: None,
                    album: None,
                    artist_id: None,
                    favorite_only: false,
                    q: Some("%".into()),
                    seed: None,
                    cursor: None,
                    limit: 60,
                }
            )
            .await
            .unwrap()
            .unwrap()
            .total,
        0
    );

    // Track lookup preserves exact requested order, preserves duplicates, spans libraries, omits missing.
    let missing = ItemId::generate();
    let looked_up = db
        .music()
        .lookup_tracks(
            alice,
            &[
                beta_echoes_d2.id,
                missing,
                alpha_echoes_1.id,
                other_track.id,
                alpha_echoes_1.id,
            ],
        )
        .await
        .unwrap();
    assert_eq!(
        looked_up.iter().map(|t| t.id).collect::<Vec<_>>(),
        [
            beta_echoes_d2.id,
            alpha_echoes_1.id,
            other_track.id,
            alpha_echoes_1.id,
        ]
    );
    assert!(looked_up[1].state.favorite);
    assert_eq!(looked_up[1].state.play_count, 2);

    // Unfavorite collection works idempotently and removes from favorite filter.
    db.music()
        .set_library_collection_favorite(alice, lib, alpha_ak, Some(alpha_bk), false)
        .await
        .unwrap();
    db.music()
        .set_library_collection_favorite(alice, lib, alpha_ak, Some(alpha_bk), false)
        .await
        .unwrap();
    assert_eq!(
        db.music()
            .list_albums(
                alice,
                &AlbumListQuery {
                    library_id: lib,
                    sort: MusicAlbumSort::Title,
                    order: MusicSortOrder::Asc,
                    genre: None,
                    year_from: None,
                    year_to: None,
                    artist_id: None,
                    favorite_only: true,
                    q: None,
                    seed: None,
                    cursor: None,
                    limit: 60,
                }
            )
            .await
            .unwrap()
            .unwrap()
            .total,
        0
    );

    // Pre-migration backfill: clearing tags_version makes scan_index report music_indexed = false.
    use crate::entities::track_metadata as tm;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, sea_query::Expr as SqExpr};
    tm::Entity::update_many()
        .col_expr(tm::Column::TagsVersion, SqExpr::val(Option::<i64>::None))
        .filter(tm::Column::ItemId.eq(alpha_echoes_1.id.to_string()))
        .exec(db.main())
        .await
        .unwrap();
    let index = db.items().scan_index(lib).await.unwrap();
    let stale = index.iter().find(|i| i.id == alpha_echoes_1.id).unwrap();
    assert!(!stale.music_indexed);
    let fresh = index.iter().find(|i| i.id == alpha_echoes_2.id).unwrap();
    assert!(fresh.music_indexed);
    // Re-applying the track restores music_indexed = true.
    db.items()
        .apply_scan_batch(lib, std::slice::from_ref(&alpha_echoes_1), &[])
        .await
        .unwrap();
    assert!(
        db.items()
            .scan_index(lib)
            .await
            .unwrap()
            .iter()
            .all(|i| i.music_indexed)
    );

    // Unseeded random paging stays stable via the seed embedded inside the cursor.
    let mut unseeded = Vec::new();
    let mut rand_cursor = None;
    loop {
        let query = AlbumListQuery {
            library_id: lib,
            sort: MusicAlbumSort::Random,
            order: MusicSortOrder::Asc,
            genre: None,
            year_from: None,
            year_to: None,
            artist_id: None,
            favorite_only: false,
            q: None,
            seed: None,
            cursor: rand_cursor.clone(),
            limit: 1,
        };
        let page = db
            .music()
            .list_albums(alice, &query)
            .await
            .unwrap()
            .unwrap();
        unseeded.extend(page.albums.into_iter().map(|a| a.id));
        match page.next_cursor {
            Some(raw) => {
                // Switching seed with an unseeded cursor must be rejected.
                let mismatch = AlbumListQuery {
                    seed: Some("other".into()),
                    ..query.clone()
                };
                assert!(CatalogCursor::decode_albums(&raw, &mismatch).is_none());
                rand_cursor =
                    Some(CatalogCursor::decode_albums(&raw, &query).expect("unseeded cursor"));
            }
            None => break,
        }
    }
    assert_eq!(unseeded.len(), 4);
    assert_eq!(unseeded.iter().collect::<HashSet<_>>().len(), 4);

    // Artist detail topTracks caps at 10 even when an artist has more tracks.
    let mut extra_tracks = Vec::new();
    for idx in 1..=12u32 {
        extra_tracks.push(tagged_track(
            &format!("Extra {idx:02}"),
            &format!("/media/Catalog/alpha/extra/{idx:02}.flac"),
            TrackTags {
                title: Some(format!("Extra {idx:02}")),
                artist: Some("Alpha".into()),
                album_artist: Some("Alpha".into()),
                album: Some("Vault".into()),
                disc_number: Some(1),
                track_number: Some(idx),
                year: Some(2022),
                ..Default::default()
            },
            1_000,
        ));
    }
    db.items()
        .apply_scan_batch(lib, &extra_tracks, &[])
        .await
        .unwrap();
    let capped_artist = db
        .music()
        .get_artist_detail(bob, lib, alpha_ak)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(capped_artist.top_tracks.len(), 10);
    assert_eq!(
        capped_artist
            .albums
            .iter()
            .map(|a| a.year)
            .collect::<Vec<_>>(),
        [Some(2022), Some(2020), Some(2015)]
    );

    // Deleting a library cleans up music_albums, music_album_pending, and track_genres.
    db.libraries().delete(lib).await.unwrap();
    assert!(
        db.music()
            .list_albums(
                alice,
                &AlbumListQuery {
                    library_id: lib,
                    sort: MusicAlbumSort::Title,
                    order: MusicSortOrder::Asc,
                    genre: None,
                    year_from: None,
                    year_to: None,
                    artist_id: None,
                    favorite_only: false,
                    q: None,
                    seed: None,
                    cursor: None,
                    limit: 60,
                }
            )
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        crate::entities::music_albums::Entity::find()
            .filter(crate::entities::music_albums::Column::LibraryId.eq(lib.to_string()))
            .all(db.main())
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        crate::entities::track_genres::Entity::find()
            .filter(crate::entities::track_genres::Column::LibraryId.eq(lib.to_string()))
            .all(db.main())
            .await
            .unwrap()
            .is_empty()
    );
    h.finish().await;
}

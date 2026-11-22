//! Rich music library catalog endpoints: albums, artists, genres, tracks,
//! track lookup and collection favorites.

use axum::{
    Json,
    extract::{
        Path, Query, State,
        rejection::{PathRejection, QueryRejection},
    },
    http::StatusCode,
};
use palveni_core::{
    LibraryId,
    music::{
        MusicAlbumDetail, MusicAlbumPage, MusicAlbumSort, MusicArtistDetail, MusicArtistPage,
        MusicArtistSort, MusicGenreList, MusicSortOrder, MusicTrackLookupRequest,
        MusicTrackLookupResponse, MusicTrackPage, MusicTrackSort,
    },
};
use palveni_db::music::{
    is_group_key,
    library::{AlbumListQuery, ArtistListQuery, CatalogCursor, TrackListQuery},
    parse_album_id,
};
use serde::Deserialize;

use crate::{
    AppState,
    auth::CurrentUser,
    error::ApiError,
    json::ApiJson,
    routes::{MAX_PAGE_SIZE, parse_limit, path_id},
};

const DEFAULT_CATALOG_LIMIT: u32 = 60;
const MAX_LOOKUP_IDS: usize = 500;
const MAX_TEXT_FILTER_CHARS: usize = 128;
const MAX_SEED_CHARS: usize = 64;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlbumsQuery {
    #[serde(default)]
    sort: MusicAlbumSort,
    order: Option<MusicSortOrder>,
    genre: Option<String>,
    year_from: Option<i32>,
    year_to: Option<i32>,
    artist_id: Option<String>,
    favorite: Option<bool>,
    q: Option<String>,
    seed: Option<String>,
    cursor: Option<String>,
    limit: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtistsQuery {
    #[serde(default)]
    sort: MusicArtistSort,
    order: Option<MusicSortOrder>,
    favorite: Option<bool>,
    q: Option<String>,
    seed: Option<String>,
    cursor: Option<String>,
    limit: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TracksQuery {
    #[serde(default)]
    sort: MusicTrackSort,
    order: Option<MusicSortOrder>,
    genre: Option<String>,
    album_id: Option<String>,
    artist_id: Option<String>,
    favorite: Option<bool>,
    q: Option<String>,
    seed: Option<String>,
    cursor: Option<String>,
    limit: Option<String>,
}

fn parse_year(value: Option<i32>, field: &'static str) -> Result<Option<i32>, ApiError> {
    if let Some(year) = value
        && !(1..=9999).contains(&year)
    {
        return Err(ApiError::validation("Year must be between 1 and 9999").field(field));
    }
    Ok(value)
}

fn parse_text(
    raw: Option<String>,
    field: &'static str,
    max_chars: usize,
) -> Result<Option<String>, ApiError> {
    let Some(value) = raw else {
        return Ok(None);
    };
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    if trimmed.chars().count() > max_chars || trimmed.chars().any(char::is_control) {
        return Err(ApiError::validation("Invalid filter value").field(field));
    }
    Ok(Some(trimmed.to_owned()))
}

fn parse_artist_filter(raw: Option<String>) -> Result<Option<String>, ApiError> {
    let Some(value) = raw.as_deref().map(str::trim).filter(|s| !s.is_empty()) else {
        return Ok(None);
    };
    if !is_group_key(value) {
        return Err(ApiError::validation("Invalid artist identifier").field("artistId"));
    }
    Ok(Some(value.to_owned()))
}

fn parse_album_filter(raw: Option<String>) -> Result<Option<(String, String)>, ApiError> {
    let Some(value) = raw.as_deref().map(str::trim).filter(|s| !s.is_empty()) else {
        return Ok(None);
    };
    let (ak, bk) = parse_album_id(value)
        .ok_or_else(|| ApiError::validation("Invalid album identifier").field("albumId"))?;
    Ok(Some((ak.to_owned(), bk.to_owned())))
}

pub async fn albums(
    user: CurrentUser,
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
    params: Result<Query<AlbumsQuery>, QueryRejection>,
) -> Result<Json<MusicAlbumPage>, ApiError> {
    let library_id: LibraryId = path_id(&path?.0, "Library")?;
    let q = params?.0;
    let limit = parse_limit(q.limit.as_deref(), DEFAULT_CATALOG_LIMIT, MAX_PAGE_SIZE)?;
    let order = q.order.unwrap_or_else(|| q.sort.default_order());
    let mut query = AlbumListQuery {
        library_id,
        sort: q.sort,
        order,
        genre: parse_text(q.genre, "genre", MAX_TEXT_FILTER_CHARS)?,
        year_from: parse_year(q.year_from, "yearFrom")?,
        year_to: parse_year(q.year_to, "yearTo")?,
        artist_id: parse_artist_filter(q.artist_id)?,
        favorite_only: q.favorite.unwrap_or(false),
        q: parse_text(q.q, "q", MAX_TEXT_FILTER_CHARS)?,
        seed: parse_text(q.seed, "seed", MAX_SEED_CHARS)?,
        cursor: None,
        limit,
    };
    if let Some(raw) = q.cursor.as_deref() {
        let cursor = CatalogCursor::decode_albums(raw, &query)
            .ok_or_else(|| ApiError::validation("Invalid music album cursor").field("cursor"))?;
        query.cursor = Some(cursor);
    }
    let page = state
        .db
        .music()
        .list_albums(user.user.id, &query)
        .await?
        .ok_or_else(|| ApiError::not_found("Music library not found"))?;
    Ok(Json(page))
}

pub async fn album_detail(
    user: CurrentUser,
    State(state): State<AppState>,
    path: Result<Path<(String, String)>, PathRejection>,
) -> Result<Json<MusicAlbumDetail>, ApiError> {
    let Path((raw_lib, raw_album)) = path?;
    let library_id: LibraryId = path_id(&raw_lib, "Library")?;
    let (artist_key, album_key) =
        parse_album_id(&raw_album).ok_or_else(|| ApiError::not_found("Album not found"))?;
    let detail = state
        .db
        .music()
        .get_album_detail(user.user.id, library_id, artist_key, album_key)
        .await?
        .ok_or_else(|| ApiError::not_found("Album not found"))?;
    Ok(Json(detail))
}

pub async fn artists(
    user: CurrentUser,
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
    params: Result<Query<ArtistsQuery>, QueryRejection>,
) -> Result<Json<MusicArtistPage>, ApiError> {
    let library_id: LibraryId = path_id(&path?.0, "Library")?;
    let q = params?.0;
    let limit = parse_limit(q.limit.as_deref(), DEFAULT_CATALOG_LIMIT, MAX_PAGE_SIZE)?;
    let order = q.order.unwrap_or_else(|| q.sort.default_order());
    let mut query = ArtistListQuery {
        library_id,
        sort: q.sort,
        order,
        favorite_only: q.favorite.unwrap_or(false),
        q: parse_text(q.q, "q", MAX_TEXT_FILTER_CHARS)?,
        seed: parse_text(q.seed, "seed", MAX_SEED_CHARS)?,
        cursor: None,
        limit,
    };
    if let Some(raw) = q.cursor.as_deref() {
        let cursor = CatalogCursor::decode_artists(raw, &query)
            .ok_or_else(|| ApiError::validation("Invalid music artist cursor").field("cursor"))?;
        query.cursor = Some(cursor);
    }
    let page = state
        .db
        .music()
        .list_artists(user.user.id, &query)
        .await?
        .ok_or_else(|| ApiError::not_found("Music library not found"))?;
    Ok(Json(page))
}

pub async fn artist_detail(
    user: CurrentUser,
    State(state): State<AppState>,
    path: Result<Path<(String, String)>, PathRejection>,
) -> Result<Json<MusicArtistDetail>, ApiError> {
    let Path((raw_lib, raw_artist)) = path?;
    let library_id: LibraryId = path_id(&raw_lib, "Library")?;
    if !is_group_key(&raw_artist) {
        return Err(ApiError::not_found("Artist not found"));
    }
    let detail = state
        .db
        .music()
        .get_artist_detail(user.user.id, library_id, &raw_artist)
        .await?
        .ok_or_else(|| ApiError::not_found("Artist not found"))?;
    Ok(Json(detail))
}

pub async fn genres(
    _: CurrentUser,
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
) -> Result<Json<MusicGenreList>, ApiError> {
    let library_id: LibraryId = path_id(&path?.0, "Library")?;
    let list = state
        .db
        .music()
        .list_genres(library_id)
        .await?
        .ok_or_else(|| ApiError::not_found("Music library not found"))?;
    Ok(Json(list))
}

pub async fn tracks(
    user: CurrentUser,
    State(state): State<AppState>,
    path: Result<Path<String>, PathRejection>,
    params: Result<Query<TracksQuery>, QueryRejection>,
) -> Result<Json<MusicTrackPage>, ApiError> {
    let library_id: LibraryId = path_id(&path?.0, "Library")?;
    let q = params?.0;
    let limit = parse_limit(q.limit.as_deref(), DEFAULT_CATALOG_LIMIT, MAX_PAGE_SIZE)?;
    let order = q.order.unwrap_or_else(|| q.sort.default_order());
    let mut query = TrackListQuery {
        library_id,
        sort: q.sort,
        order,
        genre: parse_text(q.genre, "genre", MAX_TEXT_FILTER_CHARS)?,
        album: parse_album_filter(q.album_id)?,
        artist_id: parse_artist_filter(q.artist_id)?,
        favorite_only: q.favorite.unwrap_or(false),
        q: parse_text(q.q, "q", MAX_TEXT_FILTER_CHARS)?,
        seed: parse_text(q.seed, "seed", MAX_SEED_CHARS)?,
        cursor: None,
        limit,
    };
    if let Some(raw) = q.cursor.as_deref() {
        let cursor = CatalogCursor::decode_tracks(raw, &query)
            .ok_or_else(|| ApiError::validation("Invalid music track cursor").field("cursor"))?;
        query.cursor = Some(cursor);
    }
    let page = state
        .db
        .music()
        .list_tracks(user.user.id, &query)
        .await?
        .ok_or_else(|| ApiError::not_found("Music library not found"))?;
    Ok(Json(page))
}

pub async fn lookup_tracks(
    user: CurrentUser,
    State(state): State<AppState>,
    ApiJson(body): ApiJson<MusicTrackLookupRequest>,
) -> Result<Json<MusicTrackLookupResponse>, ApiError> {
    if body.ids.len() > MAX_LOOKUP_IDS {
        return Err(ApiError::validation(format!(
            "At most {MAX_LOOKUP_IDS} track IDs are allowed"
        ))
        .field("ids"));
    }
    let tracks = state
        .db
        .music()
        .lookup_tracks(user.user.id, &body.ids)
        .await?;
    Ok(Json(MusicTrackLookupResponse { tracks }))
}

async fn set_album_favorite(
    user: CurrentUser,
    state: AppState,
    path: Result<Path<(String, String)>, PathRejection>,
    favorite: bool,
) -> Result<StatusCode, ApiError> {
    let Path((raw_lib, raw_album)) = path?;
    let library_id: LibraryId = path_id(&raw_lib, "Library")?;
    let (artist_key, album_key) =
        parse_album_id(&raw_album).ok_or_else(|| ApiError::not_found("Album not found"))?;
    state
        .db
        .music()
        .set_library_collection_favorite(
            user.user.id,
            library_id,
            artist_key,
            Some(album_key),
            favorite,
        )
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn favorite_album(
    user: CurrentUser,
    State(state): State<AppState>,
    path: Result<Path<(String, String)>, PathRejection>,
) -> Result<StatusCode, ApiError> {
    set_album_favorite(user, state, path, true).await
}

pub async fn unfavorite_album(
    user: CurrentUser,
    State(state): State<AppState>,
    path: Result<Path<(String, String)>, PathRejection>,
) -> Result<StatusCode, ApiError> {
    set_album_favorite(user, state, path, false).await
}

async fn set_artist_favorite(
    user: CurrentUser,
    state: AppState,
    path: Result<Path<(String, String)>, PathRejection>,
    favorite: bool,
) -> Result<StatusCode, ApiError> {
    let Path((raw_lib, raw_artist)) = path?;
    let library_id: LibraryId = path_id(&raw_lib, "Library")?;
    if !is_group_key(&raw_artist) {
        return Err(ApiError::not_found("Artist not found"));
    }
    state
        .db
        .music()
        .set_library_collection_favorite(user.user.id, library_id, &raw_artist, None, favorite)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn favorite_artist(
    user: CurrentUser,
    State(state): State<AppState>,
    path: Result<Path<(String, String)>, PathRejection>,
) -> Result<StatusCode, ApiError> {
    set_artist_favorite(user, state, path, true).await
}

pub async fn unfavorite_artist(
    user: CurrentUser,
    State(state): State<AppState>,
    path: Result<Path<(String, String)>, PathRejection>,
) -> Result<StatusCode, ApiError> {
    set_artist_favorite(user, state, path, false).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::response::IntoResponse;

    #[test]
    fn validates_years_text_and_group_filters() {
        assert_eq!(parse_year(Some(2024), "yearFrom").unwrap(), Some(2024));
        assert!(parse_year(Some(0), "yearFrom").is_err());
        assert!(parse_year(Some(10000), "yearTo").is_err());
        assert_eq!(parse_text(Some("   ".into()), "q", 128).unwrap(), None);
        assert_eq!(
            parse_text(Some(" Rock ".into()), "genre", 128).unwrap(),
            Some("Rock".into())
        );
        assert!(parse_text(Some("a\0b".into()), "q", 128).is_err());
        assert!(parse_text(Some("a".repeat(129)), "q", 128).is_err());
        assert!(parse_artist_filter(Some("not-hex".into())).is_err());
        assert!(parse_album_filter(Some("not-an-album".into())).is_err());
        let valid_key = "a".repeat(64);
        assert_eq!(
            parse_artist_filter(Some(valid_key.clone())).unwrap(),
            Some(valid_key.clone())
        );
        assert_eq!(
            parse_album_filter(Some(format!("{valid_key}.{valid_key}"))).unwrap(),
            Some((valid_key.clone(), valid_key))
        );
    }

    #[test]
    fn parses_query_params_and_defaults() {
        let q = Query::<AlbumsQuery>::try_from_uri(
            &"/albums?sort=year&yearFrom=1990&yearTo=2020&favorite=true&genre=Rock&q=echo&seed=s1&limit=25"
                .parse()
                .unwrap(),
        )
        .unwrap()
        .0;
        assert_eq!(q.sort, MusicAlbumSort::Year);
        assert_eq!(q.order, None);
        assert_eq!(q.sort.default_order(), MusicSortOrder::Desc);
        assert_eq!(q.year_from, Some(1990));
        assert_eq!(q.year_to, Some(2020));
        assert_eq!(q.favorite, Some(true));
        assert_eq!(q.genre.as_deref(), Some("Rock"));
        assert_eq!(q.q.as_deref(), Some("echo"));
        assert_eq!(q.seed.as_deref(), Some("s1"));
        assert_eq!(q.limit.as_deref(), Some("25"));

        let bad_sort = Query::<AlbumsQuery>::try_from_uri(&"/albums?sort=nope".parse().unwrap())
            .map_err(ApiError::from)
            .unwrap_err();
        assert_eq!(bad_sort.into_response().status(), StatusCode::BAD_REQUEST);

        let bad_track_sort =
            Query::<TracksQuery>::try_from_uri(&"/tracks?sort=bogus".parse().unwrap())
                .map_err(ApiError::from)
                .unwrap_err();
        assert_eq!(
            bad_track_sort.into_response().status(),
            StatusCode::BAD_REQUEST
        );

        let bad_artist_sort =
            Query::<ArtistsQuery>::try_from_uri(&"/artists?sort=year".parse().unwrap())
                .map_err(ApiError::from)
                .unwrap_err();
        assert_eq!(
            bad_artist_sort.into_response().status(),
            StatusCode::BAD_REQUEST
        );
    }

    #[tokio::test]
    async fn handlers_cover_all_endpoints_errors_and_lookup_limit() {
        use palveni_core::{
            FeatureRegistry, ItemId, ItemKind, LibraryKind, PerfTier, Role, ServerId, SessionId,
            music::{AudioProperties, TrackTags},
            personal::PlaybackUpdate,
        };
        use palveni_db::{Db, DbOptions, items::ItemUpsert, libraries::NewLibrary};
        use palveni_jobs::JobService;
        use palveni_media::{HardwareService, ProbeOptions};
        use palveni_plugin_host::{PluginCatalog, skin_store::SkinStore};
        use std::sync::Arc;
        use tokio_util::sync::CancellationToken;

        let dir = std::env::temp_dir().join(format!("palveni-music-api-{}", ItemId::generate()));
        std::fs::create_dir_all(&dir).unwrap();
        let db = Db::open(&DbOptions {
            main_url: format!("sqlite://{}/main.db?mode=rwc", dir.display()),
            ops_url: format!("sqlite://{}/ops.db?mode=rwc", dir.display()),
            max_connections: Some(4),
            min_connections: Some(1),
        })
        .await
        .unwrap();
        let user = db
            .users()
            .create_first_admin("tester", "hash")
            .await
            .unwrap()
            .unwrap();
        let lib = db
            .libraries()
            .create(NewLibrary {
                name: "Music".into(),
                kind: LibraryKind::Music,
                paths: vec!["/media/music".into()],
            })
            .await
            .unwrap();
        let t1_id = ItemId::generate();
        let t2_id = ItemId::generate();
        db.items()
            .apply_scan_batch(
                lib.id,
                &[
                    ItemUpsert {
                        id: t1_id,
                        parent_id: None,
                        kind: ItemKind::Track,
                        title: "First".into(),
                        sort_title: "first".into(),
                        search_text: "first".into(),
                        path: "/media/music/1.flac".into(),
                        size_bytes: Some(1234),
                        modified_at: Some(1_700_000_000_000),
                        music: Some(TrackTags {
                            title: Some("First".into()),
                            artist: Some("Artist One".into()),
                            album_artist: Some("Artist One".into()),
                            album: Some("Same Title".into()),
                            disc_number: Some(1),
                            track_number: Some(1),
                            duration_ms: Some(200_000),
                            year: Some(2021),
                            genres: vec!["Rock".into(), "Jazz".into()],
                            audio: AudioProperties {
                                codec: Some("flac".into()),
                                bitrate_kbps: Some(900),
                                sample_rate_hz: Some(44_100),
                                bit_depth: Some(16),
                                channels: Some(2),
                            },
                            ..Default::default()
                        }),
                    },
                    ItemUpsert {
                        id: t2_id,
                        parent_id: None,
                        kind: ItemKind::Track,
                        title: "Second".into(),
                        sort_title: "second".into(),
                        search_text: "second".into(),
                        path: "/media/music/2.mp3".into(),
                        size_bytes: Some(567),
                        modified_at: Some(1_700_000_000_000),
                        music: Some(TrackTags {
                            title: Some("Second".into()),
                            artist: Some("Artist Two".into()),
                            album_artist: Some("Artist Two".into()),
                            album: Some("Same Title".into()),
                            disc_number: Some(1),
                            track_number: Some(1),
                            duration_ms: Some(180_000),
                            year: Some(2022),
                            genres: vec!["Rock".into()],
                            audio: AudioProperties {
                                codec: Some("mp3".into()),
                                bitrate_kbps: Some(320),
                                sample_rate_hz: Some(44_100),
                                bit_depth: None,
                                channels: Some(2),
                            },
                            ..Default::default()
                        }),
                    },
                ],
                &[],
            )
            .await
            .unwrap();

        let bus = db.event_bus(16).await.unwrap();
        let jobs = JobService::new(db.clone(), bus.clone(), 1);
        let state = AppState {
            db: db.clone(),
            music_search: Arc::new(palveni_library::music_search::MusicSearch::new(
                dir.join("search"),
            )),
            identity: Arc::new(palveni_link::NodeIdentity::generate().unwrap()),
            link: Arc::new(crate::LinkPairings::default()),
            login_limit: Arc::new(crate::LoginLimit::new(4)),
            subsonic_limit: Arc::new(crate::LoginLimit::new(4)),
            bus,
            jobs,
            hardware: Arc::new(HardwareService::new(db.clone(), ProbeOptions::default())),
            plugins: Arc::new(PluginCatalog::new()),
            settings: Arc::new(tokio::sync::RwLock::new(Default::default())),
            boot_tier: PerfTier::detect(0, 1, false),
            skins: Arc::new(SkinStore::open(&dir.join("skins")).unwrap()),
            skin_writes: Arc::new(tokio::sync::Semaphore::new(1)),
            features: FeatureRegistry::new(),
            server: Arc::new(crate::ServerInfo {
                id: ServerId::generate(),
                name: "Test".into(),
                version: "0.1.0".into(),
            }),
            shutdown: CancellationToken::new(),
        };
        let auth = CurrentUser {
            user: palveni_core::User {
                id: user.id,
                username: user.username,
                role: Role::Admin,
                created_at: user.created_at,
            },
            session_id: SessionId::generate(),
            device_id: None,
        };

        // List albums & confirm two distinct album IDs for same album name.
        let alb_page = albums(
            auth.clone(),
            State(state.clone()),
            Ok(Path(lib.id.to_string())),
            Ok(Query(
                Query::<AlbumsQuery>::try_from_uri(&"/albums?sort=year".parse().unwrap())
                    .unwrap()
                    .0,
            )),
        )
        .await
        .unwrap()
        .0;
        assert_eq!(alb_page.total, 2);
        assert_ne!(alb_page.albums[0].id, alb_page.albums[1].id);
        assert_eq!(alb_page.albums[0].year, Some(2022));

        // Favorite album & artist -> 204, reflected in details.
        let alb_id = alb_page.albums[0].id.clone();
        let art_id = alb_page.albums[0].artist_id.clone();
        assert_eq!(
            favorite_album(
                auth.clone(),
                State(state.clone()),
                Ok(Path((lib.id.to_string(), alb_id.clone()))),
            )
            .await
            .unwrap(),
            StatusCode::NO_CONTENT
        );
        assert_eq!(
            favorite_artist(
                auth.clone(),
                State(state.clone()),
                Ok(Path((lib.id.to_string(), art_id.clone()))),
            )
            .await
            .unwrap(),
            StatusCode::NO_CONTENT
        );

        let alb_detail = album_detail(
            auth.clone(),
            State(state.clone()),
            Ok(Path((lib.id.to_string(), alb_id.clone()))),
        )
        .await
        .unwrap()
        .0;
        assert!(alb_detail.album.favorite);
        assert_eq!(alb_detail.tracks.len(), 1);

        // Progress with completed=true updates playCount & lastPlayedAt.
        let updated_state = crate::routes::personal::progress(
            auth.clone(),
            State(state.clone()),
            Ok(Path(t2_id.to_string())),
            ApiJson(PlaybackUpdate {
                expected_version: 0,
                position_ms: 90_000,
                completed: true,
            }),
        )
        .await
        .unwrap()
        .0;
        assert_eq!(updated_state.play_count, 1);
        assert!(updated_state.last_played_at.is_some());

        let art_detail = artist_detail(
            auth.clone(),
            State(state.clone()),
            Ok(Path((lib.id.to_string(), art_id.clone()))),
        )
        .await
        .unwrap()
        .0;
        assert!(art_detail.artist.favorite);
        assert_eq!(art_detail.artist.play_count, 1);
        assert_eq!(art_detail.top_tracks[0].id, t2_id);

        // Genres list.
        let genre_list = genres(
            auth.clone(),
            State(state.clone()),
            Ok(Path(lib.id.to_string())),
        )
        .await
        .unwrap()
        .0;
        assert_eq!(genre_list.genres.len(), 2);

        // Tracks list & artists list.
        let tr_page = tracks(
            auth.clone(),
            State(state.clone()),
            Ok(Path(lib.id.to_string())),
            Ok(Query(
                Query::<TracksQuery>::try_from_uri(&"/tracks?sort=plays".parse().unwrap())
                    .unwrap()
                    .0,
            )),
        )
        .await
        .unwrap()
        .0;
        assert_eq!(tr_page.total, 2);
        assert_eq!(tr_page.tracks[0].id, t2_id);

        let art_page = artists(
            auth.clone(),
            State(state.clone()),
            Ok(Path(lib.id.to_string())),
            Ok(Query(
                Query::<ArtistsQuery>::try_from_uri(&"/artists?favorite=true".parse().unwrap())
                    .unwrap()
                    .0,
            )),
        )
        .await
        .unwrap()
        .0;
        assert_eq!(art_page.total, 1);
        assert_eq!(art_page.artists[0].id, art_id);

        // Lookup preserves order and rejects >500 IDs.
        let looked = lookup_tracks(
            auth.clone(),
            State(state.clone()),
            ApiJson(MusicTrackLookupRequest {
                ids: vec![t2_id, t1_id, t2_id],
            }),
        )
        .await
        .unwrap()
        .0;
        assert_eq!(
            looked.tracks.iter().map(|t| t.id).collect::<Vec<_>>(),
            [t2_id, t1_id, t2_id]
        );
        let too_many = lookup_tracks(
            auth.clone(),
            State(state.clone()),
            ApiJson(MusicTrackLookupRequest {
                ids: (0..501).map(|_| ItemId::generate()).collect(),
            }),
        )
        .await
        .unwrap_err();
        assert_eq!(too_many.into_response().status(), StatusCode::BAD_REQUEST);

        // Unfavorite & 404 checks.
        assert_eq!(
            unfavorite_album(
                auth.clone(),
                State(state.clone()),
                Ok(Path((lib.id.to_string(), alb_id))),
            )
            .await
            .unwrap(),
            StatusCode::NO_CONTENT
        );
        assert_eq!(
            unfavorite_artist(
                auth.clone(),
                State(state.clone()),
                Ok(Path((lib.id.to_string(), art_id))),
            )
            .await
            .unwrap(),
            StatusCode::NO_CONTENT
        );
        let missing_alb = album_detail(
            auth.clone(),
            State(state.clone()),
            Ok(Path((lib.id.to_string(), "bad-id".into()))),
        )
        .await
        .unwrap_err();
        assert_eq!(missing_alb.into_response().status(), StatusCode::NOT_FOUND);

        db.close().await.unwrap();
        let _ = std::fs::remove_dir_all(&dir);
    }
}

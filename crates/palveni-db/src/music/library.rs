//! Rich music library catalog queries: albums, artists, genres, tracks, lookup
//! and collection favorites with per-user personal state.

use std::collections::{BTreeMap, HashMap, HashSet};

use palveni_core::{
    ItemId, LibraryId, UserId,
    music::{
        MusicAlbum, MusicAlbumDetail, MusicAlbumPage, MusicAlbumSort, MusicArtist,
        MusicArtistDetail, MusicArtistPage, MusicArtistSort, MusicGenre, MusicGenreList,
        MusicSortOrder, MusicTrack, MusicTrackPage, MusicTrackSort, MusicTrackState,
    },
};
use sea_orm::sea_query::{
    Alias, Asterisk, BinOper, Condition, Expr, ExprTrait, Func, JoinType, LikeExpr, Order, Query,
    SelectStatement,
};
use sea_orm::{
    ColumnTrait, ConnectionTrait, EntityTrait, FromQueryResult, QueryFilter, QueryOrder,
    QuerySelect,
};
use serde::{Deserialize, Serialize};

use super::{Music, SHUFFLE_MODULUS, album_id, visible_recordings};
use crate::{
    DbError,
    convert::{opt_ts, parse_id, sha256_hex, ts},
    entities::{
        items, libraries, music_albums as albums, music_collection_favorites as favs,
        subsonic_folders as folders, track_genres as genres, track_metadata as tracks,
        user_item_state as state,
    },
};

#[derive(Clone, Debug)]
pub struct AlbumListQuery {
    pub library_id: LibraryId,
    pub sort: MusicAlbumSort,
    pub order: MusicSortOrder,
    pub genre: Option<String>,
    pub year_from: Option<i32>,
    pub year_to: Option<i32>,
    pub artist_id: Option<String>,
    pub favorite_only: bool,
    pub q: Option<String>,
    pub seed: Option<String>,
    pub cursor: Option<CatalogCursor>,
    pub limit: u32,
}

#[derive(Clone, Debug)]
pub struct ArtistListQuery {
    pub library_id: LibraryId,
    pub sort: MusicArtistSort,
    pub order: MusicSortOrder,
    pub favorite_only: bool,
    pub q: Option<String>,
    pub seed: Option<String>,
    pub cursor: Option<CatalogCursor>,
    pub limit: u32,
}

#[derive(Clone, Debug)]
pub struct TrackListQuery {
    pub library_id: LibraryId,
    pub sort: MusicTrackSort,
    pub order: MusicSortOrder,
    pub genre: Option<String>,
    pub album: Option<(String, String)>,
    pub artist_id: Option<String>,
    pub favorite_only: bool,
    pub q: Option<String>,
    pub seed: Option<String>,
    pub cursor: Option<CatalogCursor>,
    pub limit: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "t", content = "v")]
enum CursorKey {
    Str(String),
    Int(i64),
    OptInt(Option<i64>),
}

/// Opaque, scope-bound keyset cursor for music album, artist and track lists.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogCursor {
    scope: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    seed: Option<String>,
    keys: Vec<CursorKey>,
}

impl CatalogCursor {
    fn encode(&self) -> Result<String, DbError> {
        serde_json::to_vec(self)
            .map(hex::encode)
            .map_err(|e| DbError::Corrupt(format!("encode music cursor: {e}")))
    }

    pub fn decode_albums(raw: &str, query: &AlbumListQuery) -> Option<Self> {
        Self::decode(
            raw,
            &album_scope(query),
            query.sort == MusicAlbumSort::Random,
        )
    }

    pub fn decode_artists(raw: &str, query: &ArtistListQuery) -> Option<Self> {
        Self::decode(
            raw,
            &artist_scope(query),
            query.sort == MusicArtistSort::Random,
        )
    }

    pub fn decode_tracks(raw: &str, query: &TrackListQuery) -> Option<Self> {
        Self::decode(
            raw,
            &track_scope(query),
            query.sort == MusicTrackSort::Random,
        )
    }

    fn decode(raw: &str, expected_scope: &str, random: bool) -> Option<Self> {
        if raw.len() > 2048 {
            return None;
        }
        let cursor: Self = serde_json::from_slice(&hex::decode(raw).ok()?).ok()?;
        if cursor.scope != expected_scope {
            return None;
        }
        if random != cursor.seed.is_some() {
            return None;
        }
        (!cursor.keys.is_empty()).then_some(cursor)
    }
}

fn album_scope(q: &AlbumListQuery) -> String {
    sha256_hex(&format!(
        "albums\0{}\0{:?}\0{:?}\0{}\0{:?}\0{:?}\0{}\0{}\0{}\0{}",
        q.library_id,
        q.sort,
        q.order,
        q.genre.as_deref().unwrap_or("").trim().to_lowercase(),
        q.year_from,
        q.year_to,
        q.artist_id.as_deref().unwrap_or(""),
        q.favorite_only,
        q.q.as_deref().unwrap_or("").trim().to_lowercase(),
        q.seed.as_deref().unwrap_or(""),
    ))
}

fn artist_scope(q: &ArtistListQuery) -> String {
    sha256_hex(&format!(
        "artists\0{}\0{:?}\0{:?}\0{}\0{}\0{}",
        q.library_id,
        q.sort,
        q.order,
        q.favorite_only,
        q.q.as_deref().unwrap_or("").trim().to_lowercase(),
        q.seed.as_deref().unwrap_or(""),
    ))
}

fn track_scope(q: &TrackListQuery) -> String {
    let (ak, bk) = q
        .album
        .as_ref()
        .map_or(("", ""), |(a, b)| (a.as_str(), b.as_str()));
    sha256_hex(&format!(
        "tracks\0{}\0{:?}\0{:?}\0{}\0{ak}\0{bk}\0{}\0{}\0{}\0{}",
        q.library_id,
        q.sort,
        q.order,
        q.genre.as_deref().unwrap_or("").trim().to_lowercase(),
        q.artist_id.as_deref().unwrap_or(""),
        q.favorite_only,
        q.q.as_deref().unwrap_or("").trim().to_lowercase(),
        q.seed.as_deref().unwrap_or(""),
    ))
}

fn sea_order(order: MusicSortOrder) -> Order {
    match order {
        MusicSortOrder::Asc => Order::Asc,
        MusicSortOrder::Desc => Order::Desc,
    }
}

fn shuffle_params(seed: &str) -> (i64, i64) {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(format!("palveni:music:shuffle:v1\0{seed}").as_bytes());
    let a = u32::from_be_bytes([digest[0], digest[1], digest[2], digest[3]]);
    let b = u32::from_be_bytes([digest[4], digest[5], digest[6], digest[7]]);
    // Keep the multiplier far from 0 and 1 so it is coprime to SHUFFLE_MODULUS
    // and scatters consecutive or small inputs uniformly across [0, SHUFFLE_MODULUS).
    let span = SHUFFLE_MODULUS - 131_074;
    let mul = 65_537 + (i64::from(a & 0x7fff_ffff) % span);
    let add = i64::from(b & 0x7fff_ffff) % SHUFFLE_MODULUS;
    (mul, add)
}

fn shuffle_rank(base: i64, mul: i64, add: i64) -> i64 {
    let base = base.rem_euclid(SHUFFLE_MODULUS);
    (base * mul + add) % SHUFFLE_MODULUS
}

fn shuffle_expr(col: Expr, mul: i64, add: i64) -> Expr {
    col.mul(mul)
        .add(add)
        .binary(BinOper::Mod, Expr::val(SHUFFLE_MODULUS))
}

fn effective_seed(explicit: Option<&str>, cursor: Option<&CatalogCursor>) -> String {
    if let Some(seed) = explicit {
        return seed.to_owned();
    }
    if let Some(seed) = cursor.and_then(|c| c.seed.as_deref()) {
        return seed.to_owned();
    }
    let mut bytes = [0u8; 8];
    let _ = getrandom::fill(&mut bytes);
    hex::encode(bytes)
}

fn like_pattern(term: &str) -> String {
    let literal = term
        .to_lowercase()
        .replace('!', "!!")
        .replace('%', "!%")
        .replace('_', "!_");
    format!("%{literal}%")
}

fn text_match(cols: &[Expr], term: &str) -> Condition {
    let pattern = like_pattern(term);
    cols.iter().fold(Condition::any(), |acc, col| {
        acc.add(Expr::expr(Func::lower(col.clone())).like(LikeExpr::new(&pattern).escape('!')))
    })
}

fn parse_i64_text(raw: &str) -> i64 {
    raw.trim().parse::<i64>().unwrap_or(0)
}

pub(crate) fn rounded_stars(star_sum: i64, star_count: i64) -> i32 {
    if star_count <= 0 || star_sum <= 0 {
        return 0;
    }
    let two_sum = star_sum * 2;
    if two_sum >= star_count * 9 {
        5
    } else if two_sum >= star_count * 7 {
        4
    } else if two_sum >= star_count * 5 {
        3
    } else if two_sum >= star_count * 3 {
        2
    } else {
        1
    }
}

fn sql_rounded_stars(star_sum: Expr, star_count: Expr) -> Expr {
    let two_sum = star_sum.mul(2i64);
    Expr::case(
        Condition::any()
            .add(star_count.clone().is_null())
            .add(star_count.clone().lte(0i64)),
        0i32,
    )
    .case(two_sum.clone().gte(star_count.clone().mul(9i64)), 5i32)
    .case(two_sum.clone().gte(star_count.clone().mul(7i64)), 4i32)
    .case(two_sum.clone().gte(star_count.clone().mul(5i64)), 3i32)
    .case(two_sum.gte(star_count.mul(3i64)), 2i32)
    .finally(1i32)
    .into()
}

fn lex_after(terms: &[(Expr, &CursorKey, MusicSortOrder)]) -> Condition {
    let Some(((expr, bound, order), rest)) = terms.split_first() else {
        return Condition::all();
    };
    let tail = (!rest.is_empty()).then(|| lex_after(rest));
    let strict_str = |e: Expr, v: &str| match order {
        MusicSortOrder::Asc => e.gt(v),
        MusicSortOrder::Desc => e.lt(v),
    };
    let strict_int = |e: Expr, v: i64| match order {
        MusicSortOrder::Asc => e.gt(v),
        MusicSortOrder::Desc => e.lt(v),
    };
    match bound {
        CursorKey::Str(v) => {
            let mut cond = Condition::any().add(strict_str(expr.clone(), v));
            if let Some(tail) = tail {
                cond = cond.add(Condition::all().add(expr.clone().eq(v.as_str())).add(tail));
            }
            cond
        }
        CursorKey::Int(v) => {
            let mut cond = Condition::any().add(strict_int(expr.clone(), *v));
            if let Some(tail) = tail {
                cond = cond.add(Condition::all().add(expr.clone().eq(*v)).add(tail));
            }
            cond
        }
        // NULLs sort last in both Asc and Desc orders on all backends.
        CursorKey::OptInt(Some(v)) => {
            let mut cond = Condition::any()
                .add(expr.clone().is_null())
                .add(strict_int(expr.clone(), *v));
            if let Some(tail) = tail {
                cond = cond.add(Condition::all().add(expr.clone().eq(*v)).add(tail));
            }
            cond
        }
        CursorKey::OptInt(None) => match tail {
            Some(tail) => Condition::all().add(expr.clone().is_null()).add(tail),
            None => Condition::all().add(Expr::val(0i32).eq(1i32)),
        },
    }
}

type OrderSpec = (Expr, bool, MusicSortOrder);
type ShuffleSeed = (String, i64, i64);

fn apply_order(stmt: &mut SelectStatement, terms: &[OrderSpec]) {
    for (expr, nullable, order) in terms {
        if *nullable {
            let null_last: Expr = Expr::case(expr.clone().is_null(), 1i32)
                .finally(0i32)
                .into();
            stmt.order_by_expr(null_last, Order::Asc);
        }
        stmt.order_by_expr(expr.clone(), sea_order(*order));
    }
}

fn apply_cursor(
    stmt: &mut SelectStatement,
    cursor: Option<&CatalogCursor>,
    specs: &[OrderSpec],
) -> Result<(), DbError> {
    let Some(cursor) = cursor else {
        return Ok(());
    };
    if cursor.keys.len() != specs.len() {
        return Err(DbError::Invalid("Invalid cursor"));
    }
    let terms: Vec<(Expr, &CursorKey, MusicSortOrder)> = specs
        .iter()
        .zip(cursor.keys.iter())
        .map(|((expr, _, order), key)| (expr.clone(), key, *order))
        .collect();
    stmt.cond_where(lex_after(&terms));
    Ok(())
}

async fn query_all_models<T: FromQueryResult>(
    conn: &impl ConnectionTrait,
    stmt: &SelectStatement,
) -> Result<Vec<T>, DbError> {
    let rows = conn.query_all(stmt).await?;
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        out.push(T::from_query_result(&row, "")?);
    }
    Ok(out)
}

async fn query_count(conn: &impl ConnectionTrait, stmt: &SelectStatement) -> Result<i64, DbError> {
    #[derive(FromQueryResult)]
    struct CountRow {
        total: String,
    }
    let Some(row) = conn.query_one(stmt).await? else {
        return Ok(0);
    };
    Ok(parse_i64_text(
        &CountRow::from_query_result(&row, "")?.total,
    ))
}

#[derive(Default, Clone, Copy)]
struct AlbumUserStats {
    play_count: i64,
    last_played_at: Option<i64>,
    stars: i32,
}

#[derive(FromQueryResult)]
struct AlbumRow {
    library_id: String,
    artist_key: String,
    album_key: String,
    name: Option<String>,
    name_sort: String,
    artist: Option<String>,
    artist_sort: String,
    year: Option<i64>,
    track_count: i64,
    disc_count: i64,
    duration_ms: i64,
    artwork_item_id: String,
    added_at: i64,
    shuffle: i64,
}

#[derive(FromQueryResult)]
struct AlbumUserRow {
    library_id: String,
    artist_key: String,
    album_key: String,
    name: Option<String>,
    name_sort: String,
    artist: Option<String>,
    artist_sort: String,
    year: Option<i64>,
    track_count: i64,
    disc_count: i64,
    duration_ms: i64,
    artwork_item_id: String,
    added_at: i64,
    shuffle: i64,
    play_count: String,
    last_played_at: Option<i64>,
    stars: String,
}

#[derive(FromQueryResult)]
struct ArtistRow {
    artist_key: String,
    artist_sort: String,
    name: Option<String>,
    album_count: String,
    track_count: String,
    artwork_item_id: String,
    base_shuffle: i64,
    play_count: Option<String>,
}

#[derive(FromQueryResult)]
struct TrackRow {
    item_id: String,
    library_id: String,
    title: String,
    artist: Option<String>,
    album_artist: Option<String>,
    album: Option<String>,
    artist_key: String,
    album_key: String,
    album_sort: String,
    track_sort: String,
    title_sort: Option<String>,
    track_artist_sort: Option<String>,
    artist_sort: String,
    track_number: Option<i64>,
    disc_number: Option<i64>,
    year: Option<i64>,
    duration_ms: Option<i64>,
    codec: Option<String>,
    bitrate_kbps: Option<i64>,
    sample_rate_hz: Option<i64>,
    bit_depth: Option<i64>,
    channels: Option<i64>,
    shuffle: Option<i64>,
    size_bytes: Option<i64>,
    created_at: i64,
    favorite_order: Option<i64>,
    stars: Option<i32>,
    play_count: Option<i64>,
    last_played_at: Option<i64>,
}

fn to_track(row: TrackRow, genres: Vec<String>) -> Result<MusicTrack, DbError> {
    let id: ItemId = parse_id(&row.item_id, "track id")?;
    let library_id: LibraryId = parse_id(&row.library_id, "library id")?;
    let alb_id = album_id(&row.artist_key, &row.album_key);
    Ok(MusicTrack {
        id,
        library_id,
        title: row.title,
        artist: row.artist,
        album_artist: row.album_artist,
        album: row.album,
        album_id: Some(alb_id),
        artist_id: Some(row.artist_key),
        track_number: row.track_number.and_then(|n| u32::try_from(n).ok()),
        disc_number: row.disc_number.and_then(|n| u32::try_from(n).ok()),
        year: row.year.and_then(|n| i32::try_from(n).ok()),
        genres,
        duration_ms: row.duration_ms,
        artwork_item_id: Some(id),
        codec: row.codec,
        bitrate_kbps: row.bitrate_kbps.and_then(|n| u32::try_from(n).ok()),
        sample_rate_hz: row.sample_rate_hz.and_then(|n| u32::try_from(n).ok()),
        bit_depth: row.bit_depth.and_then(|n| u8::try_from(n).ok()),
        channels: row.channels.and_then(|n| u8::try_from(n).ok()),
        size_bytes: row.size_bytes,
        added_at: ts(row.created_at)?,
        state: MusicTrackState {
            favorite: row.favorite_order.is_some(),
            stars: row.stars.unwrap_or(0),
            play_count: row.play_count.unwrap_or(0),
            last_played_at: opt_ts(row.last_played_at)?,
        },
    })
}

fn to_album(
    row: &AlbumRow,
    genres: Vec<String>,
    favorite: bool,
    stats: AlbumUserStats,
) -> Result<MusicAlbum, DbError> {
    Ok(MusicAlbum {
        id: album_id(&row.artist_key, &row.album_key),
        library_id: parse_id(&row.library_id, "library id")?,
        name: row.name.clone(),
        artist: row.artist.clone(),
        artist_id: row.artist_key.clone(),
        year: row.year.and_then(|n| i32::try_from(n).ok()),
        genres,
        track_count: row.track_count,
        disc_count: row.disc_count,
        duration_ms: row.duration_ms,
        artwork_item_id: Some(parse_id(&row.artwork_item_id, "artwork item id")?),
        added_at: ts(row.added_at)?,
        favorite,
        stars: stats.stars,
        play_count: stats.play_count,
        last_played_at: opt_ts(stats.last_played_at)?,
    })
}

fn album_filter_conditions(q: &AlbumListQuery, user: UserId, folder_id: i32) -> Condition {
    use albums::Column as A;
    let mut cond = Condition::all()
        .add(Expr::col((albums::Entity, A::LibraryId)).eq(q.library_id.to_string()));
    if let Some(artist) = q.artist_id.as_deref() {
        cond = cond.add(Expr::col((albums::Entity, A::ArtistKey)).eq(artist));
    }
    if let Some(from) = q.year_from {
        cond = cond.add(Expr::col((albums::Entity, A::Year)).gte(i64::from(from)));
    }
    if let Some(to) = q.year_to {
        cond = cond.add(Expr::col((albums::Entity, A::Year)).lte(i64::from(to)));
    }
    if let Some(genre) = q.genre.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        let gkey = sha256_hex(&genre.to_lowercase());
        let sub = Query::select()
            .expr(Expr::val(1i32))
            .from(genres::Entity)
            .and_where(
                Expr::col((genres::Entity, genres::Column::LibraryId)).eq(q.library_id.to_string()),
            )
            .and_where(Expr::col((genres::Entity, genres::Column::GenreKey)).eq(gkey))
            .and_where(
                Expr::col((genres::Entity, genres::Column::ArtistKey))
                    .equals((albums::Entity, A::ArtistKey)),
            )
            .and_where(
                Expr::col((genres::Entity, genres::Column::AlbumKey))
                    .equals((albums::Entity, A::AlbumKey)),
            )
            .to_owned();
        cond = cond.add(Expr::exists(sub));
    }
    if q.favorite_only {
        let sub = Query::select()
            .expr(Expr::val(1i32))
            .from(favs::Entity)
            .and_where(Expr::col((favs::Entity, favs::Column::UserId)).eq(user.to_string()))
            .and_where(Expr::col((favs::Entity, favs::Column::FolderId)).eq(folder_id))
            .and_where(
                Expr::col((favs::Entity, favs::Column::ArtistKey))
                    .equals((albums::Entity, A::ArtistKey)),
            )
            .and_where(
                Expr::col((favs::Entity, favs::Column::AlbumKey))
                    .equals((albums::Entity, A::AlbumKey)),
            )
            .to_owned();
        cond = cond.add(Expr::exists(sub));
    }
    if let Some(term) = q.q.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        cond = cond.add(text_match(
            &[
                Expr::col((albums::Entity, A::Name)),
                Expr::col((albums::Entity, A::Artist)),
            ],
            term,
        ));
    }
    cond
}

fn album_user_stats_subquery(user: UserId, library_id: LibraryId) -> SelectStatement {
    use state::Column as S;
    use tracks::Column as T;
    let rated_stars: Expr = Expr::case(
        Expr::col((state::Entity, S::Stars)).gt(0i32),
        Expr::col((state::Entity, S::Stars)),
    )
    .finally(0i32)
    .into();
    let rated_one: Expr = Expr::case(Expr::col((state::Entity, S::Stars)).gt(0i32), 1i32)
        .finally(0i32)
        .into();
    let mut sub = Query::select();
    sub.from(state::Entity)
        .inner_join(
            tracks::Entity,
            Expr::col((tracks::Entity, T::ItemId)).equals((state::Entity, S::ItemId)),
        )
        .and_where(Expr::col((state::Entity, S::UserId)).eq(user.to_string()))
        .and_where(Expr::col((tracks::Entity, T::LibraryId)).eq(library_id.to_string()))
        .cond_where(visible_recordings())
        .column((tracks::Entity, T::ArtistKey))
        .column((tracks::Entity, T::AlbumKey))
        .expr_as(
            Expr::col((state::Entity, S::PlayCount)).sum(),
            Alias::new("play_count"),
        )
        .expr_as(
            Expr::col((state::Entity, S::LastPlayedAt)).max(),
            Alias::new("last_played_at"),
        )
        .expr_as(rated_stars.sum(), Alias::new("star_sum"))
        .expr_as(rated_one.sum(), Alias::new("star_count"))
        .group_by_columns([
            (tracks::Entity, T::ArtistKey),
            (tracks::Entity, T::AlbumKey),
        ]);
    sub
}

fn artist_play_counts_subquery(user: UserId, library_id: LibraryId) -> SelectStatement {
    use state::Column as S;
    use tracks::Column as T;
    let mut sub = Query::select();
    sub.from(state::Entity)
        .inner_join(
            tracks::Entity,
            Expr::col((tracks::Entity, T::ItemId)).equals((state::Entity, S::ItemId)),
        )
        .and_where(Expr::col((state::Entity, S::UserId)).eq(user.to_string()))
        .and_where(Expr::col((tracks::Entity, T::LibraryId)).eq(library_id.to_string()))
        .cond_where(visible_recordings())
        .column((tracks::Entity, T::ArtistKey))
        .expr_as(
            Expr::col((state::Entity, S::PlayCount)).sum(),
            Alias::new("play_count"),
        )
        .group_by_col((tracks::Entity, T::ArtistKey));
    sub
}

fn track_select_base(user: UserId, join_type: JoinType) -> SelectStatement {
    use items::Column as I;
    use state::Column as S;
    use tracks::Column as T;
    let mut stmt = Query::select();
    stmt.from(tracks::Entity)
        .inner_join(
            items::Entity,
            Expr::col((items::Entity, I::Id)).equals((tracks::Entity, T::ItemId)),
        )
        .join(
            join_type,
            state::Entity,
            Condition::all()
                .add(Expr::col((state::Entity, S::ItemId)).equals((tracks::Entity, T::ItemId)))
                .add(Expr::col((state::Entity, S::UserId)).eq(user.to_string())),
        )
        .column((tracks::Entity, T::ItemId))
        .column((tracks::Entity, T::LibraryId))
        .column((tracks::Entity, T::Title))
        .column((tracks::Entity, T::Artist))
        .column((tracks::Entity, T::AlbumArtist))
        .column((tracks::Entity, T::Album))
        .column((tracks::Entity, T::ArtistKey))
        .column((tracks::Entity, T::AlbumKey))
        .column((tracks::Entity, T::AlbumSort))
        .column((tracks::Entity, T::TrackSort))
        .column((tracks::Entity, T::TitleSort))
        .column((tracks::Entity, T::TrackArtistSort))
        .column((tracks::Entity, T::ArtistSort))
        .column((tracks::Entity, T::TrackNumber))
        .column((tracks::Entity, T::DiscNumber))
        .column((tracks::Entity, T::Year))
        .column((tracks::Entity, T::DurationMs))
        .column((tracks::Entity, T::Codec))
        .column((tracks::Entity, T::BitrateKbps))
        .column((tracks::Entity, T::SampleRateHz))
        .column((tracks::Entity, T::BitDepth))
        .column((tracks::Entity, T::Channels))
        .column((tracks::Entity, T::Shuffle))
        .column((items::Entity, I::SizeBytes))
        .column((items::Entity, I::CreatedAt))
        .column((state::Entity, S::FavoriteOrder))
        .column((state::Entity, S::Stars))
        .column((state::Entity, S::PlayCount))
        .column((state::Entity, S::LastPlayedAt));
    stmt
}

impl Music<'_> {
    async fn music_folder_id(&self, library: LibraryId) -> Result<Option<i32>, DbError> {
        Ok(folders::Entity::find()
            .select_only()
            .column(folders::Column::Id)
            .join(
                sea_orm::JoinType::InnerJoin,
                sea_orm::RelationTrait::def(&folders::Relation::Library),
            )
            .filter(folders::Column::LibraryId.eq(library.to_string()))
            .filter(libraries::Column::Kind.eq("music"))
            .filter(libraries::Column::DeletedAt.is_null())
            .into_tuple::<i32>()
            .one(self.db.main())
            .await?)
    }

    async fn load_album_genres(
        &self,
        library: LibraryId,
        pairs: &[(String, String)],
    ) -> Result<HashMap<(String, String), Vec<String>>, DbError> {
        #[derive(FromQueryResult)]
        struct Row {
            artist_key: String,
            album_key: String,
            genre: String,
        }
        let mut map: HashMap<(String, String), Vec<String>> = HashMap::new();
        let lib_str = library.to_string();
        for chunk in pairs.chunks(100) {
            let cond = chunk.iter().fold(Condition::any(), |acc, (ak, bk)| {
                acc.add(
                    Condition::all()
                        .add(Expr::col((genres::Entity, genres::Column::ArtistKey)).eq(ak.as_str()))
                        .add(Expr::col((genres::Entity, genres::Column::AlbumKey)).eq(bk.as_str())),
                )
            });
            let mut stmt = Query::select();
            stmt.from(genres::Entity)
                .column(genres::Column::ArtistKey)
                .column(genres::Column::AlbumKey)
                .expr_as(Expr::col(genres::Column::Genre).min(), Alias::new("genre"))
                .and_where(Expr::col(genres::Column::LibraryId).eq(lib_str.as_str()))
                .cond_where(cond)
                .group_by_columns([
                    genres::Column::ArtistKey,
                    genres::Column::AlbumKey,
                    genres::Column::GenreSort,
                    genres::Column::GenreKey,
                ])
                .order_by(genres::Column::GenreSort, Order::Asc)
                .order_by(genres::Column::GenreKey, Order::Asc);
            for row in query_all_models::<Row>(self.db.main(), &stmt).await? {
                map.entry((row.artist_key, row.album_key))
                    .or_default()
                    .push(row.genre);
            }
        }
        Ok(map)
    }

    async fn load_track_genres(
        &self,
        item_ids: &[String],
    ) -> Result<HashMap<String, Vec<String>>, DbError> {
        let mut map: HashMap<String, Vec<String>> = HashMap::new();
        for chunk in item_ids.chunks(200) {
            let rows = genres::Entity::find()
                .select_only()
                .column(genres::Column::ItemId)
                .column(genres::Column::Genre)
                .filter(genres::Column::ItemId.is_in(chunk.iter().cloned()))
                .order_by_asc(genres::Column::GenreSort)
                .order_by_asc(genres::Column::GenreKey)
                .into_tuple::<(String, String)>()
                .all(self.db.main())
                .await?;
            for (id, genre) in rows {
                map.entry(id).or_default().push(genre);
            }
        }
        Ok(map)
    }

    async fn load_album_favorites(
        &self,
        user: UserId,
        folder_id: i32,
        pairs: &[(String, String)],
    ) -> Result<HashSet<(String, String)>, DbError> {
        let mut set = HashSet::new();
        for chunk in pairs.chunks(100) {
            let cond = chunk.iter().fold(Condition::any(), |acc, (ak, bk)| {
                acc.add(
                    Condition::all()
                        .add(favs::Column::ArtistKey.eq(ak.as_str()))
                        .add(favs::Column::AlbumKey.eq(bk.as_str())),
                )
            });
            let rows = favs::Entity::find()
                .select_only()
                .columns([favs::Column::ArtistKey, favs::Column::AlbumKey])
                .filter(favs::Column::UserId.eq(user.to_string()))
                .filter(favs::Column::FolderId.eq(folder_id))
                .filter(cond)
                .into_tuple::<(String, String)>()
                .all(self.db.main())
                .await?;
            set.extend(rows);
        }
        Ok(set)
    }

    async fn load_artist_favorites(
        &self,
        user: UserId,
        folder_id: i32,
        artists: &[String],
    ) -> Result<HashSet<String>, DbError> {
        let mut set = HashSet::new();
        for chunk in artists.chunks(200) {
            let rows = favs::Entity::find()
                .select_only()
                .column(favs::Column::ArtistKey)
                .filter(favs::Column::UserId.eq(user.to_string()))
                .filter(favs::Column::FolderId.eq(folder_id))
                .filter(favs::Column::AlbumKey.eq(""))
                .filter(favs::Column::ArtistKey.is_in(chunk.iter().cloned()))
                .into_tuple::<String>()
                .all(self.db.main())
                .await?;
            set.extend(rows);
        }
        Ok(set)
    }

    async fn load_album_user_stats(
        &self,
        user: UserId,
        library: LibraryId,
        pairs: &[(String, String)],
    ) -> Result<HashMap<(String, String), AlbumUserStats>, DbError> {
        #[derive(FromQueryResult)]
        struct Row {
            artist_key: String,
            album_key: String,
            play_count: Option<String>,
            last_played_at: Option<i64>,
            star_sum: Option<String>,
            star_count: Option<String>,
        }
        let mut map = HashMap::new();
        for chunk in pairs.chunks(100) {
            let cond = chunk.iter().fold(Condition::any(), |acc, (ak, bk)| {
                acc.add(
                    Condition::all()
                        .add(Expr::col((tracks::Entity, tracks::Column::ArtistKey)).eq(ak.as_str()))
                        .add(Expr::col((tracks::Entity, tracks::Column::AlbumKey)).eq(bk.as_str())),
                )
            });
            let rated_stars: Expr = Expr::case(
                Expr::col((state::Entity, state::Column::Stars)).gt(0i32),
                Expr::col((state::Entity, state::Column::Stars)),
            )
            .finally(0i32)
            .into();
            let rated_one: Expr = Expr::case(
                Expr::col((state::Entity, state::Column::Stars)).gt(0i32),
                1i32,
            )
            .finally(0i32)
            .into();
            let mut stmt = Query::select();
            stmt.from(state::Entity)
                .inner_join(
                    tracks::Entity,
                    Expr::col((tracks::Entity, tracks::Column::ItemId))
                        .equals((state::Entity, state::Column::ItemId)),
                )
                .and_where(Expr::col((state::Entity, state::Column::UserId)).eq(user.to_string()))
                .and_where(
                    Expr::col((tracks::Entity, tracks::Column::LibraryId)).eq(library.to_string()),
                )
                .cond_where(visible_recordings())
                .cond_where(cond)
                .column((tracks::Entity, tracks::Column::ArtistKey))
                .column((tracks::Entity, tracks::Column::AlbumKey))
                .expr_as(
                    Expr::col((state::Entity, state::Column::PlayCount))
                        .sum()
                        .cast_as("CHAR(24)"),
                    Alias::new("play_count"),
                )
                .expr_as(
                    Expr::col((state::Entity, state::Column::LastPlayedAt)).max(),
                    Alias::new("last_played_at"),
                )
                .expr_as(
                    rated_stars.sum().cast_as("CHAR(24)"),
                    Alias::new("star_sum"),
                )
                .expr_as(
                    rated_one.sum().cast_as("CHAR(24)"),
                    Alias::new("star_count"),
                )
                .group_by_columns([
                    (tracks::Entity, tracks::Column::ArtistKey),
                    (tracks::Entity, tracks::Column::AlbumKey),
                ]);
            for row in query_all_models::<Row>(self.db.main(), &stmt).await? {
                let star_sum = row.star_sum.as_deref().map_or(0, parse_i64_text);
                let star_count = row.star_count.as_deref().map_or(0, parse_i64_text);
                map.insert(
                    (row.artist_key, row.album_key),
                    AlbumUserStats {
                        play_count: row.play_count.as_deref().map_or(0, parse_i64_text),
                        last_played_at: row.last_played_at,
                        stars: rounded_stars(star_sum, star_count),
                    },
                );
            }
        }
        Ok(map)
    }

    async fn load_artist_play_counts(
        &self,
        user: UserId,
        library: LibraryId,
        artists: &[String],
    ) -> Result<HashMap<String, i64>, DbError> {
        #[derive(FromQueryResult)]
        struct Row {
            artist_key: String,
            play_count: Option<String>,
        }
        let mut map = HashMap::new();
        for chunk in artists.chunks(200) {
            let mut stmt = Query::select();
            stmt.from(state::Entity)
                .inner_join(
                    tracks::Entity,
                    Expr::col((tracks::Entity, tracks::Column::ItemId))
                        .equals((state::Entity, state::Column::ItemId)),
                )
                .and_where(Expr::col((state::Entity, state::Column::UserId)).eq(user.to_string()))
                .and_where(
                    Expr::col((tracks::Entity, tracks::Column::LibraryId)).eq(library.to_string()),
                )
                .cond_where(visible_recordings())
                .and_where(
                    Expr::col((tracks::Entity, tracks::Column::ArtistKey))
                        .is_in(chunk.iter().cloned()),
                )
                .column((tracks::Entity, tracks::Column::ArtistKey))
                .expr_as(
                    Expr::col((state::Entity, state::Column::PlayCount))
                        .sum()
                        .cast_as("CHAR(24)"),
                    Alias::new("play_count"),
                )
                .group_by_col((tracks::Entity, tracks::Column::ArtistKey));
            for row in query_all_models::<Row>(self.db.main(), &stmt).await? {
                map.insert(
                    row.artist_key,
                    row.play_count.as_deref().map_or(0, parse_i64_text),
                );
            }
        }
        Ok(map)
    }

    pub async fn list_albums(
        &self,
        user: UserId,
        query: &AlbumListQuery,
    ) -> Result<Option<MusicAlbumPage>, DbError> {
        let Some(folder_id) = self.music_folder_id(query.library_id).await? else {
            return Ok(None);
        };
        self.refresh_albums(query.library_id).await?;
        let limit = query.limit.clamp(1, 200) as usize;
        let filters = album_filter_conditions(query, user, folder_id);

        let mut count_stmt = Query::select();
        count_stmt
            .from(albums::Entity)
            .expr_as(
                Expr::col(Asterisk).count().cast_as("CHAR(24)"),
                Alias::new("total"),
            )
            .cond_where(filters.clone());
        let total = query_count(self.db.main(), &count_stmt).await?;
        if total == 0 {
            return Ok(Some(MusicAlbumPage {
                albums: Vec::new(),
                next_cursor: None,
                total: 0,
            }));
        }

        let scope = album_scope(query);
        let user_sorted = matches!(
            query.sort,
            MusicAlbumSort::Played | MusicAlbumSort::Plays | MusicAlbumSort::Rating
        );

        let (base_rows, preloaded_stats, seed_used) = if !user_sorted {
            use albums::Column as A;
            let mut stmt = Query::select();
            stmt.from(albums::Entity)
                .columns([
                    (albums::Entity, A::LibraryId),
                    (albums::Entity, A::ArtistKey),
                    (albums::Entity, A::AlbumKey),
                    (albums::Entity, A::Name),
                    (albums::Entity, A::NameSort),
                    (albums::Entity, A::Artist),
                    (albums::Entity, A::ArtistSort),
                    (albums::Entity, A::Year),
                    (albums::Entity, A::TrackCount),
                    (albums::Entity, A::DiscCount),
                    (albums::Entity, A::DurationMs),
                    (albums::Entity, A::ArtworkItemId),
                    (albums::Entity, A::AddedAt),
                    (albums::Entity, A::Shuffle),
                ])
                .cond_where(filters);

            let (specs, seed_used): (Vec<OrderSpec>, Option<ShuffleSeed>) = match query.sort {
                MusicAlbumSort::Title => (
                    vec![
                        (Expr::col((albums::Entity, A::NameSort)), false, query.order),
                        (
                            Expr::col((albums::Entity, A::ArtistKey)),
                            false,
                            MusicSortOrder::Asc,
                        ),
                        (
                            Expr::col((albums::Entity, A::AlbumKey)),
                            false,
                            MusicSortOrder::Asc,
                        ),
                    ],
                    None,
                ),
                MusicAlbumSort::Artist => (
                    vec![
                        (
                            Expr::col((albums::Entity, A::ArtistSort)),
                            false,
                            query.order,
                        ),
                        (
                            Expr::col((albums::Entity, A::NameSort)),
                            false,
                            MusicSortOrder::Asc,
                        ),
                        (
                            Expr::col((albums::Entity, A::ArtistKey)),
                            false,
                            MusicSortOrder::Asc,
                        ),
                        (
                            Expr::col((albums::Entity, A::AlbumKey)),
                            false,
                            MusicSortOrder::Asc,
                        ),
                    ],
                    None,
                ),
                MusicAlbumSort::Year => (
                    vec![
                        (Expr::col((albums::Entity, A::Year)), true, query.order),
                        (
                            Expr::col((albums::Entity, A::NameSort)),
                            false,
                            MusicSortOrder::Asc,
                        ),
                        (
                            Expr::col((albums::Entity, A::ArtistKey)),
                            false,
                            MusicSortOrder::Asc,
                        ),
                        (
                            Expr::col((albums::Entity, A::AlbumKey)),
                            false,
                            MusicSortOrder::Asc,
                        ),
                    ],
                    None,
                ),
                MusicAlbumSort::Added => (
                    vec![
                        (Expr::col((albums::Entity, A::AddedAt)), false, query.order),
                        (
                            Expr::col((albums::Entity, A::ArtistKey)),
                            false,
                            MusicSortOrder::Asc,
                        ),
                        (
                            Expr::col((albums::Entity, A::AlbumKey)),
                            false,
                            MusicSortOrder::Asc,
                        ),
                    ],
                    None,
                ),
                MusicAlbumSort::Random => {
                    let seed = effective_seed(query.seed.as_deref(), query.cursor.as_ref());
                    let (mul, add) = shuffle_params(&seed);
                    (
                        vec![
                            (
                                shuffle_expr(Expr::col((albums::Entity, A::Shuffle)), mul, add),
                                false,
                                query.order,
                            ),
                            (
                                Expr::col((albums::Entity, A::ArtistKey)),
                                false,
                                MusicSortOrder::Asc,
                            ),
                            (
                                Expr::col((albums::Entity, A::AlbumKey)),
                                false,
                                MusicSortOrder::Asc,
                            ),
                        ],
                        Some((seed, mul, add)),
                    )
                }
                MusicAlbumSort::Played | MusicAlbumSort::Plays | MusicAlbumSort::Rating => {
                    unreachable!()
                }
            };
            apply_cursor(&mut stmt, query.cursor.as_ref(), &specs)?;
            apply_order(&mut stmt, &specs);
            stmt.limit((limit + 1) as u64);
            let rows = query_all_models::<AlbumRow>(self.db.main(), &stmt).await?;
            (rows, None, seed_used)
        } else {
            use albums::Column as A;
            let stats_alias = Alias::new("u");
            let sub_alias = Alias::new("sub");
            let mut inner = Query::select();
            inner
                .from(albums::Entity)
                .join_subquery(
                    JoinType::LeftJoin,
                    album_user_stats_subquery(user, query.library_id),
                    stats_alias.clone(),
                    Condition::all()
                        .add(
                            Expr::col((stats_alias.clone(), Alias::new("artist_key")))
                                .equals((albums::Entity, A::ArtistKey)),
                        )
                        .add(
                            Expr::col((stats_alias.clone(), Alias::new("album_key")))
                                .equals((albums::Entity, A::AlbumKey)),
                        ),
                )
                .cond_where(filters)
                .columns([
                    (albums::Entity, A::LibraryId),
                    (albums::Entity, A::ArtistKey),
                    (albums::Entity, A::AlbumKey),
                    (albums::Entity, A::Name),
                    (albums::Entity, A::NameSort),
                    (albums::Entity, A::Artist),
                    (albums::Entity, A::ArtistSort),
                    (albums::Entity, A::Year),
                    (albums::Entity, A::TrackCount),
                    (albums::Entity, A::DiscCount),
                    (albums::Entity, A::DurationMs),
                    (albums::Entity, A::ArtworkItemId),
                    (albums::Entity, A::AddedAt),
                    (albums::Entity, A::Shuffle),
                ])
                .expr_as(
                    Func::coalesce([
                        Expr::col((stats_alias.clone(), Alias::new("play_count"))),
                        Expr::val(0i64),
                    ]),
                    Alias::new("play_count"),
                )
                .expr_as(
                    Expr::col((stats_alias.clone(), Alias::new("last_played_at"))),
                    Alias::new("last_played_at"),
                )
                .expr_as(
                    sql_rounded_stars(
                        Expr::col((stats_alias.clone(), Alias::new("star_sum"))),
                        Expr::col((stats_alias, Alias::new("star_count"))),
                    ),
                    Alias::new("stars"),
                );

            let mut outer = Query::select();
            outer
                .from_subquery(inner, sub_alias.clone())
                .column((sub_alias.clone(), Alias::new("library_id")))
                .column((sub_alias.clone(), Alias::new("artist_key")))
                .column((sub_alias.clone(), Alias::new("album_key")))
                .column((sub_alias.clone(), Alias::new("name")))
                .column((sub_alias.clone(), Alias::new("name_sort")))
                .column((sub_alias.clone(), Alias::new("artist")))
                .column((sub_alias.clone(), Alias::new("artist_sort")))
                .column((sub_alias.clone(), Alias::new("year")))
                .column((sub_alias.clone(), Alias::new("track_count")))
                .column((sub_alias.clone(), Alias::new("disc_count")))
                .column((sub_alias.clone(), Alias::new("duration_ms")))
                .column((sub_alias.clone(), Alias::new("artwork_item_id")))
                .column((sub_alias.clone(), Alias::new("added_at")))
                .column((sub_alias.clone(), Alias::new("shuffle")))
                .expr_as(
                    Expr::col((sub_alias.clone(), Alias::new("play_count"))).cast_as("CHAR(24)"),
                    Alias::new("play_count"),
                )
                .column((sub_alias.clone(), Alias::new("last_played_at")))
                .expr_as(
                    Expr::col((sub_alias.clone(), Alias::new("stars"))).cast_as("CHAR(24)"),
                    Alias::new("stars"),
                );

            let specs = match query.sort {
                MusicAlbumSort::Played => vec![
                    (
                        Expr::col((sub_alias.clone(), Alias::new("last_played_at"))),
                        true,
                        query.order,
                    ),
                    (
                        Expr::col((sub_alias.clone(), Alias::new("artist_key"))),
                        false,
                        MusicSortOrder::Asc,
                    ),
                    (
                        Expr::col((sub_alias, Alias::new("album_key"))),
                        false,
                        MusicSortOrder::Asc,
                    ),
                ],
                MusicAlbumSort::Plays => vec![
                    (
                        Expr::col((sub_alias.clone(), Alias::new("play_count"))),
                        false,
                        query.order,
                    ),
                    (
                        Expr::col((sub_alias.clone(), Alias::new("artist_key"))),
                        false,
                        MusicSortOrder::Asc,
                    ),
                    (
                        Expr::col((sub_alias, Alias::new("album_key"))),
                        false,
                        MusicSortOrder::Asc,
                    ),
                ],
                MusicAlbumSort::Rating => vec![
                    (
                        Expr::col((sub_alias.clone(), Alias::new("stars"))),
                        false,
                        query.order,
                    ),
                    (
                        Expr::col((sub_alias.clone(), Alias::new("artist_key"))),
                        false,
                        MusicSortOrder::Asc,
                    ),
                    (
                        Expr::col((sub_alias, Alias::new("album_key"))),
                        false,
                        MusicSortOrder::Asc,
                    ),
                ],
                _ => unreachable!(),
            };
            apply_cursor(&mut outer, query.cursor.as_ref(), &specs)?;
            apply_order(&mut outer, &specs);
            outer.limit((limit + 1) as u64);
            let rows = query_all_models::<AlbumUserRow>(self.db.main(), &outer).await?;
            let mut base = Vec::with_capacity(rows.len());
            let mut stats = HashMap::with_capacity(rows.len());
            for r in rows {
                stats.insert(
                    (r.artist_key.clone(), r.album_key.clone()),
                    AlbumUserStats {
                        play_count: parse_i64_text(&r.play_count),
                        last_played_at: r.last_played_at,
                        stars: i32::try_from(parse_i64_text(&r.stars)).unwrap_or(0),
                    },
                );
                base.push(AlbumRow {
                    library_id: r.library_id,
                    artist_key: r.artist_key,
                    album_key: r.album_key,
                    name: r.name,
                    name_sort: r.name_sort,
                    artist: r.artist,
                    artist_sort: r.artist_sort,
                    year: r.year,
                    track_count: r.track_count,
                    disc_count: r.disc_count,
                    duration_ms: r.duration_ms,
                    artwork_item_id: r.artwork_item_id,
                    added_at: r.added_at,
                    shuffle: r.shuffle,
                });
            }
            (base, Some(stats), None)
        };

        let mut rows = base_rows;
        let has_more = rows.len() > limit;
        rows.truncate(limit);

        let pairs: Vec<(String, String)> = rows
            .iter()
            .map(|r| (r.artist_key.clone(), r.album_key.clone()))
            .collect();
        let mut genre_map = self.load_album_genres(query.library_id, &pairs).await?;
        let fav_set = if query.favorite_only {
            pairs.iter().cloned().collect()
        } else {
            self.load_album_favorites(user, folder_id, &pairs).await?
        };
        let stats_map = match preloaded_stats {
            Some(map) => map,
            None => {
                self.load_album_user_stats(user, query.library_id, &pairs)
                    .await?
            }
        };

        let next_cursor = if has_more {
            let last = rows.last().expect("non-empty page");
            let key = (last.artist_key.clone(), last.album_key.clone());
            let st = stats_map.get(&key).copied().unwrap_or_default();
            let keys = match query.sort {
                MusicAlbumSort::Title => vec![
                    CursorKey::Str(last.name_sort.clone()),
                    CursorKey::Str(last.artist_key.clone()),
                    CursorKey::Str(last.album_key.clone()),
                ],
                MusicAlbumSort::Artist => vec![
                    CursorKey::Str(last.artist_sort.clone()),
                    CursorKey::Str(last.name_sort.clone()),
                    CursorKey::Str(last.artist_key.clone()),
                    CursorKey::Str(last.album_key.clone()),
                ],
                MusicAlbumSort::Year => vec![
                    CursorKey::OptInt(last.year),
                    CursorKey::Str(last.name_sort.clone()),
                    CursorKey::Str(last.artist_key.clone()),
                    CursorKey::Str(last.album_key.clone()),
                ],
                MusicAlbumSort::Added => vec![
                    CursorKey::Int(last.added_at),
                    CursorKey::Str(last.artist_key.clone()),
                    CursorKey::Str(last.album_key.clone()),
                ],
                MusicAlbumSort::Played => vec![
                    CursorKey::OptInt(st.last_played_at),
                    CursorKey::Str(last.artist_key.clone()),
                    CursorKey::Str(last.album_key.clone()),
                ],
                MusicAlbumSort::Plays => vec![
                    CursorKey::Int(st.play_count),
                    CursorKey::Str(last.artist_key.clone()),
                    CursorKey::Str(last.album_key.clone()),
                ],
                MusicAlbumSort::Rating => vec![
                    CursorKey::Int(i64::from(st.stars)),
                    CursorKey::Str(last.artist_key.clone()),
                    CursorKey::Str(last.album_key.clone()),
                ],
                MusicAlbumSort::Random => {
                    let (_, mul, add) = seed_used.as_ref().expect("random seed");
                    vec![
                        CursorKey::Int(shuffle_rank(last.shuffle, *mul, *add)),
                        CursorKey::Str(last.artist_key.clone()),
                        CursorKey::Str(last.album_key.clone()),
                    ]
                }
            };
            Some(
                CatalogCursor {
                    scope,
                    seed: seed_used.map(|(s, _, _)| s),
                    keys,
                }
                .encode()?,
            )
        } else {
            None
        };

        let mut albums_out = Vec::with_capacity(rows.len());
        for row in &rows {
            let key = (row.artist_key.clone(), row.album_key.clone());
            let genres = genre_map.remove(&key).unwrap_or_default();
            let favorite = fav_set.contains(&key);
            let stats = stats_map.get(&key).copied().unwrap_or_default();
            albums_out.push(to_album(row, genres, favorite, stats)?);
        }
        Ok(Some(MusicAlbumPage {
            albums: albums_out,
            next_cursor,
            total,
        }))
    }

    pub async fn get_album_detail(
        &self,
        user: UserId,
        library: LibraryId,
        artist_key: &str,
        album_key: &str,
    ) -> Result<Option<MusicAlbumDetail>, DbError> {
        let Some(folder_id) = self.music_folder_id(library).await? else {
            return Ok(None);
        };
        self.refresh_albums(library).await?;
        let Some(album_model) = albums::Entity::find_by_id((
            library.to_string(),
            artist_key.to_owned(),
            album_key.to_owned(),
        ))
        .one(self.db.main())
        .await?
        else {
            return Ok(None);
        };

        let mut stmt = track_select_base(user, JoinType::LeftJoin);
        stmt.cond_where(visible_recordings())
            .and_where(
                Expr::col((tracks::Entity, tracks::Column::LibraryId)).eq(library.to_string()),
            )
            .and_where(Expr::col((tracks::Entity, tracks::Column::ArtistKey)).eq(artist_key))
            .and_where(Expr::col((tracks::Entity, tracks::Column::AlbumKey)).eq(album_key))
            .and_where(Expr::col((items::Entity, items::Column::DeletedAt)).is_null())
            .and_where(Expr::col((items::Entity, items::Column::Kind)).eq("track"))
            .order_by((tracks::Entity, tracks::Column::TrackSort), Order::Asc)
            .order_by((tracks::Entity, tracks::Column::ItemId), Order::Asc);
        let track_rows = query_all_models::<TrackRow>(self.db.main(), &stmt).await?;

        let genre_rows = genres::Entity::find()
            .select_only()
            .columns([
                genres::Column::ItemId,
                genres::Column::GenreSort,
                genres::Column::GenreKey,
                genres::Column::Genre,
            ])
            .filter(genres::Column::LibraryId.eq(library.to_string()))
            .filter(genres::Column::ArtistKey.eq(artist_key))
            .filter(genres::Column::AlbumKey.eq(album_key))
            .order_by_asc(genres::Column::GenreSort)
            .order_by_asc(genres::Column::GenreKey)
            .into_tuple::<(String, String, String, String)>()
            .all(self.db.main())
            .await?;
        let mut per_track_genres: HashMap<String, Vec<String>> = HashMap::new();
        let mut album_genres_map: BTreeMap<(String, String), String> = BTreeMap::new();
        for (item_id, gsort, gkey, gname) in genre_rows {
            per_track_genres
                .entry(item_id)
                .or_default()
                .push(gname.clone());
            album_genres_map
                .entry((gsort, gkey))
                .and_modify(|existing| {
                    if gname < *existing {
                        *existing = gname.clone();
                    }
                })
                .or_insert(gname);
        }

        let mut play_count = 0i64;
        let mut last_played_at: Option<i64> = None;
        let mut star_sum = 0i64;
        let mut star_count = 0i64;
        let mut tracks_out = Vec::with_capacity(track_rows.len());
        for row in track_rows {
            play_count += row.play_count.unwrap_or(0);
            if let Some(at) = row.last_played_at {
                last_played_at = Some(last_played_at.map_or(at, |prev| std::cmp::max(prev, at)));
            }
            if let Some(s) = row.stars
                && s > 0
            {
                star_sum += i64::from(s);
                star_count += 1;
            }
            let genres = per_track_genres.remove(&row.item_id).unwrap_or_default();
            tracks_out.push(to_track(row, genres)?);
        }

        let favorite = favs::Entity::find_by_id((
            user.to_string(),
            folder_id,
            artist_key.to_owned(),
            album_key.to_owned(),
        ))
        .one(self.db.main())
        .await?
        .is_some();

        let album_row = AlbumRow {
            library_id: album_model.library_id,
            artist_key: album_model.artist_key,
            album_key: album_model.album_key,
            name: album_model.name,
            name_sort: album_model.name_sort,
            artist: album_model.artist,
            artist_sort: album_model.artist_sort,
            year: album_model.year,
            track_count: album_model.track_count,
            disc_count: album_model.disc_count,
            duration_ms: album_model.duration_ms,
            artwork_item_id: album_model.artwork_item_id,
            added_at: album_model.added_at,
            shuffle: album_model.shuffle,
        };
        let album = to_album(
            &album_row,
            album_genres_map.into_values().collect(),
            favorite,
            AlbumUserStats {
                play_count,
                last_played_at,
                stars: rounded_stars(star_sum, star_count),
            },
        )?;
        Ok(Some(MusicAlbumDetail {
            album,
            tracks: tracks_out,
        }))
    }

    pub async fn list_artists(
        &self,
        user: UserId,
        query: &ArtistListQuery,
    ) -> Result<Option<MusicArtistPage>, DbError> {
        let Some(folder_id) = self.music_folder_id(query.library_id).await? else {
            return Ok(None);
        };
        self.refresh_albums(query.library_id).await?;
        let limit = query.limit.clamp(1, 200) as usize;

        use albums::Column as A;
        let mut base_cond = Condition::all()
            .add(Expr::col((albums::Entity, A::LibraryId)).eq(query.library_id.to_string()));
        if query.favorite_only {
            let sub = Query::select()
                .expr(Expr::val(1i32))
                .from(favs::Entity)
                .and_where(Expr::col((favs::Entity, favs::Column::UserId)).eq(user.to_string()))
                .and_where(Expr::col((favs::Entity, favs::Column::FolderId)).eq(folder_id))
                .and_where(
                    Expr::col((favs::Entity, favs::Column::ArtistKey))
                        .equals((albums::Entity, A::ArtistKey)),
                )
                .and_where(Expr::col((favs::Entity, favs::Column::AlbumKey)).eq(""))
                .to_owned();
            base_cond = base_cond.add(Expr::exists(sub));
        }
        if let Some(term) = query.q.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            base_cond = base_cond.add(text_match(&[Expr::col((albums::Entity, A::Artist))], term));
        }

        let mut count_stmt = Query::select();
        count_stmt
            .from(albums::Entity)
            .expr_as(
                Expr::col((albums::Entity, A::ArtistKey))
                    .count_distinct()
                    .cast_as("CHAR(24)"),
                Alias::new("total"),
            )
            .cond_where(base_cond.clone());
        let total = query_count(self.db.main(), &count_stmt).await?;
        if total == 0 {
            return Ok(Some(MusicArtistPage {
                artists: Vec::new(),
                next_cursor: None,
                total: 0,
            }));
        }

        let join_plays = query.sort == MusicArtistSort::Plays;
        let stats_alias = Alias::new("u");
        let mut inner = Query::select();
        inner
            .from(albums::Entity)
            .cond_where(base_cond)
            .column((albums::Entity, A::ArtistKey))
            .expr_as(
                Expr::col((albums::Entity, A::ArtistSort)).min(),
                Alias::new("artist_sort"),
            )
            .expr_as(
                Expr::col((albums::Entity, A::Artist)).min(),
                Alias::new("name"),
            )
            .expr_as(Expr::col(Asterisk).count(), Alias::new("album_count"))
            .expr_as(
                Expr::col((albums::Entity, A::TrackCount)).sum(),
                Alias::new("track_count"),
            )
            .expr_as(
                Expr::col((albums::Entity, A::ArtworkItemId)).min(),
                Alias::new("artwork_item_id"),
            )
            .expr_as(
                Expr::col((albums::Entity, A::Shuffle)).min(),
                Alias::new("base_shuffle"),
            )
            .group_by_col((albums::Entity, A::ArtistKey));
        if join_plays {
            inner
                .join_subquery(
                    JoinType::LeftJoin,
                    artist_play_counts_subquery(user, query.library_id),
                    stats_alias.clone(),
                    Expr::col((stats_alias.clone(), Alias::new("artist_key")))
                        .equals((albums::Entity, A::ArtistKey)),
                )
                .expr_as(
                    Func::coalesce([
                        Expr::col((stats_alias.clone(), Alias::new("play_count"))),
                        Expr::val(0i64),
                    ]),
                    Alias::new("play_count"),
                )
                .group_by_col((stats_alias, Alias::new("play_count")));
        }

        let sub = Alias::new("sub");
        let mut outer = Query::select();
        outer
            .from_subquery(inner, sub.clone())
            .column((sub.clone(), Alias::new("artist_key")))
            .column((sub.clone(), Alias::new("artist_sort")))
            .column((sub.clone(), Alias::new("name")))
            .expr_as(
                Expr::col((sub.clone(), Alias::new("album_count"))).cast_as("CHAR(24)"),
                Alias::new("album_count"),
            )
            .expr_as(
                Expr::col((sub.clone(), Alias::new("track_count"))).cast_as("CHAR(24)"),
                Alias::new("track_count"),
            )
            .column((sub.clone(), Alias::new("artwork_item_id")))
            .column((sub.clone(), Alias::new("base_shuffle")));
        if join_plays {
            outer.expr_as(
                Expr::col((sub.clone(), Alias::new("play_count"))).cast_as("CHAR(24)"),
                Alias::new("play_count"),
            );
        } else {
            outer.expr_as(
                Expr::val(Option::<String>::None).cast_as("CHAR(24)"),
                Alias::new("play_count"),
            );
        }

        let (specs, seed_used): (Vec<OrderSpec>, Option<ShuffleSeed>) = match query.sort {
            MusicArtistSort::Name => (
                vec![
                    (
                        Expr::col((sub.clone(), Alias::new("artist_sort"))),
                        false,
                        query.order,
                    ),
                    (
                        Expr::col((sub, Alias::new("artist_key"))),
                        false,
                        MusicSortOrder::Asc,
                    ),
                ],
                None,
            ),
            MusicArtistSort::Albums => (
                vec![
                    (
                        Expr::col((sub.clone(), Alias::new("album_count"))),
                        false,
                        query.order,
                    ),
                    (
                        Expr::col((sub, Alias::new("artist_key"))),
                        false,
                        MusicSortOrder::Asc,
                    ),
                ],
                None,
            ),
            MusicArtistSort::Tracks => (
                vec![
                    (
                        Expr::col((sub.clone(), Alias::new("track_count"))),
                        false,
                        query.order,
                    ),
                    (
                        Expr::col((sub, Alias::new("artist_key"))),
                        false,
                        MusicSortOrder::Asc,
                    ),
                ],
                None,
            ),
            MusicArtistSort::Plays => (
                vec![
                    (
                        Expr::col((sub.clone(), Alias::new("play_count"))),
                        false,
                        query.order,
                    ),
                    (
                        Expr::col((sub, Alias::new("artist_key"))),
                        false,
                        MusicSortOrder::Asc,
                    ),
                ],
                None,
            ),
            MusicArtistSort::Random => {
                let seed = effective_seed(query.seed.as_deref(), query.cursor.as_ref());
                let (mul, add) = shuffle_params(&seed);
                (
                    vec![
                        (
                            shuffle_expr(
                                Expr::col((sub.clone(), Alias::new("base_shuffle"))),
                                mul,
                                add,
                            ),
                            false,
                            query.order,
                        ),
                        (
                            Expr::col((sub, Alias::new("artist_key"))),
                            false,
                            MusicSortOrder::Asc,
                        ),
                    ],
                    Some((seed, mul, add)),
                )
            }
        };
        apply_cursor(&mut outer, query.cursor.as_ref(), &specs)?;
        apply_order(&mut outer, &specs);
        outer.limit((limit + 1) as u64);

        let mut rows = query_all_models::<ArtistRow>(self.db.main(), &outer).await?;
        let has_more = rows.len() > limit;
        rows.truncate(limit);

        let artist_keys: Vec<String> = rows.iter().map(|r| r.artist_key.clone()).collect();
        let fav_set = if query.favorite_only {
            artist_keys.iter().cloned().collect()
        } else {
            self.load_artist_favorites(user, folder_id, &artist_keys)
                .await?
        };
        let play_map = if join_plays {
            HashMap::new()
        } else {
            self.load_artist_play_counts(user, query.library_id, &artist_keys)
                .await?
        };

        let scope = artist_scope(query);
        let next_cursor = if has_more {
            let last = rows.last().expect("non-empty page");
            let plays = if join_plays {
                last.play_count.as_deref().map_or(0, parse_i64_text)
            } else {
                play_map.get(&last.artist_key).copied().unwrap_or(0)
            };
            let keys = match query.sort {
                MusicArtistSort::Name => vec![
                    CursorKey::Str(last.artist_sort.clone()),
                    CursorKey::Str(last.artist_key.clone()),
                ],
                MusicArtistSort::Albums => vec![
                    CursorKey::Int(parse_i64_text(&last.album_count)),
                    CursorKey::Str(last.artist_key.clone()),
                ],
                MusicArtistSort::Tracks => vec![
                    CursorKey::Int(parse_i64_text(&last.track_count)),
                    CursorKey::Str(last.artist_key.clone()),
                ],
                MusicArtistSort::Plays => vec![
                    CursorKey::Int(plays),
                    CursorKey::Str(last.artist_key.clone()),
                ],
                MusicArtistSort::Random => {
                    let (_, mul, add) = seed_used.as_ref().expect("random seed");
                    vec![
                        CursorKey::Int(shuffle_rank(last.base_shuffle, *mul, *add)),
                        CursorKey::Str(last.artist_key.clone()),
                    ]
                }
            };
            Some(
                CatalogCursor {
                    scope,
                    seed: seed_used.map(|(s, _, _)| s),
                    keys,
                }
                .encode()?,
            )
        } else {
            None
        };

        let mut artists_out = Vec::with_capacity(rows.len());
        for row in rows {
            let plays = if join_plays {
                row.play_count.as_deref().map_or(0, parse_i64_text)
            } else {
                play_map.get(&row.artist_key).copied().unwrap_or(0)
            };
            artists_out.push(MusicArtist {
                id: row.artist_key.clone(),
                library_id: query.library_id,
                name: row.name,
                album_count: parse_i64_text(&row.album_count),
                track_count: parse_i64_text(&row.track_count),
                artwork_item_id: Some(parse_id(&row.artwork_item_id, "artwork item id")?),
                favorite: fav_set.contains(&row.artist_key),
                play_count: plays,
            });
        }
        Ok(Some(MusicArtistPage {
            artists: artists_out,
            next_cursor,
            total,
        }))
    }

    pub async fn get_artist_detail(
        &self,
        user: UserId,
        library: LibraryId,
        artist_key: &str,
    ) -> Result<Option<MusicArtistDetail>, DbError> {
        let Some(folder_id) = self.music_folder_id(library).await? else {
            return Ok(None);
        };
        self.refresh_albums(library).await?;

        use albums::Column as A;
        let mut album_stmt = Query::select();
        album_stmt
            .from(albums::Entity)
            .columns([
                (albums::Entity, A::LibraryId),
                (albums::Entity, A::ArtistKey),
                (albums::Entity, A::AlbumKey),
                (albums::Entity, A::Name),
                (albums::Entity, A::NameSort),
                (albums::Entity, A::Artist),
                (albums::Entity, A::ArtistSort),
                (albums::Entity, A::Year),
                (albums::Entity, A::TrackCount),
                (albums::Entity, A::DiscCount),
                (albums::Entity, A::DurationMs),
                (albums::Entity, A::ArtworkItemId),
                (albums::Entity, A::AddedAt),
                (albums::Entity, A::Shuffle),
            ])
            .and_where(Expr::col((albums::Entity, A::LibraryId)).eq(library.to_string()))
            .and_where(Expr::col((albums::Entity, A::ArtistKey)).eq(artist_key));
        apply_order(
            &mut album_stmt,
            &[
                (
                    Expr::col((albums::Entity, A::Year)),
                    true,
                    MusicSortOrder::Desc,
                ),
                (
                    Expr::col((albums::Entity, A::NameSort)),
                    false,
                    MusicSortOrder::Asc,
                ),
                (
                    Expr::col((albums::Entity, A::AlbumKey)),
                    false,
                    MusicSortOrder::Asc,
                ),
            ],
        );
        let album_rows = query_all_models::<AlbumRow>(self.db.main(), &album_stmt).await?;
        if album_rows.is_empty() {
            return Ok(None);
        }

        let pairs: Vec<(String, String)> = album_rows
            .iter()
            .map(|r| (r.artist_key.clone(), r.album_key.clone()))
            .collect();
        let mut genre_map = self.load_album_genres(library, &pairs).await?;
        let fav_albums = self.load_album_favorites(user, folder_id, &pairs).await?;
        let fav_artist = favs::Entity::find_by_id((
            user.to_string(),
            folder_id,
            artist_key.to_owned(),
            String::new(),
        ))
        .one(self.db.main())
        .await?
        .is_some();
        let stats_map = self.load_album_user_stats(user, library, &pairs).await?;

        let mut total_tracks = 0i64;
        let mut total_plays = 0i64;
        let mut artist_name: Option<String> = None;
        let mut artwork_id: Option<String> = None;
        let mut albums_out = Vec::with_capacity(album_rows.len());
        for row in &album_rows {
            total_tracks += row.track_count;
            if let Some(name) = row.artist.as_ref()
                && artist_name.as_ref().is_none_or(|cur| name < cur)
            {
                artist_name = Some(name.clone());
            }
            if artwork_id
                .as_ref()
                .is_none_or(|cur| row.artwork_item_id < *cur)
            {
                artwork_id = Some(row.artwork_item_id.clone());
            }
            let key = (row.artist_key.clone(), row.album_key.clone());
            let stats = stats_map.get(&key).copied().unwrap_or_default();
            total_plays += stats.play_count;
            let genres = genre_map.remove(&key).unwrap_or_default();
            albums_out.push(to_album(row, genres, fav_albums.contains(&key), stats)?);
        }

        let artist = MusicArtist {
            id: artist_key.to_owned(),
            library_id: library,
            name: artist_name,
            album_count: i64::try_from(albums_out.len()).unwrap_or(i64::MAX),
            track_count: total_tracks,
            artwork_item_id: artwork_id
                .as_deref()
                .map(|id| parse_id(id, "artwork item id"))
                .transpose()?,
            favorite: fav_artist,
            play_count: total_plays,
        };

        let mut top_stmt = track_select_base(user, JoinType::LeftJoin);
        top_stmt
            .cond_where(visible_recordings())
            .and_where(
                Expr::col((tracks::Entity, tracks::Column::LibraryId)).eq(library.to_string()),
            )
            .and_where(Expr::col((tracks::Entity, tracks::Column::ArtistKey)).eq(artist_key))
            .and_where(Expr::col((items::Entity, items::Column::DeletedAt)).is_null())
            .and_where(Expr::col((items::Entity, items::Column::Kind)).eq("track"))
            .order_by_expr(
                Func::coalesce([
                    Expr::col((state::Entity, state::Column::PlayCount)),
                    Expr::val(0i64),
                ])
                .into(),
                Order::Desc,
            )
            .order_by((tracks::Entity, tracks::Column::AlbumSort), Order::Asc)
            .order_by((tracks::Entity, tracks::Column::AlbumKey), Order::Asc)
            .order_by((tracks::Entity, tracks::Column::TrackSort), Order::Asc)
            .order_by((tracks::Entity, tracks::Column::ItemId), Order::Asc)
            .limit(10);
        let top_rows = query_all_models::<TrackRow>(self.db.main(), &top_stmt).await?;
        let top_ids: Vec<String> = top_rows.iter().map(|r| r.item_id.clone()).collect();
        let mut top_genres = self.load_track_genres(&top_ids).await?;
        let mut top_tracks = Vec::with_capacity(top_rows.len());
        for row in top_rows {
            let genres = top_genres.remove(&row.item_id).unwrap_or_default();
            top_tracks.push(to_track(row, genres)?);
        }

        Ok(Some(MusicArtistDetail {
            artist,
            albums: albums_out,
            top_tracks,
        }))
    }

    pub async fn list_genres(&self, library: LibraryId) -> Result<Option<MusicGenreList>, DbError> {
        if !self.has_library(library).await? {
            return Ok(None);
        }
        #[derive(FromQueryResult)]
        struct Row {
            name: String,
            album_count: String,
            track_count: String,
        }
        let mut stmt = Query::select();
        stmt.from(genres::Entity)
            .expr_as(Expr::col(genres::Column::Genre).min(), Alias::new("name"))
            .expr_as(
                Expr::col(genres::Column::AlbumId)
                    .count_distinct()
                    .cast_as("CHAR(24)"),
                Alias::new("album_count"),
            )
            .expr_as(
                Expr::col(Asterisk).count().cast_as("CHAR(24)"),
                Alias::new("track_count"),
            )
            .and_where(Expr::col(genres::Column::LibraryId).eq(library.to_string()))
            .group_by_columns([genres::Column::GenreSort, genres::Column::GenreKey])
            .order_by(genres::Column::GenreSort, Order::Asc)
            .order_by(genres::Column::GenreKey, Order::Asc);
        let rows = query_all_models::<Row>(self.db.main(), &stmt).await?;
        let genres = rows
            .into_iter()
            .map(|r| MusicGenre {
                name: r.name,
                album_count: parse_i64_text(&r.album_count),
                track_count: parse_i64_text(&r.track_count),
            })
            .collect();
        Ok(Some(MusicGenreList { genres }))
    }

    pub async fn list_tracks(
        &self,
        user: UserId,
        query: &TrackListQuery,
    ) -> Result<Option<MusicTrackPage>, DbError> {
        if !self.has_library(query.library_id).await? {
            return Ok(None);
        }
        let limit = query.limit.clamp(1, 200) as usize;

        use items::Column as I;
        use state::Column as S;
        use tracks::Column as T;
        let mut filter = Condition::all()
            .add(visible_recordings())
            .add(Expr::col((tracks::Entity, T::LibraryId)).eq(query.library_id.to_string()))
            .add(Expr::col((items::Entity, I::DeletedAt)).is_null())
            .add(Expr::col((items::Entity, I::Kind)).eq("track"));
        if let Some((ak, bk)) = query.album.as_ref() {
            filter = filter
                .add(Expr::col((tracks::Entity, T::ArtistKey)).eq(ak.as_str()))
                .add(Expr::col((tracks::Entity, T::AlbumKey)).eq(bk.as_str()));
        }
        if let Some(artist) = query.artist_id.as_deref() {
            filter = filter.add(Expr::col((tracks::Entity, T::ArtistKey)).eq(artist));
        }
        if let Some(genre) = query
            .genre
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            let gkey = sha256_hex(&genre.to_lowercase());
            let sub = Query::select()
                .expr(Expr::val(1i32))
                .from(genres::Entity)
                .and_where(
                    Expr::col((genres::Entity, genres::Column::ItemId))
                        .equals((tracks::Entity, T::ItemId)),
                )
                .and_where(Expr::col((genres::Entity, genres::Column::GenreKey)).eq(gkey))
                .to_owned();
            filter = filter.add(Expr::exists(sub));
        }
        if query.favorite_only {
            filter = filter.add(Expr::col((state::Entity, S::FavoriteOrder)).is_not_null());
        }
        if let Some(term) = query.q.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            filter = filter.add(text_match(
                &[
                    Expr::col((tracks::Entity, T::Title)),
                    Expr::col((tracks::Entity, T::Artist)),
                    Expr::col((tracks::Entity, T::AlbumArtist)),
                    Expr::col((tracks::Entity, T::Album)),
                ],
                term,
            ));
        }

        let mut count_stmt = Query::select();
        count_stmt
            .from(tracks::Entity)
            .inner_join(
                items::Entity,
                Expr::col((items::Entity, I::Id)).equals((tracks::Entity, T::ItemId)),
            )
            .expr_as(
                Expr::col(Asterisk).count().cast_as("CHAR(24)"),
                Alias::new("total"),
            );
        if query.favorite_only {
            count_stmt.inner_join(
                state::Entity,
                Condition::all()
                    .add(Expr::col((state::Entity, S::ItemId)).equals((tracks::Entity, T::ItemId)))
                    .add(Expr::col((state::Entity, S::UserId)).eq(user.to_string())),
            );
        }
        count_stmt.cond_where(filter.clone());
        let total = query_count(self.db.main(), &count_stmt).await?;
        if total == 0 {
            return Ok(Some(MusicTrackPage {
                tracks: Vec::new(),
                next_cursor: None,
                total: 0,
            }));
        }

        let join_type = if query.favorite_only {
            JoinType::InnerJoin
        } else {
            JoinType::LeftJoin
        };
        let mut stmt = track_select_base(user, join_type);
        stmt.cond_where(filter);

        let title_sort_expr: Expr =
            Func::coalesce([Expr::col((tracks::Entity, T::TitleSort)), Expr::val("")]).into();
        let artist_sort_expr: Expr = Func::coalesce([
            Expr::col((tracks::Entity, T::TrackArtistSort)),
            Expr::col((tracks::Entity, T::ArtistSort)),
        ])
        .into();
        let plays_expr: Expr =
            Func::coalesce([Expr::col((state::Entity, S::PlayCount)), Expr::val(0i64)]).into();
        let stars_expr: Expr =
            Func::coalesce([Expr::col((state::Entity, S::Stars)), Expr::val(0i32)]).into();
        let shuffle_base: Expr =
            Func::coalesce([Expr::col((tracks::Entity, T::Shuffle)), Expr::val(0i64)]).into();

        let (specs, seed_used): (Vec<OrderSpec>, Option<ShuffleSeed>) = match query.sort {
            MusicTrackSort::Title => (
                vec![
                    (title_sort_expr, false, query.order),
                    (
                        Expr::col((tracks::Entity, T::ItemId)),
                        false,
                        MusicSortOrder::Asc,
                    ),
                ],
                None,
            ),
            MusicTrackSort::Artist => (
                vec![
                    (artist_sort_expr, false, query.order),
                    (title_sort_expr, false, MusicSortOrder::Asc),
                    (
                        Expr::col((tracks::Entity, T::ItemId)),
                        false,
                        MusicSortOrder::Asc,
                    ),
                ],
                None,
            ),
            MusicTrackSort::Album => (
                vec![
                    (
                        Expr::col((tracks::Entity, T::AlbumSort)),
                        false,
                        query.order,
                    ),
                    (Expr::col((tracks::Entity, T::AlbumKey)), false, query.order),
                    (
                        Expr::col((tracks::Entity, T::TrackSort)),
                        false,
                        MusicSortOrder::Asc,
                    ),
                    (
                        Expr::col((tracks::Entity, T::ItemId)),
                        false,
                        MusicSortOrder::Asc,
                    ),
                ],
                None,
            ),
            MusicTrackSort::Year => (
                vec![
                    (Expr::col((tracks::Entity, T::Year)), true, query.order),
                    (title_sort_expr, false, MusicSortOrder::Asc),
                    (
                        Expr::col((tracks::Entity, T::ItemId)),
                        false,
                        MusicSortOrder::Asc,
                    ),
                ],
                None,
            ),
            MusicTrackSort::Added => (
                vec![
                    (Expr::col((items::Entity, I::CreatedAt)), false, query.order),
                    (
                        Expr::col((tracks::Entity, T::ItemId)),
                        false,
                        MusicSortOrder::Asc,
                    ),
                ],
                None,
            ),
            MusicTrackSort::Duration => (
                vec![
                    (
                        Expr::col((tracks::Entity, T::DurationMs)),
                        true,
                        query.order,
                    ),
                    (
                        Expr::col((tracks::Entity, T::ItemId)),
                        false,
                        MusicSortOrder::Asc,
                    ),
                ],
                None,
            ),
            MusicTrackSort::Plays => (
                vec![
                    (plays_expr, false, query.order),
                    (
                        Expr::col((tracks::Entity, T::ItemId)),
                        false,
                        MusicSortOrder::Asc,
                    ),
                ],
                None,
            ),
            MusicTrackSort::Played => (
                vec![
                    (
                        Expr::col((state::Entity, S::LastPlayedAt)),
                        true,
                        query.order,
                    ),
                    (
                        Expr::col((tracks::Entity, T::ItemId)),
                        false,
                        MusicSortOrder::Asc,
                    ),
                ],
                None,
            ),
            MusicTrackSort::Rating => (
                vec![
                    (stars_expr, false, query.order),
                    (
                        Expr::col((tracks::Entity, T::ItemId)),
                        false,
                        MusicSortOrder::Asc,
                    ),
                ],
                None,
            ),
            MusicTrackSort::Random => {
                let seed = effective_seed(query.seed.as_deref(), query.cursor.as_ref());
                let (mul, add) = shuffle_params(&seed);
                (
                    vec![
                        (shuffle_expr(shuffle_base, mul, add), false, query.order),
                        (
                            Expr::col((tracks::Entity, T::ItemId)),
                            false,
                            MusicSortOrder::Asc,
                        ),
                    ],
                    Some((seed, mul, add)),
                )
            }
        };
        apply_cursor(&mut stmt, query.cursor.as_ref(), &specs)?;
        apply_order(&mut stmt, &specs);
        stmt.limit((limit + 1) as u64);

        let mut rows = query_all_models::<TrackRow>(self.db.main(), &stmt).await?;
        let has_more = rows.len() > limit;
        rows.truncate(limit);

        let scope = track_scope(query);
        let next_cursor = if has_more {
            let last = rows.last().expect("non-empty page");
            let tsort = last.title_sort.clone().unwrap_or_default();
            let asort = last
                .track_artist_sort
                .clone()
                .unwrap_or_else(|| last.artist_sort.clone());
            let keys = match query.sort {
                MusicTrackSort::Title => {
                    vec![CursorKey::Str(tsort), CursorKey::Str(last.item_id.clone())]
                }
                MusicTrackSort::Artist => vec![
                    CursorKey::Str(asort),
                    CursorKey::Str(tsort),
                    CursorKey::Str(last.item_id.clone()),
                ],
                MusicTrackSort::Album => vec![
                    CursorKey::Str(last.album_sort.clone()),
                    CursorKey::Str(last.album_key.clone()),
                    CursorKey::Str(last.track_sort.clone()),
                    CursorKey::Str(last.item_id.clone()),
                ],
                MusicTrackSort::Year => vec![
                    CursorKey::OptInt(last.year),
                    CursorKey::Str(tsort),
                    CursorKey::Str(last.item_id.clone()),
                ],
                MusicTrackSort::Added => vec![
                    CursorKey::Int(last.created_at),
                    CursorKey::Str(last.item_id.clone()),
                ],
                MusicTrackSort::Duration => vec![
                    CursorKey::OptInt(last.duration_ms),
                    CursorKey::Str(last.item_id.clone()),
                ],
                MusicTrackSort::Plays => vec![
                    CursorKey::Int(last.play_count.unwrap_or(0)),
                    CursorKey::Str(last.item_id.clone()),
                ],
                MusicTrackSort::Played => vec![
                    CursorKey::OptInt(last.last_played_at),
                    CursorKey::Str(last.item_id.clone()),
                ],
                MusicTrackSort::Rating => vec![
                    CursorKey::Int(i64::from(last.stars.unwrap_or(0))),
                    CursorKey::Str(last.item_id.clone()),
                ],
                MusicTrackSort::Random => {
                    let (_, mul, add) = seed_used.as_ref().expect("random seed");
                    vec![
                        CursorKey::Int(shuffle_rank(last.shuffle.unwrap_or(0), *mul, *add)),
                        CursorKey::Str(last.item_id.clone()),
                    ]
                }
            };
            Some(
                CatalogCursor {
                    scope,
                    seed: seed_used.map(|(s, _, _)| s),
                    keys,
                }
                .encode()?,
            )
        } else {
            None
        };

        let ids: Vec<String> = rows.iter().map(|r| r.item_id.clone()).collect();
        let mut genre_map = self.load_track_genres(&ids).await?;
        let mut tracks_out = Vec::with_capacity(rows.len());
        for row in rows {
            let genres = genre_map.remove(&row.item_id).unwrap_or_default();
            tracks_out.push(to_track(row, genres)?);
        }
        Ok(Some(MusicTrackPage {
            tracks: tracks_out,
            next_cursor,
            total,
        }))
    }

    pub async fn lookup_tracks(
        &self,
        user: UserId,
        ids: &[ItemId],
    ) -> Result<Vec<MusicTrack>, DbError> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut unique: Vec<String> = Vec::with_capacity(ids.len());
        let mut seen = HashSet::with_capacity(ids.len());
        for id in ids {
            let raw = id.to_string();
            if seen.insert(raw.clone()) {
                unique.push(raw);
            }
        }
        let mut genre_map = self.load_track_genres(&unique).await?;
        let mut found: HashMap<String, MusicTrack> = HashMap::with_capacity(unique.len());
        for chunk in unique.chunks(200) {
            let mut stmt = track_select_base(user, JoinType::LeftJoin);
            stmt.inner_join(
                libraries::Entity,
                Expr::col((libraries::Entity, libraries::Column::Id))
                    .equals((tracks::Entity, tracks::Column::LibraryId)),
            )
            .cond_where(visible_recordings())
            .and_where(Expr::col((libraries::Entity, libraries::Column::Kind)).eq("music"))
            .and_where(Expr::col((libraries::Entity, libraries::Column::DeletedAt)).is_null())
            .and_where(Expr::col((items::Entity, items::Column::DeletedAt)).is_null())
            .and_where(Expr::col((items::Entity, items::Column::Kind)).eq("track"))
            .and_where(
                Expr::col((tracks::Entity, tracks::Column::ItemId)).is_in(chunk.iter().cloned()),
            );
            for row in query_all_models::<TrackRow>(self.db.main(), &stmt).await? {
                let id_str = row.item_id.clone();
                let genres = genre_map.remove(&id_str).unwrap_or_default();
                found.insert(id_str, to_track(row, genres)?);
            }
        }
        Ok(ids
            .iter()
            .filter_map(|id| found.get(&id.to_string()).cloned())
            .collect())
    }

    pub async fn set_library_collection_favorite(
        &self,
        user: UserId,
        library: LibraryId,
        artist_key: &str,
        album_key: Option<&str>,
        favorite: bool,
    ) -> Result<(), DbError> {
        let Some(folder_id) = self.music_folder_id(library).await? else {
            return Err(DbError::NotFound("Music library"));
        };
        self.set_collection_favorite(user, folder_id, artist_key, album_key, favorite)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::DbBackend;
    use sea_orm::sea_query::{MysqlQueryBuilder, PostgresQueryBuilder, SqliteQueryBuilder};

    #[test]
    fn rounded_stars_matches_half_up_averages() {
        assert_eq!(rounded_stars(0, 0), 0);
        assert_eq!(rounded_stars(1, 1), 1);
        assert_eq!(rounded_stars(3, 2), 2);
        assert_eq!(rounded_stars(7, 3), 2);
        assert_eq!(rounded_stars(8, 3), 3);
        assert_eq!(rounded_stars(9, 2), 5);
        assert_eq!(rounded_stars(10, 2), 5);
    }

    #[test]
    fn shuffle_rank_is_bijective_and_bounded() {
        let (mul, add) = shuffle_params("seed-42");
        let mut seen = HashSet::new();
        for base in 0..1000i64 {
            let r = shuffle_rank(base, mul, add);
            assert!((0..SHUFFLE_MODULUS).contains(&r));
            assert!(seen.insert(r));
        }
        assert_ne!(shuffle_params("a"), shuffle_params("b"));
    }

    #[test]
    fn album_user_sort_sql_builds_cleanly_for_all_backends() {
        let user = UserId::generate();
        let lib = LibraryId::generate();
        let q = AlbumListQuery {
            library_id: lib,
            sort: MusicAlbumSort::Rating,
            order: MusicSortOrder::Desc,
            genre: Some("Rock".into()),
            year_from: Some(1990),
            year_to: Some(2025),
            artist_id: None,
            favorite_only: true,
            q: Some("test".into()),
            seed: None,
            cursor: None,
            limit: 60,
        };
        let stats_alias = Alias::new("u");
        let mut inner = Query::select();
        inner
            .from(albums::Entity)
            .join_subquery(
                JoinType::LeftJoin,
                album_user_stats_subquery(user, lib),
                stats_alias.clone(),
                Expr::col((stats_alias.clone(), Alias::new("artist_key")))
                    .equals((albums::Entity, albums::Column::ArtistKey)),
            )
            .cond_where(album_filter_conditions(&q, user, 1))
            .expr_as(
                sql_rounded_stars(
                    Expr::col((stats_alias.clone(), Alias::new("star_sum"))),
                    Expr::col((stats_alias, Alias::new("star_count"))),
                ),
                Alias::new("stars"),
            );
        for backend in [DbBackend::Sqlite, DbBackend::Postgres, DbBackend::MySql] {
            let sql = match backend {
                DbBackend::Sqlite => inner.to_string(SqliteQueryBuilder),
                DbBackend::Postgres => inner.to_string(PostgresQueryBuilder),
                DbBackend::MySql => inner.to_string(MysqlQueryBuilder),
                _ => unreachable!(),
            };
            assert!(sql.contains("LEFT JOIN"), "{backend:?}: {sql}");
            assert!(sql.contains("CASE WHEN"), "{backend:?}: {sql}");
            assert!(sql.contains("EXISTS"), "{backend:?}: {sql}");
        }
    }
}

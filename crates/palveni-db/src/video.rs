//! Movie and series views are separate from the underlying file tree.
mod detail;
mod home;

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use palveni_core::{
    ItemId, ItemKind, LibraryId, LibraryKind, UserId,
    video::{
        SortDir, VideoEntry, VideoGenre, VideoGenreList, VideoKind, VideoOrder, VideoPage,
        VideoPersonKind, VideoUserState, WorkKind,
    },
};
use sea_orm::sea_query::{
    Alias, Asterisk, BinOper, Expr, ExprTrait, Func, IntoCondition, OnConflict, Order, Query,
};
use sea_orm::{
    ColumnTrait, Condition, ConnectionTrait, EntityTrait, FromQueryResult, IntoSimpleExpr,
    JoinType, QueryFilter, QueryOrder, QuerySelect, QueryTrait, RelationTrait, Select, Set,
    TransactionTrait,
};
use serde::{Deserialize, Serialize};

use crate::{
    Db, DbError,
    convert::{now_ms, opt_ts, parse_id, sha256_hex, truncate_chars, ts},
    entities::{
        items, libraries, user_item_state, video_favorites, video_genres,
        video_metadata::{self as video, Column as C},
        video_people,
    },
    music::TrackLocation,
};

pub struct Videos<'a> {
    pub(crate) db: &'a Db,
}

#[derive(Clone, Debug)]
pub struct VideoPersonInput {
    pub kind: VideoPersonKind,
    pub name: String,
    pub role: Option<String>,
    pub image: Option<String>,
}

pub struct VideoMetadata {
    pub item_id: ItemId,
    pub stamp: String,
    pub work_group: String,
    pub work_title: String,
    pub year: Option<i32>,
    pub work_overview: Option<String>,
    pub work_poster: Option<String>,
    pub work_original_title: Option<String>,
    pub work_tagline: Option<String>,
    /// Tenths of a point, 0–100.
    pub work_rating: Option<i32>,
    pub work_official_rating: Option<String>,
    pub work_premiered: Option<String>,
    pub work_studios: Vec<String>,
    pub work_backdrop: Option<String>,
    pub season: Option<i32>,
    pub season_poster: Option<String>,
    pub episode: Option<i32>,
    pub title: String,
    pub overview: Option<String>,
    pub episode_poster: Option<String>,
    pub runtime_ms: Option<i64>,
    /// Tenths of a point, 0–100.
    pub rating: Option<i32>,
    pub premiered: Option<String>,
    pub thumb: Option<String>,
    pub genres: Vec<String>,
    pub people: Vec<VideoPersonInput>,
}

pub struct VideoThumbLocation {
    pub video: TrackLocation,
    pub local_thumb: Option<TrackLocation>,
    pub runtime_ms: Option<i64>,
}

pub fn sidecar_fingerprint(value: &str) -> String {
    sha256_hex(value)
}

pub fn person_key(name: &str) -> String {
    sha256_hex(&name.trim().to_lowercase())
}

pub(crate) fn sort_key(value: &str) -> String {
    let lower = value.to_lowercase();
    let buf = lower.as_bytes();
    hex::encode(&buf[..buf.len().min(80)])
}

pub(crate) fn rating_from_tenths(tenths: Option<i32>) -> Option<f64> {
    tenths.map(|r| f64::from(r) / 10.0)
}

/// First-letter filter matching the hex-encoded `work_sort` index.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StartsWith {
    /// Lowercase ASCII letter `'a'..='z'`.
    Letter(char),
    /// Anything not starting with a Latin letter (digits, CJK, symbols).
    Other,
}

impl StartsWith {
    pub fn parse(raw: &str) -> Option<Self> {
        let mut chars = raw.chars();
        let ch = chars.next()?;
        if chars.next().is_some() {
            return None;
        }
        if ch == '#' {
            Some(Self::Other)
        } else if ch.is_ascii_alphabetic() {
            Some(Self::Letter(ch.to_ascii_lowercase()))
        } else {
            None
        }
    }

    fn token(self) -> String {
        match self {
            Self::Letter(ch) => ch.to_string(),
            Self::Other => "#".into(),
        }
    }

    fn condition(self) -> Condition {
        match self {
            Self::Letter(ch) => {
                let byte = ch as u8;
                let lo = format!("{byte:02x}");
                let hi = format!("{:02x}", byte + 1);
                Condition::all()
                    .add(C::WorkSort.gte(lo))
                    .add(C::WorkSort.lt(hi))
            }
            Self::Other => Condition::any()
                .add(C::WorkSort.lt("61"))
                .add(C::WorkSort.gte("7b")),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct VideoCursor {
    scope: String,
    sort: String,
    id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    has_poster: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    offset: Option<u64>,
}

#[derive(Clone, Copy, Default)]
pub struct VideoFilters<'a> {
    pub order: VideoOrder,
    pub dir: Option<SortDir>,
    pub seed: Option<&'a str>,
    pub title: &'a str,
    pub kind: Option<WorkKind>,
    pub genre: Option<&'a str>,
    pub year_from: Option<i32>,
    pub year_to: Option<i32>,
    pub played: Option<bool>,
    pub favorite: Option<bool>,
    pub starts_with: Option<StartsWith>,
}

impl VideoFilters<'_> {
    pub fn effective_dir(self) -> SortDir {
        self.dir.unwrap_or_else(|| self.order.default_dir())
    }

    pub fn has_extended_filters(self) -> bool {
        self.genre.is_some()
            || self.year_from.is_some()
            || self.year_to.is_some()
            || self.played.is_some()
            || self.favorite.is_some()
            || self.starts_with.is_some()
    }

    pub fn filtered(self) -> bool {
        !self.title.is_empty() || self.kind.is_some() || self.has_extended_filters()
    }

    pub fn uses_keyset(self) -> bool {
        self.order.is_keyset() && self.effective_dir() == SortDir::Asc
    }

    fn is_legacy_scope(self) -> bool {
        self.uses_keyset() && !self.has_extended_filters()
    }

    fn scope(self, library: LibraryId, series: Option<&str>, season: Option<i32>) -> String {
        let base = format!("{library}/{}/{season:?}", series.unwrap_or(""));
        if series.is_some() || season.is_some() {
            return base;
        }
        if self.is_legacy_scope() {
            if !self.filtered() {
                return base;
            }
            let kind = match self.kind {
                None => "all",
                Some(WorkKind::Movie) => "movie",
                Some(WorkKind::Series) => "series",
            };
            return format!(
                "{base}/{kind}/{}",
                sha256_hex(&self.title.to_ascii_lowercase())
            );
        }
        let kind = match self.kind {
            None => "all",
            Some(WorkKind::Movie) => "movie",
            Some(WorkKind::Series) => "series",
        };
        let order = match self.order {
            VideoOrder::Title => "title",
            VideoOrder::Poster => "poster",
            VideoOrder::Year => "year",
            VideoOrder::Added => "added",
            VideoOrder::Rating => "rating",
            VideoOrder::Runtime => "runtime",
            VideoOrder::Random => "random",
        };
        let dir = match self.effective_dir() {
            SortDir::Asc => "asc",
            SortDir::Desc => "desc",
        };
        let seed = if self.order == VideoOrder::Random {
            self.seed.unwrap_or("")
        } else {
            ""
        };
        let raw = format!(
            "{base}|o={order}|d={dir}|s={seed}|k={kind}|q={}|g={}|yf={:?}|yt={:?}|p={:?}|f={:?}|sw={}",
            self.title.to_lowercase(),
            self.genre.map(str::to_lowercase).unwrap_or_default(),
            self.year_from,
            self.year_to,
            self.played,
            self.favorite,
            self.starts_with.map(StartsWith::token).unwrap_or_default(),
        );
        format!("v2/{}", sha256_hex(&raw))
    }
}

impl VideoCursor {
    pub fn decode(
        raw: &str,
        library: LibraryId,
        series: Option<&str>,
        season: Option<i32>,
    ) -> Option<Self> {
        Self::decode_for_order(raw, library, series, season, VideoOrder::Title)
    }

    pub fn decode_for_order(
        raw: &str,
        library: LibraryId,
        series: Option<&str>,
        season: Option<i32>,
        order: VideoOrder,
    ) -> Option<Self> {
        Self::decode_filtered(
            raw,
            library,
            series,
            season,
            VideoFilters {
                order,
                ..Default::default()
            },
        )
    }

    pub fn decode_filtered(
        raw: &str,
        library: LibraryId,
        series: Option<&str>,
        season: Option<i32>,
        filters: VideoFilters<'_>,
    ) -> Option<Self> {
        if raw.len() > 1024 {
            return None;
        }
        let cursor: Self = serde_json::from_slice(&hex::decode(raw).ok()?).ok()?;
        let is_works = series.is_none() && season.is_none();
        let expect_offset = is_works && !filters.uses_keyset();
        let expect_poster =
            is_works && filters.uses_keyset() && filters.order == VideoOrder::Poster;
        (cursor.scope == filters.scope(library, series, season)
            && cursor.offset.is_some() == expect_offset
            && cursor.has_poster.is_some() == expect_poster)
            .then_some(cursor)
    }
}

fn after(sort: impl IntoSimpleExpr, key: C, cursor: &VideoCursor) -> Condition {
    let sort = sort.into_simple_expr();
    Condition::any()
        .add(sort.clone().gt(cursor.sort.as_str()))
        .add(
            Condition::all()
                .add(sort.eq(cursor.sort.as_str()))
                .add(key.gt(cursor.id.as_str())),
        )
}

pub(crate) fn active(library: LibraryId) -> Select<video::Entity> {
    video::Entity::find()
        .join(JoinType::InnerJoin, video::Relation::Item.def())
        .filter(C::LibraryId.eq(library.to_string()))
        .filter(items::Column::DeletedAt.is_null())
}

pub(crate) fn poster_sample(poster: C) -> Expr {
    let available = Expr::case(poster.is_not_null(), Expr::col((video::Entity, C::ItemId)));
    Func::coalesce([Expr::from(Func::min(available)), C::ItemId.min()]).into()
}

pub(crate) fn nulls_last(expr: Expr) -> Expr {
    Expr::Case(Box::new(Expr::case(expr.is_null(), 1_i32).finally(0_i32)))
}

pub(crate) fn join_user_state(
    select: Select<video::Entity>,
    user: UserId,
) -> Select<video::Entity> {
    let uid = user.to_string();
    select.join(
        JoinType::LeftJoin,
        video::Relation::UserState
            .def()
            .on_condition(move |_, right| {
                Expr::col((right, user_item_state::Column::UserId))
                    .eq(uid.clone())
                    .into_condition()
            }),
    )
}

#[derive(FromQueryResult)]
pub(crate) struct VideoWithItem {
    #[sea_orm(nested)]
    pub meta: video::Model,
    pub created_at: i64,
    pub size_bytes: Option<i64>,
    pub item_title: String,
}

#[derive(FromQueryResult)]
pub(crate) struct StateSample {
    pub work_key: String,
    pub season: Option<i32>,
    pub completed: Option<bool>,
    pub position_ms: Option<i64>,
    pub play_count: Option<i64>,
    pub last_played_at: Option<i64>,
}

#[derive(Clone, Copy)]
pub enum PosterLevel {
    Work,
    Season,
    Episode,
}

#[derive(Default)]
struct WorkStateAcc {
    is_series: bool,
    total_items: u64,
    completed_items: u64,
    play_count: i64,
    last_played_at: Option<i64>,
    resume_position_ms: i64,
    resume_at: Option<i64>,
}

impl WorkStateAcc {
    fn push(&mut self, row: &StateSample) {
        if row.season.is_some() {
            self.is_series = true;
        }
        self.total_items += 1;
        let completed = row.completed.unwrap_or(false);
        let position_ms = row.position_ms.unwrap_or(0);
        let play_count = row.play_count.unwrap_or(0);
        if completed {
            self.completed_items += 1;
        } else if position_ms > 0 {
            let replace = match (self.resume_at, row.last_played_at) {
                (None, _) => self.resume_position_ms == 0 || row.last_played_at.is_some(),
                (Some(prev), Some(curr)) => curr >= prev,
                (Some(_), None) => false,
            };
            if replace {
                self.resume_position_ms = position_ms;
                self.resume_at = row.last_played_at;
            }
        }
        self.play_count = self.play_count.saturating_add(play_count);
        if let Some(at) = row.last_played_at {
            self.last_played_at = Some(
                self.last_played_at
                    .map_or(at, |prev| std::cmp::max(prev, at)),
            );
        }
    }

    fn finish(self, favorite: bool) -> Result<VideoUserState, DbError> {
        let played = if self.is_series {
            self.total_items > 0 && self.completed_items == self.total_items
        } else {
            self.completed_items > 0
        };
        let position_ms = if self.is_series || played {
            0
        } else {
            self.resume_position_ms
        };
        let unplayed_count = self
            .is_series
            .then_some(self.total_items.saturating_sub(self.completed_items));
        Ok(VideoUserState {
            played,
            position_ms,
            play_count: self.play_count,
            favorite,
            last_played_at: opt_ts(self.last_played_at)?,
            unplayed_count,
        })
    }
}

pub(crate) fn episode_user_state(
    state: Option<&palveni_core::personal::MediaState>,
) -> VideoUserState {
    match state {
        Some(s) => VideoUserState {
            played: s.completed,
            position_ms: if s.completed { 0 } else { s.position_ms },
            play_count: s.play_count,
            favorite: s.favorite,
            last_played_at: s.last_played_at,
            unplayed_count: None,
        },
        None => VideoUserState::default(),
    }
}

pub(crate) fn build_episode_entry(
    library: LibraryId,
    row: video::Model,
    added_at: DateTime<Utc>,
    genres: Vec<String>,
    state: Option<VideoUserState>,
    file_count: u64,
) -> Result<VideoEntry, DbError> {
    let id = parse_id(&row.item_id, "episode id")?;
    Ok(VideoEntry {
        id: row.item_id,
        kind: VideoKind::Episode,
        library_id: library,
        title: row.title,
        file_count,
        year: row.work_year,
        overview: row.overview,
        season_number: row.season,
        episode_number: row.episode,
        poster_item_id: row.episode_poster.is_some().then_some(id),
        item_id: Some(id),
        series_id: Some(row.work_key),
        series_title: Some(row.work_title),
        original_title: None,
        tagline: None,
        genres,
        runtime_ms: row.runtime_ms,
        community_rating: rating_from_tenths(row.rating),
        official_rating: row.work_official_rating,
        premiered: row.premiered,
        backdrop_item_id: row.work_backdrop.is_some().then_some(id),
        thumb_item_id: Some(id),
        added_at,
        state,
    })
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn build_work_entry(
    library: LibraryId,
    key: String,
    sample: video::Model,
    file_count: u64,
    poster_item_id: Option<ItemId>,
    backdrop_item_id: Option<ItemId>,
    runtime_ms: Option<i64>,
    added_at: DateTime<Utc>,
    genres: Vec<String>,
    state: Option<VideoUserState>,
) -> Result<VideoEntry, DbError> {
    let sample_id = parse_id(&sample.item_id, "video id")?;
    let kind = if sample.season.is_some() {
        VideoKind::Series
    } else {
        VideoKind::Movie
    };
    Ok(VideoEntry {
        id: key,
        kind,
        library_id: library,
        title: sample.work_title,
        file_count,
        year: sample.work_year,
        overview: sample.work_overview,
        season_number: None,
        episode_number: None,
        poster_item_id,
        item_id: (kind == VideoKind::Movie).then_some(sample_id),
        series_id: None,
        series_title: None,
        original_title: sample.work_original_title,
        tagline: sample.work_tagline,
        genres,
        runtime_ms,
        community_rating: rating_from_tenths(sample.work_rating),
        official_rating: sample.work_official_rating,
        premiered: sample.work_premiered,
        backdrop_item_id,
        thumb_item_id: (kind == VideoKind::Movie && sample.thumb.is_some()).then_some(sample_id),
        added_at,
        state,
    })
}

#[derive(Clone)]
pub(super) struct WorkRowSummary {
    pub(super) sort: String,
    pub(super) key: String,
    pub(super) sample_id: String,
    pub(super) backdrop_id: String,
    pub(super) files: i64,
    pub(super) poster_count: i64,
    pub(super) backdrop_count: i64,
    pub(super) max_created_at: i64,
    pub(super) max_runtime_ms: Option<i64>,
}

fn apply_work_filters(
    mut query: Select<video::Entity>,
    library: LibraryId,
    user: Option<UserId>,
    filters: VideoFilters<'_>,
) -> Select<video::Entity> {
    if let Some(kind) = filters.kind {
        query = query.filter(match kind {
            WorkKind::Movie => C::Season.is_null(),
            WorkKind::Series => C::Season.is_not_null(),
        });
    }
    if let Some(starts_with) = filters.starts_with {
        query = query.filter(starts_with.condition());
    }
    if let Some(year_from) = filters.year_from {
        query = query.filter(C::WorkYear.gte(year_from));
    }
    if let Some(year_to) = filters.year_to {
        query = query.filter(C::WorkYear.lte(year_to));
    }
    if let Some(genre) = filters.genre {
        let sub = Query::select()
            .column(video_genres::Column::ItemId)
            .from(video_genres::Entity)
            .and_where(
                Expr::from(Func::lower(video_genres::Column::Name.into_simple_expr()))
                    .eq(genre.to_lowercase()),
            )
            .to_owned();
        query = query.filter(C::ItemId.in_subquery(sub));
    }
    if let Some(fav) = filters.favorite {
        if let Some(uid) = user {
            let sub = Query::select()
                .column(video_favorites::Column::WorkKey)
                .from(video_favorites::Entity)
                .and_where(video_favorites::Column::UserId.eq(uid.to_string()))
                .and_where(video_favorites::Column::LibraryId.eq(library.to_string()))
                .to_owned();
            query = if fav {
                query.filter(C::WorkKey.in_subquery(sub))
            } else {
                query.filter(C::WorkKey.not_in_subquery(sub))
            };
        } else if fav {
            // No authenticated user can have favorites.
            query = query.filter(Expr::value(1_i32).eq(0_i32));
        }
    }
    if !filters.title.is_empty() {
        let escaped = filters
            .title
            .replace('!', "!!")
            .replace('%', "!%")
            .replace('_', "!_");
        let pattern = Expr::from(Func::lower(format!("%{escaped}%")))
            .binary(BinOper::Escape, Expr::Constant('!'.into()));
        let title = Expr::from(Func::lower(C::WorkTitle.into_simple_expr()))
            .binary(BinOper::Like, pattern.clone());
        let original = Expr::col((video::Entity, C::WorkOriginalTitle))
            .is_not_null()
            .and(
                Expr::from(Func::lower(C::WorkOriginalTitle.into_simple_expr()))
                    .binary(BinOper::Like, pattern),
            );
        let matched = Condition::any().add(title).add(original);
        // Match any indexed title for the work without dropping its other episodes or art.
        query = query
            .having(Expr::from(Func::max(Expr::case(matched, 1_i32).finally(0_i32))).eq(1_i32));
    }
    if let Some(want_played) = filters.played {
        if let Some(uid) = user {
            query = join_user_state(query, uid);
            let completed_case = Expr::case(
                Expr::col((user_item_state::Entity, user_item_state::Column::Completed)).eq(true),
                1_i32,
            );
            let completed_count = Expr::from(Func::count(completed_case));
            let total_count = C::ItemId.count();
            let is_movie_played = Condition::all()
                .add(C::Season.max().is_null())
                .add(completed_count.clone().gt(0_i64));
            let is_series_played = Condition::all()
                .add(C::Season.max().is_not_null())
                .add(completed_count.eq(total_count));
            let is_played = Condition::any().add(is_movie_played).add(is_series_played);
            query = if want_played {
                query.having(is_played)
            } else {
                query.having(is_played.not())
            };
        } else if want_played {
            query = query.filter(Expr::value(1_i32).eq(0_i32));
        }
    }
    query
}

impl Videos<'_> {
    pub async fn has_library(&self, id: LibraryId) -> Result<bool, DbError> {
        let row = libraries::Entity::find_by_id(id.to_string())
            .filter(libraries::Column::Kind.eq("video"))
            .filter(libraries::Column::DeletedAt.is_null())
            .select_only()
            .column(libraries::Column::Id)
            .into_tuple::<String>()
            .one(self.db.main())
            .await?;
        Ok(row.is_some())
    }

    pub async fn scan_index(
        &self,
        id: LibraryId,
    ) -> Result<HashMap<String, (ItemId, Option<String>)>, DbError> {
        let rows = items::Entity::find()
            .join(JoinType::LeftJoin, video::Relation::Item.def().rev())
            .filter(items::Column::LibraryId.eq(id.to_string()))
            .filter(items::Column::Kind.eq(ItemKind::Video.as_str()))
            .filter(items::Column::DeletedAt.is_null())
            .select_only()
            .column(items::Column::Path)
            .column(items::Column::Id)
            .column(C::Stamp)
            .into_tuple::<(String, String, Option<String>)>()
            .all(self.db.main())
            .await?;
        rows.into_iter()
            .map(|(path, id, stamp)| Ok((path, (parse_id(&id, "video id")?, stamp))))
            .collect()
    }

    pub async fn save_batch(
        &self,
        library: LibraryId,
        entries: Vec<VideoMetadata>,
    ) -> Result<(), DbError> {
        if entries.is_empty() {
            return Ok(());
        }
        let txn = self.db.main().begin().await?;
        let mut item_ids = Vec::with_capacity(entries.len());
        let mut meta_rows = Vec::with_capacity(entries.len());
        let mut genre_rows = Vec::new();
        let mut people_rows = Vec::new();

        for entry in entries {
            let item_id = entry.item_id.to_string();
            item_ids.push(item_id.clone());
            let episode_sort = format!(
                "{:08}/{}",
                entry.episode.unwrap_or(i32::MAX),
                sort_key(&entry.title)
            );
            let studios = if entry.work_studios.is_empty() {
                None
            } else {
                serde_json::to_string(&entry.work_studios).ok()
            };
            meta_rows.push(video::ActiveModel {
                item_id: Set(item_id.clone()),
                library_id: Set(library.to_string()),
                stamp: Set(entry.stamp),
                work_key: Set(sha256_hex(&entry.work_group)),
                work_sort: Set(sort_key(&entry.work_title)),
                work_title: Set(entry.work_title),
                work_year: Set(entry.year),
                work_overview: Set(entry.work_overview),
                work_poster: Set(entry.work_poster),
                work_original_title: Set(entry.work_original_title),
                work_tagline: Set(entry.work_tagline),
                work_rating: Set(entry.work_rating.map(|r| r.clamp(0, 100))),
                work_official_rating: Set(entry
                    .work_official_rating
                    .map(|s| truncate_chars(&s, 32))),
                work_premiered: Set(entry.work_premiered.map(|s| truncate_chars(&s, 10))),
                work_studios: Set(studios),
                work_backdrop: Set(entry.work_backdrop),
                season: Set(entry.season),
                season_poster: Set(entry.season_poster),
                episode: Set(entry.episode),
                episode_sort: Set(episode_sort),
                title: Set(entry.title),
                overview: Set(entry.overview),
                episode_poster: Set(entry.episode_poster),
                runtime_ms: Set(entry.runtime_ms),
                rating: Set(entry.rating.map(|r| r.clamp(0, 100))),
                premiered: Set(entry.premiered.map(|s| truncate_chars(&s, 10))),
                thumb: Set(entry.thumb),
            });

            let mut seen_genres = HashSet::new();
            for genre in entry.genres {
                let name = truncate_chars(genre.trim(), 128);
                if name.is_empty() {
                    continue;
                }
                if seen_genres.insert(name.to_lowercase()) {
                    genre_rows.push(video_genres::ActiveModel {
                        item_id: Set(item_id.clone()),
                        name: Set(name),
                    });
                }
            }

            for (pos, person) in entry.people.into_iter().take(100).enumerate() {
                let name = truncate_chars(person.name.trim(), 256);
                if name.is_empty() {
                    continue;
                }
                let position = i32::try_from(pos).unwrap_or(i32::MAX);
                people_rows.push(video_people::ActiveModel {
                    item_id: Set(item_id.clone()),
                    position: Set(position),
                    kind: Set(person.kind.as_str().to_owned()),
                    person_key: Set(person_key(&name)),
                    name: Set(name),
                    role: Set(person.role),
                    image: Set(person.image),
                });
            }
        }

        video::Entity::insert_many(meta_rows)
            .on_conflict(
                OnConflict::column(C::ItemId)
                    .update_columns([
                        C::Stamp,
                        C::WorkKey,
                        C::WorkSort,
                        C::WorkTitle,
                        C::WorkYear,
                        C::WorkOverview,
                        C::WorkPoster,
                        C::WorkOriginalTitle,
                        C::WorkTagline,
                        C::WorkRating,
                        C::WorkOfficialRating,
                        C::WorkPremiered,
                        C::WorkStudios,
                        C::WorkBackdrop,
                        C::Season,
                        C::SeasonPoster,
                        C::Episode,
                        C::EpisodeSort,
                        C::Title,
                        C::Overview,
                        C::EpisodePoster,
                        C::RuntimeMs,
                        C::Rating,
                        C::Premiered,
                        C::Thumb,
                    ])
                    .to_owned(),
            )
            .exec_without_returning(&txn)
            .await?;

        for chunk in item_ids.chunks(200) {
            video_genres::Entity::delete_many()
                .filter(video_genres::Column::ItemId.is_in(chunk.iter().cloned()))
                .exec(&txn)
                .await?;
            video_people::Entity::delete_many()
                .filter(video_people::Column::ItemId.is_in(chunk.iter().cloned()))
                .exec(&txn)
                .await?;
        }
        for chunk in genre_rows.chunks(200) {
            video_genres::Entity::insert_many(chunk.to_vec())
                .exec_without_returning(&txn)
                .await?;
        }
        for chunk in people_rows.chunks(200) {
            video_people::Entity::insert_many(chunk.to_vec())
                .exec_without_returning(&txn)
                .await?;
        }
        txn.commit().await?;
        Ok(())
    }

    pub async fn location(&self, id: ItemId) -> Result<Option<TrackLocation>, DbError> {
        self.db
            .items()
            .original_location(id, LibraryKind::Video)
            .await
    }

    pub async fn poster_location(
        &self,
        id: ItemId,
        level: PosterLevel,
    ) -> Result<Option<TrackLocation>, DbError> {
        let col = match level {
            PosterLevel::Work => C::WorkPoster,
            PosterLevel::Season => C::SeasonPoster,
            PosterLevel::Episode => C::EpisodePoster,
        };
        self.sidecar_location(id, col).await
    }

    pub async fn backdrop_location(&self, id: ItemId) -> Result<Option<TrackLocation>, DbError> {
        self.sidecar_location(id, C::WorkBackdrop).await
    }

    async fn sidecar_location(&self, id: ItemId, col: C) -> Result<Option<TrackLocation>, DbError> {
        let Some(mut file) = self.location(id).await? else {
            return Ok(None);
        };
        let name = video::Entity::find_by_id(id.to_string())
            .select_only()
            .column(col)
            .into_tuple::<Option<String>>()
            .one(self.db.main())
            .await?
            .flatten();
        let Some(path) = name.and_then(|name| {
            PathBuf::from(name)
                .strip_prefix(&file.root)
                .ok()
                .map(Path::to_owned)
        }) else {
            return Ok(None);
        };
        file.relative_path = path;
        Ok(Some(file))
    }

    pub async fn thumb_location(&self, id: ItemId) -> Result<Option<VideoThumbLocation>, DbError> {
        let Some(video_loc) = self.location(id).await? else {
            return Ok(None);
        };
        let row = video::Entity::find_by_id(id.to_string())
            .select_only()
            .column(C::Thumb)
            .column(C::RuntimeMs)
            .into_tuple::<(Option<String>, Option<i64>)>()
            .one(self.db.main())
            .await?;
        let (thumb_name, runtime_ms) = row.unwrap_or((None, None));
        let local_thumb = thumb_name.and_then(|name| {
            let rel = PathBuf::from(name)
                .strip_prefix(&video_loc.root)
                .ok()
                .map(Path::to_owned)?;
            Some(TrackLocation {
                root: video_loc.root.clone(),
                relative_path: rel,
                cue_frames: None,
            })
        });
        Ok(Some(VideoThumbLocation {
            video: video_loc,
            local_thumb,
            runtime_ms,
        }))
    }

    pub async fn person_image_location(
        &self,
        library: LibraryId,
        key: &str,
    ) -> Result<Option<TrackLocation>, DbError> {
        let rows = video_people::Entity::find()
            .join(JoinType::InnerJoin, video_people::Relation::Video.def())
            .join(JoinType::InnerJoin, video_people::Relation::Item.def())
            .filter(video_people::Column::PersonKey.eq(key))
            .filter(C::LibraryId.eq(library.to_string()))
            .filter(items::Column::DeletedAt.is_null())
            .filter(video_people::Column::Image.is_not_null())
            .select_only()
            .column(video_people::Column::ItemId)
            .column(video_people::Column::Image)
            .limit(8)
            .into_tuple::<(String, Option<String>)>()
            .all(self.db.main())
            .await?;
        for (item_id, image) in rows {
            let Some(image) = image else { continue };
            let Ok(id) = parse_id::<ItemId>(&item_id, "video id") else {
                continue;
            };
            let Some(mut loc) = self.location(id).await? else {
                continue;
            };
            if let Ok(rel) = PathBuf::from(&image).strip_prefix(&loc.root) {
                loc.relative_path = rel.to_owned();
                return Ok(Some(loc));
            }
        }
        Ok(None)
    }

    pub async fn genres(&self, library: LibraryId) -> Result<VideoGenreList, DbError> {
        let rows = video_genres::Entity::find()
            .join(JoinType::InnerJoin, video_genres::Relation::Video.def())
            .join(JoinType::InnerJoin, video_genres::Relation::Item.def())
            .filter(C::LibraryId.eq(library.to_string()))
            .filter(items::Column::DeletedAt.is_null())
            .select_only()
            .column(video_genres::Column::Name)
            .column_as(
                Expr::col((video::Entity, C::WorkKey)).count_distinct(),
                "count",
            )
            .group_by(video_genres::Column::Name)
            .into_tuple::<(String, i64)>()
            .all(self.db.main())
            .await?;
        let mut genres: Vec<VideoGenre> = rows
            .into_iter()
            .map(|(name, count)| VideoGenre {
                name,
                count: u64::try_from(count).unwrap_or(0),
            })
            .collect();
        genres.sort_by(|a, b| {
            a.name
                .to_lowercase()
                .cmp(&b.name.to_lowercase())
                .then_with(|| a.name.cmp(&b.name))
        });
        Ok(VideoGenreList { genres })
    }

    pub async fn set_work_played(
        &self,
        user: UserId,
        library: LibraryId,
        work_key: &str,
        played: bool,
    ) -> Result<bool, DbError> {
        if !self.has_library(library).await? {
            return Ok(false);
        }
        let item_ids = active(library)
            .filter(C::WorkKey.eq(work_key))
            .select_only()
            .column(C::ItemId)
            .into_tuple::<String>()
            .all(self.db.main())
            .await?;
        if item_ids.is_empty() {
            return Ok(false);
        }
        let txn = self.db.main().begin().await?;
        crate::personal::apply_played_batch(&txn, user, &item_ids, played).await?;
        txn.commit().await?;
        Ok(true)
    }

    pub async fn set_work_favorite(
        &self,
        user: UserId,
        library: LibraryId,
        work_key: &str,
        favorite: bool,
    ) -> Result<bool, DbError> {
        if !self.has_library(library).await? {
            return Ok(false);
        }
        let exists = active(library)
            .filter(C::WorkKey.eq(work_key))
            .select_only()
            .column(C::ItemId)
            .limit(1)
            .into_tuple::<String>()
            .one(self.db.main())
            .await?
            .is_some();
        if !exists {
            return Ok(false);
        }
        let uid = user.to_string();
        let lib = library.to_string();
        if favorite {
            video_favorites::Entity::insert(video_favorites::ActiveModel {
                user_id: Set(uid),
                library_id: Set(lib),
                work_key: Set(work_key.to_owned()),
                created_at: Set(now_ms()),
            })
            .on_conflict(
                OnConflict::columns([
                    video_favorites::Column::UserId,
                    video_favorites::Column::LibraryId,
                    video_favorites::Column::WorkKey,
                ])
                .do_nothing()
                .to_owned(),
            )
            .exec_without_returning(self.db.main())
            .await?;
        } else {
            video_favorites::Entity::delete_by_id((uid, lib, work_key.to_owned()))
                .exec(self.db.main())
                .await?;
        }
        Ok(true)
    }

    pub(crate) async fn genres_for_items(
        &self,
        item_ids: &[String],
    ) -> Result<HashMap<String, Vec<String>>, DbError> {
        let mut map: HashMap<String, Vec<String>> = HashMap::new();
        for chunk in item_ids.chunks(200) {
            let rows = video_genres::Entity::find()
                .filter(video_genres::Column::ItemId.is_in(chunk.iter().cloned()))
                .order_by_asc(video_genres::Column::Name)
                .all(self.db.main())
                .await?;
            for row in rows {
                map.entry(row.item_id).or_default().push(row.name);
            }
        }
        Ok(map)
    }

    pub(crate) async fn work_states_for(
        &self,
        user: UserId,
        library: LibraryId,
        work_keys: &[String],
    ) -> Result<HashMap<String, VideoUserState>, DbError> {
        let mut favs = HashSet::new();
        let mut accs: HashMap<String, WorkStateAcc> = HashMap::new();
        let uid = user.to_string();
        let lib = library.to_string();
        for chunk in work_keys.chunks(100) {
            let fav_rows = video_favorites::Entity::find()
                .filter(video_favorites::Column::UserId.eq(&uid))
                .filter(video_favorites::Column::LibraryId.eq(&lib))
                .filter(video_favorites::Column::WorkKey.is_in(chunk.iter().cloned()))
                .select_only()
                .column(video_favorites::Column::WorkKey)
                .into_tuple::<String>()
                .all(self.db.main())
                .await?;
            favs.extend(fav_rows);

            let rows = join_user_state(active(library), user)
                .filter(C::WorkKey.is_in(chunk.iter().cloned()))
                .select_only()
                .column(C::WorkKey)
                .column(C::Season)
                .column_as(
                    Expr::col((user_item_state::Entity, user_item_state::Column::Completed)),
                    "completed",
                )
                .column_as(
                    Expr::col((user_item_state::Entity, user_item_state::Column::PositionMs)),
                    "position_ms",
                )
                .column_as(
                    Expr::col((user_item_state::Entity, user_item_state::Column::PlayCount)),
                    "play_count",
                )
                .column_as(
                    Expr::col((
                        user_item_state::Entity,
                        user_item_state::Column::LastPlayedAt,
                    )),
                    "last_played_at",
                )
                .into_model::<StateSample>()
                .all(self.db.main())
                .await?;
            for row in rows {
                accs.entry(row.work_key.clone()).or_default().push(&row);
            }
        }
        let mut result = HashMap::with_capacity(work_keys.len());
        for key in work_keys {
            let acc = accs.remove(key).unwrap_or_default();
            result.insert(key.clone(), acc.finish(favs.contains(key))?);
        }
        Ok(result)
    }

    async fn count_works(
        &self,
        library: LibraryId,
        user: Option<UserId>,
        filters: VideoFilters<'_>,
    ) -> Result<u64, DbError> {
        let grouped = apply_work_filters(active(library), library, user, filters)
            .select_only()
            .column(C::WorkKey)
            .group_by(C::WorkKey)
            .into_query();
        let count_stmt = Query::select()
            .expr(Expr::col(Asterisk).count())
            .from_subquery(grouped, Alias::new("video_works"))
            .to_owned();
        let row = self
            .db
            .main()
            .query_one(&count_stmt)
            .await?
            .ok_or_else(|| DbError::Corrupt("missing count row".into()))?;
        let total: i64 = row.try_get_by_index(0)?;
        Ok(u64::try_from(total).unwrap_or(0))
    }

    pub async fn browse(
        &self,
        library: LibraryId,
        series: Option<&str>,
        season: Option<i32>,
        limit: u32,
        cursor: Option<VideoCursor>,
    ) -> Result<VideoPage, DbError> {
        self.browse_ordered(library, series, season, VideoOrder::Title, limit, cursor)
            .await
    }

    pub async fn browse_ordered(
        &self,
        library: LibraryId,
        series: Option<&str>,
        season: Option<i32>,
        order: VideoOrder,
        limit: u32,
        cursor: Option<VideoCursor>,
    ) -> Result<VideoPage, DbError> {
        self.browse_filtered(
            library,
            series,
            season,
            VideoFilters {
                order,
                ..Default::default()
            },
            limit,
            cursor,
        )
        .await
    }

    pub async fn browse_filtered(
        &self,
        library: LibraryId,
        series: Option<&str>,
        season: Option<i32>,
        filters: VideoFilters<'_>,
        limit: u32,
        cursor: Option<VideoCursor>,
    ) -> Result<VideoPage, DbError> {
        self.browse_for(None, library, series, season, filters, limit, cursor)
            .await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn browse_for(
        &self,
        user: Option<UserId>,
        library: LibraryId,
        series: Option<&str>,
        season: Option<i32>,
        filters: VideoFilters<'_>,
        limit: u32,
        cursor: Option<VideoCursor>,
    ) -> Result<VideoPage, DbError> {
        let order = filters.order;
        if (series.is_some() || season.is_some())
            && (filters.filtered()
                || order != VideoOrder::Title
                || filters.dir.is_some_and(|d| d != SortDir::Asc))
        {
            return Err(DbError::Invalid(
                "Filters and custom ordering apply only to works",
            ));
        }
        let is_works = series.is_none() && season.is_none();
        let expect_offset = is_works && !filters.uses_keyset();
        let expect_poster = is_works && filters.uses_keyset() && order == VideoOrder::Poster;
        if cursor.as_ref().is_some_and(|c| {
            c.offset.is_some() != expect_offset || c.has_poster.is_some() != expect_poster
        }) {
            return Err(DbError::Invalid("Video cursor order does not match"));
        }
        let scope = filters.scope(library, series, season);
        if cursor.as_ref().is_some_and(|c| c.scope != scope) {
            return Err(DbError::Invalid("Video cursor filters do not match"));
        }
        let limit_u32 = limit.clamp(1, 200);
        let limit_usize = limit_u32 as usize;
        let fetch = u64::from(limit_u32) + 1;

        if let (Some(series_key), Some(season_num)) = (series, season) {
            return self
                .browse_episodes(
                    user,
                    library,
                    series_key,
                    season_num,
                    scope,
                    limit_usize,
                    fetch,
                    cursor,
                )
                .await;
        }
        if let Some(series_key) = series {
            return self
                .browse_seasons(user, library, series_key, scope, limit_usize, fetch, cursor)
                .await;
        }
        self.browse_works(user, library, filters, scope, limit_usize, fetch, cursor)
            .await
    }

    #[allow(clippy::too_many_arguments)]
    async fn browse_episodes(
        &self,
        user: Option<UserId>,
        library: LibraryId,
        series: &str,
        season: i32,
        scope: String,
        limit: usize,
        fetch: u64,
        cursor: Option<VideoCursor>,
    ) -> Result<VideoPage, DbError> {
        let total = active(library)
            .filter(C::WorkKey.eq(series))
            .filter(C::Season.eq(season))
            .select_only()
            .column_as(C::ItemId.count(), "count")
            .into_tuple::<i64>()
            .one(self.db.main())
            .await?
            .unwrap_or(0);
        let total = u64::try_from(total).unwrap_or(0);

        let mut query = active(library)
            .filter(C::WorkKey.eq(series))
            .filter(C::Season.eq(season));
        if let Some(cursor) = cursor {
            query = query.filter(after(C::EpisodeSort, C::ItemId, &cursor));
        }
        let mut rows = query
            .column_as(items::Column::CreatedAt, "created_at")
            .column_as(items::Column::SizeBytes, "size_bytes")
            .column_as(items::Column::Title, "item_title")
            .order_by_asc(C::EpisodeSort)
            .order_by_asc(C::ItemId)
            .limit(fetch)
            .into_model::<VideoWithItem>()
            .all(self.db.main())
            .await?;

        let has_more = rows.len() > limit;
        rows.truncate(limit);
        let item_ids: Vec<String> = rows.iter().map(|r| r.meta.item_id.clone()).collect();
        let mut genres_map = self.genres_for_items(&item_ids).await?;
        let states_map = match user {
            Some(uid) => self.db.personal().states_for(uid, &item_ids).await?,
            None => HashMap::new(),
        };

        let mut next_cursor = None;
        let mut entries = Vec::with_capacity(rows.len());
        for (idx, row) in rows.into_iter().enumerate() {
            let sort = row.meta.episode_sort.clone();
            let id_str = row.meta.item_id.clone();
            let genres = genres_map.remove(&id_str).unwrap_or_default();
            let state = user.map(|_| episode_user_state(states_map.get(&id_str)));
            let entry =
                build_episode_entry(library, row.meta, ts(row.created_at)?, genres, state, 1)?;
            if has_more && idx + 1 == limit {
                next_cursor = serde_json::to_vec(&VideoCursor {
                    scope: scope.clone(),
                    sort,
                    id: id_str,
                    has_poster: None,
                    offset: None,
                })
                .ok()
                .map(hex::encode);
            }
            entries.push(entry);
        }
        Ok(VideoPage {
            entries,
            next_cursor,
            total,
        })
    }

    #[allow(clippy::too_many_arguments)]
    async fn browse_seasons(
        &self,
        user: Option<UserId>,
        library: LibraryId,
        series: &str,
        scope: String,
        limit: usize,
        fetch: u64,
        cursor: Option<VideoCursor>,
    ) -> Result<VideoPage, DbError> {
        let total = active(library)
            .filter(C::WorkKey.eq(series))
            .filter(C::Season.is_not_null())
            .select_only()
            .column_as(
                Expr::col((video::Entity, C::Season)).count_distinct(),
                "count",
            )
            .into_tuple::<i64>()
            .one(self.db.main())
            .await?
            .unwrap_or(0);
        let total = u64::try_from(total).unwrap_or(0);

        let mut query = active(library)
            .filter(C::WorkKey.eq(series))
            .filter(C::Season.is_not_null());
        if let Some(cursor) = cursor {
            let position = cursor
                .sort
                .parse::<i32>()
                .map_err(|_| DbError::Invalid("invalid season position"))?;
            query = query.filter(C::Season.gt(position));
        }
        let mut rows = query
            .select_only()
            .column(C::Season)
            .column_as(poster_sample(C::SeasonPoster), "sample_id")
            .column_as(poster_sample(C::WorkBackdrop), "backdrop_id")
            .column_as(C::ItemId.count(), "files")
            .column_as(C::SeasonPoster.count(), "poster_count")
            .column_as(C::WorkBackdrop.count(), "backdrop_count")
            .column_as(items::Column::CreatedAt.max(), "max_created_at")
            .group_by(C::Season)
            .order_by_asc(C::Season)
            .limit(fetch)
            .into_tuple::<(i32, String, String, i64, i64, i64, i64)>()
            .all(self.db.main())
            .await?;

        let has_more = rows.len() > limit;
        rows.truncate(limit);
        if rows.is_empty() {
            return Ok(VideoPage {
                entries: Vec::new(),
                next_cursor: None,
                total,
            });
        }

        let sample_ids: Vec<String> = rows.iter().map(|r| r.1.clone()).collect();
        let meta_rows = active(library)
            .filter(C::ItemId.is_in(sample_ids.clone()))
            .all(self.db.main())
            .await?;
        let mut samples: HashMap<_, _> = meta_rows
            .into_iter()
            .map(|r| (r.item_id.clone(), r))
            .collect();
        let mut genres_map = self.genres_for_items(&sample_ids).await?;

        let season_states = if let Some(uid) = user {
            let season_nums: Vec<i32> = rows.iter().map(|r| r.0).collect();
            let ep_rows = join_user_state(active(library), uid)
                .filter(C::WorkKey.eq(series))
                .filter(C::Season.is_in(season_nums))
                .select_only()
                .column(C::Season)
                .column_as(
                    Expr::col((user_item_state::Entity, user_item_state::Column::Completed)),
                    "completed",
                )
                .column_as(
                    Expr::col((user_item_state::Entity, user_item_state::Column::PlayCount)),
                    "play_count",
                )
                .column_as(
                    Expr::col((
                        user_item_state::Entity,
                        user_item_state::Column::LastPlayedAt,
                    )),
                    "last_played_at",
                )
                .into_tuple::<(Option<i32>, Option<bool>, Option<i64>, Option<i64>)>()
                .all(self.db.main())
                .await?;
            let mut by_season: HashMap<i32, (u64, u64, i64, Option<i64>)> = HashMap::new();
            for (s, completed, plays, last_at) in ep_rows {
                let Some(s) = s else { continue };
                let acc = by_season.entry(s).or_default();
                acc.0 += 1;
                if completed.unwrap_or(false) {
                    acc.1 += 1;
                }
                acc.2 = acc.2.saturating_add(plays.unwrap_or(0));
                if let Some(at) = last_at {
                    acc.3 = Some(acc.3.map_or(at, |prev| std::cmp::max(prev, at)));
                }
            }
            Some(by_season)
        } else {
            None
        };

        let mut next_cursor = None;
        let mut entries = Vec::with_capacity(rows.len());
        for (idx, (number, sample_id, backdrop_id, count, poster_count, backdrop_count, created)) in
            rows.into_iter().enumerate()
        {
            let Some(row) = samples.remove(&sample_id) else {
                continue;
            };
            let genres = genres_map.remove(&sample_id).unwrap_or_default();
            let poster_item_id = if poster_count > 0 {
                Some(parse_id(&sample_id, "poster id")?)
            } else {
                None
            };
            let backdrop_item_id = if backdrop_count > 0 {
                Some(parse_id(&backdrop_id, "backdrop id")?)
            } else {
                None
            };
            let state = match &season_states {
                Some(map) => {
                    let (tot, done, plays, last_at) = map.get(&number).copied().unwrap_or_default();
                    Some(VideoUserState {
                        played: tot > 0 && done == tot,
                        position_ms: 0,
                        play_count: plays,
                        favorite: false,
                        last_played_at: opt_ts(last_at)?,
                        unplayed_count: Some(tot.saturating_sub(done)),
                    })
                }
                None => None,
            };
            let id_str = number.to_string();
            if has_more && idx + 1 == limit {
                next_cursor = serde_json::to_vec(&VideoCursor {
                    scope: scope.clone(),
                    sort: id_str.clone(),
                    id: id_str.clone(),
                    has_poster: None,
                    offset: None,
                })
                .ok()
                .map(hex::encode);
            }
            entries.push(VideoEntry {
                id: id_str,
                kind: VideoKind::Season,
                library_id: library,
                title: row.work_title.clone(),
                file_count: u64::try_from(count).unwrap_or(0),
                year: row.work_year,
                overview: row.work_overview,
                season_number: Some(number),
                episode_number: None,
                poster_item_id,
                item_id: None,
                series_id: Some(row.work_key),
                series_title: Some(row.work_title),
                original_title: row.work_original_title,
                tagline: row.work_tagline,
                genres,
                runtime_ms: None,
                community_rating: rating_from_tenths(row.work_rating),
                official_rating: row.work_official_rating,
                premiered: row.work_premiered,
                backdrop_item_id,
                thumb_item_id: None,
                added_at: ts(created)?,
                state,
            });
        }
        Ok(VideoPage {
            entries,
            next_cursor,
            total,
        })
    }

    #[allow(clippy::too_many_arguments)]
    async fn browse_works(
        &self,
        user: Option<UserId>,
        library: LibraryId,
        filters: VideoFilters<'_>,
        scope: String,
        limit: usize,
        fetch: u64,
        cursor: Option<VideoCursor>,
    ) -> Result<VideoPage, DbError> {
        let order = filters.order;
        let dir = filters.effective_dir();

        if order == VideoOrder::Random {
            return self
                .browse_works_random(user, library, filters, scope, limit, cursor)
                .await;
        }

        let total = self.count_works(library, user, filters).await?;
        let mut query = apply_work_filters(active(library), library, user, filters);
        let uses_keyset = filters.uses_keyset();
        let sql_dir = match dir {
            SortDir::Asc => Order::Asc,
            SortDir::Desc => Order::Desc,
        };
        let offset = cursor.as_ref().and_then(|c| c.offset).unwrap_or(0);

        let sort_col: Expr = if uses_keyset {
            match order {
                VideoOrder::Title if filters.filtered() => {
                    let sort = C::WorkSort.min();
                    if let Some(cursor) = &cursor {
                        query = query.having(after(sort.clone(), C::WorkKey, cursor));
                    }
                    query = query.order_by_asc(sort.clone()).order_by_asc(C::WorkKey);
                    sort
                }
                VideoOrder::Title => {
                    if let Some(cursor) = &cursor {
                        query = query.filter(after(C::WorkSort, C::WorkKey, cursor));
                    }
                    query = query
                        .group_by(C::WorkSort)
                        .order_by_asc(C::WorkSort)
                        .order_by_asc(C::WorkKey);
                    C::WorkSort.into_simple_expr()
                }
                VideoOrder::Poster => {
                    let sort = C::WorkSort.min();
                    let without_poster = C::WorkPoster.count().eq(0_i64);
                    if let Some(cursor) = &cursor {
                        // From a covered work, every uncovered work follows, regardless of title.
                        let next = if cursor.has_poster == Some(true) {
                            Condition::any()
                        } else {
                            Condition::all()
                        };
                        query = query.having(next.add(without_poster.clone()).add(after(
                            sort.clone(),
                            C::WorkKey,
                            cursor,
                        )));
                    }
                    query = query
                        .order_by_asc(without_poster)
                        .order_by_asc(sort.clone())
                        .order_by_asc(C::WorkKey);
                    sort
                }
                _ => unreachable!(),
            }
        } else {
            let sort = C::WorkSort.min();
            match order {
                VideoOrder::Title => {
                    query = query
                        .order_by(sort.clone(), sql_dir.clone())
                        .order_by(C::WorkKey, sql_dir);
                }
                VideoOrder::Poster => {
                    let without_poster = C::WorkPoster.count().eq(0_i64);
                    query = query
                        .order_by(without_poster, sql_dir)
                        .order_by_asc(sort.clone())
                        .order_by_asc(C::WorkKey);
                }
                VideoOrder::Year => {
                    let metric = C::WorkYear.max();
                    query = query
                        .order_by_asc(nulls_last(metric.clone()))
                        .order_by(metric, sql_dir)
                        .order_by_asc(sort.clone())
                        .order_by_asc(C::WorkKey);
                }
                VideoOrder::Added => {
                    let metric = items::Column::CreatedAt.max();
                    query = query
                        .order_by(metric, sql_dir)
                        .order_by_asc(sort.clone())
                        .order_by_asc(C::WorkKey);
                }
                VideoOrder::Rating => {
                    let metric = C::WorkRating.max();
                    query = query
                        .order_by_asc(nulls_last(metric.clone()))
                        .order_by(metric, sql_dir)
                        .order_by_asc(sort.clone())
                        .order_by_asc(C::WorkKey);
                }
                VideoOrder::Runtime => {
                    let metric = C::RuntimeMs.max();
                    query = query
                        .order_by_asc(nulls_last(metric.clone()))
                        .order_by(metric, sql_dir)
                        .order_by_asc(sort.clone())
                        .order_by_asc(C::WorkKey);
                }
                VideoOrder::Random => unreachable!(),
            }
            if offset > 0 {
                query = query.offset(offset);
            }
            sort
        };

        let raw_rows = query
            .select_only()
            .column_as(sort_col, "sort_position")
            .column(C::WorkKey)
            .column_as(poster_sample(C::WorkPoster), "sample_id")
            .column_as(poster_sample(C::WorkBackdrop), "backdrop_id")
            .column_as(C::ItemId.count(), "files")
            .column_as(C::WorkPoster.count(), "poster_count")
            .column_as(C::WorkBackdrop.count(), "backdrop_count")
            .column_as(items::Column::CreatedAt.max(), "max_created_at")
            .column_as(C::RuntimeMs.max(), "max_runtime_ms")
            .group_by(C::WorkKey)
            .limit(fetch)
            .into_tuple::<(
                String,
                String,
                String,
                String,
                i64,
                i64,
                i64,
                i64,
                Option<i64>,
            )>()
            .all(self.db.main())
            .await?;

        let has_more = raw_rows.len() > limit;
        let mut summaries: Vec<WorkRowSummary> = raw_rows
            .into_iter()
            .take(limit)
            .map(
                |(
                    sort,
                    key,
                    sample_id,
                    backdrop_id,
                    files,
                    poster_count,
                    backdrop_count,
                    max_created_at,
                    max_runtime_ms,
                )| WorkRowSummary {
                    sort,
                    key,
                    sample_id,
                    backdrop_id,
                    files,
                    poster_count,
                    backdrop_count,
                    max_created_at,
                    max_runtime_ms,
                },
            )
            .collect();

        let entries = self.hydrate_works(user, library, &summaries).await?;
        let mut next_cursor = None;
        if has_more && !entries.is_empty() {
            let last = summaries.pop();
            if uses_keyset {
                if let Some(last) = last {
                    next_cursor = serde_json::to_vec(&VideoCursor {
                        scope,
                        sort: last.sort,
                        id: last.key,
                        has_poster: (order == VideoOrder::Poster).then_some(last.poster_count > 0),
                        offset: None,
                    })
                    .ok()
                    .map(hex::encode);
                }
            } else {
                let next_offset = offset.saturating_add(entries.len() as u64);
                next_cursor = serde_json::to_vec(&VideoCursor {
                    scope,
                    sort: String::new(),
                    id: String::new(),
                    has_poster: None,
                    offset: Some(next_offset),
                })
                .ok()
                .map(hex::encode);
            }
        }
        Ok(VideoPage {
            entries,
            next_cursor,
            total,
        })
    }

    async fn browse_works_random(
        &self,
        user: Option<UserId>,
        library: LibraryId,
        filters: VideoFilters<'_>,
        scope: String,
        limit: usize,
        cursor: Option<VideoCursor>,
    ) -> Result<VideoPage, DbError> {
        let query = apply_work_filters(active(library), library, user, filters);
        let raw_rows = query
            .select_only()
            .column_as(C::WorkSort.min(), "sort_position")
            .column(C::WorkKey)
            .column_as(poster_sample(C::WorkPoster), "sample_id")
            .column_as(poster_sample(C::WorkBackdrop), "backdrop_id")
            .column_as(C::ItemId.count(), "files")
            .column_as(C::WorkPoster.count(), "poster_count")
            .column_as(C::WorkBackdrop.count(), "backdrop_count")
            .column_as(items::Column::CreatedAt.max(), "max_created_at")
            .column_as(C::RuntimeMs.max(), "max_runtime_ms")
            .group_by(C::WorkKey)
            .into_tuple::<(
                String,
                String,
                String,
                String,
                i64,
                i64,
                i64,
                i64,
                Option<i64>,
            )>()
            .all(self.db.main())
            .await?;

        let total = raw_rows.len() as u64;
        let seed = filters.seed.unwrap_or("");
        let mut keyed: Vec<(String, WorkRowSummary)> = raw_rows
            .into_iter()
            .map(
                |(
                    sort,
                    key,
                    sample_id,
                    backdrop_id,
                    files,
                    poster_count,
                    backdrop_count,
                    max_created_at,
                    max_runtime_ms,
                )| {
                    let rank = sha256_hex(&format!("{seed}\0{key}"));
                    (
                        rank,
                        WorkRowSummary {
                            sort,
                            key,
                            sample_id,
                            backdrop_id,
                            files,
                            poster_count,
                            backdrop_count,
                            max_created_at,
                            max_runtime_ms,
                        },
                    )
                },
            )
            .collect();
        if filters.effective_dir() == SortDir::Desc {
            keyed.sort_unstable_by(|a, b| b.0.cmp(&a.0).then_with(|| b.1.key.cmp(&a.1.key)));
        } else {
            keyed.sort_unstable_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.key.cmp(&b.1.key)));
        }
        let offset = cursor.as_ref().and_then(|c| c.offset).unwrap_or(0);
        let offset_usize = usize::try_from(offset).unwrap_or(usize::MAX);
        let page_summaries: Vec<WorkRowSummary> = keyed
            .into_iter()
            .skip(offset_usize)
            .take(limit)
            .map(|(_, s)| s)
            .collect();
        let next_offset = offset.saturating_add(page_summaries.len() as u64);
        let has_more = next_offset < total;
        let entries = self.hydrate_works(user, library, &page_summaries).await?;
        let next_cursor = if has_more && !entries.is_empty() {
            serde_json::to_vec(&VideoCursor {
                scope,
                sort: String::new(),
                id: String::new(),
                has_poster: None,
                offset: Some(next_offset),
            })
            .ok()
            .map(hex::encode)
        } else {
            None
        };
        Ok(VideoPage {
            entries,
            next_cursor,
            total,
        })
    }

    pub(crate) async fn hydrate_works(
        &self,
        user: Option<UserId>,
        library: LibraryId,
        summaries: &[WorkRowSummary],
    ) -> Result<Vec<VideoEntry>, DbError> {
        if summaries.is_empty() {
            return Ok(Vec::new());
        }
        let sample_ids: Vec<String> = summaries.iter().map(|s| s.sample_id.clone()).collect();
        let work_keys: Vec<String> = summaries.iter().map(|s| s.key.clone()).collect();
        let rows = active(library)
            .filter(C::ItemId.is_in(sample_ids.clone()))
            .all(self.db.main())
            .await?;
        let mut samples: HashMap<_, _> = rows.into_iter().map(|r| (r.item_id.clone(), r)).collect();
        let mut genres_map = self.genres_for_items(&sample_ids).await?;
        let mut states_map = match user {
            Some(uid) => self.work_states_for(uid, library, &work_keys).await?,
            None => HashMap::new(),
        };

        let mut entries = Vec::with_capacity(summaries.len());
        for summary in summaries {
            let Some(row) = samples.remove(&summary.sample_id) else {
                continue;
            };
            let poster_item_id = if summary.poster_count > 0 {
                Some(parse_id(&summary.sample_id, "poster id")?)
            } else {
                None
            };
            let backdrop_item_id = if summary.backdrop_count > 0 {
                Some(parse_id(&summary.backdrop_id, "backdrop id")?)
            } else {
                None
            };
            let genres = genres_map.remove(&summary.sample_id).unwrap_or_default();
            let state = states_map.remove(&summary.key);
            entries.push(build_work_entry(
                library,
                summary.key.clone(),
                row,
                u64::try_from(summary.files).unwrap_or(0),
                poster_item_id,
                backdrop_item_id,
                summary.max_runtime_ms,
                ts(summary.max_created_at)?,
                genres,
                state,
            )?);
        }
        Ok(entries)
    }
}

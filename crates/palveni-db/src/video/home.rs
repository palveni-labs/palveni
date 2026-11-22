use std::collections::{HashMap, HashSet};

use palveni_core::{
    LibraryId, UserId,
    video::{VideoEntry, VideoHome, VideoUserState},
};
use sea_orm::sea_query::{Expr, ExprTrait, IntoCondition};
use sea_orm::{
    ColumnTrait, EntityTrait, FromQueryResult, JoinType, QueryFilter, QueryOrder, QuerySelect,
    RelationTrait, Select,
};

use super::{
    VideoWithItem, Videos, WorkRowSummary, build_episode_entry, build_work_entry,
    detail::{EpisodeCandidate, pick_next_up},
    episode_user_state, nulls_last, poster_sample,
};
use crate::{
    DbError,
    convert::{opt_ts, parse_id, ts},
    entities::{
        items, libraries, user_item_state, video_favorites,
        video_metadata::{self as video, Column as C},
    },
};

const HOME_ROW_LIMIT: usize = 24;
const RECENT_EPISODE_WINDOW_MS: i64 = 24 * 60 * 60 * 1000;

fn active_in(lib_ids: &[String]) -> Select<video::Entity> {
    video::Entity::find()
        .join(JoinType::InnerJoin, video::Relation::Item.def())
        .filter(C::LibraryId.is_in(lib_ids.iter().cloned()))
        .filter(items::Column::DeletedAt.is_null())
}

#[derive(FromQueryResult)]
struct ContinueRow {
    #[sea_orm(nested)]
    meta: video::Model,
    created_at: i64,
    position_ms: i64,
    play_count: i64,
    last_played_at: Option<i64>,
    favorite_order: Option<i64>,
}

impl Videos<'_> {
    pub async fn home(
        &self,
        user: UserId,
        library: Option<LibraryId>,
    ) -> Result<Option<VideoHome>, DbError> {
        let lib_ids: Vec<String> = match library {
            Some(id) => {
                if !self.has_library(id).await? {
                    return Ok(None);
                }
                vec![id.to_string()]
            }
            None => {
                libraries::Entity::find()
                    .filter(libraries::Column::Kind.eq("video"))
                    .filter(libraries::Column::DeletedAt.is_null())
                    .select_only()
                    .column(libraries::Column::Id)
                    .into_tuple::<String>()
                    .all(self.db.main())
                    .await?
            }
        };
        if lib_ids.is_empty() {
            return Ok(Some(VideoHome::default()));
        }

        let continue_watching = self.home_continue_watching(user, &lib_ids).await?;
        let next_up = self.home_next_up(user, &lib_ids).await?;
        let latest_movies = self.home_latest_movies(user, &lib_ids).await?;
        let latest_episodes = self.home_latest_episodes(user, &lib_ids).await?;

        Ok(Some(VideoHome {
            continue_watching,
            next_up,
            latest_movies,
            latest_episodes,
        }))
    }

    async fn home_continue_watching(
        &self,
        user: UserId,
        lib_ids: &[String],
    ) -> Result<Vec<VideoEntry>, DbError> {
        let uid = user.to_string();
        let missing_at = nulls_last(Expr::col((
            user_item_state::Entity,
            user_item_state::Column::LastPlayedAt,
        )));
        let rows = active_in(lib_ids)
            .join(
                JoinType::InnerJoin,
                video::Relation::UserState
                    .def()
                    .on_condition(move |_, right| {
                        Expr::col((right, user_item_state::Column::UserId))
                            .eq(uid.clone())
                            .into_condition()
                    }),
            )
            .filter(user_item_state::Column::Completed.eq(false))
            .filter(user_item_state::Column::PositionMs.gt(0_i64))
            .column_as(items::Column::CreatedAt, "created_at")
            .column_as(user_item_state::Column::PositionMs, "position_ms")
            .column_as(user_item_state::Column::PlayCount, "play_count")
            .column_as(user_item_state::Column::LastPlayedAt, "last_played_at")
            .column_as(user_item_state::Column::FavoriteOrder, "favorite_order")
            .order_by_asc(missing_at)
            .order_by_desc(user_item_state::Column::LastPlayedAt)
            .order_by_desc(user_item_state::Column::PlaybackVersion)
            .order_by_asc(C::ItemId)
            .limit(HOME_ROW_LIMIT as u64)
            .into_model::<ContinueRow>()
            .all(self.db.main())
            .await?;

        if rows.is_empty() {
            return Ok(Vec::new());
        }
        let item_ids: Vec<String> = rows.iter().map(|r| r.meta.item_id.clone()).collect();
        let mut genres_map = self.genres_for_items(&item_ids).await?;

        let movie_works: Vec<String> = rows
            .iter()
            .filter(|r| r.meta.season.is_none())
            .map(|r| r.meta.work_key.clone())
            .collect();
        let mut fav_movies: HashSet<(String, String)> = HashSet::new();
        if !movie_works.is_empty() {
            let fav_rows = video_favorites::Entity::find()
                .filter(video_favorites::Column::UserId.eq(user.to_string()))
                .filter(video_favorites::Column::LibraryId.is_in(lib_ids.iter().cloned()))
                .filter(video_favorites::Column::WorkKey.is_in(movie_works))
                .select_only()
                .column(video_favorites::Column::LibraryId)
                .column(video_favorites::Column::WorkKey)
                .into_tuple::<(String, String)>()
                .all(self.db.main())
                .await?;
            fav_movies.extend(fav_rows);
        }

        let mut entries = Vec::with_capacity(rows.len());
        for row in rows {
            let lib: LibraryId = parse_id(&row.meta.library_id, "library id")?;
            let id_str = row.meta.item_id.clone();
            let genres = genres_map.remove(&id_str).unwrap_or_default();
            let added_at = ts(row.created_at)?;
            if row.meta.season.is_some() {
                let state = VideoUserState {
                    played: false,
                    position_ms: row.position_ms,
                    play_count: row.play_count,
                    favorite: row.favorite_order.is_some(),
                    last_played_at: opt_ts(row.last_played_at)?,
                    unplayed_count: None,
                };
                entries.push(build_episode_entry(
                    lib,
                    row.meta,
                    added_at,
                    genres,
                    Some(state),
                    1,
                )?);
            } else {
                let item_id = parse_id(&id_str, "movie id")?;
                let poster = row.meta.work_poster.is_some().then_some(item_id);
                let backdrop = row.meta.work_backdrop.is_some().then_some(item_id);
                let runtime = row.meta.runtime_ms;
                let work_key = row.meta.work_key.clone();
                let fav = fav_movies.contains(&(row.meta.library_id.clone(), work_key.clone()));
                let state = VideoUserState {
                    played: false,
                    position_ms: row.position_ms,
                    play_count: row.play_count,
                    favorite: fav,
                    last_played_at: opt_ts(row.last_played_at)?,
                    unplayed_count: None,
                };
                entries.push(build_work_entry(
                    lib,
                    work_key,
                    row.meta,
                    1,
                    poster,
                    backdrop,
                    runtime,
                    added_at,
                    genres,
                    Some(state),
                )?);
            }
        }
        Ok(entries)
    }

    async fn home_next_up(
        &self,
        user: UserId,
        lib_ids: &[String],
    ) -> Result<Vec<VideoEntry>, DbError> {
        let uid = user.to_string();
        let last_at = Expr::col((
            user_item_state::Entity,
            user_item_state::Column::LastPlayedAt,
        ))
        .max();
        let missing_at = nulls_last(last_at.clone());
        let watched_series = active_in(lib_ids)
            .filter(C::Season.is_not_null())
            .join(
                JoinType::InnerJoin,
                video::Relation::UserState
                    .def()
                    .on_condition(move |_, right| {
                        Expr::col((right, user_item_state::Column::UserId))
                            .eq(uid.clone())
                            .into_condition()
                    }),
            )
            .filter(user_item_state::Column::Completed.eq(true))
            .select_only()
            .column(C::LibraryId)
            .column(C::WorkKey)
            .column_as(last_at.clone(), "last_at")
            .group_by(C::LibraryId)
            .group_by(C::WorkKey)
            .order_by_asc(missing_at)
            .order_by_desc(last_at)
            .order_by_asc(C::WorkKey)
            .limit(64)
            .into_tuple::<(String, String, Option<i64>)>()
            .all(self.db.main())
            .await?;

        if watched_series.is_empty() {
            return Ok(Vec::new());
        }
        let target_set: HashSet<(String, String)> = watched_series
            .iter()
            .map(|(l, k, _)| (l.clone(), k.clone()))
            .collect();
        let keys: Vec<String> = watched_series.iter().map(|(_, k, _)| k.clone()).collect();

        let all_episodes = active_in(lib_ids)
            .filter(C::Season.is_not_null())
            .filter(C::WorkKey.is_in(keys))
            .column_as(items::Column::CreatedAt, "created_at")
            .column_as(items::Column::SizeBytes, "size_bytes")
            .column_as(items::Column::Title, "item_title")
            .into_model::<VideoWithItem>()
            .all(self.db.main())
            .await?;

        let filtered: Vec<VideoWithItem> = all_episodes
            .into_iter()
            .filter(|ep| {
                target_set.contains(&(ep.meta.library_id.clone(), ep.meta.work_key.clone()))
            })
            .collect();
        let all_item_ids: Vec<String> = filtered.iter().map(|r| r.meta.item_id.clone()).collect();
        let states_map = self.db.personal().states_for(user, &all_item_ids).await?;

        let mut by_series: HashMap<(String, String), Vec<&VideoWithItem>> = HashMap::new();
        for ep in &filtered {
            by_series
                .entry((ep.meta.library_id.clone(), ep.meta.work_key.clone()))
                .or_default()
                .push(ep);
        }

        let mut chosen: Vec<(Option<i64>, &VideoWithItem)> = Vec::new();
        for (lib_str, work_key, _) in &watched_series {
            let Some(eps) = by_series.get(&(lib_str.clone(), work_key.clone())) else {
                continue;
            };
            let candidates: Vec<EpisodeCandidate<'_>> = eps
                .iter()
                .map(|row| EpisodeCandidate {
                    row,
                    state: states_map.get(&row.meta.item_id),
                })
                .collect();
            if let Some((next_ep, watched_at)) = pick_next_up(&candidates, false, false) {
                chosen.push((watched_at, next_ep));
            }
        }
        chosen.sort_by(|a, b| {
            b.0.is_some()
                .cmp(&a.0.is_some())
                .then_with(|| b.0.cmp(&a.0))
                .then_with(|| a.1.meta.work_sort.cmp(&b.1.meta.work_sort))
                .then_with(|| a.1.meta.work_key.cmp(&b.1.meta.work_key))
        });
        chosen.truncate(HOME_ROW_LIMIT);

        let chosen_ids: Vec<String> = chosen.iter().map(|(_, r)| r.meta.item_id.clone()).collect();
        let mut genres_map = self.genres_for_items(&chosen_ids).await?;
        let mut entries = Vec::with_capacity(chosen.len());
        for (_, ep) in chosen {
            let lib: LibraryId = parse_id(&ep.meta.library_id, "library id")?;
            let genres = genres_map.remove(&ep.meta.item_id).unwrap_or_default();
            let st = episode_user_state(states_map.get(&ep.meta.item_id));
            entries.push(build_episode_entry(
                lib,
                ep.meta.clone(),
                ts(ep.created_at)?,
                genres,
                Some(st),
                1,
            )?);
        }
        Ok(entries)
    }

    async fn home_latest_movies(
        &self,
        user: UserId,
        lib_ids: &[String],
    ) -> Result<Vec<VideoEntry>, DbError> {
        let rows = active_in(lib_ids)
            .filter(C::Season.is_null())
            .select_only()
            .column(C::LibraryId)
            .column_as(C::WorkSort.min(), "sort_position")
            .column(C::WorkKey)
            .column_as(poster_sample(C::WorkPoster), "sample_id")
            .column_as(poster_sample(C::WorkBackdrop), "backdrop_id")
            .column_as(C::ItemId.count(), "files")
            .column_as(C::WorkPoster.count(), "poster_count")
            .column_as(C::WorkBackdrop.count(), "backdrop_count")
            .column_as(items::Column::CreatedAt.max(), "max_created_at")
            .column_as(C::RuntimeMs.max(), "max_runtime_ms")
            .group_by(C::LibraryId)
            .group_by(C::WorkKey)
            .order_by_desc(items::Column::CreatedAt.max())
            .order_by_desc(C::WorkKey)
            .limit(HOME_ROW_LIMIT as u64)
            .into_tuple::<(
                String,
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

        if rows.is_empty() {
            return Ok(Vec::new());
        }
        let mut by_lib: HashMap<String, Vec<WorkRowSummary>> = HashMap::new();
        let mut order_keys = Vec::with_capacity(rows.len());
        for (
            lib_str,
            sort,
            key,
            sample_id,
            backdrop_id,
            files,
            poster_count,
            backdrop_count,
            max_created_at,
            max_runtime_ms,
        ) in rows
        {
            order_keys.push((lib_str.clone(), key.clone()));
            by_lib.entry(lib_str).or_default().push(WorkRowSummary {
                sort,
                key,
                sample_id,
                backdrop_id,
                files,
                poster_count,
                backdrop_count,
                max_created_at,
                max_runtime_ms,
            });
        }
        let mut hydrated: HashMap<(String, String), VideoEntry> = HashMap::new();
        for (lib_str, summaries) in by_lib {
            let lib: LibraryId = parse_id(&lib_str, "library id")?;
            for entry in self.hydrate_works(Some(user), lib, &summaries).await? {
                hydrated.insert((lib_str.clone(), entry.id.clone()), entry);
            }
        }
        Ok(order_keys
            .into_iter()
            .filter_map(|k| hydrated.remove(&k))
            .collect())
    }

    async fn home_latest_episodes(
        &self,
        user: UserId,
        lib_ids: &[String],
    ) -> Result<Vec<VideoEntry>, DbError> {
        let series_rows = active_in(lib_ids)
            .filter(C::Season.is_not_null())
            .select_only()
            .column(C::LibraryId)
            .column(C::WorkKey)
            .column_as(items::Column::CreatedAt.max(), "max_created_at")
            .group_by(C::LibraryId)
            .group_by(C::WorkKey)
            .order_by_desc(items::Column::CreatedAt.max())
            .order_by_desc(C::WorkKey)
            .limit(HOME_ROW_LIMIT as u64)
            .into_tuple::<(String, String, i64)>()
            .all(self.db.main())
            .await?;

        if series_rows.is_empty() {
            return Ok(Vec::new());
        }
        let target_set: HashSet<(String, String)> = series_rows
            .iter()
            .map(|(l, k, _)| (l.clone(), k.clone()))
            .collect();
        let keys: Vec<String> = series_rows.iter().map(|(_, k, _)| k.clone()).collect();

        let episodes = active_in(lib_ids)
            .filter(C::Season.is_not_null())
            .filter(C::WorkKey.is_in(keys))
            .column_as(items::Column::CreatedAt, "created_at")
            .column_as(items::Column::SizeBytes, "size_bytes")
            .column_as(items::Column::Title, "item_title")
            .into_model::<VideoWithItem>()
            .all(self.db.main())
            .await?;

        let mut by_series: HashMap<(String, String), Vec<VideoWithItem>> = HashMap::new();
        for ep in episodes {
            let key = (ep.meta.library_id.clone(), ep.meta.work_key.clone());
            if target_set.contains(&key) {
                by_series.entry(key).or_default().push(ep);
            }
        }

        let mut picked: Vec<(VideoWithItem, u64)> = Vec::with_capacity(series_rows.len());
        for (lib_str, work_key, max_created_at) in series_rows {
            let Some(mut eps) = by_series.remove(&(lib_str, work_key)) else {
                continue;
            };
            let cutoff = max_created_at.saturating_sub(RECENT_EPISODE_WINDOW_MS);
            let recent_count = eps.iter().filter(|ep| ep.created_at >= cutoff).count() as u64;
            eps.sort_by(|a, b| {
                b.created_at
                    .cmp(&a.created_at)
                    .then_with(|| b.meta.season.unwrap_or(0).cmp(&a.meta.season.unwrap_or(0)))
                    .then_with(|| b.meta.episode_sort.cmp(&a.meta.episode_sort))
                    .then_with(|| b.meta.item_id.cmp(&a.meta.item_id))
            });
            if let Some(latest) = eps.into_iter().next() {
                picked.push((latest, std::cmp::max(recent_count, 1)));
            }
        }

        let picked_ids: Vec<String> = picked.iter().map(|(r, _)| r.meta.item_id.clone()).collect();
        let mut genres_map = self.genres_for_items(&picked_ids).await?;
        let states_map = self.db.personal().states_for(user, &picked_ids).await?;
        let mut entries = Vec::with_capacity(picked.len());
        for (ep, recent_count) in picked {
            let lib: LibraryId = parse_id(&ep.meta.library_id, "library id")?;
            let id_str = ep.meta.item_id.clone();
            let genres = genres_map.remove(&id_str).unwrap_or_default();
            let st = episode_user_state(states_map.get(&id_str));
            entries.push(build_episode_entry(
                lib,
                ep.meta,
                ts(ep.created_at)?,
                genres,
                Some(st),
                recent_count,
            )?);
        }
        Ok(entries)
    }
}

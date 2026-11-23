use std::collections::{BTreeMap, HashMap, HashSet};

use palveni_core::{
    ItemId, LibraryId, UserId,
    personal::MediaState,
    video::{
        VideoEntry, VideoFile, VideoKind, VideoPerson, VideoPersonKind, VideoUserState,
        VideoWorkDetail,
    },
};
use sea_orm::sea_query::{Expr, ExprTrait};
use sea_orm::{
    ColumnTrait, EntityTrait, JoinType, QueryFilter, QueryOrder, QuerySelect, RelationTrait,
};

use super::{
    VideoWithItem, Videos, WorkRowSummary, active, build_episode_entry, build_work_entry,
    episode_user_state, poster_sample, rating_from_tenths,
};
use crate::{
    DbError,
    convert::{opt_ts, parse_id, ts},
    entities::{
        items, video_favorites, video_genres,
        video_metadata::{self as video, Column as C},
        video_people,
    },
};

pub(super) struct EpisodeCandidate<'a> {
    pub(super) row: &'a VideoWithItem,
    pub(super) state: Option<&'a MediaState>,
}

/// Emby-style next-up selection over a series's episodes.
/// Returns `(chosen_episode, last_watched_timestamp_ms)` when found.
pub(super) fn pick_next_up<'a>(
    episodes: &[EpisodeCandidate<'a>],
    include_in_progress: bool,
    fallback_first: bool,
) -> Option<(&'a VideoWithItem, Option<i64>)> {
    let mut regular: Vec<&EpisodeCandidate<'a>> = episodes
        .iter()
        .filter(|e| e.row.meta.season.is_some_and(|s| s > 0))
        .collect();
    if regular.is_empty() {
        regular = episodes.iter().collect();
    }
    if regular.is_empty() {
        return None;
    }
    regular.sort_by(|a, b| {
        a.row
            .meta
            .season
            .unwrap_or(0)
            .cmp(&b.row.meta.season.unwrap_or(0))
            .then_with(|| a.row.meta.episode_sort.cmp(&b.row.meta.episode_sort))
            .then_with(|| a.row.meta.item_id.cmp(&b.row.meta.item_id))
    });

    if include_in_progress
        && let Some(in_progress) = regular
            .iter()
            .filter(|e| e.state.is_some_and(|s| !s.completed && s.position_ms > 0))
            .max_by(|a, b| {
                let a_at = a
                    .state
                    .and_then(|s| s.last_played_at)
                    .map(|t| t.timestamp_millis())
                    .unwrap_or(i64::MIN);
                let b_at = b
                    .state
                    .and_then(|s| s.last_played_at)
                    .map(|t| t.timestamp_millis())
                    .unwrap_or(i64::MIN);
                a_at.cmp(&b_at)
                    .then_with(|| {
                        a.row
                            .meta
                            .season
                            .unwrap_or(0)
                            .cmp(&b.row.meta.season.unwrap_or(0))
                    })
                    .then_with(|| a.row.meta.episode_sort.cmp(&b.row.meta.episode_sort))
                    .then_with(|| a.row.meta.item_id.cmp(&b.row.meta.item_id))
            })
    {
        let at = in_progress
            .state
            .and_then(|s| s.last_played_at)
            .map(|t| t.timestamp_millis());
        return Some((in_progress.row, at));
    }

    let last_watched = regular
        .iter()
        .enumerate()
        .filter(|(_, e)| e.state.is_some_and(|s| s.completed))
        .max_by(|(_, a), (_, b)| {
            let a_at = a
                .state
                .and_then(|s| s.last_played_at)
                .map(|t| t.timestamp_millis())
                .unwrap_or(i64::MIN);
            let b_at = b
                .state
                .and_then(|s| s.last_played_at)
                .map(|t| t.timestamp_millis())
                .unwrap_or(i64::MIN);
            a_at.cmp(&b_at)
                .then_with(|| {
                    a.row
                        .meta
                        .season
                        .unwrap_or(0)
                        .cmp(&b.row.meta.season.unwrap_or(0))
                })
                .then_with(|| a.row.meta.episode_sort.cmp(&b.row.meta.episode_sort))
                .then_with(|| a.row.meta.item_id.cmp(&b.row.meta.item_id))
        });

    if let Some((last_idx, watched)) = last_watched {
        let watched_at = watched
            .state
            .and_then(|s| s.last_played_at)
            .map(|t| t.timestamp_millis());
        let next = regular[last_idx + 1..]
            .iter()
            .find(|e| !e.state.is_some_and(|s| s.completed))?;
        if !include_in_progress && next.state.is_some_and(|s| s.position_ms > 0) {
            // Already surfaced in continueWatching.
            return None;
        }
        return Some((next.row, watched_at));
    }

    if fallback_first {
        return regular.first().map(|e| (e.row, None));
    }
    None
}

impl Videos<'_> {
    pub async fn work_detail(
        &self,
        user: UserId,
        library: LibraryId,
        work_key: &str,
    ) -> Result<Option<VideoWorkDetail>, DbError> {
        if !self.has_library(library).await? {
            return Ok(None);
        }
        let rows = active(library)
            .filter(C::WorkKey.eq(work_key))
            .column_as(items::Column::CreatedAt, "created_at")
            .column_as(items::Column::SizeBytes, "size_bytes")
            .column_as(items::Column::Title, "item_title")
            .order_by_asc(C::Season)
            .order_by_asc(C::EpisodeSort)
            .order_by_asc(C::ItemId)
            .into_model::<VideoWithItem>()
            .all(self.db.main())
            .await?;
        if rows.is_empty() {
            return Ok(None);
        }

        let is_series = rows.iter().any(|r| r.meta.season.is_some());
        let item_ids: Vec<String> = rows.iter().map(|r| r.meta.item_id.clone()).collect();
        let states_map = self.db.personal().states_for(user, &item_ids).await?;
        let favorite = video_favorites::Entity::find_by_id((
            user.to_string(),
            library.to_string(),
            work_key.to_owned(),
        ))
        .one(self.db.main())
        .await?
        .is_some();

        let genres_map = self.genres_for_items(&item_ids).await?;
        let mut seen_genres = HashSet::new();
        let mut work_genres = Vec::new();
        for item_id in &item_ids {
            if let Some(list) = genres_map.get(item_id) {
                for g in list {
                    if seen_genres.insert(g.to_lowercase()) {
                        work_genres.push(g.clone());
                    }
                }
            }
        }
        work_genres.sort_by_key(|a| a.to_lowercase());

        let poster_row = rows
            .iter()
            .find(|r| r.meta.work_poster.is_some())
            .unwrap_or(&rows[0]);
        let backdrop_row = rows.iter().find(|r| r.meta.work_backdrop.is_some());
        let meta_row = rows
            .iter()
            .find(|r| {
                r.meta.work_overview.is_some()
                    || r.meta.work_studios.is_some()
                    || r.meta.work_rating.is_some()
            })
            .unwrap_or(poster_row);

        let poster_item_id = if poster_row.meta.work_poster.is_some() {
            Some(parse_id(&poster_row.meta.item_id, "poster id")?)
        } else {
            None
        };
        let backdrop_item_id = match backdrop_row {
            Some(r) => Some(parse_id(&r.meta.item_id, "backdrop id")?),
            None => None,
        };
        let max_created_at = rows.iter().map(|r| r.created_at).max().unwrap_or(0);
        let max_runtime_ms = rows.iter().filter_map(|r| r.meta.runtime_ms).max();

        // Summarize user state for the work.
        let mut completed_count: u64 = 0;
        let mut play_count: i64 = 0;
        let mut last_played_ms: Option<i64> = None;
        let mut resume_pos: i64 = 0;
        let mut resume_at: Option<i64> = None;
        for row in &rows {
            if let Some(st) = states_map.get(&row.meta.item_id) {
                if st.completed {
                    completed_count += 1;
                } else if st.position_ms > 0 {
                    let at = st.last_played_at.map(|t| t.timestamp_millis());
                    let replace = match (resume_at, at) {
                        (None, _) => resume_pos == 0 || at.is_some(),
                        (Some(prev), Some(curr)) => curr >= prev,
                        (Some(_), None) => false,
                    };
                    if replace {
                        resume_pos = st.position_ms;
                        resume_at = at;
                    }
                }
                play_count = play_count.saturating_add(st.play_count);
                if let Some(at) = st.last_played_at.map(|t| t.timestamp_millis()) {
                    last_played_ms = Some(last_played_ms.map_or(at, |p| std::cmp::max(p, at)));
                }
            }
        }
        let total_items = rows.len() as u64;
        let played = if is_series {
            total_items > 0 && completed_count == total_items
        } else {
            completed_count > 0
        };
        let work_state = VideoUserState {
            played,
            position_ms: if is_series || played { 0 } else { resume_pos },
            play_count,
            favorite,
            last_played_at: opt_ts(last_played_ms)?,
            unplayed_count: is_series.then_some(total_items.saturating_sub(completed_count)),
        };

        let studios: Vec<String> = meta_row
            .meta
            .work_studios
            .as_deref()
            .and_then(|raw| serde_json::from_str(raw).ok())
            .unwrap_or_default();

        // Load people from the first item in the work that carries people rows.
        let people_rows = self.load_work_people(&item_ids).await?;
        let people: Vec<VideoPerson> = people_rows
            .into_iter()
            .map(|p| VideoPerson {
                name: p.name,
                role: p.role,
                kind: VideoPersonKind::parse(&p.kind).unwrap_or(VideoPersonKind::Actor),
                image_url: p.image.map(|_| {
                    format!(
                        "/api/v1/libraries/{library}/videos/people/{}/image",
                        p.person_key
                    )
                }),
            })
            .collect();

        // Build seasons (series only) and files (movies only).
        let mut seasons = Vec::new();
        let mut files = Vec::new();
        let mut next_up = None;
        if is_series {
            let mut by_season: BTreeMap<i32, Vec<&VideoWithItem>> = BTreeMap::new();
            for row in &rows {
                if let Some(s) = row.meta.season {
                    by_season.entry(s).or_default().push(row);
                }
            }
            for (season_num, eps) in by_season {
                let s_poster = eps.iter().find(|e| e.meta.season_poster.is_some());
                let s_poster_id = match s_poster {
                    Some(e) => Some(parse_id(&e.meta.item_id, "season poster id")?),
                    None => None,
                };
                let s_created = eps.iter().map(|e| e.created_at).max().unwrap_or(0);
                let mut s_done = 0u64;
                let mut s_plays = 0i64;
                let mut s_last: Option<i64> = None;
                for ep in &eps {
                    if let Some(st) = states_map.get(&ep.meta.item_id) {
                        if st.completed {
                            s_done += 1;
                        }
                        s_plays = s_plays.saturating_add(st.play_count);
                        if let Some(at) = st.last_played_at.map(|t| t.timestamp_millis()) {
                            s_last = Some(s_last.map_or(at, |p| std::cmp::max(p, at)));
                        }
                    }
                }
                let s_total = eps.len() as u64;
                seasons.push(VideoEntry {
                    id: season_num.to_string(),
                    kind: VideoKind::Season,
                    library_id: library,
                    title: meta_row.meta.work_title.clone(),
                    file_count: s_total,
                    year: meta_row.meta.work_year,
                    overview: meta_row.meta.work_overview.clone(),
                    season_number: Some(season_num),
                    episode_number: None,
                    poster_item_id: s_poster_id,
                    item_id: None,
                    series_id: Some(work_key.to_owned()),
                    series_title: Some(meta_row.meta.work_title.clone()),
                    original_title: meta_row.meta.work_original_title.clone(),
                    tagline: meta_row.meta.work_tagline.clone(),
                    genres: work_genres.clone(),
                    runtime_ms: None,
                    community_rating: rating_from_tenths(meta_row.meta.work_rating),
                    official_rating: meta_row.meta.work_official_rating.clone(),
                    premiered: meta_row.meta.work_premiered.clone(),
                    backdrop_item_id,
                    thumb_item_id: None,
                    added_at: ts(s_created)?,
                    state: Some(VideoUserState {
                        played: s_total > 0 && s_done == s_total,
                        position_ms: 0,
                        play_count: s_plays,
                        favorite: false,
                        last_played_at: opt_ts(s_last)?,
                        unplayed_count: Some(s_total.saturating_sub(s_done)),
                    }),
                });
            }

            let candidates: Vec<EpisodeCandidate<'_>> = rows
                .iter()
                .map(|row| EpisodeCandidate {
                    row,
                    state: states_map.get(&row.meta.item_id),
                })
                .collect();
            if let Some((chosen, _)) = pick_next_up(&candidates, true, true) {
                let st = episode_user_state(states_map.get(&chosen.meta.item_id));
                next_up = Some(build_episode_entry(
                    library,
                    chosen.meta.clone(),
                    ts(chosen.created_at)?,
                    work_genres.clone(),
                    Some(st),
                    1,
                )?);
            }
        } else {
            let mut movie_rows: Vec<&VideoWithItem> = rows.iter().collect();
            movie_rows.sort_by(|a, b| {
                b.size_bytes
                    .unwrap_or(-1)
                    .cmp(&a.size_bytes.unwrap_or(-1))
                    .then_with(|| a.meta.item_id.cmp(&b.meta.item_id))
            });
            for row in movie_rows {
                let item_id: ItemId = parse_id(&row.meta.item_id, "movie file id")?;
                let title = if rows.len() > 1
                    && !row.meta.title.is_empty()
                    && row.meta.title != meta_row.meta.work_title
                {
                    row.meta.title.clone()
                } else {
                    row.item_title.clone()
                };
                files.push(VideoFile {
                    item_id,
                    title,
                    size_bytes: row.size_bytes,
                });
            }
        }

        let primary_item_id = if !is_series {
            files
                .first()
                .map(|f| f.item_id.to_string())
                .unwrap_or_else(|| poster_row.meta.item_id.clone())
        } else {
            poster_row.meta.item_id.clone()
        };
        let sample_for_work = video::Model {
            item_id: primary_item_id,
            ..meta_row.meta.clone()
        };
        let work = build_work_entry(
            library,
            work_key.to_owned(),
            sample_for_work,
            total_items,
            poster_item_id,
            backdrop_item_id,
            max_runtime_ms,
            ts(max_created_at)?,
            work_genres.clone(),
            Some(work_state),
        )?;

        let similar = self
            .similar_works(user, library, work_key, &work_genres)
            .await?;

        Ok(Some(VideoWorkDetail {
            work,
            studios,
            people,
            seasons,
            files,
            next_up,
            similar,
        }))
    }

    async fn load_work_people(
        &self,
        item_ids: &[String],
    ) -> Result<Vec<video_people::Model>, DbError> {
        let mut by_item: HashMap<String, Vec<video_people::Model>> = HashMap::new();
        for chunk in item_ids.chunks(50) {
            let rows = video_people::Entity::find()
                .filter(video_people::Column::ItemId.is_in(chunk.iter().cloned()))
                .order_by_asc(video_people::Column::Position)
                .all(self.db.main())
                .await?;
            for row in rows {
                by_item.entry(row.item_id.clone()).or_default().push(row);
            }
            if let Some(first_id) = chunk.iter().find(|id| by_item.contains_key(*id))
                && let Some(people) = by_item.remove(first_id)
            {
                return Ok(people);
            }
        }
        Ok(Vec::new())
    }

    async fn similar_works(
        &self,
        user: UserId,
        library: LibraryId,
        work_key: &str,
        genres: &[String],
    ) -> Result<Vec<VideoEntry>, DbError> {
        if genres.is_empty() {
            return Ok(Vec::new());
        }
        let matching = video_genres::Entity::find()
            .join(JoinType::InnerJoin, video_genres::Relation::Video.def())
            .join(JoinType::InnerJoin, video_genres::Relation::Item.def())
            .filter(C::LibraryId.eq(library.to_string()))
            .filter(items::Column::DeletedAt.is_null())
            .filter(C::WorkKey.ne(work_key))
            .filter(video_genres::Column::Name.is_in(genres.iter().cloned()))
            .select_only()
            .column(C::WorkKey)
            .column_as(
                Expr::col((video_genres::Entity, video_genres::Column::Name)).count_distinct(),
                "shared_genres",
            )
            .column_as(C::WorkRating.max(), "rating")
            .column_as(C::WorkSort.min(), "sort")
            .group_by(C::WorkKey)
            .into_tuple::<(String, i64, Option<i32>, String)>()
            .all(self.db.main())
            .await?;

        if matching.is_empty() {
            return Ok(Vec::new());
        }
        let mut ranked = matching;
        ranked.sort_by(|a, b| {
            b.1.cmp(&a.1)
                .then_with(|| b.2.is_some().cmp(&a.2.is_some()))
                .then_with(|| b.2.cmp(&a.2))
                .then_with(|| a.3.cmp(&b.3))
                .then_with(|| a.0.cmp(&b.0))
        });
        ranked.truncate(12);
        let keys: Vec<String> = ranked.into_iter().map(|(k, _, _, _)| k).collect();

        let summary_rows = active(library)
            .filter(C::WorkKey.is_in(keys.clone()))
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

        let mut by_key: HashMap<String, WorkRowSummary> = summary_rows
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
                    (
                        key.clone(),
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

        let ordered: Vec<WorkRowSummary> =
            keys.into_iter().filter_map(|k| by_key.remove(&k)).collect();
        self.hydrate_works(Some(user), library, &ordered).await
    }
}

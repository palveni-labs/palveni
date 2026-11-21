//! Movie and series views are separate from the underlying file tree.
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use palveni_core::{
    ItemId, ItemKind, LibraryId,
    video::{VideoEntry, VideoKind, VideoOrder, VideoPage},
};
use sea_orm::sea_query::{ExprTrait, OnConflict};
use sea_orm::{
    ColumnTrait, Condition, EntityTrait, IntoSimpleExpr, JoinType, QueryFilter, QueryOrder,
    QuerySelect, RelationTrait, Select, Set,
};
use serde::{Deserialize, Serialize};

use crate::{
    Db, DbError,
    convert::{parse_id, sha256_hex},
    entities::{
        items, libraries, library_paths,
        video_metadata::{self as video, Column as C},
    },
    music::TrackLocation,
};

pub struct Videos<'a> {
    pub(crate) db: &'a Db,
}

pub struct VideoMetadata {
    pub item_id: ItemId,
    pub stamp: String,
    pub work_group: String,
    pub work_title: String,
    pub year: Option<i32>,
    pub work_overview: Option<String>,
    pub work_poster: Option<String>,
    pub season: Option<i32>,
    pub season_poster: Option<String>,
    pub episode: Option<i32>,
    pub title: String,
    pub overview: Option<String>,
    pub episode_poster: Option<String>,
}

pub fn sidecar_fingerprint(value: &str) -> String {
    sha256_hex(value)
}

fn sort_key(value: &str) -> String {
    let lower = value.to_lowercase();
    let buf = lower.as_bytes();
    hex::encode(&buf[..buf.len().min(80)])
}

#[derive(Serialize, Deserialize)]
pub struct VideoCursor {
    scope: String,
    sort: String,
    id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    has_poster: Option<bool>,
}

fn scope(library: LibraryId, series: Option<&str>, season: Option<i32>) -> String {
    format!("{library}/{}/{season:?}", series.unwrap_or(""))
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
        if raw.len() > 1024 {
            return None;
        }
        let cursor: Self = serde_json::from_slice(&hex::decode(raw).ok()?).ok()?;
        (cursor.scope == scope(library, series, season)
            && cursor.has_poster.is_some() == (order == VideoOrder::Poster))
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

fn active(library: LibraryId) -> Select<video::Entity> {
    video::Entity::find()
        .join(JoinType::InnerJoin, video::Relation::Item.def())
        .filter(C::LibraryId.eq(library.to_string()))
        .filter(items::Column::DeletedAt.is_null())
}

fn poster_sample(poster: C) -> sea_orm::sea_query::Expr {
    use sea_orm::sea_query::{Expr, Func};
    let available = Expr::case(poster.is_not_null(), Expr::col((video::Entity, C::ItemId)));
    Func::coalesce([Expr::from(Func::min(available)), C::ItemId.min()]).into()
}

#[derive(Clone, Copy)]
pub enum PosterLevel {
    Work,
    Season,
    Episode,
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
        let rows = entries.into_iter().map(|entry| {
            let episode_sort = format!(
                "{:08}/{}",
                entry.episode.unwrap_or(i32::MAX),
                sort_key(&entry.title)
            );
            video::ActiveModel {
                item_id: Set(entry.item_id.to_string()),
                library_id: Set(library.to_string()),
                stamp: Set(entry.stamp),
                work_key: Set(sha256_hex(&entry.work_group)),
                work_sort: Set(sort_key(&entry.work_title)),
                work_title: Set(entry.work_title),
                work_year: Set(entry.year),
                work_overview: Set(entry.work_overview),
                work_poster: Set(entry.work_poster),
                season: Set(entry.season),
                season_poster: Set(entry.season_poster),
                episode: Set(entry.episode),
                episode_sort: Set(episode_sort),
                title: Set(entry.title),
                overview: Set(entry.overview),
                episode_poster: Set(entry.episode_poster),
            }
        });
        video::Entity::insert_many(rows)
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
                        C::Season,
                        C::SeasonPoster,
                        C::Episode,
                        C::EpisodeSort,
                        C::Title,
                        C::Overview,
                        C::EpisodePoster,
                    ])
                    .to_owned(),
            )
            .exec_without_returning(self.db.main())
            .await?;
        Ok(())
    }

    pub async fn location(&self, id: ItemId) -> Result<Option<TrackLocation>, DbError> {
        let rows = items::Entity::find_by_id(id.to_string())
            .join(
                JoinType::InnerJoin,
                items::Entity::belongs_to(libraries::Entity)
                    .from(items::Column::LibraryId)
                    .to(libraries::Column::Id)
                    .into(),
            )
            .join_rev(
                JoinType::InnerJoin,
                library_paths::Entity::belongs_to(libraries::Entity)
                    .from(library_paths::Column::LibraryId)
                    .to(libraries::Column::Id)
                    .into(),
            )
            .filter(items::Column::Kind.eq("video"))
            .filter(items::Column::DeletedAt.is_null())
            .filter(libraries::Column::Kind.eq("video"))
            .filter(libraries::Column::DeletedAt.is_null())
            .select_only()
            .column_as(items::Column::Path, "file_path")
            .column_as(library_paths::Column::Path, "root_path")
            .into_tuple::<(String, String)>()
            .all(self.db.main())
            .await?;
        Ok(rows.into_iter().find_map(|(file, root)| {
            let relative_path = Path::new(&file).strip_prefix(&root).ok()?.to_owned();
            Some(TrackLocation {
                root: root.into(),
                relative_path,
            })
        }))
    }

    pub async fn poster_location(
        &self,
        id: ItemId,
        level: PosterLevel,
    ) -> Result<Option<TrackLocation>, DbError> {
        let Some(mut file) = self.location(id).await? else {
            return Ok(None);
        };
        let col = match level {
            PosterLevel::Work => C::WorkPoster,
            PosterLevel::Season => C::SeasonPoster,
            PosterLevel::Episode => C::EpisodePoster,
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
        if order == VideoOrder::Poster && (series.is_some() || season.is_some()) {
            return Err(DbError::Invalid("Poster ordering applies only to works"));
        }
        if cursor
            .as_ref()
            .is_some_and(|c| c.has_poster.is_some() != (order == VideoOrder::Poster))
        {
            return Err(DbError::Invalid("Video cursor order does not match"));
        }
        let n = u64::from(limit.clamp(1, 200)) + 1;
        let mut entries: Vec<(String, Option<bool>, VideoEntry)> = Vec::new();
        let mut query = active(library);
        if let Some(series) = series {
            query = query
                .filter(C::WorkKey.eq(series))
                .filter(C::Season.is_not_null());
        }
        if let Some(season) = season {
            query = query.filter(C::Season.eq(season));
            if let Some(cursor) = cursor {
                query = query.filter(after(C::EpisodeSort, C::ItemId, &cursor));
            }
            for row in query
                .order_by_asc(C::EpisodeSort)
                .order_by_asc(C::ItemId)
                .limit(n)
                .all(self.db.main())
                .await?
            {
                let id = parse_id(&row.item_id, "episode id")?;
                let entry = VideoEntry {
                    id: row.item_id.clone(),
                    kind: VideoKind::Episode,
                    title: row.title,
                    file_count: 1,
                    year: row.work_year,
                    overview: row.overview,
                    season_number: row.season,
                    episode_number: row.episode,
                    poster_item_id: row.episode_poster.is_some().then_some(id),
                    item_id: Some(id),
                };
                entries.push((row.episode_sort, None, entry));
            }
        } else {
            let seasons = series.is_some();
            let mut groups = Vec::new();
            if seasons {
                if let Some(cursor) = cursor {
                    let position = cursor
                        .sort
                        .parse::<i32>()
                        .map_err(|_| DbError::Invalid("invalid season position"))?;
                    query = query.filter(C::Season.gt(position));
                }
                let rows = query
                    .select_only()
                    .column(C::Season)
                    .column_as(poster_sample(C::SeasonPoster), "sample_id")
                    .column_as(C::ItemId.count(), "files")
                    .group_by(C::Season)
                    .order_by_asc(C::Season)
                    .limit(n)
                    .into_tuple::<(i32, String, i64)>()
                    .all(self.db.main())
                    .await?;
                groups.extend(rows.into_iter().map(|(number, sample, count)| {
                    (number.to_string(), number.to_string(), sample, count, None)
                }));
            } else {
                let sort = match order {
                    VideoOrder::Title => {
                        if let Some(cursor) = &cursor {
                            query = query.filter(after(C::WorkSort, C::WorkKey, cursor));
                        }
                        query = query.group_by(C::WorkSort);
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
                        query = query.order_by_asc(without_poster);
                        sort
                    }
                };
                let rows = query
                    .select_only()
                    .column_as(sort.clone(), "sort_position")
                    .column(C::WorkKey)
                    .column_as(poster_sample(C::WorkPoster), "sample_id")
                    .column_as(C::ItemId.count(), "files")
                    .column_as(C::WorkPoster.count(), "poster_count")
                    .group_by(C::WorkKey)
                    .order_by_asc(sort)
                    .order_by_asc(C::WorkKey)
                    .limit(n)
                    .into_tuple::<(String, String, String, i64, i64)>()
                    .all(self.db.main())
                    .await?;
                groups.extend(rows.into_iter().map(|(sort, key, sample, count, posters)| {
                    (
                        sort,
                        key,
                        sample,
                        count,
                        (order == VideoOrder::Poster).then_some(posters > 0),
                    )
                }));
            }
            if !groups.is_empty() {
                let rows = active(library)
                    .filter(C::ItemId.is_in(groups.iter().map(|g| g.2.clone())))
                    .all(self.db.main())
                    .await?;
                let mut samples: HashMap<_, _> =
                    rows.into_iter().map(|r| (r.item_id.clone(), r)).collect();
                for (sort, key, sample, count, has_poster) in groups {
                    let Some(row) = samples.remove(&sample) else {
                        continue;
                    };
                    let id = parse_id(&sample, "video id")?;
                    let kind = if seasons {
                        VideoKind::Season
                    } else if row.season.is_some() {
                        VideoKind::Series
                    } else {
                        VideoKind::Movie
                    };
                    let poster = if seasons {
                        row.season_poster
                    } else {
                        row.work_poster
                    };
                    entries.push((
                        sort,
                        has_poster,
                        VideoEntry {
                            id: key,
                            kind,
                            title: row.work_title,
                            file_count: count.try_into().unwrap_or(0),
                            year: row.work_year,
                            overview: row.work_overview,
                            season_number: if seasons { row.season } else { None },
                            episode_number: None,
                            poster_item_id: poster.is_some().then_some(id),
                            item_id: (kind == VideoKind::Movie).then_some(id),
                        },
                    ));
                }
            }
        }
        let limit = limit.clamp(1, 200) as usize;
        let mut next_cursor = None;
        if entries.len() > limit {
            entries.truncate(limit);
            if let Some((sort, has_poster, entry)) = entries.last() {
                next_cursor = serde_json::to_vec(&VideoCursor {
                    scope: scope(library, series, season),
                    sort: sort.clone(),
                    id: entry.id.clone(),
                    has_poster: *has_poster,
                })
                .ok()
                .map(hex::encode);
            }
        }
        Ok(VideoPage {
            entries: entries.into_iter().map(|(_, _, entry)| entry).collect(),
            next_cursor,
        })
    }
}

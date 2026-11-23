//! Photos timeline, memories, place clusters, EXIF details and personal albums.

use std::collections::{BTreeMap, HashMap, HashSet};

use chrono::{DateTime, Datelike, Utc};
use palveni_core::photos::{
    PhotoAlbumRef, PhotoAlbumSummary, PhotoCameraBucket, PhotoInfo, PhotoMemories,
    PhotoMonthBucket, PhotoPlaceCluster, PhotoTimelineItem, PhotoTimelinePage, format_camera_name,
};
use palveni_core::{ItemId, ItemKind, LibraryId, LibraryKind, PhotoAlbumId, UserId};
use sea_orm::sea_query::{Expr, OnConflict};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, ConnectionTrait, EntityTrait, ExprTrait, JoinType,
    QueryFilter, QueryOrder, QuerySelect, RelationTrait, Set, TransactionTrait,
};
use serde::{Deserialize, Serialize};

use crate::convert::{now_ms, parse_id, parse_opt_id, sha256_hex, truncate_chars, ts};
use crate::entities::{
    items, libraries, library_paths,
    photo_album_items::{self as album_items, Column as AI},
    photo_albums::{self as albums, Column as A},
    photo_metadata::{self as meta, Column as M},
    user_item_state::{self as state, Column as S},
};
use crate::items::SORT_GROUP_LEAF;
use crate::music::TrackLocation;
use crate::{Db, DbError};

pub struct Photos<'a> {
    pub(crate) db: &'a Db,
}

#[derive(Clone, Debug)]
pub struct PhotoScanCandidate {
    pub id: ItemId,
    pub path: String,
    pub size_bytes: Option<i64>,
    pub modified_at: Option<i64>,
    pub stamp: Option<String>,
}

#[derive(Clone, Debug)]
pub struct PhotoMetadataUpsert {
    pub item_id: ItemId,
    pub stamp: String,
    pub taken_at: i64,
    pub day_bucket: String,
    pub month_bucket: String,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub orientation: i32,
    pub camera_make: Option<String>,
    pub camera_model: Option<String>,
    pub lens_model: Option<String>,
    pub iso: Option<i32>,
    pub f_number: Option<f64>,
    pub exposure_time: Option<String>,
    pub focal_length: Option<f64>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub content_hash: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct PhotoTimelineQuery<'a> {
    pub library_id: Option<LibraryId>,
    pub album_id: Option<PhotoAlbumId>,
    pub favorite: Option<bool>,
    pub has_location: Option<bool>,
    pub camera: Option<&'a str>,
    pub month: Option<&'a str>,
    pub place: Option<(i32, i32)>,
    pub q: Option<&'a str>,
    pub cursor: Option<PhotoCursor>,
    pub limit: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhotoCursor {
    #[serde(rename = "t")]
    pub taken_at: i64,
    #[serde(rename = "i")]
    pub item_id: String,
    #[serde(rename = "f")]
    pub filter_hash: String,
}

impl PhotoCursor {
    pub fn encode(&self) -> String {
        hex::encode(serde_json::to_vec(self).unwrap_or_default())
    }

    pub fn decode(raw: &str, expected_filter_hash: &str) -> Option<Self> {
        if raw.len() > 2048 {
            return None;
        }
        let bytes = hex::decode(raw).ok()?;
        let cursor: Self = serde_json::from_slice(&bytes).ok()?;
        (cursor.filter_hash == expected_filter_hash).then_some(cursor)
    }
}

pub fn timeline_filter_hash(query: &PhotoTimelineQuery<'_>) -> String {
    let place = query
        .place
        .map_or_else(String::new, |(la, lo)| format!("{la}:{lo}"));
    let raw = format!(
        "v1|lib={}|alb={}|fav={:?}|loc={:?}|cam={}|mon={}|plc={}|q={}",
        query
            .library_id
            .map_or_else(String::new, |id| id.to_string()),
        query.album_id.map_or_else(String::new, |id| id.to_string()),
        query.favorite,
        query.has_location,
        query.camera.unwrap_or(""),
        query.month.unwrap_or(""),
        place,
        query.q.unwrap_or(""),
    );
    sha256_hex(&raw)
}

/// Converts a latitude (-90..=90) into a 0.1-degree cell index (-900..=899).
pub fn lat_cell_index(lat: f64) -> i32 {
    #[allow(clippy::cast_possible_truncation)]
    let raw = (lat * 10.0).floor() as i32;
    raw.clamp(-900, 899)
}

/// Converts a longitude (-180..=180) into a 0.1-degree cell index (-1800..=1799).
pub fn lon_cell_index(lon: f64) -> i32 {
    #[allow(clippy::cast_possible_truncation)]
    let raw = (lon * 10.0).floor() as i32;
    raw.clamp(-1800, 1799)
}

pub fn format_place_key(lat_idx: i32, lon_idx: i32) -> String {
    format!(
        "{:.1},{:.1}",
        f64::from(lat_idx) / 10.0,
        f64::from(lon_idx) / 10.0
    )
}

pub fn parse_place_key(raw: &str) -> Option<(i32, i32)> {
    let (lat_str, lon_str) = raw.trim().split_once(',')?;
    let lat: f64 = lat_str.trim().parse().ok()?;
    let lon: f64 = lon_str.trim().parse().ok()?;
    if !lat.is_finite()
        || !lon.is_finite()
        || !(-90.0..=90.0).contains(&lat)
        || !(-180.0..=180.0).contains(&lon)
    {
        return None;
    }
    #[allow(clippy::cast_possible_truncation)]
    let lat_idx = (lat * 10.0).round() as i32;
    #[allow(clippy::cast_possible_truncation)]
    let lon_idx = (lon * 10.0).round() as i32;
    Some((lat_idx.clamp(-900, 899), lon_idx.clamp(-1800, 1799)))
}

pub fn format_place_label(lat: f64, lon: f64) -> String {
    let ns = if lat >= 0.0 { 'N' } else { 'S' };
    let ew = if lon >= 0.0 { 'E' } else { 'W' };
    format!("{:.1}°{ns}, {:.1}°{ew}", lat.abs(), lon.abs())
}

/// Computes lowercase hex SHA-256 over a sequence of byte slices.
pub fn sha256_bytes_chunks(chunks: &[&[u8]]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    for chunk in chunks {
        hasher.update(chunk);
    }
    hex::encode(hasher.finalize())
}

fn place_bounds(lat_idx: i32, lon_idx: i32) -> (f64, f64, f64, f64) {
    let low_lat = f64::from(lat_idx) / 10.0;
    let high_lat = if lat_idx >= 899 {
        90.000_1
    } else {
        f64::from(lat_idx + 1) / 10.0
    };
    let low_lon = f64::from(lon_idx) / 10.0;
    let high_lon = if lon_idx >= 1799 {
        180.000_1
    } else {
        f64::from(lon_idx + 1) / 10.0
    };
    (low_lat, high_lat, low_lon, high_lon)
}

#[derive(sea_orm::FromQueryResult)]
struct TimelineRow {
    item_id: String,
    library_id: String,
    title: String,
    taken_at: i64,
    day_bucket: String,
    month_bucket: String,
    width: Option<i32>,
    height: Option<i32>,
    orientation: i32,
    camera_make: Option<String>,
    camera_model: Option<String>,
    latitude: Option<f64>,
    longitude: Option<f64>,
    favorite_order: Option<i64>,
}

impl TimelineRow {
    fn into_item(self) -> Result<PhotoTimelineItem, DbError> {
        Ok(PhotoTimelineItem {
            id: parse_id(&self.item_id, "item id")?,
            library_id: parse_id(&self.library_id, "library id")?,
            title: self.title,
            taken_at: ts(self.taken_at)?,
            day_bucket: self.day_bucket,
            month_bucket: self.month_bucket,
            width: self.width,
            height: self.height,
            orientation: self.orientation,
            camera_make: self.camera_make,
            camera_model: self.camera_model,
            latitude: self.latitude,
            longitude: self.longitude,
            favorite: self.favorite_order.is_some_and(|v| v > 0),
        })
    }
}

fn base_photo_select(user_id: UserId) -> sea_orm::Select<meta::Entity> {
    let uid = user_id.to_string();
    meta::Entity::find()
        .join(JoinType::InnerJoin, meta::Relation::Item.def())
        .join(JoinType::InnerJoin, meta::Relation::Library.def())
        .join(
            JoinType::LeftJoin,
            meta::Relation::UserState
                .def()
                .on_condition(move |_left, right| {
                    Condition::all().add(Expr::col((right, S::UserId)).eq(uid.clone()))
                }),
        )
        .filter(items::Column::DeletedAt.is_null())
        .filter(items::Column::Kind.eq(ItemKind::Photo.as_str()))
        .filter(items::Column::SortGroup.eq(SORT_GROUP_LEAF))
        .filter(libraries::Column::DeletedAt.is_null())
        .filter(libraries::Column::Kind.eq(LibraryKind::Photo.as_str()))
}

fn select_timeline_columns(select: sea_orm::Select<meta::Entity>) -> sea_orm::Select<meta::Entity> {
    select
        .select_only()
        .column_as(M::ItemId, "item_id")
        .column_as(M::LibraryId, "library_id")
        .column_as(items::Column::Title, "title")
        .column_as(M::TakenAt, "taken_at")
        .column_as(M::DayBucket, "day_bucket")
        .column_as(M::MonthBucket, "month_bucket")
        .column_as(M::Width, "width")
        .column_as(M::Height, "height")
        .column_as(M::Orientation, "orientation")
        .column_as(M::CameraMake, "camera_make")
        .column_as(M::CameraModel, "camera_model")
        .column_as(M::Latitude, "latitude")
        .column_as(M::Longitude, "longitude")
        .column_as(S::FavoriteOrder, "favorite_order")
}

impl Photos<'_> {
    pub async fn has_photo_library(&self, library_id: LibraryId) -> Result<bool, DbError> {
        Ok(libraries::Entity::find_by_id(library_id.to_string())
            .filter(libraries::Column::DeletedAt.is_null())
            .filter(libraries::Column::Kind.eq(LibraryKind::Photo.as_str()))
            .select_only()
            .column(libraries::Column::Id)
            .into_tuple::<String>()
            .one(self.db.main())
            .await?
            .is_some())
    }

    pub async fn scan_index(
        &self,
        library_id: LibraryId,
    ) -> Result<Vec<PhotoScanCandidate>, DbError> {
        let rows = items::Entity::find()
            .join_rev(
                JoinType::LeftJoin,
                meta::Entity::belongs_to(items::Entity)
                    .from(M::ItemId)
                    .to(items::Column::Id)
                    .into(),
            )
            .filter(items::Column::LibraryId.eq(library_id.to_string()))
            .filter(items::Column::Kind.eq(ItemKind::Photo.as_str()))
            .filter(items::Column::SortGroup.eq(SORT_GROUP_LEAF))
            .filter(items::Column::DeletedAt.is_null())
            .filter(items::Column::Path.is_not_null())
            .select_only()
            .column(items::Column::Id)
            .column(items::Column::Path)
            .column(items::Column::SizeBytes)
            .column(items::Column::ModifiedAt)
            .column(M::Stamp)
            .into_tuple::<(
                String,
                Option<String>,
                Option<i64>,
                Option<i64>,
                Option<String>,
            )>()
            .all(self.db.main())
            .await?;

        let mut out = Vec::with_capacity(rows.len());
        for (id, path, size_bytes, modified_at, stamp) in rows {
            let Some(path) = path else { continue };
            out.push(PhotoScanCandidate {
                id: parse_id(&id, "item id")?,
                path,
                size_bytes,
                modified_at,
                stamp,
            });
        }
        Ok(out)
    }

    pub async fn upsert_batch(
        &self,
        library_id: LibraryId,
        rows: &[PhotoMetadataUpsert],
    ) -> Result<(), DbError> {
        if rows.is_empty() {
            return Ok(());
        }
        let txn = self.db.main().begin().await?;
        let live = libraries::Entity::find_by_id(library_id.to_string())
            .filter(libraries::Column::DeletedAt.is_null())
            .one(&txn)
            .await?;
        if live.is_none() {
            txn.rollback().await?;
            return Err(DbError::NotFound("library"));
        }

        let lib_str = library_id.to_string();
        let models: Vec<meta::ActiveModel> = rows
            .iter()
            .map(|r| meta::ActiveModel {
                item_id: Set(r.item_id.to_string()),
                library_id: Set(lib_str.clone()),
                stamp: Set(truncate_chars(&r.stamp, 64)),
                taken_at: Set(r.taken_at),
                day_bucket: Set(truncate_chars(&r.day_bucket, 10)),
                month_bucket: Set(truncate_chars(&r.month_bucket, 7)),
                width: Set(r.width),
                height: Set(r.height),
                orientation: Set(r.orientation.clamp(1, 8)),
                camera_make: Set(r.camera_make.as_deref().map(|s| truncate_chars(s, 128))),
                camera_model: Set(r.camera_model.as_deref().map(|s| truncate_chars(s, 128))),
                lens_model: Set(r.lens_model.as_deref().map(|s| truncate_chars(s, 128))),
                iso: Set(r.iso),
                f_number: Set(r.f_number),
                exposure_time: Set(r.exposure_time.as_deref().map(|s| truncate_chars(s, 32))),
                focal_length: Set(r.focal_length),
                latitude: Set(r.latitude),
                longitude: Set(r.longitude),
                content_hash: Set(r.content_hash.as_deref().map(|s| truncate_chars(s, 64))),
            })
            .collect();

        meta::Entity::insert_many(models)
            .on_conflict(
                OnConflict::column(M::ItemId)
                    .update_columns([
                        M::LibraryId,
                        M::Stamp,
                        M::TakenAt,
                        M::DayBucket,
                        M::MonthBucket,
                        M::Width,
                        M::Height,
                        M::Orientation,
                        M::CameraMake,
                        M::CameraModel,
                        M::LensModel,
                        M::Iso,
                        M::FNumber,
                        M::ExposureTime,
                        M::FocalLength,
                        M::Latitude,
                        M::Longitude,
                        M::ContentHash,
                    ])
                    .to_owned(),
            )
            .exec_without_returning(&txn)
            .await?;

        txn.commit().await?;
        Ok(())
    }

    async fn camera_condition(&self, camera_name: &str) -> Result<Condition, DbError> {
        let target = camera_name.trim();
        let pairs = meta::Entity::find()
            .filter(
                Condition::any()
                    .add(M::CameraMake.is_not_null())
                    .add(M::CameraModel.is_not_null()),
            )
            .select_only()
            .column(M::CameraMake)
            .column(M::CameraModel)
            .group_by(M::CameraMake)
            .group_by(M::CameraModel)
            .into_tuple::<(Option<String>, Option<String>)>()
            .all(self.db.main())
            .await?;

        let mut cond = Condition::any();
        let mut matched = false;
        for (make, model) in pairs {
            let matches_display = format_camera_name(make.as_deref(), model.as_deref())
                .is_some_and(|n| n.eq_ignore_ascii_case(target));
            let matches_raw = make
                .as_deref()
                .is_some_and(|m| m.trim().eq_ignore_ascii_case(target))
                || model
                    .as_deref()
                    .is_some_and(|m| m.trim().eq_ignore_ascii_case(target));
            if matches_display || matches_raw {
                matched = true;
                let mut pair_cond = Condition::all();
                pair_cond = match make {
                    Some(mk) => pair_cond.add(M::CameraMake.eq(mk)),
                    None => pair_cond.add(M::CameraMake.is_null()),
                };
                pair_cond = match model {
                    Some(md) => pair_cond.add(M::CameraModel.eq(md)),
                    None => pair_cond.add(M::CameraModel.is_null()),
                };
                cond = cond.add(pair_cond);
            }
        }
        if matched {
            Ok(cond)
        } else {
            Ok(Condition::all().add(M::ItemId.eq("")))
        }
    }

    pub async fn timeline(
        &self,
        user_id: UserId,
        query: &PhotoTimelineQuery<'_>,
    ) -> Result<PhotoTimelinePage, DbError> {
        if let Some(library_id) = query.library_id
            && !self.has_photo_library(library_id).await?
        {
            return Err(DbError::NotFound("library"));
        }
        if let Some(album_id) = query.album_id {
            let exists = albums::Entity::find_by_id(album_id.to_string())
                .filter(A::UserId.eq(user_id.to_string()))
                .select_only()
                .column(A::Id)
                .into_tuple::<String>()
                .one(self.db.main())
                .await?
                .is_some();
            if !exists {
                return Err(DbError::NotFound("photo album"));
            }
        }

        let camera_cond = match query.camera.map(str::trim).filter(|s| !s.is_empty()) {
            Some(cam) => Some(self.camera_condition(cam).await?),
            None => None,
        };

        let apply_filters =
            |mut sel: sea_orm::Select<meta::Entity>| -> sea_orm::Select<meta::Entity> {
                if let Some(library_id) = query.library_id {
                    sel = sel.filter(M::LibraryId.eq(library_id.to_string()));
                }
                if let Some(album_id) = query.album_id {
                    let aid = album_id.to_string();
                    sel = sel
                        .join_rev(
                            JoinType::InnerJoin,
                            album_items::Entity::belongs_to(meta::Entity)
                                .from(AI::ItemId)
                                .to(M::ItemId)
                                .into(),
                        )
                        .filter(AI::AlbumId.eq(aid));
                }
                if let Some(fav) = query.favorite {
                    if fav {
                        sel = sel.filter(S::FavoriteOrder.gt(0));
                    } else {
                        sel = sel.filter(
                            Condition::any()
                                .add(S::FavoriteOrder.is_null())
                                .add(S::FavoriteOrder.lte(0)),
                        );
                    }
                }
                if let Some(has_loc) = query.has_location {
                    if has_loc {
                        sel = sel
                            .filter(M::Latitude.is_not_null())
                            .filter(M::Longitude.is_not_null());
                    } else {
                        sel = sel.filter(
                            Condition::any()
                                .add(M::Latitude.is_null())
                                .add(M::Longitude.is_null()),
                        );
                    }
                }
                if let Some((lat_idx, lon_idx)) = query.place {
                    let (low_lat, high_lat, low_lon, high_lon) = place_bounds(lat_idx, lon_idx);
                    sel = sel
                        .filter(M::Latitude.gte(low_lat))
                        .filter(M::Latitude.lt(high_lat))
                        .filter(M::Longitude.gte(low_lon))
                        .filter(M::Longitude.lt(high_lon));
                }
                if let Some(ref cam_cond) = camera_cond {
                    sel = sel.filter(cam_cond.clone());
                }
                if let Some(q) = query.q.map(str::trim).filter(|s| !s.is_empty()) {
                    let lower = q.to_lowercase();
                    sel = sel.filter(
                        Condition::any()
                            .add(items::Column::SearchText.contains(&lower))
                            .add(M::DayBucket.contains(&lower)),
                    );
                }
                sel
            };

        let month_rows = apply_filters(base_photo_select(user_id))
            .select_only()
            .column(M::MonthBucket)
            .column_as(M::ItemId.count(), "cnt")
            .group_by(M::MonthBucket)
            .order_by_desc(M::MonthBucket)
            .into_tuple::<(String, i64)>()
            .all(self.db.main())
            .await?;

        let months: Vec<PhotoMonthBucket> = month_rows
            .into_iter()
            .map(|(month, cnt)| PhotoMonthBucket {
                month,
                count: u64::try_from(cnt).unwrap_or(0),
            })
            .collect();

        let total = match query.month.map(str::trim).filter(|s| !s.is_empty()) {
            Some(m) => months.iter().find(|b| b.month == m).map_or(0, |b| b.count),
            None => months.iter().map(|b| b.count).sum(),
        };

        let limit = query.limit.clamp(1, 200);
        let mut item_sel = select_timeline_columns(apply_filters(base_photo_select(user_id)));
        if let Some(m) = query.month.map(str::trim).filter(|s| !s.is_empty()) {
            item_sel = item_sel.filter(M::MonthBucket.eq(m));
        }
        if let Some(ref cursor) = query.cursor {
            item_sel = item_sel.filter(
                Condition::any().add(M::TakenAt.lt(cursor.taken_at)).add(
                    Condition::all()
                        .add(M::TakenAt.eq(cursor.taken_at))
                        .add(M::ItemId.lt(cursor.item_id.clone())),
                ),
            );
        }

        let mut rows = item_sel
            .order_by_desc(M::TakenAt)
            .order_by_desc(M::ItemId)
            .limit(u64::from(limit) + 1)
            .into_model::<TimelineRow>()
            .all(self.db.main())
            .await?;

        let page_len = usize::try_from(limit).unwrap_or(usize::MAX);
        let has_more = rows.len() > page_len;
        rows.truncate(page_len);
        let filter_hash = timeline_filter_hash(query);
        let next_cursor = if has_more {
            rows.last().map(|last| {
                PhotoCursor {
                    taken_at: last.taken_at,
                    item_id: last.item_id.clone(),
                    filter_hash,
                }
                .encode()
            })
        } else {
            None
        };

        let items = rows
            .into_iter()
            .map(TimelineRow::into_item)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(PhotoTimelinePage {
            items,
            months,
            next_cursor,
            total,
        })
    }

    pub async fn memories(
        &self,
        user_id: UserId,
        now: DateTime<Utc>,
    ) -> Result<PhotoMemories, DbError> {
        let mm_dd = format!("-{:02}-{:02}", now.month(), now.day());
        let year_cutoff = format!("{:04}-01-01", now.year());

        let on_this_day_rows = select_timeline_columns(base_photo_select(user_id))
            .filter(M::DayBucket.ends_with(&mm_dd))
            .filter(M::DayBucket.lt(year_cutoff))
            .order_by_desc(M::TakenAt)
            .order_by_desc(M::ItemId)
            .limit(24)
            .into_model::<TimelineRow>()
            .all(self.db.main())
            .await?;
        let on_this_day = on_this_day_rows
            .into_iter()
            .map(TimelineRow::into_item)
            .collect::<Result<Vec<_>, _>>()?;

        let fav_rows = select_timeline_columns(base_photo_select(user_id))
            .filter(S::FavoriteOrder.gt(0))
            .order_by_desc(M::TakenAt)
            .order_by_desc(M::ItemId)
            .limit(12)
            .into_model::<TimelineRow>()
            .all(self.db.main())
            .await?;
        let mut recent_highlights = Vec::with_capacity(24);
        let mut seen_ids = HashSet::new();
        for row in fav_rows {
            seen_ids.insert(row.item_id.clone());
            recent_highlights.push(row.into_item()?);
        }
        if recent_highlights.len() < 24 {
            let recent_rows = select_timeline_columns(base_photo_select(user_id))
                .order_by_desc(M::TakenAt)
                .order_by_desc(M::ItemId)
                .limit(36)
                .into_model::<TimelineRow>()
                .all(self.db.main())
                .await?;
            for row in recent_rows {
                if recent_highlights.len() >= 24 {
                    break;
                }
                if seen_ids.insert(row.item_id.clone()) {
                    recent_highlights.push(row.into_item()?);
                }
            }
        }

        let summary_rows = base_photo_select(user_id)
            .select_only()
            .column(M::ItemId)
            .column(M::CameraMake)
            .column(M::CameraModel)
            .column(M::Latitude)
            .column(M::Longitude)
            .order_by_desc(M::TakenAt)
            .order_by_desc(M::ItemId)
            .into_tuple::<(
                String,
                Option<String>,
                Option<String>,
                Option<f64>,
                Option<f64>,
            )>()
            .all(self.db.main())
            .await?;

        let total_photos = u64::try_from(summary_rows.len()).unwrap_or(u64::MAX);
        let mut geotagged_count: u64 = 0;
        let mut camera_counts: BTreeMap<String, u64> = BTreeMap::new();

        struct ClusterAcc {
            sum_lat: f64,
            sum_lon: f64,
            count: u64,
            cover_item_id: String,
        }
        let mut place_map: BTreeMap<(i32, i32), ClusterAcc> = BTreeMap::new();

        for (item_id, make, model, lat, lon) in summary_rows {
            if let Some(name) = format_camera_name(make.as_deref(), model.as_deref()) {
                *camera_counts.entry(name).or_insert(0) += 1;
            }
            if let (Some(la), Some(lo)) = (lat, lon)
                && la.is_finite()
                && lo.is_finite()
                && (-90.0..=90.0).contains(&la)
                && (-180.0..=180.0).contains(&lo)
            {
                geotagged_count += 1;
                let cell = (lat_cell_index(la), lon_cell_index(lo));
                let entry = place_map.entry(cell).or_insert_with(|| ClusterAcc {
                    sum_lat: 0.0,
                    sum_lon: 0.0,
                    count: 0,
                    cover_item_id: item_id.clone(),
                });
                entry.sum_lat += la;
                entry.sum_lon += lo;
                entry.count += 1;
            }
        }

        let mut cameras: Vec<PhotoCameraBucket> = camera_counts
            .into_iter()
            .map(|(name, count)| PhotoCameraBucket { name, count })
            .collect();
        cameras.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.name.cmp(&b.name)));

        let mut places: Vec<PhotoPlaceCluster> = Vec::with_capacity(place_map.len());
        for ((lat_idx, lon_idx), acc) in place_map {
            #[allow(clippy::cast_precision_loss)]
            let denom = std::cmp::Ord::max(acc.count, 1) as f64;
            let mean_lat = acc.sum_lat / denom;
            let mean_lon = acc.sum_lon / denom;
            let key = format_place_key(lat_idx, lon_idx);
            let label = format_place_label(mean_lat, mean_lon);
            places.push(PhotoPlaceCluster {
                key,
                label,
                latitude: (mean_lat * 10_000.0).round() / 10_000.0,
                longitude: (mean_lon * 10_000.0).round() / 10_000.0,
                count: acc.count,
                cover_item_id: parse_id(&acc.cover_item_id, "cover item id")?,
            });
        }
        places.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.key.cmp(&b.key)));

        Ok(PhotoMemories {
            on_this_day,
            recent_highlights,
            places,
            cameras,
            total_photos,
            geotagged_count,
        })
    }

    pub async fn photo_info(
        &self,
        user_id: UserId,
        item_id: ItemId,
        include_host_path: bool,
    ) -> Result<Option<PhotoInfo>, DbError> {
        let item_row = items::Entity::find_by_id(item_id.to_string())
            .join(
                JoinType::InnerJoin,
                items::Entity::belongs_to(libraries::Entity)
                    .from(items::Column::LibraryId)
                    .to(libraries::Column::Id)
                    .into(),
            )
            .filter(items::Column::DeletedAt.is_null())
            .filter(items::Column::Kind.eq(ItemKind::Photo.as_str()))
            .filter(items::Column::SortGroup.eq(SORT_GROUP_LEAF))
            .filter(libraries::Column::DeletedAt.is_null())
            .filter(libraries::Column::Kind.eq(LibraryKind::Photo.as_str()))
            .one(self.db.main())
            .await?;

        let Some(item) = item_row else {
            return Ok(None);
        };

        let meta_row = meta::Entity::find_by_id(item.id.clone())
            .one(self.db.main())
            .await?;

        let fav_row = state::Entity::find_by_id((user_id.to_string(), item.id.clone()))
            .one(self.db.main())
            .await?;
        let favorite = fav_row.is_some_and(|r| r.favorite_order.is_some_and(|v| v > 0));

        let album_rows = albums::Entity::find()
            .join(JoinType::InnerJoin, albums::Relation::Items.def())
            .filter(A::UserId.eq(user_id.to_string()))
            .filter(AI::ItemId.eq(item.id.clone()))
            .order_by_desc(A::UpdatedAt)
            .order_by_asc(A::Title)
            .select_only()
            .column(A::Id)
            .column(A::Title)
            .into_tuple::<(String, String)>()
            .all(self.db.main())
            .await?;

        let mut albums_out = Vec::with_capacity(album_rows.len());
        for (aid, atitle) in album_rows {
            albums_out.push(PhotoAlbumRef {
                id: parse_id(&aid, "photo album id")?,
                title: atitle,
            });
        }

        let raw_path = item.path.clone().unwrap_or_default();
        let file_name = std::path::Path::new(&raw_path)
            .file_name()
            .and_then(|n| n.to_str())
            .map_or_else(|| item.title.clone(), str::to_owned);

        let visible_path = if include_host_path {
            raw_path
        } else {
            let roots = library_paths::Entity::find()
                .filter(library_paths::Column::LibraryId.eq(item.library_id.clone()))
                .select_only()
                .column(library_paths::Column::Path)
                .into_tuple::<String>()
                .all(self.db.main())
                .await?;
            roots
                .iter()
                .find_map(|root| {
                    std::path::Path::new(&raw_path)
                        .strip_prefix(root)
                        .ok()
                        .and_then(|rel| rel.to_str())
                        .map(str::to_owned)
                })
                .unwrap_or_else(|| file_name.clone())
        };

        let fallback_ms = item.modified_at.unwrap_or(item.created_at);
        let fallback_dt = ts(fallback_ms)?;
        let (
            taken_at,
            day_bucket,
            month_bucket,
            width,
            height,
            orientation,
            camera_make,
            camera_model,
            lens_model,
            iso,
            f_number,
            exposure_time,
            focal_length,
            latitude,
            longitude,
            content_hash,
        ) = match meta_row {
            Some(m) => (
                ts(m.taken_at)?,
                m.day_bucket,
                m.month_bucket,
                m.width,
                m.height,
                m.orientation,
                m.camera_make,
                m.camera_model,
                m.lens_model,
                m.iso,
                m.f_number,
                m.exposure_time,
                m.focal_length,
                m.latitude,
                m.longitude,
                m.content_hash,
            ),
            None => (
                fallback_dt,
                fallback_dt.format("%Y-%m-%d").to_string(),
                fallback_dt.format("%Y-%m").to_string(),
                None,
                None,
                1,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            ),
        };

        Ok(Some(PhotoInfo {
            id: parse_id(&item.id, "item id")?,
            library_id: parse_id(&item.library_id, "library id")?,
            title: item.title,
            file_name,
            path: visible_path,
            size_bytes: item.size_bytes,
            taken_at,
            day_bucket,
            month_bucket,
            width,
            height,
            orientation,
            camera_make,
            camera_model,
            lens_model,
            iso,
            f_number,
            exposure_time,
            focal_length,
            latitude,
            longitude,
            content_hash,
            favorite,
            albums: albums_out,
        }))
    }

    pub async fn photo_location(
        &self,
        item_id: ItemId,
    ) -> Result<Option<(TrackLocation, String)>, DbError> {
        let Some(loc) = self
            .db
            .items()
            .original_location(item_id, LibraryKind::Photo)
            .await?
        else {
            return Ok(None);
        };
        let file_name = loc
            .relative_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("photo.jpg")
            .to_owned();
        Ok(Some((loc, file_name)))
    }

    pub async fn list_albums(&self, user_id: UserId) -> Result<Vec<PhotoAlbumSummary>, DbError> {
        let rows = albums::Entity::find()
            .filter(A::UserId.eq(user_id.to_string()))
            .order_by_desc(A::UpdatedAt)
            .order_by_desc(A::Id)
            .all(self.db.main())
            .await?;
        assemble_album_summaries(self.db.main(), rows).await
    }

    pub async fn get_album(
        &self,
        user_id: UserId,
        album_id: PhotoAlbumId,
    ) -> Result<Option<PhotoAlbumSummary>, DbError> {
        let row = albums::Entity::find_by_id(album_id.to_string())
            .filter(A::UserId.eq(user_id.to_string()))
            .one(self.db.main())
            .await?;
        match row {
            Some(r) => {
                let mut list = assemble_album_summaries(self.db.main(), vec![r]).await?;
                Ok(list.pop())
            }
            None => Ok(None),
        }
    }

    pub async fn create_album(
        &self,
        user_id: UserId,
        title: &str,
        description: Option<&str>,
        cover_item_id: Option<ItemId>,
    ) -> Result<PhotoAlbumSummary, DbError> {
        let clean_title = validate_album_title(title)?;
        let clean_desc = sanitize_description(description)?;
        if let Some(cover) = cover_item_id {
            ensure_live_photo(self.db.main(), cover).await?;
        }
        let now = now_ms();
        let id = PhotoAlbumId::generate();
        let model = albums::Model {
            id: id.to_string(),
            user_id: user_id.to_string(),
            title: clean_title,
            description: clean_desc,
            cover_item_id: cover_item_id.map(|c| c.to_string()),
            created_at: now,
            updated_at: now,
        };
        albums::Entity::insert(albums::ActiveModel::from(model.clone()))
            .exec_without_returning(self.db.main())
            .await?;
        let mut list = assemble_album_summaries(self.db.main(), vec![model]).await?;
        list.pop()
            .ok_or_else(|| DbError::Corrupt("missing created photo album".into()))
    }

    pub async fn update_album(
        &self,
        user_id: UserId,
        album_id: PhotoAlbumId,
        title: Option<&str>,
        description: Option<&str>,
        cover_item_id: Option<ItemId>,
    ) -> Result<PhotoAlbumSummary, DbError> {
        let txn = self.db.main().begin().await?;
        let Some(mut row) = albums::Entity::find_by_id(album_id.to_string())
            .filter(A::UserId.eq(user_id.to_string()))
            .one(&txn)
            .await?
        else {
            txn.rollback().await?;
            return Err(DbError::NotFound("photo album"));
        };

        if let Some(t) = title {
            row.title = validate_album_title(t)?;
        }
        if let Some(d) = description {
            row.description = sanitize_description(Some(d))?;
        }
        if let Some(cover) = cover_item_id {
            ensure_live_photo(&txn, cover).await?;
            row.cover_item_id = Some(cover.to_string());
        }
        row.updated_at = now_ms();
        let updated = row.clone();
        let active: albums::ActiveModel = row.into();
        active.reset_all().update(&txn).await?;
        let mut list = assemble_album_summaries(&txn, vec![updated]).await?;
        txn.commit().await?;
        list.pop()
            .ok_or_else(|| DbError::Corrupt("missing updated photo album".into()))
    }

    pub async fn delete_album(
        &self,
        user_id: UserId,
        album_id: PhotoAlbumId,
    ) -> Result<bool, DbError> {
        let res = albums::Entity::delete_many()
            .filter(A::Id.eq(album_id.to_string()))
            .filter(A::UserId.eq(user_id.to_string()))
            .exec(self.db.main())
            .await?;
        Ok(res.rows_affected > 0)
    }

    pub async fn add_album_items(
        &self,
        user_id: UserId,
        album_id: PhotoAlbumId,
        item_ids: &[ItemId],
    ) -> Result<PhotoAlbumSummary, DbError> {
        if item_ids.is_empty() || item_ids.len() > 500 {
            return Err(DbError::Invalid("itemIds must contain 1-500 items"));
        }
        let txn = self.db.main().begin().await?;
        let Some(mut album) = albums::Entity::find_by_id(album_id.to_string())
            .filter(A::UserId.eq(user_id.to_string()))
            .one(&txn)
            .await?
        else {
            txn.rollback().await?;
            return Err(DbError::NotFound("photo album"));
        };

        let mut unique_ids = Vec::with_capacity(item_ids.len());
        let mut seen = HashSet::with_capacity(item_ids.len());
        for &id in item_ids {
            let s = id.to_string();
            if seen.insert(s.clone()) {
                unique_ids.push(s);
            }
        }

        let live_ids: HashSet<String> = items::Entity::find()
            .join(
                JoinType::InnerJoin,
                items::Entity::belongs_to(libraries::Entity)
                    .from(items::Column::LibraryId)
                    .to(libraries::Column::Id)
                    .into(),
            )
            .filter(items::Column::Id.is_in(unique_ids.iter().cloned()))
            .filter(items::Column::DeletedAt.is_null())
            .filter(items::Column::Kind.eq(ItemKind::Photo.as_str()))
            .filter(libraries::Column::DeletedAt.is_null())
            .filter(libraries::Column::Kind.eq(LibraryKind::Photo.as_str()))
            .select_only()
            .column(items::Column::Id)
            .into_tuple::<String>()
            .all(&txn)
            .await?
            .into_iter()
            .collect();

        if live_ids.len() != unique_ids.len() {
            txn.rollback().await?;
            return Err(DbError::NotFound("photo item"));
        }

        let existing_rows = album_items::Entity::find()
            .filter(AI::AlbumId.eq(album.id.clone()))
            .order_by_desc(AI::Position)
            .all(&txn)
            .await?;
        let mut next_pos = existing_rows
            .first()
            .map_or(0i32, |r| r.position.saturating_add(1));
        let existing_set: HashSet<String> = existing_rows.into_iter().map(|r| r.item_id).collect();

        let now = now_ms();
        let mut inserts = Vec::new();
        for item_id in &unique_ids {
            if existing_set.contains(item_id) {
                continue;
            }
            inserts.push(album_items::ActiveModel {
                album_id: Set(album.id.clone()),
                item_id: Set(item_id.clone()),
                position: Set(next_pos),
                added_at: Set(now),
            });
            next_pos = next_pos.saturating_add(1);
        }

        if !inserts.is_empty() {
            album_items::Entity::insert_many(inserts)
                .exec_without_returning(&txn)
                .await?;
            if album.cover_item_id.is_none() {
                album.cover_item_id = unique_ids.first().cloned();
            }
            album.updated_at = now;
            let active: albums::ActiveModel = album.clone().into();
            active.reset_all().update(&txn).await?;
        }

        let mut list = assemble_album_summaries(&txn, vec![album]).await?;
        txn.commit().await?;
        list.pop()
            .ok_or_else(|| DbError::Corrupt("missing updated photo album".into()))
    }

    pub async fn remove_album_items(
        &self,
        user_id: UserId,
        album_id: PhotoAlbumId,
        item_ids: &[ItemId],
    ) -> Result<PhotoAlbumSummary, DbError> {
        if item_ids.is_empty() || item_ids.len() > 500 {
            return Err(DbError::Invalid("itemIds must contain 1-500 items"));
        }
        let txn = self.db.main().begin().await?;
        let Some(mut album) = albums::Entity::find_by_id(album_id.to_string())
            .filter(A::UserId.eq(user_id.to_string()))
            .one(&txn)
            .await?
        else {
            txn.rollback().await?;
            return Err(DbError::NotFound("photo album"));
        };

        let target_ids: Vec<String> = item_ids.iter().map(ToString::to_string).collect();
        let del = album_items::Entity::delete_many()
            .filter(AI::AlbumId.eq(album.id.clone()))
            .filter(AI::ItemId.is_in(target_ids.iter().cloned()))
            .exec(&txn)
            .await?;

        if del.rows_affected > 0 {
            if album
                .cover_item_id
                .as_ref()
                .is_some_and(|c| target_ids.contains(c))
            {
                album.cover_item_id = None;
            }
            album.updated_at = now_ms();
            let active: albums::ActiveModel = album.clone().into();
            active.reset_all().update(&txn).await?;
        }

        let mut list = assemble_album_summaries(&txn, vec![album]).await?;
        txn.commit().await?;
        list.pop()
            .ok_or_else(|| DbError::Corrupt("missing updated photo album".into()))
    }
}

fn validate_album_title(raw: &str) -> Result<String, DbError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.chars().count() > 128 || trimmed.chars().any(char::is_control)
    {
        return Err(DbError::Invalid(
            "title must contain 1-128 printable characters",
        ));
    }
    Ok(trimmed.to_owned())
}

fn sanitize_description(raw: Option<&str>) -> Result<Option<String>, DbError> {
    let Some(text) = raw else { return Ok(None) };
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    if trimmed.chars().count() > 1024 || trimmed.chars().any(|c| c.is_control() && c != '\n') {
        return Err(DbError::Invalid("description exceeds 1024 characters"));
    }
    Ok(Some(trimmed.to_owned()))
}

async fn ensure_live_photo(conn: &impl ConnectionTrait, item_id: ItemId) -> Result<(), DbError> {
    let live = items::Entity::find_by_id(item_id.to_string())
        .join(
            JoinType::InnerJoin,
            items::Entity::belongs_to(libraries::Entity)
                .from(items::Column::LibraryId)
                .to(libraries::Column::Id)
                .into(),
        )
        .filter(items::Column::DeletedAt.is_null())
        .filter(items::Column::Kind.eq(ItemKind::Photo.as_str()))
        .filter(libraries::Column::DeletedAt.is_null())
        .filter(libraries::Column::Kind.eq(LibraryKind::Photo.as_str()))
        .select_only()
        .column(items::Column::Id)
        .into_tuple::<String>()
        .one(conn)
        .await?
        .is_some();
    if !live {
        return Err(DbError::NotFound("photo item"));
    }
    Ok(())
}

async fn assemble_album_summaries(
    conn: &impl ConnectionTrait,
    rows: Vec<albums::Model>,
) -> Result<Vec<PhotoAlbumSummary>, DbError> {
    if rows.is_empty() {
        return Ok(Vec::new());
    }
    let album_ids: Vec<String> = rows.iter().map(|r| r.id.clone()).collect();
    let member_rows = album_items::Entity::find()
        .join(JoinType::InnerJoin, album_items::Relation::Item.def())
        .join(
            JoinType::InnerJoin,
            items::Entity::belongs_to(libraries::Entity)
                .from(items::Column::LibraryId)
                .to(libraries::Column::Id)
                .into(),
        )
        .filter(AI::AlbumId.is_in(album_ids))
        .filter(items::Column::DeletedAt.is_null())
        .filter(items::Column::Kind.eq(ItemKind::Photo.as_str()))
        .filter(libraries::Column::DeletedAt.is_null())
        .filter(libraries::Column::Kind.eq(LibraryKind::Photo.as_str()))
        .order_by_asc(AI::Position)
        .order_by_asc(AI::ItemId)
        .select_only()
        .column(AI::AlbumId)
        .column(AI::ItemId)
        .into_tuple::<(String, String)>()
        .all(conn)
        .await?;

    let mut counts: HashMap<String, u64> = HashMap::new();
    let mut previews: HashMap<String, Vec<ItemId>> = HashMap::new();
    let mut live_members: HashMap<String, HashSet<String>> = HashMap::new();

    for (aid, iid) in member_rows {
        *counts.entry(aid.clone()).or_insert(0) += 1;
        live_members
            .entry(aid.clone())
            .or_default()
            .insert(iid.clone());
        let list = previews.entry(aid).or_default();
        if list.len() < 4 {
            list.push(parse_id(&iid, "album preview item id")?);
        }
    }

    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let preview_item_ids = previews.remove(&row.id).unwrap_or_default();
        let explicit_cover = parse_opt_id::<ItemId>(row.cover_item_id.as_deref(), "cover item id")?;
        let cover_item_id = match explicit_cover {
            Some(cid) => {
                if live_members
                    .get(&row.id)
                    .is_some_and(|set| set.contains(&cid.to_string()))
                {
                    Some(cid)
                } else {
                    preview_item_ids.first().copied().or(Some(cid))
                }
            }
            None => preview_item_ids.first().copied(),
        };
        let item_count = counts.get(&row.id).copied().unwrap_or(0);
        out.push(PhotoAlbumSummary {
            id: parse_id(&row.id, "photo album id")?,
            title: row.title,
            description: row.description,
            cover_item_id,
            preview_item_ids,
            item_count,
            created_at: ts(row.created_at)?,
            updated_at: ts(row.updated_at)?,
        });
    }
    Ok(out)
}

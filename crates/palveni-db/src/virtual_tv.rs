//! Persistence and 24x7 clock-synced EPG scheduler for Virtual Live TV (`virtual_channels`).

use chrono::{DateTime, Utc};
use palveni_core::ids::{ItemId, JobId, LibraryId};
use palveni_core::virtual_tv::{
    SchedulableVideoItem, VirtualChannel, VirtualChannelGuide, VirtualTvGuide,
    compute_channel_schedule,
};
use sea_orm::sea_query::{Expr, ExprTrait, Func, Query};
use sea_orm::{
    ActiveValue::Set, ColumnTrait, EntityTrait, IntoSimpleExpr, JoinType, QueryFilter, QueryOrder,
    QuerySelect, RelationTrait,
};

use crate::Db;
use crate::convert::{now_ms, parse_id, truncate_chars, ts};
use crate::entities::{items, libraries, video_genres, video_metadata, virtual_channels};
use crate::error::DbError;

type VideoScheduleRow = (
    String,
    String,
    String,
    String,
    Option<i32>,
    Option<i32>,
    Option<i64>,
);

pub struct VirtualTv<'a> {
    pub(crate) db: &'a Db,
}

fn row_to_channel(row: virtual_channels::Model) -> Result<VirtualChannel, DbError> {
    let library_id: LibraryId = parse_id(&row.library_id, "library_id")?;
    Ok(VirtualChannel {
        id: row.id,
        number: row.number,
        name: row.name,
        library_id,
        genre_filter: row.genre_filter,
        enabled: row.enabled != 0,
        created_at: ts(row.created_at)?,
    })
}

impl VirtualTv<'_> {
    pub async fn list_channels(&self) -> Result<Vec<VirtualChannel>, DbError> {
        let rows = virtual_channels::Entity::find()
            .order_by_asc(virtual_channels::Column::Number)
            .order_by_asc(virtual_channels::Column::Id)
            .all(self.db.main())
            .await?;
        rows.into_iter().map(row_to_channel).collect()
    }

    pub async fn create_channel(
        &self,
        number: i32,
        name: &str,
        library_id: LibraryId,
        genre_filter: Option<&str>,
        enabled: bool,
    ) -> Result<VirtualChannel, DbError> {
        let lib_exists = libraries::Entity::find_by_id(library_id.to_string())
            .filter(libraries::Column::DeletedAt.is_null())
            .one(self.db.main())
            .await?
            .is_some();
        if !lib_exists {
            return Err(DbError::Invalid("Target video library does not exist"));
        }

        let dup = virtual_channels::Entity::find()
            .filter(virtual_channels::Column::Number.eq(number))
            .one(self.db.main())
            .await?;
        if dup.is_some() {
            return Err(DbError::Conflict(format!(
                "Channel number {number} is already in use"
            )));
        }

        let id = JobId::generate().to_string();
        let created_at = now_ms();
        let clean_name = truncate_chars(name.trim(), 128);
        let clean_genre = genre_filter
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|g| truncate_chars(g, 128));

        virtual_channels::Entity::insert(virtual_channels::ActiveModel {
            id: Set(id.clone()),
            number: Set(number),
            name: Set(clean_name.clone()),
            library_id: Set(library_id.to_string()),
            genre_filter: Set(clean_genre.clone()),
            enabled: Set(i32::from(enabled)),
            created_at: Set(created_at),
        })
        .exec_without_returning(self.db.main())
        .await?;

        Ok(VirtualChannel {
            id,
            number,
            name: clean_name,
            library_id,
            genre_filter: clean_genre,
            enabled,
            created_at: ts(created_at)?,
        })
    }

    pub async fn delete_channel(&self, id: &str) -> Result<bool, DbError> {
        let res = virtual_channels::Entity::delete_by_id(id.to_owned())
            .exec(self.db.main())
            .await?;
        Ok(res.rows_affected > 0)
    }

    pub async fn schedulable_items_for_channel(
        &self,
        library_id: LibraryId,
        genre_filter: Option<&str>,
    ) -> Result<Vec<SchedulableVideoItem>, DbError> {
        let mut query = video_metadata::Entity::find()
            .join(JoinType::InnerJoin, video_metadata::Relation::Item.def())
            .filter(video_metadata::Column::LibraryId.eq(library_id.to_string()))
            .filter(items::Column::DeletedAt.is_null())
            .filter(items::Column::Path.is_not_null());

        if let Some(genre) = genre_filter.map(str::trim).filter(|s| !s.is_empty()) {
            let sub = Query::select()
                .column(video_genres::Column::ItemId)
                .from(video_genres::Entity)
                .and_where(
                    Expr::from(Func::lower(video_genres::Column::Name.into_simple_expr()))
                        .eq(genre.to_lowercase()),
                )
                .to_owned();
            query = query.filter(video_metadata::Column::ItemId.in_subquery(sub));
        }

        let rows: Vec<VideoScheduleRow> = query
            .order_by_asc(video_metadata::Column::WorkSort)
            .order_by_asc(video_metadata::Column::WorkKey)
            .order_by_asc(video_metadata::Column::Season)
            .order_by_asc(video_metadata::Column::EpisodeSort)
            .order_by_asc(video_metadata::Column::ItemId)
            .select_only()
            .column(video_metadata::Column::ItemId)
            .column(video_metadata::Column::LibraryId)
            .column(video_metadata::Column::Title)
            .column(video_metadata::Column::WorkTitle)
            .column(video_metadata::Column::Season)
            .column(video_metadata::Column::Episode)
            .column(video_metadata::Column::RuntimeMs)
            .into_tuple()
            .all(self.db.main())
            .await?;

        let mut out = Vec::with_capacity(rows.len());
        for (item_id, lib_id, title, work_title, season, episode, runtime_ms) in rows {
            let parsed_item: ItemId = parse_id(&item_id, "item_id")?;
            let parsed_lib: LibraryId = parse_id(&lib_id, "library_id")?;
            out.push(SchedulableVideoItem {
                item_id: parsed_item,
                library_id: parsed_lib,
                title,
                work_title,
                season,
                episode,
                runtime_ms,
            });
        }
        Ok(out)
    }

    pub async fn guide_at(
        &self,
        now: DateTime<Utc>,
        playlist_url: String,
        epg_url: String,
    ) -> Result<VirtualTvGuide, DbError> {
        let rows = virtual_channels::Entity::find()
            .filter(virtual_channels::Column::Enabled.ne(0))
            .order_by_asc(virtual_channels::Column::Number)
            .order_by_asc(virtual_channels::Column::Id)
            .all(self.db.main())
            .await?;

        let mut channels = Vec::with_capacity(rows.len());
        for row in rows {
            let channel = row_to_channel(row)?;
            let items = self
                .schedulable_items_for_channel(channel.library_id, channel.genre_filter.as_deref())
                .await?;
            let total_items = items.len();
            let (cycle_duration_ms, now_playing, programmes) =
                compute_channel_schedule(&items, now);
            channels.push(VirtualChannelGuide {
                channel,
                total_items,
                cycle_duration_ms,
                now_playing,
                programmes,
            });
        }

        Ok(VirtualTvGuide {
            generated_at: now,
            channels,
            playlist_url,
            epg_url,
        })
    }
}

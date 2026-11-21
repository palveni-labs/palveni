//! Ordered personal music playlists. A saved version covers both the name
//! and membership, including distinct positions for duplicate tracks.
use std::collections::BTreeSet;

use palveni_core::{
    ItemId, UserId,
    ids::PlaylistId,
    personal::{Playlist, PlaylistPage, PlaylistSummary, PlaylistTrack, PlaylistTrackPage},
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, JoinType, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect, RelationTrait, Set, TransactionTrait,
    sea_query::{Expr, ExprTrait, IntoCondition},
};

use crate::{
    Db, DbError,
    convert::parse_id,
    entities::{
        items, playlist_entries as entries,
        playlists::{self as lists, Column as C},
        track_metadata as tags,
    },
    revision::next_revision,
};

pub struct Playlists<'a> {
    pub(crate) db: &'a Db,
}

const TRACK_BATCH: usize = 200;
pub const MAX_ENTRIES: usize = 5000;

fn summary(row: lists::Model) -> Result<PlaylistSummary, DbError> {
    Ok(PlaylistSummary {
        id: parse_id(&row.id, "playlist ID")?,
        name: row.name,
        version: row.version,
    })
}

async fn remove_entries(conn: &impl ConnectionTrait, id: PlaylistId) -> Result<(), DbError> {
    entries::Entity::delete_many()
        .filter(entries::Column::PlaylistId.eq(id.to_string()))
        .exec(conn)
        .await?;
    Ok(())
}

fn version_conflict() -> DbError {
    DbError::Conflict("Playlist has changed".into())
}

#[derive(sea_orm::FromQueryResult)]
struct TrackRow {
    position: i32,
    item_id: String,
    library_id: String,
    title: String,
    artist: Option<String>,
    album: Option<String>,
    duration_ms: Option<i64>,
}

#[derive(sea_orm::FromQueryResult)]
pub struct ClientPlaylist {
    pub id: String,
    pub name: String,
    pub comment: Option<String>,
    pub created_at: Option<i64>,
    pub changed_at: Option<i64>,
    pub count: String,
    pub duration_ms: Option<String>,
}
impl Playlists<'_> {
    pub async fn client_headers(
        &self,
        user: UserId,
        id: Option<PlaylistId>,
    ) -> Result<Vec<ClientPlaylist>, DbError> {
        let mut query = lists::Entity::find()
            .filter(C::UserId.eq(user.to_string()))
            .filter(C::Deleted.eq(false))
            .join_rev(
                JoinType::LeftJoin,
                entries::Entity::belongs_to(lists::Entity)
                    .from(entries::Column::PlaylistId)
                    .to(C::Id)
                    .into(),
            )
            .join(
                JoinType::LeftJoin,
                entries::Relation::Item.def().on_condition(|_, right| {
                    Expr::col((right.clone(), items::Column::DeletedAt))
                        .is_null()
                        .and(Expr::col((right, items::Column::Kind)).eq("track"))
                        .into_condition()
                }),
            )
            .join_rev(JoinType::LeftJoin, tags::Relation::Item.def())
            .select_only()
            .columns([C::Id, C::Name, C::Comment, C::CreatedAt, C::ChangedAt])
            .column_as(items::Column::Id.count().cast_as("CHAR(24)"), "count")
            .column_as(
                tags::Column::DurationMs.sum().cast_as("CHAR(24)"),
                "duration_ms",
            )
            .group_by(C::Id)
            .group_by(C::Name)
            .group_by(C::Comment)
            .group_by(C::CreatedAt)
            .group_by(C::ChangedAt)
            .order_by_asc(C::Name)
            .order_by_asc(C::Id);
        if let Some(id) = id {
            query = query.filter(C::Id.eq(id.to_string()));
        }
        Ok(query.into_model().all(self.db.main()).await?)
    }

    pub async fn list(
        &self,
        user: UserId,
        before: Option<i64>,
        limit: u32,
    ) -> Result<PlaylistPage, DbError> {
        let mut query = lists::Entity::find()
            .filter(C::UserId.eq(user.to_string()))
            .filter(C::Deleted.eq(false));
        if let Some(cursor) = before {
            query = query.filter(C::Version.lt(cursor));
        }
        let limit = limit.clamp(1, 200);
        let mut rows = query
            .order_by_desc(C::Version)
            .limit(u64::from(limit) + 1)
            .all(self.db.main())
            .await?;
        let more = rows.len() > limit as usize;
        rows.truncate(limit as usize);
        let next_before = rows.last().filter(|_| more).map(|r| r.version);
        let playlists = rows.into_iter().map(summary).collect::<Result<_, _>>()?;
        Ok(PlaylistPage {
            playlists,
            next_before,
        })
    }

    pub async fn get(&self, user: UserId, id: PlaylistId) -> Result<Playlist, DbError> {
        // One statement keeps the version and order from the same snapshot, including
        // when a rescan has removed all entries or another device is reordering them.
        let rows = lists::Entity::find_by_id(id.to_string())
            .filter(C::UserId.eq(user.to_string()))
            .filter(C::Deleted.eq(false))
            .join_rev(
                JoinType::LeftJoin,
                entries::Entity::belongs_to(lists::Entity)
                    .from(entries::Column::PlaylistId)
                    .to(C::Id)
                    .into(),
            )
            .join(
                JoinType::LeftJoin,
                entries::Relation::Item.def().on_condition(|_, right| {
                    Expr::col((right.clone(), items::Column::DeletedAt))
                        .is_null()
                        .and(Expr::col((right, items::Column::Kind)).eq("track"))
                        .into_condition()
                }),
            )
            .select_only()
            .columns([C::Name, C::Version])
            .column_as(Expr::col((items::Entity, items::Column::Id)), "live_track")
            .order_by_asc(entries::Column::Position)
            .into_tuple::<(String, i64, Option<String>)>()
            .all(self.db.main())
            .await?;
        let first = rows.first().ok_or(DbError::NotFound("Playlist"))?;
        let summary = PlaylistSummary {
            id,
            name: first.0.clone(),
            version: first.1,
        };
        let track_ids = rows
            .into_iter()
            .filter_map(|(_, _, item)| item)
            .map(|item| parse_id(&item, "track ID"))
            .collect::<Result<_, _>>()?;
        Ok(Playlist { summary, track_ids })
    }

    pub async fn tracks(
        &self,
        user: UserId,
        id: PlaylistId,
        version: i64,
        after: Option<i32>,
        limit: u32,
    ) -> Result<PlaylistTrackPage, DbError> {
        use entries::Column as E;
        let header = lists::Entity::find_by_id(id.to_string())
            .filter(C::UserId.eq(user.to_string()))
            .filter(C::Deleted.eq(false));
        let mut query = header
            .clone()
            .filter(C::Version.eq(version))
            .join_rev(
                JoinType::InnerJoin,
                entries::Entity::belongs_to(lists::Entity)
                    .from(E::PlaylistId)
                    .to(C::Id)
                    .into(),
            )
            .join(JoinType::InnerJoin, entries::Relation::Item.def())
            .join_rev(JoinType::LeftJoin, tags::Relation::Item.def())
            .filter(items::Column::DeletedAt.is_null())
            .filter(items::Column::Kind.eq("track"));
        if let Some(pos) = after {
            query = query.filter(E::Position.gt(pos));
        }
        let limit = limit.clamp(1, 200);
        let mut rows = query
            .select_only()
            .columns([E::Position, E::ItemId])
            .column(items::Column::LibraryId)
            .column_as(crate::music::display_title(), "title")
            .columns([
                tags::Column::Artist,
                tags::Column::Album,
                tags::Column::DurationMs,
            ])
            .order_by_asc(E::Position)
            .limit(u64::from(limit) + 1)
            .into_model::<TrackRow>()
            .all(self.db.main())
            .await?;
        // A nonempty page already proves ownership and version in the same statement.
        if rows.is_empty() {
            let current = header
                .select_only()
                .column(C::Version)
                .into_tuple::<i64>()
                .one(self.db.main())
                .await?
                .ok_or(DbError::NotFound("Playlist"))?;
            if current != version {
                return Err(version_conflict());
            }
        }
        let more = rows.len() > limit as usize;
        rows.truncate(limit as usize);
        let next_after = rows.last().filter(|_| more).map(|r| r.position);
        let tracks = rows
            .into_iter()
            .map(|r| {
                Ok(PlaylistTrack {
                    position: r.position,
                    item_id: parse_id(&r.item_id, "item ID")?,
                    library_id: parse_id(&r.library_id, "library ID")?,
                    title: r.title,
                    artist: r.artist,
                    album: r.album,
                    duration_ms: r.duration_ms,
                })
            })
            .collect::<Result<Vec<_>, DbError>>()?;
        Ok(PlaylistTrackPage { tracks, next_after })
    }

    pub async fn save(
        &self,
        user: UserId,
        id: PlaylistId,
        expected: i64,
        name: &str,
        tracks: Vec<ItemId>,
    ) -> Result<Playlist, DbError> {
        self.save_with_comment(user, id, expected, name, tracks, None)
            .await
    }

    pub async fn save_with_comment(
        &self,
        user: UserId,
        id: PlaylistId,
        expected: i64,
        name: &str,
        tracks: Vec<ItemId>,
        comment: Option<&str>,
    ) -> Result<Playlist, DbError> {
        if comment.is_some_and(|text| text.chars().count() > 4096) {
            return Err(DbError::Invalid("Playlist description too long"));
        }
        let now = crate::convert::now_ms();
        let name = name.trim();
        if name.is_empty() || name.chars().count() > 128 || name.chars().any(char::is_control) {
            return Err(DbError::Invalid(
                "Playlist name must contain 1–128 printable characters",
            ));
        }
        if tracks.len() > MAX_ENTRIES || expected < 0 {
            return Err(DbError::Invalid(
                "Invalid playlist version or too many tracks",
            ));
        }
        let ids: Vec<_> = tracks.iter().map(ToString::to_string).collect();
        let unique: BTreeSet<_> = ids.iter().cloned().collect();
        let txn = self.db.main().begin().await?;
        let version = next_revision(&txn).await?;
        let owner = user.to_string();
        let existing = lists::Entity::find_by_id(id.to_string()).one(&txn).await?;
        match &existing {
            Some(row) if row.user_id != owner || row.deleted => {
                return Err(DbError::NotFound("Playlist"));
            }
            Some(row) if row.version != expected => return Err(version_conflict()),
            None if expected != 0 => return Err(DbError::NotFound("Playlist")),
            _ => (),
        }
        let distinct: Vec<_> = unique.into_iter().collect();
        for batch in distinct.chunks(TRACK_BATCH) {
            let found = items::Entity::find()
                .filter(items::Column::Id.is_in(batch.iter().cloned()))
                .filter(items::Column::Kind.eq("track"))
                .filter(items::Column::DeletedAt.is_null())
                .count(&txn)
                .await?;
            if found != batch.len() as u64 {
                return Err(DbError::NotFound("Track"));
            }
        }
        match existing {
            Some(row) => {
                let mut active: lists::ActiveModel = row.into();
                active.name = Set(name.to_owned());
                if let Some(comment) = comment {
                    active.comment = Set(Some(comment.to_owned()));
                }
                active.changed_at = Set(Some(now));
                active.version = Set(version);
                active.update(&txn).await?;
                remove_entries(&txn, id).await?;
            }
            None => {
                lists::Entity::insert(lists::ActiveModel {
                    id: Set(id.to_string()),
                    user_id: Set(owner),
                    name: Set(name.to_owned()),
                    version: Set(version),
                    deleted: Set(false),
                    comment: Set(comment.map(str::to_owned)),
                    created_at: Set(Some(now)),
                    changed_at: Set(Some(now)),
                })
                .exec_without_returning(&txn)
                .await?;
            }
        }
        let playlist = id.to_string();
        let mut position = 0i32;
        for batch in ids.chunks(TRACK_BATCH) {
            let entries = batch.iter().map(|item| {
                let entry = entries::ActiveModel {
                    playlist_id: Set(playlist.clone()),
                    position: Set(position),
                    item_id: Set(item.clone()),
                };
                position += 1;
                entry
            });
            entries::Entity::insert_many(entries)
                .exec_without_returning(&txn)
                .await?;
        }
        txn.commit().await?;
        Ok(Playlist {
            summary: PlaylistSummary {
                id,
                name: name.to_owned(),
                version,
            },
            track_ids: tracks,
        })
    }

    pub async fn delete(&self, user: UserId, id: PlaylistId, expected: i64) -> Result<(), DbError> {
        let txn = self.db.main().begin().await?;
        let version = next_revision(&txn).await?;
        let row = lists::Entity::find_by_id(id.to_string())
            .filter(C::UserId.eq(user.to_string()))
            .filter(C::Deleted.eq(false))
            .one(&txn)
            .await?
            .ok_or(DbError::NotFound("Playlist"))?;
        if row.version != expected {
            return Err(version_conflict());
        }
        let mut active: lists::ActiveModel = row.into();
        active.deleted = Set(true);
        active.version = Set(version);
        active.update(&txn).await?;
        remove_entries(&txn, id).await?;
        txn.commit().await?;
        Ok(())
    }
}

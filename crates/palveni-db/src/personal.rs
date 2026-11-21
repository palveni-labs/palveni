//! Per-user media state. Playback uses its own revision so rating a song
//! from another device cannot invalidate an in-flight progress report.
use palveni_core::{
    ItemId, UserId,
    personal::{MediaState, MediaView, PersonalItem, PersonalItemPage, PlaybackUpdate},
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, JoinType, QueryFilter, QueryOrder,
    QuerySelect, RelationTrait, TransactionTrait,
};

use crate::{
    Db, DbError,
    convert::{now_ms, opt_ts, parse_id},
    entities::{
        items, track_metadata,
        user_item_state::{self as state, Column as C},
    },
    revision::next_revision,
};

pub struct PersonalMedia<'a> {
    pub(crate) db: &'a Db,
}

#[derive(Clone, Copy)]
pub enum MediaChange {
    Favorite(bool),
    Rating(i32),
    Progress(PlaybackUpdate),
}

async fn load(
    conn: &impl ConnectionTrait,
    user: UserId,
    id: ItemId,
) -> Result<(state::Model, bool), DbError> {
    let live = items::Entity::find_by_id(id.to_string())
        .filter(items::Column::DeletedAt.is_null())
        .filter(items::Column::Kind.is_in(["track", "video"]))
        .select_only()
        .column(items::Column::Id)
        .into_tuple::<String>()
        .one(conn)
        .await?
        .is_some();
    if !live {
        return Err(DbError::NotFound("Media item"));
    }
    let key = (user.to_string(), id.to_string());
    if let Some(row) = state::Entity::find_by_id(key.clone()).one(conn).await? {
        return Ok((row, true));
    }
    Ok((
        state::Model {
            user_id: key.0,
            item_id: key.1,
            favorite_order: None,
            stars: 0,
            position_ms: 0,
            completed: false,
            play_count: 0,
            last_played_at: None,
            playback_version: 0,
            previous_version: None,
        },
        false,
    ))
}

#[derive(sea_orm::FromQueryResult)]
struct ListedItem {
    #[sea_orm(nested)]
    state: state::Model,
    library_id: String,
    kind: String,
    title: String,
}

fn media_state(row: &state::Model) -> Result<MediaState, DbError> {
    Ok(MediaState {
        favorite: row.favorite_order.is_some(),
        stars: row.stars,
        position_ms: if row.completed { 0 } else { row.position_ms },
        completed: row.completed,
        play_count: row.play_count,
        last_played_at: opt_ts(row.last_played_at)?,
        playback_version: row.playback_version,
    })
}

impl PersonalMedia<'_> {
    pub async fn get(&self, user: UserId, id: ItemId) -> Result<MediaState, DbError> {
        media_state(&load(self.db.main(), user, id).await?.0)
    }

    pub async fn update(
        &self,
        user: UserId,
        id: ItemId,
        change: MediaChange,
    ) -> Result<MediaState, DbError> {
        let txn = self.db.main().begin().await?;
        let revision = next_revision(&txn).await?;
        let (mut row, exists) = load(&txn, user, id).await?;
        match change {
            MediaChange::Favorite(value) => {
                if row.favorite_order.is_some() == value {
                    return media_state(&row);
                }
                row.favorite_order = value.then_some(revision);
            }
            MediaChange::Rating(value) => {
                if !(0..=5).contains(&value) {
                    return Err(DbError::Invalid("stars must be between 0 and 5"));
                }
                if row.stars == value {
                    return media_state(&row);
                }
                row.stars = value;
            }
            MediaChange::Progress(p) => {
                if p.expected_version < 0 || p.position_ms < 0 {
                    return Err(DbError::Invalid(
                        "Playback version and position must be nonnegative",
                    ));
                }
                if p.expected_version != row.playback_version {
                    if row.previous_version == Some(p.expected_version)
                        && p.position_ms == row.position_ms
                        && p.completed == row.completed
                    {
                        return media_state(&row);
                    }
                    return Err(DbError::Conflict("Playback progress has changed".into()));
                }
                row.previous_version = Some(row.playback_version);
                row.playback_version = revision;
                row.last_played_at = Some(now_ms());
                row.position_ms = p.position_ms;
                row.completed = p.completed;
                if p.completed {
                    row.play_count += 1;
                }
            }
        }
        let result = media_state(&row)?;
        let active: state::ActiveModel = row.into();
        if exists {
            active.reset_all().update(&txn).await?;
        } else {
            state::Entity::insert(active)
                .exec_without_returning(&txn)
                .await?;
        }
        txn.commit().await?;
        Ok(result)
    }

    pub async fn list(
        &self,
        user: UserId,
        view: MediaView,
        before: Option<i64>,
        limit: u32,
    ) -> Result<PersonalItemPage, DbError> {
        let order = match view {
            MediaView::Favorites => C::FavoriteOrder,
            _ => C::PlaybackVersion,
        };
        let mut query = state::Entity::find()
            .join(JoinType::InnerJoin, state::Relation::Item.def())
            .filter(C::UserId.eq(user.to_string()))
            .filter(order.gt(0))
            .filter(items::Column::DeletedAt.is_null())
            .filter(items::Column::Kind.is_in(["track", "video"]));
        if matches!(view, MediaView::Continue) {
            query = query
                .filter(C::Completed.eq(false))
                .filter(C::PositionMs.gt(0));
        }
        if let Some(cursor) = before {
            query = query.filter(order.lt(cursor));
        }
        let limit = limit.clamp(1, 200);
        let mut rows = query
            .order_by_desc(order)
            .limit(u64::from(limit) + 1)
            .join_rev(JoinType::LeftJoin, track_metadata::Relation::Item.def())
            .column_as(items::Column::LibraryId, "library_id")
            .column_as(items::Column::Kind, "kind")
            .column_as(crate::music::display_title(), "title")
            .into_model::<ListedItem>()
            .all(self.db.main())
            .await?;
        let more = rows.len() > limit as usize;
        rows.truncate(limit as usize);
        let mut next_before = None;
        let mut items = Vec::with_capacity(rows.len());
        for item in rows {
            let state = item.state;
            next_before = match view {
                MediaView::Favorites => state.favorite_order,
                _ => Some(state.playback_version),
            };
            items.push(PersonalItem {
                item_id: parse_id(&state.item_id, "item ID")?,
                library_id: parse_id(&item.library_id, "library ID")?,
                title: item.title,
                kind: parse_id(&item.kind, "item kind")?,
                state: media_state(&state)?,
            });
        }
        Ok(PersonalItemPage {
            items,
            next_before: next_before.filter(|_| more),
        })
    }
}

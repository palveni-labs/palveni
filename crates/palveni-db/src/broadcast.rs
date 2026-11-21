use std::collections::HashSet;

use palveni_core::{UserId, broadcast::*};
use sea_orm::{
    ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect, Set,
    TransactionTrait, sea_query::Query,
};
use sha2::{Digest, Sha256};

use crate::{
    Db, DbError,
    entities::{
        broadcast_entries as entries, broadcast_favorites as favorites,
        broadcast_sources as sources,
    },
    revision::next_revision,
};

fn summary(row: sources::Model) -> Result<BroadcastSource, DbError> {
    let kind = match row.kind.as_str() {
        "radio" => BroadcastKind::Radio,
        "iptv" => BroadcastKind::Iptv,
        "podcast" => BroadcastKind::Podcast,
        _ => return Err(DbError::Corrupt("Unknown broadcast source kind".into())),
    };
    Ok(BroadcastSource {
        id: row.id,
        kind,
        name: row.name,
        version: row.version,
        entry_count: row.entry_count,
        feed_url: row.feed_url,
        guide_url: row.guide_url,
    })
}

fn changed() -> DbError {
    DbError::Conflict("Broadcast source has changed".into())
}

async fn get_source(conn: &impl ConnectionTrait, id: &str) -> Result<sources::Model, DbError> {
    sources::Entity::find_by_id(id)
        .one(conn)
        .await?
        .ok_or(DbError::NotFound("Broadcast source"))
}

impl Db {
    pub async fn broadcast_sources(
        &self,
        kind: Option<BroadcastKind>,
    ) -> Result<Vec<BroadcastSource>, DbError> {
        let mut query = sources::Entity::find();
        if let Some(kind) = kind {
            query = query.filter(sources::Column::Kind.eq(kind.to_string()));
        }
        query
            .order_by_asc(sources::Column::Name)
            .order_by_asc(sources::Column::Id)
            .all(self.main())
            .await?
            .into_iter()
            .map(summary)
            .collect()
    }

    pub async fn save_broadcast(
        &self,
        id: &str,
        incoming: &BroadcastImport,
        rows: Vec<ImportedEntry>,
    ) -> Result<BroadcastSource, DbError> {
        if rows.is_empty() || rows.len() > MAX_BROADCAST_ENTRIES {
            return Err(DbError::Invalid(
                "The source must contain 1–5000 playable entries",
            ));
        }
        let count = i32::try_from(rows.len()).map_err(|_| DbError::Invalid("Too many entries"))?;
        // Complete serialisation and size checks before taking the writer transaction.
        let mut prepared = Vec::with_capacity(rows.len());
        for (index, row) in rows.into_iter().enumerate() {
            let entry_id = hex::encode(Sha256::digest(row.key.as_bytes()));
            let payload =
                serde_json::to_string(&row).map_err(|e| DbError::Internal(e.to_string()))?;
            if payload.len() > 60_000 {
                return Err(DbError::Invalid(
                    "An entry or programme list exceeds the size limit",
                ));
            }
            prepared.push(entries::ActiveModel {
                source_id: Set(id.to_owned()),
                entry_id: Set(entry_id),
                position: Set(
                    i32::try_from(index).map_err(|_| DbError::Invalid("Too many entries"))?
                ),
                payload: Set(payload),
            });
        }
        let txn = self.main().begin().await?;
        let version = next_revision(&txn).await?;
        let previous = sources::Entity::find_by_id(id).one(&txn).await?;
        if previous.as_ref().map_or(0, |s| s.version) != incoming.expected_version {
            return Err(changed());
        }
        if previous
            .as_ref()
            .is_some_and(|s| s.kind != incoming.kind.to_string())
        {
            return Err(DbError::Invalid("Existing source kind cannot be changed"));
        }
        let row = sources::ActiveModel {
            id: Set(id.to_owned()),
            kind: Set(incoming.kind.to_string()),
            name: Set(incoming.name.clone()),
            version: Set(version),
            entry_count: Set(count),
            feed_url: Set(incoming.feed_url.clone()),
            guide_url: Set(incoming.guide_url.clone()),
        };
        if previous.is_some() {
            sources::Entity::update(row).exec(&txn).await?;
            entries::Entity::delete_many()
                .filter(entries::Column::SourceId.eq(id))
                .exec(&txn)
                .await?;
        } else {
            sources::Entity::insert(row)
                .exec_without_returning(&txn)
                .await?;
        }
        let mut iter = prepared.into_iter();
        loop {
            // Under SQLite's native bind-variable cap and MySQL's per-packet limit.
            let chunk: Vec<_> = iter.by_ref().take(32).collect();
            if chunk.is_empty() {
                break;
            }
            entries::Entity::insert_many(chunk)
                .exec_without_returning(&txn)
                .await?;
        }
        if previous.is_some() {
            let retained = Query::select()
                .column(entries::Column::EntryId)
                .from(entries::Entity)
                .and_where(entries::Column::SourceId.eq(id))
                .to_owned();
            favorites::Entity::delete_many()
                .filter(favorites::Column::SourceId.eq(id))
                .filter(favorites::Column::EntryId.not_in_subquery(retained))
                .exec(&txn)
                .await?;
        }
        let source = summary(get_source(&txn, id).await?)?;
        txn.commit().await?;
        Ok(source)
    }

    pub async fn delete_broadcast(&self, id: &str, expected: i64) -> Result<(), DbError> {
        let txn = self.main().begin().await?;
        next_revision(&txn).await?;
        if get_source(&txn, id).await?.version != expected {
            return Err(changed());
        }
        sources::Entity::delete_by_id(id).exec(&txn).await?;
        txn.commit().await?;
        Ok(())
    }

    pub async fn broadcast_entries(
        &self,
        user: UserId,
        id: &str,
        expected: i64,
        offset: u32,
        limit: u32,
    ) -> Result<BroadcastEntryPage, DbError> {
        let txn = self.main().begin().await?;
        if get_source(&txn, id).await?.version != expected {
            return Err(changed());
        }
        let limit = limit.clamp(1, 200);
        let mut rows = entries::Entity::find()
            .filter(entries::Column::SourceId.eq(id))
            .filter(entries::Column::Position.gte(i64::from(offset)))
            .order_by_asc(entries::Column::Position)
            .limit(u64::from(limit) + 1)
            .all(&txn)
            .await?;
        let next_offset = rows
            .get(limit as usize)
            .and_then(|r| u32::try_from(r.position).ok());
        rows.truncate(limit as usize);
        let keys = rows.iter().map(|r| r.entry_id.clone()).collect::<Vec<_>>();
        let fav: HashSet<String> = if keys.is_empty() {
            HashSet::new()
        } else {
            favorites::Entity::find()
                .select_only()
                .column(favorites::Column::EntryId)
                .filter(favorites::Column::UserId.eq(user.to_string()))
                .filter(favorites::Column::SourceId.eq(id))
                .filter(favorites::Column::EntryId.is_in(keys))
                .into_tuple()
                .all(&txn)
                .await?
                .into_iter()
                .collect()
        };
        // PostgreSQL's default read-committed snapshot can advance between statements.
        if get_source(&txn, id).await?.version != expected {
            return Err(changed());
        }
        txn.commit().await?;
        let now = palveni_core::clock::now();
        let entries = rows
            .into_iter()
            .map(|row| {
                let entry: ImportedEntry = serde_json::from_str(&row.payload)
                    .map_err(|_| DbError::Corrupt("Invalid stored broadcast entry".into()))?;
                let current = entry
                    .programmes
                    .iter()
                    .find(|p| p.start <= now && p.end > now)
                    .cloned();
                let next = entry.programmes.iter().find(|p| p.start > now).cloned();
                Ok(BroadcastEntry {
                    favorite: fav.contains(&row.entry_id),
                    id: row.entry_id,
                    title: entry.title,
                    group: entry.group,
                    streams: entry.streams,
                    published_at: entry.published_at,
                    current,
                    next,
                })
            })
            .collect::<Result<_, DbError>>()?;
        Ok(BroadcastEntryPage {
            entries,
            next_offset,
        })
    }

    pub async fn broadcast_favorite(
        &self,
        user: UserId,
        id: &str,
        entry: &str,
        favorite: bool,
    ) -> Result<(), DbError> {
        let txn = self.main().begin().await?;
        next_revision(&txn).await?;
        if entries::Entity::find_by_id((id.to_owned(), entry.to_owned()))
            .one(&txn)
            .await?
            .is_none()
        {
            return Err(DbError::NotFound("Broadcast entry"));
        }
        let key = (user.to_string(), id.to_owned(), entry.to_owned());
        if favorite {
            if favorites::Entity::find_by_id(key.clone())
                .one(&txn)
                .await?
                .is_none()
            {
                favorites::Entity::insert(favorites::ActiveModel {
                    user_id: Set(key.0),
                    source_id: Set(key.1),
                    entry_id: Set(key.2),
                })
                .exec_without_returning(&txn)
                .await?;
            }
        } else {
            favorites::Entity::delete_by_id(key).exec(&txn).await?;
        }
        txn.commit().await?;
        Ok(())
    }
}

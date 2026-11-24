//! Persistence for external downloader clients, RSS/Atom media subscriptions,
//! and reversible media organize batches (`m20261123_000028_mcp_downloads_organizer`).

use std::collections::HashMap;

use palveni_core::downloads::{
    CreateSubscriptionRequest, DownloaderKind, MediaSubscription, OrganizeBatchSummary,
    OrganizeMode, OrganizeOperationRecord, SubscriptionMediaKind,
};
use palveni_core::ids::{ConnectionId, ItemId, JobId, LibraryId, UserId};
use sea_orm::sea_query::Expr;
use sea_orm::{
    ActiveValue, ColumnTrait, Condition, EntityTrait, QueryFilter, QueryOrder, QuerySelect,
    TransactionTrait,
};

use crate::Db;
use crate::convert::{now_ms, opt_ts, parse_id, sha256_hex, ts};
use crate::entities::{
    downloader_clients, items, libraries, media_subscriptions, organize_batches, reading_metadata,
    track_metadata, video_metadata,
};
use crate::error::DbError;
use crate::items::SORT_GROUP_LEAF;
use crate::revision;

pub struct Downloads<'a> {
    pub(crate) db: &'a Db,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DownloaderClientRecord {
    pub id: String,
    pub name: String,
    pub kind: DownloaderKind,
    pub base_url: String,
    pub credential_cipher: Option<String>,
    pub save_path: Option<String>,
    pub enabled: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Clone, Debug)]
pub struct SaveDownloaderClient<'a> {
    pub id: Option<&'a str>,
    pub name: &'a str,
    pub kind: DownloaderKind,
    pub base_url: &'a str,
    pub credential_cipher: Option<String>,
    pub keep_existing_credential: bool,
    pub save_path: Option<String>,
    pub enabled: bool,
}

#[derive(Clone, Debug)]
pub struct OrganizeItemMeta {
    pub item_id: ItemId,
    pub path: String,
    pub size_bytes: Option<i64>,
    pub title: String,
    pub year: Option<i32>,
    pub season: Option<i32>,
    pub episode: Option<i32>,
    pub artist: Option<String>,
    pub author: Option<String>,
}

impl Downloads<'_> {
    // -----------------------------------------------------------------------
    // External Downloader Clients
    // -----------------------------------------------------------------------

    pub async fn list_clients(&self) -> Result<Vec<DownloaderClientRecord>, DbError> {
        let rows = downloader_clients::Entity::find()
            .order_by_asc(downloader_clients::Column::CreatedAt)
            .order_by_asc(downloader_clients::Column::Id)
            .all(self.db.main())
            .await?;
        rows.into_iter().map(to_client_record).collect()
    }

    pub async fn get_client(&self, id: &str) -> Result<Option<DownloaderClientRecord>, DbError> {
        downloader_clients::Entity::find_by_id(id.to_owned())
            .one(self.db.main())
            .await?
            .map(to_client_record)
            .transpose()
    }

    pub async fn save_client(
        &self,
        input: SaveDownloaderClient<'_>,
    ) -> Result<DownloaderClientRecord, DbError> {
        let SaveDownloaderClient {
            id,
            name,
            kind,
            base_url,
            credential_cipher,
            keep_existing_credential,
            save_path,
            enabled,
        } = input;
        let now = now_ms();
        if let Some(existing_id) = id
            && let Some(existing) = downloader_clients::Entity::find_by_id(existing_id.to_owned())
                .one(self.db.main())
                .await?
        {
            let next_cipher = if credential_cipher.is_some() {
                credential_cipher
            } else if keep_existing_credential {
                existing.credential_cipher.clone()
            } else {
                None
            };
            let active = downloader_clients::ActiveModel {
                id: ActiveValue::Unchanged(existing.id),
                name: ActiveValue::Set(name.to_owned()),
                kind: ActiveValue::Set(kind.as_str().to_owned()),
                base_url: ActiveValue::Set(base_url.to_owned()),
                credential_cipher: ActiveValue::Set(next_cipher),
                save_path: ActiveValue::Set(save_path),
                enabled: ActiveValue::Set(i32::from(enabled)),
                created_at: ActiveValue::Unchanged(existing.created_at),
            };
            downloader_clients::Entity::update(active)
                .exec(self.db.main())
                .await?;
            return self
                .get_client(existing_id)
                .await?
                .ok_or(DbError::NotFound("downloader_client"));
        }

        let new_id = id
            .filter(|s| !s.trim().is_empty())
            .map_or_else(|| ConnectionId::generate().to_string(), str::to_owned);
        let active = downloader_clients::ActiveModel {
            id: ActiveValue::Set(new_id.clone()),
            name: ActiveValue::Set(name.to_owned()),
            kind: ActiveValue::Set(kind.as_str().to_owned()),
            base_url: ActiveValue::Set(base_url.to_owned()),
            credential_cipher: ActiveValue::Set(credential_cipher),
            save_path: ActiveValue::Set(save_path),
            enabled: ActiveValue::Set(i32::from(enabled)),
            created_at: ActiveValue::Set(now),
        };
        downloader_clients::Entity::insert(active)
            .exec_without_returning(self.db.main())
            .await?;
        self.get_client(&new_id)
            .await?
            .ok_or(DbError::NotFound("downloader_client"))
    }

    pub async fn delete_client(&self, id: &str) -> Result<bool, DbError> {
        let res = downloader_clients::Entity::delete_by_id(id.to_owned())
            .exec(self.db.main())
            .await?;
        Ok(res.rows_affected > 0)
    }

    // -----------------------------------------------------------------------
    // RSS / Atom Media Subscriptions
    // -----------------------------------------------------------------------

    pub async fn list_subscriptions(
        &self,
        owner_id: Option<UserId>,
    ) -> Result<Vec<MediaSubscription>, DbError> {
        let mut query = media_subscriptions::Entity::find();
        if let Some(uid) = owner_id {
            query = query.filter(media_subscriptions::Column::OwnerId.eq(uid.to_string()));
        }
        let rows = query
            .order_by_desc(media_subscriptions::Column::CreatedAt)
            .order_by_desc(media_subscriptions::Column::Id)
            .all(self.db.main())
            .await?;
        rows.into_iter().map(to_subscription).collect()
    }

    pub async fn get_subscription(&self, id: &str) -> Result<Option<MediaSubscription>, DbError> {
        media_subscriptions::Entity::find_by_id(id.to_owned())
            .one(self.db.main())
            .await?
            .map(to_subscription)
            .transpose()
    }

    pub async fn create_subscription(
        &self,
        owner_id: UserId,
        req: &CreateSubscriptionRequest,
    ) -> Result<MediaSubscription, DbError> {
        let id = JobId::generate().to_string();
        let now = now_ms();
        let active = media_subscriptions::ActiveModel {
            id: ActiveValue::Set(id.clone()),
            owner_id: ActiveValue::Set(owner_id.to_string()),
            title: ActiveValue::Set(req.title.trim().to_owned()),
            media_kind: ActiveValue::Set(req.media_kind.as_str().to_owned()),
            quality_rule: ActiveValue::Set(
                req.quality_rule
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_owned),
            ),
            feed_url: ActiveValue::Set(
                req.feed_url
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_owned),
            ),
            target_library_id: ActiveValue::Set(
                req.target_library_id
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_owned),
            ),
            enabled: ActiveValue::Set(i32::from(req.enabled)),
            last_checked_at: ActiveValue::Set(None),
            created_at: ActiveValue::Set(now),
        };
        media_subscriptions::Entity::insert(active)
            .exec_without_returning(self.db.main())
            .await?;
        self.get_subscription(&id)
            .await?
            .ok_or(DbError::NotFound("media_subscription"))
    }

    pub async fn mark_subscription_checked(
        &self,
        id: &str,
        checked_at_ms: i64,
    ) -> Result<Option<MediaSubscription>, DbError> {
        let res = media_subscriptions::Entity::update_many()
            .col_expr(
                media_subscriptions::Column::LastCheckedAt,
                Expr::val(Some(checked_at_ms)),
            )
            .filter(media_subscriptions::Column::Id.eq(id.to_owned()))
            .exec(self.db.main())
            .await?;
        if res.rows_affected == 0 {
            return Ok(None);
        }
        self.get_subscription(id).await
    }

    pub async fn delete_subscription(
        &self,
        id: &str,
        caller_id: UserId,
        is_admin: bool,
    ) -> Result<bool, DbError> {
        let mut cond = Condition::all().add(media_subscriptions::Column::Id.eq(id.to_owned()));
        if !is_admin {
            cond = cond.add(media_subscriptions::Column::OwnerId.eq(caller_id.to_string()));
        }
        let res = media_subscriptions::Entity::delete_many()
            .filter(cond)
            .exec(self.db.main())
            .await?;
        Ok(res.rows_affected > 0)
    }

    // -----------------------------------------------------------------------
    // Media Organizer Inspection, Atomic Apply & 1-Click Undo
    // -----------------------------------------------------------------------

    pub async fn library_organize_items(
        &self,
        library_id: LibraryId,
        limit: u64,
    ) -> Result<Vec<OrganizeItemMeta>, DbError> {
        let item_rows = items::Entity::find()
            .filter(items::Column::LibraryId.eq(library_id.to_string()))
            .filter(items::Column::SortGroup.eq(SORT_GROUP_LEAF))
            .filter(items::Column::DeletedAt.is_null())
            .filter(items::Column::Path.is_not_null())
            .order_by_asc(items::Column::SortTitle)
            .order_by_asc(items::Column::Id)
            .limit(limit.clamp(1, 500))
            .all(self.db.main())
            .await?;

        if item_rows.is_empty() {
            return Ok(Vec::new());
        }

        let ids: Vec<String> = item_rows.iter().map(|r| r.id.clone()).collect();

        let videos: HashMap<String, video_metadata::Model> = video_metadata::Entity::find()
            .filter(video_metadata::Column::ItemId.is_in(ids.clone()))
            .all(self.db.main())
            .await?
            .into_iter()
            .map(|m| (m.item_id.clone(), m))
            .collect();

        let tracks: HashMap<String, track_metadata::Model> = track_metadata::Entity::find()
            .filter(track_metadata::Column::ItemId.is_in(ids.clone()))
            .all(self.db.main())
            .await?
            .into_iter()
            .map(|m| (m.item_id.clone(), m))
            .collect();

        let readings: HashMap<String, reading_metadata::Model> = reading_metadata::Entity::find()
            .filter(reading_metadata::Column::ItemId.is_in(ids))
            .all(self.db.main())
            .await?
            .into_iter()
            .map(|m| (m.item_id.clone(), m))
            .collect();

        let mut out = Vec::with_capacity(item_rows.len());
        for row in item_rows {
            let Some(path) = row.path else { continue };
            let item_id: ItemId = parse_id(&row.id, "item id")?;

            let mut title = row.title.clone();
            let mut year: Option<i32> = None;
            let mut season: Option<i32> = None;
            let mut episode: Option<i32> = None;
            let mut artist: Option<String> = None;
            let mut author: Option<String> = None;

            if let Some(v) = videos.get(&row.id) {
                if !v.work_title.trim().is_empty() {
                    title = v.work_title.clone();
                } else if !v.title.trim().is_empty() {
                    title = v.title.clone();
                }
                year = v.work_year;
                season = v.season;
                episode = v.episode;
            }
            if let Some(t) = tracks.get(&row.id) {
                if !t.title.trim().is_empty() {
                    title = t.title.clone();
                }
                artist = t
                    .artist
                    .clone()
                    .or_else(|| t.album_artist.clone())
                    .filter(|s| !s.trim().is_empty());
                if year.is_none() {
                    year = t.year.and_then(|y| i32::try_from(y).ok());
                }
            }
            if let Some(r) = readings.get(&row.id) {
                if !r.title.trim().is_empty() {
                    title = r.title.clone();
                }
                if let Ok(authors) = serde_json::from_str::<Vec<String>>(&r.authors_json) {
                    author = authors.into_iter().find(|a| !a.trim().is_empty());
                }
            }

            // Fallback heuristics from filename if metadata tables lacked year / SxxExx.
            let stem = std::path::Path::new(&path)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or(&row.title);
            if year.is_none() {
                year = parse_year_hint(stem).or_else(|| parse_year_hint(&title));
            }
            if (season.is_none() || episode.is_none())
                && let Some((s, e)) = parse_season_episode_hint(stem)
            {
                season = season.or(Some(s));
                episode = episode.or(Some(e));
            }

            out.push(OrganizeItemMeta {
                item_id,
                path,
                size_bytes: row.size_bytes,
                title,
                year,
                season,
                episode,
                artist,
                author,
            });
        }
        Ok(out)
    }

    pub async fn record_organize_batch(
        &self,
        actor_id: UserId,
        library_id: LibraryId,
        mode: OrganizeMode,
        template: &str,
        operations: &[OrganizeOperationRecord],
    ) -> Result<OrganizeBatchSummary, DbError> {
        let now = now_ms();
        let batch_id = JobId::generate().to_string();
        let ops_json = serde_json::to_string(operations)
            .map_err(|e| DbError::Internal(format!("cannot serialize organize operations: {e}")))?;

        let txn = self.db.main().begin().await?;
        let rev = revision::next_revision(&txn).await?;

        let lib_row = libraries::Entity::find_by_id(library_id.to_string())
            .filter(libraries::Column::DeletedAt.is_null())
            .one(&txn)
            .await?
            .ok_or(DbError::NotFound("library"))?;

        for op in operations {
            let new_hash = sha256_hex(&op.to_path);
            // Clear any tombstoned item at the destination hash to prevent unique-index collision.
            items::Entity::delete_many()
                .filter(items::Column::LibraryId.eq(library_id.to_string()))
                .filter(items::Column::PathHash.eq(new_hash.clone()))
                .filter(items::Column::Id.ne(op.item_id.to_string()))
                .filter(items::Column::DeletedAt.is_not_null())
                .exec(&txn)
                .await?;

            items::Entity::update_many()
                .col_expr(items::Column::Path, Expr::val(Some(op.to_path.clone())))
                .col_expr(items::Column::PathHash, Expr::val(Some(new_hash)))
                .col_expr(items::Column::UpdatedAt, Expr::val(now))
                .col_expr(items::Column::Revision, Expr::val(rev))
                .filter(items::Column::Id.eq(op.item_id.to_string()))
                .filter(items::Column::LibraryId.eq(library_id.to_string()))
                .exec(&txn)
                .await?;
        }

        organize_batches::Entity::insert(organize_batches::ActiveModel {
            id: ActiveValue::Set(batch_id.clone()),
            actor_id: ActiveValue::Set(actor_id.to_string()),
            library_id: ActiveValue::Set(library_id.to_string()),
            mode: ActiveValue::Set(mode.as_str().to_owned()),
            template: ActiveValue::Set(template.to_owned()),
            operations_json: ActiveValue::Set(ops_json),
            undone_at: ActiveValue::Set(None),
            created_at: ActiveValue::Set(now),
        })
        .exec_without_returning(&txn)
        .await?;

        txn.commit().await?;

        Ok(OrganizeBatchSummary {
            id: batch_id,
            actor_id: actor_id.to_string(),
            library_id,
            library_name: Some(lib_row.name),
            mode,
            template: template.to_owned(),
            operation_count: operations.len(),
            operations: operations.to_vec(),
            undone_at: None,
            created_at: ts(now)?,
        })
    }

    pub async fn list_organize_batches(
        &self,
        limit: u64,
    ) -> Result<Vec<OrganizeBatchSummary>, DbError> {
        let rows = organize_batches::Entity::find()
            .order_by_desc(organize_batches::Column::CreatedAt)
            .order_by_desc(organize_batches::Column::Id)
            .limit(limit.clamp(1, 200))
            .all(self.db.main())
            .await?;

        let lib_ids: Vec<String> = rows.iter().map(|r| r.library_id.clone()).collect();
        let lib_names: HashMap<String, String> = if lib_ids.is_empty() {
            HashMap::new()
        } else {
            libraries::Entity::find()
                .filter(libraries::Column::Id.is_in(lib_ids))
                .all(self.db.main())
                .await?
                .into_iter()
                .map(|l| (l.id, l.name))
                .collect()
        };

        rows.into_iter()
            .map(|r| {
                let lib_name = lib_names.get(&r.library_id).cloned();
                to_batch_summary(r, lib_name)
            })
            .collect()
    }

    pub async fn get_organize_batch(
        &self,
        id: &str,
    ) -> Result<Option<OrganizeBatchSummary>, DbError> {
        let Some(row) = organize_batches::Entity::find_by_id(id.to_owned())
            .one(self.db.main())
            .await?
        else {
            return Ok(None);
        };
        let lib_name = libraries::Entity::find_by_id(row.library_id.clone())
            .one(self.db.main())
            .await?
            .map(|l| l.name);
        Ok(Some(to_batch_summary(row, lib_name)?))
    }

    pub async fn mark_organize_batch_undone(
        &self,
        batch_id: &str,
    ) -> Result<Option<OrganizeBatchSummary>, DbError> {
        let now = now_ms();
        let txn = self.db.main().begin().await?;
        let Some(row) = organize_batches::Entity::find_by_id(batch_id.to_owned())
            .one(&txn)
            .await?
        else {
            txn.rollback().await?;
            return Ok(None);
        };
        if row.undone_at.is_some() {
            txn.rollback().await?;
            return Err(DbError::Conflict("Organize batch already undone".into()));
        }

        let ops: Vec<OrganizeOperationRecord> = serde_json::from_str(&row.operations_json)
            .map_err(|e| DbError::Corrupt(format!("invalid operations_json: {e}")))?;
        let rev = revision::next_revision(&txn).await?;

        for op in ops.iter().rev() {
            let orig_hash = sha256_hex(&op.from_path);
            items::Entity::delete_many()
                .filter(items::Column::LibraryId.eq(row.library_id.clone()))
                .filter(items::Column::PathHash.eq(orig_hash.clone()))
                .filter(items::Column::Id.ne(op.item_id.to_string()))
                .filter(items::Column::DeletedAt.is_not_null())
                .exec(&txn)
                .await?;

            items::Entity::update_many()
                .col_expr(items::Column::Path, Expr::val(Some(op.from_path.clone())))
                .col_expr(items::Column::PathHash, Expr::val(Some(orig_hash)))
                .col_expr(items::Column::UpdatedAt, Expr::val(now))
                .col_expr(items::Column::Revision, Expr::val(rev))
                .filter(items::Column::Id.eq(op.item_id.to_string()))
                .filter(items::Column::LibraryId.eq(row.library_id.clone()))
                .exec(&txn)
                .await?;
        }

        organize_batches::Entity::update_many()
            .col_expr(organize_batches::Column::UndoneAt, Expr::val(Some(now)))
            .filter(organize_batches::Column::Id.eq(batch_id.to_owned()))
            .exec(&txn)
            .await?;

        let lib_name = libraries::Entity::find_by_id(row.library_id.clone())
            .one(&txn)
            .await?
            .map(|l| l.name);

        txn.commit().await?;

        let mut summary = to_batch_summary(row, lib_name)?;
        summary.undone_at = Some(ts(now)?);
        Ok(Some(summary))
    }
}

fn to_client_record(row: downloader_clients::Model) -> Result<DownloaderClientRecord, DbError> {
    let kind = DownloaderKind::from_db(&row.kind)
        .ok_or_else(|| DbError::Corrupt(format!("unknown downloader kind `{}`", row.kind)))?;
    Ok(DownloaderClientRecord {
        id: row.id,
        name: row.name,
        kind,
        base_url: row.base_url,
        credential_cipher: row.credential_cipher,
        save_path: row.save_path,
        enabled: row.enabled != 0,
        created_at: ts(row.created_at)?,
    })
}

fn to_subscription(row: media_subscriptions::Model) -> Result<MediaSubscription, DbError> {
    let media_kind = SubscriptionMediaKind::from_db(&row.media_kind).ok_or_else(|| {
        DbError::Corrupt(format!("unknown subscription kind `{}`", row.media_kind))
    })?;
    Ok(MediaSubscription {
        id: row.id,
        owner_id: row.owner_id,
        title: row.title,
        media_kind,
        quality_rule: row.quality_rule,
        feed_url: row.feed_url,
        target_library_id: row.target_library_id,
        enabled: row.enabled != 0,
        last_checked_at: opt_ts(row.last_checked_at)?,
        created_at: ts(row.created_at)?,
    })
}

fn to_batch_summary(
    row: organize_batches::Model,
    library_name: Option<String>,
) -> Result<OrganizeBatchSummary, DbError> {
    let mode = OrganizeMode::from_db(&row.mode)
        .ok_or_else(|| DbError::Corrupt(format!("unknown organize mode `{}`", row.mode)))?;
    let operations: Vec<OrganizeOperationRecord> = serde_json::from_str(&row.operations_json)
        .map_err(|e| DbError::Corrupt(format!("invalid operations_json: {e}")))?;
    Ok(OrganizeBatchSummary {
        id: row.id,
        actor_id: row.actor_id,
        library_id: parse_id(&row.library_id, "library id")?,
        library_name,
        mode,
        template: row.template,
        operation_count: operations.len(),
        operations,
        undone_at: opt_ts(row.undone_at)?,
        created_at: ts(row.created_at)?,
    })
}

fn parse_year_hint(text: &str) -> Option<i32> {
    let bytes = text.as_bytes();
    if bytes.len() < 4 {
        return None;
    }
    for idx in 0..=bytes.len() - 4 {
        let slice = &bytes[idx..idx + 4];
        if slice.iter().all(u8::is_ascii_digit) {
            let before_ok = idx == 0 || !bytes[idx - 1].is_ascii_digit();
            let after_ok = idx + 4 == bytes.len() || !bytes[idx + 4].is_ascii_digit();
            if before_ok
                && after_ok
                && let Ok(s) = std::str::from_utf8(slice)
                && let Ok(year) = s.parse::<i32>()
                && (1900..=2099).contains(&year)
            {
                return Some(year);
            }
        }
    }
    None
}

fn parse_season_episode_hint(text: &str) -> Option<(i32, i32)> {
    let upper = text.to_ascii_uppercase();
    let bytes = upper.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'S' {
            let s_start = i + 1;
            let mut s_end = s_start;
            while s_end < bytes.len() && bytes[s_end].is_ascii_digit() {
                s_end += 1;
            }
            if s_end > s_start && s_end < bytes.len() && bytes[s_end] == b'E' {
                let e_start = s_end + 1;
                let mut e_end = e_start;
                while e_end < bytes.len() && bytes[e_end].is_ascii_digit() {
                    e_end += 1;
                }
                if e_end > e_start
                    && let (Ok(s), Ok(e)) = (
                        upper[s_start..s_end].parse::<i32>(),
                        upper[e_start..e_end].parse::<i32>(),
                    )
                {
                    return Some((s, e));
                }
            }
        }
        i += 1;
    }
    None
}

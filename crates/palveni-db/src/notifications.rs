//! Persistence for Notification Channels, Notification Deliveries, Inbound Webhooks,
//! and Scheduled Routines (main store).

use palveni_core::ids::{JobId, LibraryId, UserId};
use palveni_core::notifications::{
    InboundWebhook, InboundWebhookSourceKind, NotificationChannel, NotificationChannelConfig,
    NotificationChannelKind, NotificationDelivery, NotificationDeliveryStatus, NotificationList,
    RoutineActionKind, RoutineScheduleKind, ScheduledRoutine,
};
use sea_orm::{
    ActiveValue::Set, ColumnTrait, Condition, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
    QuerySelect, sea_query::Expr,
};

use crate::Db;
use crate::convert::{now_ms, opt_ts, parse_id, parse_opt_id, sha256_hex, truncate_chars, ts};
use crate::entities::{
    inbound_webhooks, notification_channels, notification_deliveries, scheduled_routines,
};
use crate::error::DbError;

pub struct Notifications<'a> {
    pub(crate) db: &'a Db,
}

#[derive(Clone, Debug)]
pub struct NotificationChannelRecord {
    pub id: String,
    pub owner_id: UserId,
    pub name: String,
    pub kind: NotificationChannelKind,
    pub config_cipher: String,
    pub events: Vec<String>,
    pub quiet_hours: Option<String>,
    pub enabled: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

impl NotificationChannelRecord {
    pub fn to_public(
        &self,
        decrypted_cfg: Option<&NotificationChannelConfig>,
    ) -> Result<NotificationChannel, DbError> {
        Ok(NotificationChannel {
            id: self.id.clone(),
            owner_id: self.owner_id,
            name: self.name.clone(),
            kind: self.kind,
            target_hint: decrypted_cfg.and_then(NotificationChannelConfig::redacted_hint),
            has_secret: decrypted_cfg
                .map_or(!self.config_cipher.is_empty(), |cfg| cfg.has_secret()),
            events: self.events.clone(),
            quiet_hours: self.quiet_hours.clone(),
            enabled: self.enabled,
            created_at: ts(self.created_at)?,
            updated_at: ts(self.updated_at)?,
        })
    }
}

#[derive(Clone, Debug, Default)]
pub struct NotificationChannelPatch {
    pub name: Option<String>,
    pub config_cipher: Option<String>,
    pub events: Option<Vec<String>>,
    pub quiet_hours: Option<Option<String>>,
    pub enabled: Option<bool>,
}

#[derive(Clone, Debug)]
pub struct NewNotificationDelivery {
    pub owner_id: Option<UserId>,
    pub channel_id: Option<String>,
    pub event_type: String,
    pub title: String,
    pub body: String,
    pub status: NotificationDeliveryStatus,
    pub error: Option<String>,
}

#[derive(Clone, Debug)]
pub struct NewRoutine<'a> {
    pub owner_id: UserId,
    pub name: &'a str,
    pub schedule_kind: RoutineScheduleKind,
    pub schedule_expr: &'a str,
    pub action_kind: RoutineActionKind,
    pub action_payload: &'a str,
    pub enabled: bool,
}

#[derive(Clone, Debug)]
pub struct StoredRoutine {
    pub routine: ScheduledRoutine,
    pub last_run_at_ms: Option<i64>,
}

#[derive(Clone, Debug, Default)]
pub struct RoutinePatch {
    pub name: Option<String>,
    pub schedule_kind: Option<RoutineScheduleKind>,
    pub schedule_expr: Option<String>,
    pub action_kind: Option<RoutineActionKind>,
    pub action_payload: Option<String>,
    pub enabled: Option<bool>,
}

fn row_to_channel(row: notification_channels::Model) -> Result<NotificationChannelRecord, DbError> {
    let owner_id: UserId = parse_id(&row.owner_id, "owner_id")?;
    let kind: NotificationChannelKind = row
        .kind
        .parse()
        .map_err(|e| DbError::Corrupt(format!("invalid channel kind: {e}")))?;
    let events: Vec<String> = serde_json::from_str(&row.events_json)
        .map_err(|e| DbError::Corrupt(format!("invalid events_json: {e}")))?;
    Ok(NotificationChannelRecord {
        id: row.id,
        owner_id,
        name: row.name,
        kind,
        config_cipher: row.config_cipher,
        events,
        quiet_hours: row.quiet_hours,
        enabled: row.enabled != 0,
        created_at: row.created_at,
        updated_at: row.updated_at,
    })
}

fn row_to_delivery(row: notification_deliveries::Model) -> Result<NotificationDelivery, DbError> {
    let owner_id: Option<UserId> = parse_opt_id(row.owner_id.as_deref(), "owner_id")?;
    let status: NotificationDeliveryStatus = row
        .status
        .parse()
        .map_err(|e| DbError::Corrupt(format!("invalid delivery status: {e}")))?;
    Ok(NotificationDelivery {
        id: row.id,
        owner_id,
        channel_id: row.channel_id,
        event_type: row.event_type,
        title: row.title,
        body: row.body,
        status,
        error: row.error,
        read_at: opt_ts(row.read_at)?,
        created_at: ts(row.created_at)?,
    })
}

fn row_to_webhook(row: inbound_webhooks::Model) -> Result<InboundWebhook, DbError> {
    let source_kind: InboundWebhookSourceKind = row
        .source_kind
        .parse()
        .map_err(|e| DbError::Corrupt(format!("invalid source_kind: {e}")))?;
    let library_id: Option<LibraryId> = parse_opt_id(row.library_id.as_deref(), "library_id")?;
    Ok(InboundWebhook {
        id: row.id,
        name: row.name,
        source_kind,
        library_id,
        enabled: row.enabled != 0,
        last_triggered_at: opt_ts(row.last_triggered_at)?,
        created_at: ts(row.created_at)?,
    })
}

fn row_to_stored_routine(row: scheduled_routines::Model) -> Result<StoredRoutine, DbError> {
    let owner_id: UserId = parse_id(&row.owner_id, "owner_id")?;
    let schedule_kind: RoutineScheduleKind = row
        .schedule_kind
        .parse()
        .map_err(|e| DbError::Corrupt(format!("invalid schedule_kind: {e}")))?;
    let action_kind: RoutineActionKind = row
        .action_kind
        .parse()
        .map_err(|e| DbError::Corrupt(format!("invalid action_kind: {e}")))?;
    let last_run_at_ms = row.last_run_at;
    Ok(StoredRoutine {
        routine: ScheduledRoutine {
            id: row.id,
            owner_id,
            name: row.name,
            schedule_kind,
            schedule_expr: row.schedule_expr,
            action_kind,
            action_payload: row.action_payload,
            enabled: row.enabled != 0,
            last_run_at: opt_ts(last_run_at_ms)?,
            last_status: row.last_status,
            created_at: ts(row.created_at)?,
        },
        last_run_at_ms,
    })
}

fn visible_deliveries_condition(owner_id: UserId, is_admin: bool) -> Condition {
    if is_admin {
        Condition::any()
            .add(notification_deliveries::Column::OwnerId.eq(owner_id.to_string()))
            .add(notification_deliveries::Column::OwnerId.is_null())
    } else {
        Condition::all().add(notification_deliveries::Column::OwnerId.eq(owner_id.to_string()))
    }
}

impl Notifications<'_> {
    pub async fn list_channels(
        &self,
        owner_id: UserId,
        is_admin: bool,
    ) -> Result<Vec<NotificationChannelRecord>, DbError> {
        let mut query = notification_channels::Entity::find()
            .order_by_asc(notification_channels::Column::CreatedAt)
            .order_by_asc(notification_channels::Column::Id);
        if !is_admin {
            query = query.filter(notification_channels::Column::OwnerId.eq(owner_id.to_string()));
        }
        let rows = query.all(self.db.main()).await?;
        rows.into_iter().map(row_to_channel).collect()
    }

    pub async fn list_enabled_channels(&self) -> Result<Vec<NotificationChannelRecord>, DbError> {
        let rows = notification_channels::Entity::find()
            .filter(notification_channels::Column::Enabled.ne(0))
            .order_by_asc(notification_channels::Column::CreatedAt)
            .all(self.db.main())
            .await?;
        rows.into_iter().map(row_to_channel).collect()
    }

    pub async fn get_channel(
        &self,
        caller_id: UserId,
        is_admin: bool,
        id: &str,
    ) -> Result<Option<NotificationChannelRecord>, DbError> {
        let Some(row) = notification_channels::Entity::find_by_id(id.to_owned())
            .one(self.db.main())
            .await?
        else {
            return Ok(None);
        };
        if !is_admin && row.owner_id != caller_id.to_string() {
            return Ok(None);
        }
        row_to_channel(row).map(Some)
    }

    pub async fn create_channel(
        &self,
        record: NotificationChannelRecord,
    ) -> Result<NotificationChannelRecord, DbError> {
        let events_json = serde_json::to_string(&record.events)
            .map_err(|e| DbError::Internal(format!("cannot encode events: {e}")))?;
        notification_channels::Entity::insert(notification_channels::ActiveModel {
            id: Set(record.id.clone()),
            owner_id: Set(record.owner_id.to_string()),
            name: Set(truncate_chars(&record.name, 128)),
            kind: Set(record.kind.as_str().to_owned()),
            config_cipher: Set(record.config_cipher.clone()),
            events_json: Set(events_json),
            quiet_hours: Set(record.quiet_hours.as_deref().map(|q| truncate_chars(q, 32))),
            enabled: Set(i32::from(record.enabled)),
            created_at: Set(record.created_at),
            updated_at: Set(record.updated_at),
        })
        .exec_without_returning(self.db.main())
        .await?;
        Ok(record)
    }

    pub async fn update_channel(
        &self,
        caller_id: UserId,
        is_admin: bool,
        id: &str,
        patch: NotificationChannelPatch,
    ) -> Result<Option<NotificationChannelRecord>, DbError> {
        let Some(existing) = notification_channels::Entity::find_by_id(id.to_owned())
            .one(self.db.main())
            .await?
        else {
            return Ok(None);
        };
        if !is_admin && existing.owner_id != caller_id.to_string() {
            return Ok(None);
        }
        let updated_at = now_ms();
        let mut active: notification_channels::ActiveModel = existing.into();
        if let Some(name) = patch.name {
            active.name = Set(truncate_chars(&name, 128));
        }
        if let Some(cipher) = patch.config_cipher {
            active.config_cipher = Set(cipher);
        }
        if let Some(events) = patch.events {
            let events_json = serde_json::to_string(&events)
                .map_err(|e| DbError::Internal(format!("cannot encode events: {e}")))?;
            active.events_json = Set(events_json);
        }
        if let Some(quiet_hours) = patch.quiet_hours {
            active.quiet_hours = Set(quiet_hours.map(|q| truncate_chars(&q, 32)));
        }
        if let Some(enabled) = patch.enabled {
            active.enabled = Set(i32::from(enabled));
        }
        active.updated_at = Set(updated_at);
        notification_channels::Entity::update(active)
            .exec(self.db.main())
            .await?;
        self.get_channel(caller_id, is_admin, id).await
    }

    pub async fn delete_channel(
        &self,
        caller_id: UserId,
        is_admin: bool,
        id: &str,
    ) -> Result<bool, DbError> {
        let mut del = notification_channels::Entity::delete_many()
            .filter(notification_channels::Column::Id.eq(id));
        if !is_admin {
            del = del.filter(notification_channels::Column::OwnerId.eq(caller_id.to_string()));
        }
        let res = del.exec(self.db.main()).await?;
        Ok(res.rows_affected > 0)
    }

    pub async fn record_delivery(
        &self,
        entry: NewNotificationDelivery,
    ) -> Result<NotificationDelivery, DbError> {
        let id = JobId::generate().to_string();
        let created_at = now_ms();
        let title = truncate_chars(&entry.title, 255);
        let event_type = truncate_chars(&entry.event_type, 64);
        notification_deliveries::Entity::insert(notification_deliveries::ActiveModel {
            id: Set(id.clone()),
            owner_id: Set(entry.owner_id.map(|u| u.to_string())),
            channel_id: Set(entry.channel_id.clone()),
            event_type: Set(event_type.clone()),
            title: Set(title.clone()),
            body: Set(entry.body.clone()),
            status: Set(entry.status.as_str().to_owned()),
            error: Set(entry.error.clone()),
            read_at: Set(None),
            created_at: Set(created_at),
        })
        .exec_without_returning(self.db.main())
        .await?;
        Ok(NotificationDelivery {
            id,
            owner_id: entry.owner_id,
            channel_id: entry.channel_id,
            event_type,
            title,
            body: entry.body,
            status: entry.status,
            error: entry.error,
            read_at: None,
            created_at: ts(created_at)?,
        })
    }

    pub async fn list_deliveries(
        &self,
        owner_id: UserId,
        is_admin: bool,
        limit: u64,
    ) -> Result<NotificationList, DbError> {
        let cond = visible_deliveries_condition(owner_id, is_admin);
        let unread_count = notification_deliveries::Entity::find()
            .filter(cond.clone())
            .filter(notification_deliveries::Column::ReadAt.is_null())
            .filter(notification_deliveries::Column::Status.eq("in_app"))
            .count(self.db.main())
            .await?;

        let rows = notification_deliveries::Entity::find()
            .filter(cond)
            .order_by_desc(notification_deliveries::Column::CreatedAt)
            .order_by_desc(notification_deliveries::Column::Id)
            .limit(limit)
            .all(self.db.main())
            .await?;

        let mut notifications = Vec::with_capacity(rows.len());
        for row in rows {
            notifications.push(row_to_delivery(row)?);
        }
        Ok(NotificationList {
            notifications,
            unread_count,
        })
    }

    pub async fn mark_all_read(&self, owner_id: UserId, is_admin: bool) -> Result<u64, DbError> {
        let now = now_ms();
        let res = notification_deliveries::Entity::update_many()
            .col_expr(notification_deliveries::Column::ReadAt, Expr::value(now))
            .filter(visible_deliveries_condition(owner_id, is_admin))
            .filter(notification_deliveries::Column::ReadAt.is_null())
            .exec(self.db.main())
            .await?;
        Ok(res.rows_affected)
    }

    pub async fn list_inbound_webhooks(&self) -> Result<Vec<InboundWebhook>, DbError> {
        let rows = inbound_webhooks::Entity::find()
            .order_by_desc(inbound_webhooks::Column::CreatedAt)
            .order_by_desc(inbound_webhooks::Column::Id)
            .all(self.db.main())
            .await?;
        rows.into_iter().map(row_to_webhook).collect()
    }

    pub async fn create_inbound_webhook(
        &self,
        name: &str,
        source_kind: InboundWebhookSourceKind,
        raw_token: &str,
        library_id: Option<LibraryId>,
        enabled: bool,
    ) -> Result<InboundWebhook, DbError> {
        let id = JobId::generate().to_string();
        let created_at = now_ms();
        let token_hash = sha256_hex(raw_token);
        let clean_name = truncate_chars(name.trim(), 128);
        inbound_webhooks::Entity::insert(inbound_webhooks::ActiveModel {
            id: Set(id.clone()),
            name: Set(clean_name.clone()),
            source_kind: Set(source_kind.as_str().to_owned()),
            token_hash: Set(token_hash),
            library_id: Set(library_id.map(|l| l.to_string())),
            enabled: Set(i32::from(enabled)),
            last_triggered_at: Set(None),
            created_at: Set(created_at),
        })
        .exec_without_returning(self.db.main())
        .await?;
        Ok(InboundWebhook {
            id,
            name: clean_name,
            source_kind,
            library_id,
            enabled,
            last_triggered_at: None,
            created_at: ts(created_at)?,
        })
    }

    pub async fn find_inbound_webhook_by_token(
        &self,
        raw_token: &str,
    ) -> Result<Option<InboundWebhook>, DbError> {
        let hash = sha256_hex(raw_token.trim());
        let Some(row) = inbound_webhooks::Entity::find()
            .filter(inbound_webhooks::Column::TokenHash.eq(hash))
            .one(self.db.main())
            .await?
        else {
            return Ok(None);
        };
        row_to_webhook(row).map(Some)
    }

    pub async fn touch_inbound_webhook(&self, id: &str, at_ms: i64) -> Result<(), DbError> {
        inbound_webhooks::Entity::update_many()
            .col_expr(
                inbound_webhooks::Column::LastTriggeredAt,
                Expr::value(Some(at_ms)),
            )
            .filter(inbound_webhooks::Column::Id.eq(id))
            .exec(self.db.main())
            .await?;
        Ok(())
    }

    pub async fn delete_inbound_webhook(&self, id: &str) -> Result<bool, DbError> {
        let res = inbound_webhooks::Entity::delete_by_id(id.to_owned())
            .exec(self.db.main())
            .await?;
        Ok(res.rows_affected > 0)
    }

    pub async fn list_routines(
        &self,
        owner_id: UserId,
        is_admin: bool,
    ) -> Result<Vec<ScheduledRoutine>, DbError> {
        let mut query = scheduled_routines::Entity::find()
            .order_by_asc(scheduled_routines::Column::CreatedAt)
            .order_by_asc(scheduled_routines::Column::Id);
        if !is_admin {
            query = query.filter(scheduled_routines::Column::OwnerId.eq(owner_id.to_string()));
        }
        let rows = query.all(self.db.main()).await?;
        rows.into_iter()
            .map(|r| row_to_stored_routine(r).map(|s| s.routine))
            .collect()
    }

    pub async fn list_enabled_routines(&self) -> Result<Vec<StoredRoutine>, DbError> {
        let rows = scheduled_routines::Entity::find()
            .filter(scheduled_routines::Column::Enabled.ne(0))
            .order_by_asc(scheduled_routines::Column::CreatedAt)
            .all(self.db.main())
            .await?;
        rows.into_iter().map(row_to_stored_routine).collect()
    }

    pub async fn get_routine(
        &self,
        caller_id: UserId,
        is_admin: bool,
        id: &str,
    ) -> Result<Option<ScheduledRoutine>, DbError> {
        let Some(row) = scheduled_routines::Entity::find_by_id(id.to_owned())
            .one(self.db.main())
            .await?
        else {
            return Ok(None);
        };
        if !is_admin && row.owner_id != caller_id.to_string() {
            return Ok(None);
        }
        row_to_stored_routine(row).map(|s| Some(s.routine))
    }

    pub async fn create_routine(&self, input: NewRoutine<'_>) -> Result<ScheduledRoutine, DbError> {
        let id = JobId::generate().to_string();
        let created_at = now_ms();
        let clean_name = truncate_chars(input.name.trim(), 128);
        let clean_expr = truncate_chars(input.schedule_expr.trim(), 64);
        scheduled_routines::Entity::insert(scheduled_routines::ActiveModel {
            id: Set(id.clone()),
            owner_id: Set(input.owner_id.to_string()),
            name: Set(clean_name.clone()),
            schedule_kind: Set(input.schedule_kind.as_str().to_owned()),
            schedule_expr: Set(clean_expr.clone()),
            action_kind: Set(input.action_kind.as_str().to_owned()),
            action_payload: Set(input.action_payload.to_owned()),
            enabled: Set(i32::from(input.enabled)),
            last_run_at: Set(None),
            last_status: Set(None),
            created_at: Set(created_at),
        })
        .exec_without_returning(self.db.main())
        .await?;
        Ok(ScheduledRoutine {
            id,
            owner_id: input.owner_id,
            name: clean_name,
            schedule_kind: input.schedule_kind,
            schedule_expr: clean_expr,
            action_kind: input.action_kind,
            action_payload: input.action_payload.to_owned(),
            enabled: input.enabled,
            last_run_at: None,
            last_status: None,
            created_at: ts(created_at)?,
        })
    }

    pub async fn update_routine(
        &self,
        caller_id: UserId,
        is_admin: bool,
        id: &str,
        patch: RoutinePatch,
    ) -> Result<Option<ScheduledRoutine>, DbError> {
        let Some(existing) = scheduled_routines::Entity::find_by_id(id.to_owned())
            .one(self.db.main())
            .await?
        else {
            return Ok(None);
        };
        if !is_admin && existing.owner_id != caller_id.to_string() {
            return Ok(None);
        }
        let mut active: scheduled_routines::ActiveModel = existing.into();
        if let Some(name) = patch.name {
            active.name = Set(truncate_chars(&name, 128));
        }
        if let Some(kind) = patch.schedule_kind {
            active.schedule_kind = Set(kind.as_str().to_owned());
        }
        if let Some(expr) = patch.schedule_expr {
            active.schedule_expr = Set(truncate_chars(&expr, 64));
        }
        if let Some(action_kind) = patch.action_kind {
            active.action_kind = Set(action_kind.as_str().to_owned());
        }
        if let Some(payload) = patch.action_payload {
            active.action_payload = Set(payload);
        }
        if let Some(enabled) = patch.enabled {
            active.enabled = Set(i32::from(enabled));
        }
        scheduled_routines::Entity::update(active)
            .exec(self.db.main())
            .await?;
        self.get_routine(caller_id, is_admin, id).await
    }

    pub async fn mark_routine_ran(
        &self,
        id: &str,
        ran_at_ms: i64,
        status: &str,
    ) -> Result<Option<ScheduledRoutine>, DbError> {
        let clean_status = truncate_chars(status, 64);
        scheduled_routines::Entity::update_many()
            .col_expr(
                scheduled_routines::Column::LastRunAt,
                Expr::value(Some(ran_at_ms)),
            )
            .col_expr(
                scheduled_routines::Column::LastStatus,
                Expr::value(Some(clean_status)),
            )
            .filter(scheduled_routines::Column::Id.eq(id))
            .exec(self.db.main())
            .await?;
        let Some(row) = scheduled_routines::Entity::find_by_id(id.to_owned())
            .one(self.db.main())
            .await?
        else {
            return Ok(None);
        };
        row_to_stored_routine(row).map(|s| Some(s.routine))
    }

    pub async fn delete_routine(
        &self,
        caller_id: UserId,
        is_admin: bool,
        id: &str,
    ) -> Result<bool, DbError> {
        let mut del =
            scheduled_routines::Entity::delete_many().filter(scheduled_routines::Column::Id.eq(id));
        if !is_admin {
            del = del.filter(scheduled_routines::Column::OwnerId.eq(caller_id.to_string()));
        }
        let res = del.exec(self.db.main()).await?;
        Ok(res.rows_affected > 0)
    }
}

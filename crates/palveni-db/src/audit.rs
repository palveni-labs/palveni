//! Security audit log (ops store, architecture §13).

use palveni_core::{AuditId, UserId};
use sea_orm::{ActiveValue, EntityTrait, QueryOrder, QuerySelect};

use crate::Db;
use crate::convert::{now_ms, ts};
use crate::entities::audit_log;
use crate::error::DbError;

pub struct Audit<'a> {
    pub(crate) db: &'a Db,
}

/// Well-known action names.
pub mod actions {
    pub const SETUP_COMPLETED: &str = "setup.completed";
    pub const LOGIN_SUCCEEDED: &str = "auth.login";
    pub const LOGIN_FAILED: &str = "auth.login_failed";
    pub const LOGOUT: &str = "auth.logout";
    pub const PASSWORD_CHANGED: &str = "auth.password_changed";
    pub const SESSION_REVOKED: &str = "auth.session_revoked";
    pub const USER_CREATED: &str = "user.created";
    pub const USER_UPDATED: &str = "user.updated";
    pub const USER_DELETED: &str = "user.deleted";
    pub const LIBRARY_UPDATED: &str = "library.updated";
    pub const LIBRARY_CREATED: &str = "library.created";
    pub const LIBRARY_DELETED: &str = "library.deleted";
    pub const ASSISTANT_RUN: &str = "assistant.run";
    pub const ASSISTANT_APPROVED: &str = "assistant.approved";
    pub const ASSISTANT_CANCELLED: &str = "assistant.cancelled";
    pub const ASSISTANT_CONFIG_UPDATED: &str = "assistant.config_updated";
    pub const HOME_BRIDGE_UPDATED: &str = "home.bridge_updated";
    pub const HOME_MQTT_UPDATED: &str = "home.mqtt_updated";
    pub const HOME_MQTT_DELETED: &str = "home.mqtt_deleted";
    pub const BACKUP_CREATED: &str = "system.backup_created";
    pub const BACKUP_RESTORED: &str = "system.backup_restored";
    pub const BACKUP_DELETED: &str = "system.backup_deleted";
    pub const SYSTEM_IMPORTED: &str = "system.imported";
    pub const SHARE_CREATED: &str = "share.created";
    pub const SHARE_REVOKED: &str = "share.revoked";
    pub const DOWNLOADER_UPDATED: &str = "downloads.client_updated";
    pub const DOWNLOADER_DELETED: &str = "downloads.client_deleted";
    pub const DOWNLOAD_SUBMITTED: &str = "downloads.task_submitted";
    pub const ORGANIZER_APPLIED: &str = "organizer.applied";
    pub const ORGANIZER_UNDONE: &str = "organizer.undone";
    pub const MCP_TOOL_CALLED: &str = "mcp.tool_called";
    pub const METADATA_CONFIG_UPDATED: &str = "metadata.config_updated";
    pub const METADATA_OVERRIDE_SAVED: &str = "metadata.override_saved";
    pub const METADATA_OVERRIDE_DELETED: &str = "metadata.override_deleted";
    pub const PLUGIN_INSTALLED: &str = "plugin.installed";
    pub const PLUGIN_UPDATED: &str = "plugin.updated";
    pub const PLUGIN_ROLLED_BACK: &str = "plugin.rolled_back";
    pub const PLUGIN_UNINSTALLED: &str = "plugin.uninstalled";
}

#[derive(Clone, Debug)]
pub struct AuditEntry {
    pub action: &'static str,
    pub actor: Option<UserId>,
    pub target: Option<String>,
    pub detail: Option<serde_json::Value>,
}

#[derive(Clone, Debug)]
pub struct AuditRecord {
    pub id: String,
    pub occurred_at: chrono::DateTime<chrono::Utc>,
    pub actor_id: Option<String>,
    pub action: String,
    pub target: Option<String>,
    pub detail: Option<String>,
}

impl Audit<'_> {
    pub async fn record(&self, entry: AuditEntry) -> Result<(), DbError> {
        let detail = entry
            .detail
            .map(|d| serde_json::to_string(&d))
            .transpose()
            .map_err(|e| DbError::Internal(format!("cannot encode audit detail: {e}")))?;
        audit_log::Entity::insert(audit_log::ActiveModel {
            id: ActiveValue::Set(AuditId::generate().to_string()),
            occurred_at: ActiveValue::Set(now_ms()),
            actor_id: ActiveValue::Set(entry.actor.map(|a| a.to_string())),
            action: ActiveValue::Set(entry.action.to_owned()),
            target: ActiveValue::Set(entry.target),
            detail: ActiveValue::Set(detail),
        })
        .exec_without_returning(self.db.ops())
        .await?;
        Ok(())
    }

    pub async fn recent(&self, limit: u64) -> Result<Vec<AuditRecord>, DbError> {
        let rows = audit_log::Entity::find()
            .order_by_desc(audit_log::Column::OccurredAt)
            .order_by_desc(audit_log::Column::Id)
            .limit(limit)
            .all(self.db.ops())
            .await?;
        rows.into_iter()
            .map(|row| {
                Ok(AuditRecord {
                    occurred_at: ts(row.occurred_at)?,
                    id: row.id,
                    actor_id: row.actor_id,
                    action: row.action,
                    target: row.target,
                    detail: row.detail,
                })
            })
            .collect()
    }
}

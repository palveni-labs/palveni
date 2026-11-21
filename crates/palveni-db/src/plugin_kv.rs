//! Per-plugin private key/value storage (ops store). Used by the plugin
//! host's `kv` interface; quota accounting lives in the host.

use sea_orm::sea_query::OnConflict;
use sea_orm::{ActiveValue, ColumnTrait, EntityTrait, QueryFilter};

use crate::Db;
use crate::convert::now_ms;
use crate::entities::plugin_kv;
use crate::error::DbError;

/// Largest value accepted on every backend (MySQL `BLOB` holds 64 KiB).
pub const MAX_VALUE_BYTES: usize = 65_535;

pub struct PluginKv<'a> {
    pub(crate) db: &'a Db,
}

impl PluginKv<'_> {
    pub async fn get(&self, plugin_id: &str, key: &str) -> Result<Option<Vec<u8>>, DbError> {
        Ok(
            plugin_kv::Entity::find_by_id((plugin_id.to_owned(), key.to_owned()))
                .one(self.db.ops())
                .await?
                .map(|row| row.kv_value),
        )
    }

    pub async fn set(&self, plugin_id: &str, key: &str, value: Vec<u8>) -> Result<(), DbError> {
        if value.len() > MAX_VALUE_BYTES {
            return Err(DbError::Conflict(format!(
                "value exceeds {MAX_VALUE_BYTES} bytes"
            )));
        }
        let row = plugin_kv::ActiveModel {
            plugin_id: ActiveValue::Set(plugin_id.to_owned()),
            kv_key: ActiveValue::Set(key.to_owned()),
            kv_value: ActiveValue::Set(value),
            updated_at: ActiveValue::Set(now_ms()),
        };
        let on_conflict =
            OnConflict::columns([plugin_kv::Column::PluginId, plugin_kv::Column::KvKey])
                .update_columns([plugin_kv::Column::KvValue, plugin_kv::Column::UpdatedAt])
                .to_owned();
        plugin_kv::Entity::insert(row)
            .on_conflict(on_conflict)
            .exec_without_returning(self.db.ops())
            .await?;
        Ok(())
    }

    pub async fn delete(&self, plugin_id: &str, key: &str) -> Result<bool, DbError> {
        let result = plugin_kv::Entity::delete_many()
            .filter(plugin_kv::Column::PluginId.eq(plugin_id.to_owned()))
            .filter(plugin_kv::Column::KvKey.eq(key.to_owned()))
            .exec(self.db.ops())
            .await?;
        Ok(result.rows_affected > 0)
    }

    /// Removes all data of one plugin (uninstall).
    pub async fn clear(&self, plugin_id: &str) -> Result<u64, DbError> {
        let result = plugin_kv::Entity::delete_many()
            .filter(plugin_kv::Column::PluginId.eq(plugin_id.to_owned()))
            .exec(self.db.ops())
            .await?;
        Ok(result.rows_affected)
    }
}

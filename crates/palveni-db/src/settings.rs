//! Server-wide key/value settings (main store).

use palveni_core::ServerId;
use sea_orm::sea_query::OnConflict;
use sea_orm::{ActiveValue, EntityTrait};

use crate::Db;
use crate::convert::{now_ms, parse_id};
use crate::entities::settings;
use crate::error::{DbError, is_unique_violation};

pub const KEY_RUNTIME: &str = "server.runtime";
pub const KEY_SERVER_ID: &str = "server.id";
pub const KEY_HARDWARE_REPORT: &str = "hardware.report";

pub struct Settings<'a> {
    pub(crate) db: &'a Db,
}

impl Settings<'_> {
    pub async fn get(&self, key: &str) -> Result<Option<String>, DbError> {
        Ok(settings::Entity::find_by_id(key.to_owned())
            .one(self.db.main())
            .await?
            .map(|row| row.setting_value))
    }

    /// Inserts or replaces a value.
    pub async fn set(&self, key: &str, value: &str) -> Result<(), DbError> {
        let row = settings::ActiveModel {
            setting_key: ActiveValue::Set(key.to_owned()),
            setting_value: ActiveValue::Set(value.to_owned()),
            updated_at: ActiveValue::Set(now_ms()),
        };
        let on_conflict = OnConflict::column(settings::Column::SettingKey)
            .update_columns([settings::Column::SettingValue, settings::Column::UpdatedAt])
            .to_owned();
        settings::Entity::insert(row)
            .on_conflict(on_conflict)
            .exec_without_returning(self.db.main())
            .await?;
        Ok(())
    }

    /// This installation's permanent ID, created on first use.
    pub async fn server_id(&self) -> Result<ServerId, DbError> {
        if let Some(value) = self.get(KEY_SERVER_ID).await? {
            return parse_id(&value, "server id");
        }
        let candidate = ServerId::generate();
        let row = settings::ActiveModel {
            setting_key: ActiveValue::Set(KEY_SERVER_ID.to_owned()),
            setting_value: ActiveValue::Set(candidate.to_string()),
            updated_at: ActiveValue::Set(now_ms()),
        };
        match settings::Entity::insert(row)
            .exec_without_returning(self.db.main())
            .await
        {
            Ok(_) => Ok(candidate),
            // Lost a race with another process: use the stored one.
            Err(err) if is_unique_violation(&err) => {
                let stored = self
                    .get(KEY_SERVER_ID)
                    .await?
                    .ok_or(DbError::NotFound("server id"))?;
                parse_id(&stored, "server id")
            }
            Err(err) => Err(err.into()),
        }
    }
}

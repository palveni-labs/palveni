//! `sessions` (main store): one bearer token. Only its SHA-256 is stored.

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "sessions")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub user_id: String,
    pub device_id: Option<String>,
    pub token_hash: String,
    pub created_at: i64,
    pub last_used_at: i64,
    pub expires_at: Option<i64>,
    pub revoked_at: Option<i64>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

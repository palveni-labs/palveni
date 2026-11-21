//! `plugin_kv` (ops store): per-plugin private key/value storage.

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "plugin_kv")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub plugin_id: String,
    #[sea_orm(primary_key, auto_increment = false)]
    pub kv_key: String,
    pub kv_value: Vec<u8>,
    pub updated_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

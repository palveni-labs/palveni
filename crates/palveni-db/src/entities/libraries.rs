//! `libraries` (main store). Synced to clients: carries `revision` and is
//! tombstoned (`deleted_at`) rather than deleted.

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "libraries")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub name: String,
    /// Lowercased, trimmed name used for the duplicate check.
    pub name_key: String,
    pub kind: String,
    pub last_scan_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
    pub revision: i64,
    pub deleted_at: Option<i64>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

//! `users` (main store).

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub username: String,
    /// Lowercased username; carries the unique index.
    pub username_key: String,
    pub password_hash: String,
    pub role: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub disabled_at: Option<i64>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

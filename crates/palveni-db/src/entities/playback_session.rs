//! `playback_sessions` (main store): active cross-device playback heartbeats.

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "playback_sessions")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub user_id: String,
    pub device_name: String,
    pub client_kind: String,
    pub media_kind: String,
    pub item_id: Option<String>,
    pub library_id: Option<String>,
    pub title: Option<String>,
    pub subtitle: Option<String>,
    pub artwork_id: Option<String>,
    pub state: String,
    pub position_ms: i64,
    pub duration_ms: i64,
    pub volume: i32,
    pub ambient_hex: Option<String>,
    pub updated_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::users::Entity",
        from = "Column::UserId",
        to = "super::users::Column::Id"
    )]
    User,
}

impl ActiveModelBehavior for ActiveModel {}

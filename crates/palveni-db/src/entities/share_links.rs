use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "share_links")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub token: String,
    pub creator_id: String,
    pub target_kind: String,
    pub target_id: String,
    pub library_id: Option<String>,
    pub title: String,
    pub password_hash: Option<String>,
    pub allow_download: i32,
    pub strip_gps: i32,
    pub expires_at: Option<i64>,
    pub max_views: Option<i32>,
    pub view_count: i64,
    pub revoked_at: Option<i64>,
    pub created_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::users::Entity",
        from = "Column::CreatorId",
        to = "super::users::Column::Id"
    )]
    Creator,
}

impl ActiveModelBehavior for ActiveModel {}

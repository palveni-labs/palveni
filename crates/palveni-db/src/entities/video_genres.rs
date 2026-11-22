//! One row per (video file, genre). Episodes carry their series' genres.
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "video_genres")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub item_id: String,
    #[sea_orm(primary_key, auto_increment = false)]
    pub name: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::video_metadata::Entity",
        from = "Column::ItemId",
        to = "super::video_metadata::Column::ItemId"
    )]
    Video,
    #[sea_orm(
        belongs_to = "super::items::Entity",
        from = "Column::ItemId",
        to = "super::items::Column::Id"
    )]
    Item,
}
impl ActiveModelBehavior for ActiveModel {}

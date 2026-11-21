use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "user_item_state")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub user_id: String,
    #[sea_orm(primary_key, auto_increment = false)]
    pub item_id: String,
    pub favorite_order: Option<i64>,
    pub stars: i32,
    pub position_ms: i64,
    pub completed: bool,
    pub play_count: i64,
    pub last_played_at: Option<i64>,
    pub playback_version: i64,
    pub previous_version: Option<i64>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::items::Entity",
        from = "Column::ItemId",
        to = "super::items::Column::Id"
    )]
    Item,
}
impl ActiveModelBehavior for ActiveModel {}

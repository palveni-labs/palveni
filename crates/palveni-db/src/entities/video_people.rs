//! Cast and crew in billing order. Episodes carry their series' people.
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "video_people")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub item_id: String,
    #[sea_orm(primary_key, auto_increment = false)]
    pub position: i32,
    pub kind: String,
    pub name: String,
    /// SHA-256 of the lowercase name.
    pub person_key: String,
    pub role: Option<String>,
    /// Absolute path of a local portrait inside the library.
    pub image: Option<String>,
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

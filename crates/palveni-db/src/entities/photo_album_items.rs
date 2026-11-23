use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "photo_album_items")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub album_id: String,
    #[sea_orm(primary_key, auto_increment = false)]
    pub item_id: String,
    pub position: i32,
    pub added_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::photo_albums::Entity",
        from = "Column::AlbumId",
        to = "super::photo_albums::Column::Id"
    )]
    Album,
    #[sea_orm(
        belongs_to = "super::items::Entity",
        from = "Column::ItemId",
        to = "super::items::Column::Id"
    )]
    Item,
}

impl Related<super::photo_albums::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Album.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "music_collection_favorites")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub user_id: String,
    #[sea_orm(primary_key, auto_increment = false)]
    pub folder_id: i32,
    #[sea_orm(primary_key, auto_increment = false)]
    pub artist_key: String,
    /// An empty key denotes the artist; catalog album keys are SHA-256 digests.
    #[sea_orm(primary_key, auto_increment = false)]
    pub album_key: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}

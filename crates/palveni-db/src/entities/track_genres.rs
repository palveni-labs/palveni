//! One row per distinct genre of a visible track. Album keys and `album_id` are
//! copied from `track_metadata` so album genre filters and genre counts need no join.

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "track_genres")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub item_id: String,
    #[sea_orm(primary_key, auto_increment = false)]
    pub genre_key: String,
    pub genre: String,
    pub genre_sort: String,
    pub library_id: String,
    pub artist_key: String,
    pub album_key: String,
    pub album_id: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

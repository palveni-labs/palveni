//! Album groups whose aggregates must be recomputed. `stamp` is the revision of
//! the latest change, so a refresh only clears what it actually recomputed.

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "music_album_pending")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub library_id: String,
    #[sea_orm(primary_key, auto_increment = false)]
    pub artist_key: String,
    #[sea_orm(primary_key, auto_increment = false)]
    pub album_key: String,
    pub stamp: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

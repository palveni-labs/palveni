//! Per-album aggregates of `track_metadata`, refreshed from `music_album_pending`
//! so album lists sort and page through an index instead of grouping every track.

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "music_albums")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub library_id: String,
    #[sea_orm(primary_key, auto_increment = false)]
    pub artist_key: String,
    #[sea_orm(primary_key, auto_increment = false)]
    pub album_key: String,
    pub name: Option<String>,
    pub name_sort: String,
    pub artist: Option<String>,
    pub artist_sort: String,
    pub year: Option<i64>,
    pub track_count: i64,
    pub disc_count: i64,
    pub duration_ms: i64,
    pub artwork_item_id: String,
    pub added_at: i64,
    pub shuffle: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

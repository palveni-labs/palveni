use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "track_metadata")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub item_id: String,
    pub library_id: String,
    pub artist_key: String,
    pub artist_sort: String,
    pub album_artist: Option<String>,
    pub album_key: String,
    pub album_sort: String,
    pub album: Option<String>,
    pub track_sort: String,
    pub title: String,
    pub artist: Option<String>,
    pub disc_number: Option<i64>,
    pub track_number: Option<i64>,
    pub duration_ms: Option<i64>,
    pub cue_source: Option<String>,
    pub cue_start: Option<i64>,
    pub cue_end: Option<i64>,
    pub cue_stamp: Option<String>,
    pub year: Option<i64>,
    pub codec: Option<String>,
    pub bitrate_kbps: Option<i64>,
    pub sample_rate_hz: Option<i64>,
    pub bit_depth: Option<i64>,
    pub channels: Option<i64>,
    /// Global title order; `track_sort` only orders tracks within an album.
    pub title_sort: Option<String>,
    pub track_artist_sort: Option<String>,
    /// Stable pseudo-random rank in `[0, 2^31 - 1)` for seeded shuffles.
    pub shuffle: Option<i64>,
    /// Tag reader generation; older rows are read again on the next scan.
    pub tags_version: Option<i64>,
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

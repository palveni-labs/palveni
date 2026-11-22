use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "video_metadata")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub item_id: String,
    pub library_id: String,
    pub stamp: String,
    pub work_key: String,
    pub work_sort: String,
    pub work_title: String,
    pub work_year: Option<i32>,
    pub work_overview: Option<String>,
    pub work_poster: Option<String>,
    pub season: Option<i32>,
    pub season_poster: Option<String>,
    pub episode: Option<i32>,
    pub episode_sort: String,
    pub title: String,
    pub overview: Option<String>,
    pub episode_poster: Option<String>,
    pub work_original_title: Option<String>,
    pub work_tagline: Option<String>,
    /// Tenths of a point, 0–100.
    pub work_rating: Option<i32>,
    pub work_official_rating: Option<String>,
    pub work_premiered: Option<String>,
    /// JSON array of studio names.
    pub work_studios: Option<String>,
    pub work_backdrop: Option<String>,
    pub runtime_ms: Option<i64>,
    /// Tenths of a point, 0–100.
    pub rating: Option<i32>,
    pub premiered: Option<String>,
    pub thumb: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::items::Entity",
        from = "Column::ItemId",
        to = "super::items::Column::Id"
    )]
    Item,
    #[sea_orm(
        belongs_to = "super::user_item_state::Entity",
        from = "Column::ItemId",
        to = "super::user_item_state::Column::ItemId"
    )]
    UserState,
}

impl ActiveModelBehavior for ActiveModel {}

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

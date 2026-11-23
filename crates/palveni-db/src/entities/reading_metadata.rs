use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "reading_metadata")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub item_id: String,
    pub library_id: String,
    pub stamp: String,
    pub kind: String,
    pub format: String,
    pub title: String,
    pub sort_title: String,
    pub authors_json: String,
    pub series_name: Option<String>,
    pub series_key: Option<String>,
    pub series_index: Option<f64>,
    pub summary: Option<String>,
    pub publisher: Option<String>,
    pub language: Option<String>,
    pub page_count: Option<i32>,
    pub has_cover: bool,
    pub tags_json: String,
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
        belongs_to = "super::libraries::Entity",
        from = "Column::LibraryId",
        to = "super::libraries::Column::Id"
    )]
    Library,
}

impl Related<super::items::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Item.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

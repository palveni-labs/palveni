use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "media_collection_items")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub collection_id: String,
    pub entry_kind: String,
    pub target_id: String,
    pub library_id: Option<String>,
    pub title: String,
    pub subtitle: Option<String>,
    pub artwork_id: Option<String>,
    pub position: i32,
    pub added_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::media_collections::Entity",
        from = "Column::CollectionId",
        to = "super::media_collections::Column::Id"
    )]
    Collection,
}

impl Related<super::media_collections::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Collection.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

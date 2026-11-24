use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "photo_person_tags")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub person_id: String,
    #[sea_orm(primary_key, auto_increment = false)]
    pub item_id: String,
    pub bbox_json: Option<String>,
    pub created_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::photo_person::Entity",
        from = "Column::PersonId",
        to = "super::photo_person::Column::Id"
    )]
    Person,
    #[sea_orm(
        belongs_to = "super::items::Entity",
        from = "Column::ItemId",
        to = "super::items::Column::Id"
    )]
    Item,
}

impl Related<super::photo_person::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Person.def()
    }
}

impl Related<super::items::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Item.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

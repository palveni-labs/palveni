use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "scan_review_items")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub revision: String,
    #[sea_orm(primary_key, auto_increment = false)]
    pub ordinal: i64,
    pub item_id: String,
    pub path: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::scan_reviews::Entity",
        from = "Column::Revision",
        to = "super::scan_reviews::Column::Revision"
    )]
    Review,
}
impl Related<super::scan_reviews::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Review.def()
    }
}
impl ActiveModelBehavior for ActiveModel {}

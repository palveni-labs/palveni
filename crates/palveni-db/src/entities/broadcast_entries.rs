use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "broadcast_entries")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub source_id: String,
    #[sea_orm(primary_key, auto_increment = false)]
    pub entry_id: String,
    pub position: i32,
    pub payload: String,
}
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}

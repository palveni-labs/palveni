use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "library_mounts")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub library_id: String,
    #[sea_orm(primary_key, auto_increment = false)]
    pub path_hash: String,
    pub path: String,
    pub source_hash: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}

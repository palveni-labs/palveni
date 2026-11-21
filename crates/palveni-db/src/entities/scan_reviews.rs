use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "scan_reviews")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub revision: String,
    pub library_id: String,
    pub scope_hash: String,
    pub scope: String,
    pub source_hash: String,
    pub digest: String,
    pub missing_count: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}

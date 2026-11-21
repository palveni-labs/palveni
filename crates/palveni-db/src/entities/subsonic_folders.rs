use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "subsonic_folders")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub library_id: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::libraries::Entity",
        from = "Column::LibraryId",
        to = "super::libraries::Column::Id"
    )]
    Library,
}
impl ActiveModelBehavior for ActiveModel {}

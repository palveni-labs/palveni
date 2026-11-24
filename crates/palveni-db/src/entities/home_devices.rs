use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "home_devices")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub room_id: String,
    pub bridge_id: Option<String>,
    pub external_entity_id: Option<String>,
    pub name: String,
    pub kind: String,
    pub online: bool,
    pub state_json: String,
    pub sort_order: i32,
    pub updated_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}

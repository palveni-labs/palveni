use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "plugin_installations")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub plugin_id: String,
    pub name: String,
    pub version: String,
    pub previous_version: Option<String>,
    pub runtime: String,
    pub provides_json: String,
    pub permissions_json: String,
    pub granted_permissions_json: String,
    pub settings_schema_json: String,
    pub settings_json: String,
    pub enabled: i32,
    pub source: String,
    pub installed_at: i64,
    pub updated_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

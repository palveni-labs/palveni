use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "scheduled_routines")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub owner_id: String,
    pub name: String,
    pub schedule_kind: String,
    pub schedule_expr: String,
    pub action_kind: String,
    pub action_payload: String,
    pub enabled: i32,
    pub last_run_at: Option<i64>,
    pub last_status: Option<String>,
    pub created_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::users::Entity",
        from = "Column::OwnerId",
        to = "super::users::Column::Id"
    )]
    Owner,
}

impl ActiveModelBehavior for ActiveModel {}

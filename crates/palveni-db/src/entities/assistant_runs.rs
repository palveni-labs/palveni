use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "assistant_runs")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub conversation_id: String,
    pub user_id: String,
    pub title: Option<String>,
    pub subject_json: Option<String>,
    pub request_text: Option<String>,
    pub previews_json: Option<String>,
    pub state: String,
    pub steps_json: String,
    pub blocks_json: String,
    pub pending_action_json: Option<String>,
    pub task_id: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}

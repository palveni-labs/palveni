//! `mqtt_bridges` (main store): pure-Rust MQTT 3.1.1 brokers and ambient light bindings.

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "mqtt_bridges")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: i32,
    pub username: Option<String>,
    pub password_cipher: Option<String>,
    pub topic_prefix: String,
    pub ambient_light_device_id: Option<String>,
    pub publish_playback: i32,
    pub enabled: i32,
    pub last_published_at: Option<i64>,
    pub created_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

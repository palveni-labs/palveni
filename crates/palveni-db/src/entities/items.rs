//! `items` (main store). Synced to clients: carries `revision` and is
//! tombstoned rather than deleted (except when its whole library is
//! deleted, which clients learn from the library tombstone).

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "items")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub library_id: String,
    pub parent_id: Option<String>,
    pub kind: String,
    pub title: String,
    /// Lowercase collation key, truncated to 255 characters.
    pub sort_title: String,
    /// Normalised searchable text.
    pub search_text: String,
    /// 0 for folders, 1 for leaves: folders sort first.
    pub sort_group: i32,
    /// Absolute host path; `None` for virtual items (artists, albums, …).
    pub path: Option<String>,
    /// SHA-256 of `path`; carries the per-library uniqueness constraint.
    pub path_hash: Option<String>,
    pub size_bytes: Option<i64>,
    pub modified_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
    pub revision: i64,
    pub deleted_at: Option<i64>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

use sea_orm::{ActiveValue::Set, EntityTrait, QueryOrder};

use crate::{Db, DbError, entities::media_connections as connection};

pub struct RemoteConnection {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub address: String,
    pub credentials: String,
}

impl From<connection::Model> for RemoteConnection {
    fn from(row: connection::Model) -> Self {
        Self {
            id: row.id,
            name: row.name,
            kind: row.kind,
            address: row.address,
            credentials: row.credentials,
        }
    }
}

impl Db {
    pub async fn media_connections(&self) -> Result<Vec<RemoteConnection>, DbError> {
        Ok(connection::Entity::find()
            .order_by_asc(connection::Column::Name)
            .order_by_asc(connection::Column::Id)
            .all(self.main())
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }
    pub async fn media_connection(&self, id: &str) -> Result<Option<RemoteConnection>, DbError> {
        Ok(connection::Entity::find_by_id(id.to_owned())
            .one(self.main())
            .await?
            .map(Into::into))
    }
    pub async fn add_media_connection(&self, conn: RemoteConnection) -> Result<(), DbError> {
        connection::Entity::insert(connection::ActiveModel {
            id: Set(conn.id),
            name: Set(conn.name),
            kind: Set(conn.kind),
            address: Set(conn.address),
            credentials: Set(conn.credentials),
        })
        .exec_without_returning(self.main())
        .await?;
        Ok(())
    }
    pub async fn remove_media_connection(&self, id: &str) -> Result<bool, DbError> {
        Ok(connection::Entity::delete_by_id(id.to_owned())
            .exec(self.main())
            .await?
            .rows_affected
            > 0)
    }
}

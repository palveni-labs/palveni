//! User accounts (main store).

use palveni_core::{Role, User, UserId, clock};
use sea_orm::{
    ActiveValue, ColumnTrait, ConnectionTrait, EntityTrait, PaginatorTrait, QueryFilter,
    TransactionTrait,
};

use crate::Db;
use crate::convert::{parse_id, ts};
use crate::entities::users;
use crate::error::{DbError, is_unique_violation};
use crate::revision;

pub struct Users<'a> {
    pub(crate) db: &'a Db,
}

/// A user together with the secret material needed to log in.
#[derive(Clone, Debug)]
pub struct UserCredentials {
    pub user: User,
    pub password_hash: String,
    pub disabled: bool,
}

/// Key used for case-insensitive uniqueness of usernames.
pub fn username_key(username: &str) -> String {
    username.trim().to_lowercase()
}

impl Users<'_> {
    pub async fn count(&self) -> Result<u64, DbError> {
        Ok(users::Entity::find().count(self.db.main()).await?)
    }

    /// Creates the first administrator, but only if no user exists yet.
    /// Returns `None` when setup has already been completed.
    pub async fn create_first_admin(
        &self,
        username: &str,
        password_hash: &str,
    ) -> Result<Option<User>, DbError> {
        let txn = self.db.main().begin().await?;
        // Taking the catalog write lock first serialises concurrent setup
        // requests on every backend, so exactly one of them can win.
        revision::next_revision(&txn).await?;
        if users::Entity::find().count(&txn).await? > 0 {
            txn.rollback().await?;
            return Ok(None);
        }
        let user = insert_user(&txn, username, password_hash, Role::Admin).await?;
        txn.commit().await?;
        Ok(Some(user))
    }

    pub async fn create(
        &self,
        username: &str,
        password_hash: &str,
        role: Role,
    ) -> Result<User, DbError> {
        insert_user(self.db.main(), username, password_hash, role).await
    }

    pub async fn find_credentials(
        &self,
        username: &str,
    ) -> Result<Option<UserCredentials>, DbError> {
        let row = users::Entity::find()
            .filter(users::Column::UsernameKey.eq(username_key(username)))
            .one(self.db.main())
            .await?;
        row.map(|row| {
            let disabled = row.disabled_at.is_some();
            let password_hash = row.password_hash.clone();
            Ok(UserCredentials {
                user: to_user(row)?,
                password_hash,
                disabled,
            })
        })
        .transpose()
    }

    pub async fn set_subsonic_generation(
        &self,
        id: UserId,
        value: Option<String>,
    ) -> Result<(), DbError> {
        use sea_orm::sea_query::Expr;
        users::Entity::update_many()
            .col_expr(users::Column::SubsonicGeneration, Expr::value(value))
            .filter(users::Column::Id.eq(id.to_string()))
            .filter(users::Column::DisabledAt.is_null())
            .exec(self.db.main())
            .await?;
        Ok(())
    }

    pub async fn subsonic_identity(
        &self,
        username: &str,
    ) -> Result<Option<(String, String)>, DbError> {
        use sea_orm::QuerySelect;
        Ok(users::Entity::find()
            .select_only()
            .column(users::Column::Id)
            .column(users::Column::SubsonicGeneration)
            .filter(users::Column::UsernameKey.eq(username_key(username)))
            .filter(users::Column::DisabledAt.is_null())
            .filter(users::Column::SubsonicGeneration.is_not_null())
            .into_tuple()
            .one(self.db.main())
            .await?)
    }

    pub async fn get(&self, id: UserId) -> Result<Option<User>, DbError> {
        users::Entity::find_by_id(id.to_string())
            .one(self.db.main())
            .await?
            .map(to_user)
            .transpose()
    }
}

async fn insert_user<C: ConnectionTrait>(
    conn: &C,
    username: &str,
    password_hash: &str,
    role: Role,
) -> Result<User, DbError> {
    let now = clock::now();
    let now_ms = clock::to_millis(now);
    let id = UserId::generate();
    let model = users::ActiveModel {
        id: ActiveValue::Set(id.to_string()),
        username: ActiveValue::Set(username.trim().to_owned()),
        username_key: ActiveValue::Set(username_key(username)),
        password_hash: ActiveValue::Set(password_hash.to_owned()),
        subsonic_generation: ActiveValue::Set(None),
        role: ActiveValue::Set(role.as_str().to_owned()),
        created_at: ActiveValue::Set(now_ms),
        updated_at: ActiveValue::Set(now_ms),
        disabled_at: ActiveValue::Set(None),
    };
    users::Entity::insert(model)
        .exec_without_returning(conn)
        .await
        .map_err(|err| {
            if is_unique_violation(&err) {
                DbError::Conflict("That username is already taken".to_owned())
            } else {
                DbError::Backend(err)
            }
        })?;
    Ok(User {
        id,
        username: username.trim().to_owned(),
        role,
        created_at: now,
    })
}

pub(crate) fn to_user(row: users::Model) -> Result<User, DbError> {
    Ok(User {
        id: parse_id(&row.id, "user id")?,
        role: row
            .role
            .parse::<Role>()
            .map_err(|_| DbError::Corrupt(format!("unknown role `{}`", row.role)))?,
        username: row.username,
        created_at: ts(row.created_at)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn username_keys_are_case_insensitive() {
        assert_eq!(username_key(" Alice "), "alice");
        assert_eq!(username_key("ÄNNE"), "änne");
    }
}

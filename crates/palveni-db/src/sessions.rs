//! Devices and login sessions (main store).
//!
//! Access tokens are random opaque strings; only their SHA-256 is stored, so
//! a database leak does not leak usable tokens. SHA-256 (not a slow KDF) is
//! appropriate because the token has 256 bits of entropy.

use chrono::{DateTime, Utc};
use palveni_core::{DeviceId, SessionId, User, UserId, clock};
use sea_orm::sea_query::Expr;
use sea_orm::{
    ActiveValue, ColumnTrait, Condition, EntityTrait, JoinType, QueryFilter, QueryOrder,
    QuerySelect, RelationDef, TransactionTrait,
};

use crate::Db;
use crate::convert::{now_ms, parse_id, parse_opt_id, sha256_hex, truncate_chars, ts};
use crate::entities::{devices, sessions, users};
use crate::error::{DbError, is_unique_violation};
use crate::users::to_user;

/// Prefix that makes leaked tokens recognisable to secret scanners.
pub const TOKEN_PREFIX: &str = "pvt_";
const TOKEN_BYTES: usize = 32;
const DEVICE_NAME_MAX_CHARS: usize = 128;

pub struct Sessions<'a> {
    pub(crate) db: &'a Db,
}

/// One active Link pairing on an enabled account.
pub struct PairedDevice {
    pub id: DeviceId,
    pub name: String,
    pub public_key: String,
}

/// Result of a successful login. `token` is shown to the client once.
#[derive(Clone, Debug)]
pub struct IssuedSession {
    pub token: String,
    pub session_id: SessionId,
    pub device_id: DeviceId,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug)]
pub struct AuthenticatedSession {
    pub session_id: SessionId,
    pub device_id: Option<DeviceId>,
    pub user: User,
    pub last_used_at: DateTime<Utc>,
}

/// Generates a new opaque access token.
pub fn generate_token() -> Result<String, DbError> {
    let mut bytes = [0u8; TOKEN_BYTES];
    getrandom::fill(&mut bytes)
        .map_err(|e| DbError::Internal(format!("system random source failed: {e}")))?;
    Ok(format!("{TOKEN_PREFIX}{}", hex::encode(bytes)))
}

/// Digest stored in place of the token.
pub fn hash_token(token: &str) -> String {
    sha256_hex(token)
}

impl Sessions<'_> {
    /// Registers a device for `user_id` and issues a session token on it.
    pub async fn create(
        &self,
        user_id: UserId,
        device_name: &str,
        expires_at: Option<DateTime<Utc>>,
    ) -> Result<IssuedSession, DbError> {
        let token = generate_token()?;
        let now = now_ms();
        let session_id = SessionId::generate();
        let device_id = DeviceId::generate();
        let name = truncate_chars(device_name.trim(), DEVICE_NAME_MAX_CHARS);

        let txn = self.db.main().begin().await?;
        devices::Entity::insert(devices::ActiveModel {
            id: ActiveValue::Set(device_id.to_string()),
            user_id: ActiveValue::Set(user_id.to_string()),
            name: ActiveValue::Set(name),
            public_key: ActiveValue::Set(None),
            created_at: ActiveValue::Set(now),
            last_seen_at: ActiveValue::Set(now),
            revoked_at: ActiveValue::Set(None),
        })
        .exec_without_returning(&txn)
        .await?;
        sessions::Entity::insert(sessions::ActiveModel {
            id: ActiveValue::Set(session_id.to_string()),
            user_id: ActiveValue::Set(user_id.to_string()),
            device_id: ActiveValue::Set(Some(device_id.to_string())),
            token_hash: ActiveValue::Set(hash_token(&token)),
            created_at: ActiveValue::Set(now),
            last_used_at: ActiveValue::Set(now),
            expires_at: ActiveValue::Set(expires_at.map(clock::to_millis)),
            revoked_at: ActiveValue::Set(None),
        })
        .exec_without_returning(&txn)
        .await?;
        txn.commit().await?;

        Ok(IssuedSession {
            token,
            session_id,
            device_id,
            expires_at,
        })
    }

    /// Resolves a bearer token to its live session and user.
    pub async fn authenticate(&self, token: &str) -> Result<Option<AuthenticatedSession>, DbError> {
        self.live_session(sessions::Column::TokenHash.eq(hash_token(token)))
            .await
    }

    /// Revalidates the session after the caller verifies a scoped media signature.
    pub async fn active_by_id(
        &self,
        id: SessionId,
    ) -> Result<Option<AuthenticatedSession>, DbError> {
        self.live_session(sessions::Column::Id.eq(id.to_string()))
            .await
    }

    async fn live_session(&self, key: Expr) -> Result<Option<AuthenticatedSession>, DbError> {
        let session = sessions::Entity::find()
            .filter(key)
            .filter(sessions::Column::RevokedAt.is_null())
            .one(self.db.main())
            .await?;
        let Some(session) = session else {
            return Ok(None);
        };
        if session.expires_at.is_some_and(|exp| exp <= now_ms()) {
            return Ok(None);
        }
        let user = users::Entity::find_by_id(session.user_id.clone())
            .one(self.db.main())
            .await?;
        let Some(user) = user else {
            return Ok(None);
        };
        if user.disabled_at.is_some() {
            return Ok(None);
        }
        Ok(Some(AuthenticatedSession {
            session_id: parse_id(&session.id, "session id")?,
            device_id: parse_opt_id(session.device_id.as_deref(), "device id")?,
            user: to_user(user)?,
            last_used_at: ts(session.last_used_at)?,
        }))
    }

    /// Records activity. Callers throttle this; it is two small writes.
    pub async fn touch(
        &self,
        session_id: SessionId,
        device_id: Option<DeviceId>,
    ) -> Result<(), DbError> {
        let now = now_ms();
        sessions::Entity::update_many()
            .col_expr(sessions::Column::LastUsedAt, Expr::val(now))
            .filter(sessions::Column::Id.eq(session_id.to_string()))
            .exec(self.db.main())
            .await?;
        if let Some(device_id) = device_id {
            devices::Entity::update_many()
                .col_expr(devices::Column::LastSeenAt, Expr::val(now))
                .filter(devices::Column::Id.eq(device_id.to_string()))
                .exec(self.db.main())
                .await?;
        }
        Ok(())
    }

    /// Revokes one session (logout). Idempotent.
    pub async fn revoke(&self, session_id: SessionId) -> Result<(), DbError> {
        sessions::Entity::update_many()
            .col_expr(sessions::Column::RevokedAt, Expr::val(now_ms()))
            .filter(sessions::Column::Id.eq(session_id.to_string()))
            .filter(sessions::Column::RevokedAt.is_null())
            .exec(self.db.main())
            .await?;
        Ok(())
    }

    pub async fn paired_devices(
        &self,
        owner: Option<UserId>,
    ) -> Result<Vec<PairedDevice>, DbError> {
        let user: RelationDef = devices::Entity::belongs_to(users::Entity)
            .from(devices::Column::UserId)
            .to(users::Column::Id)
            .into();
        let mut query = devices::Entity::find()
            .join(JoinType::InnerJoin, user)
            .filter(devices::Column::PublicKey.is_not_null())
            .filter(devices::Column::RevokedAt.is_null())
            .filter(users::Column::DisabledAt.is_null());
        if let Some(id) = owner {
            query = query.filter(devices::Column::UserId.eq(id.to_string()));
        }
        query
            .select_only()
            .columns([
                devices::Column::Id,
                devices::Column::Name,
                devices::Column::PublicKey,
            ])
            .order_by_asc(devices::Column::Id)
            .into_tuple::<(String, String, String)>()
            .all(self.db.main())
            .await?
            .into_iter()
            .map(|(id, name, public_key)| {
                Ok(PairedDevice {
                    id: parse_id(&id, "device id")?,
                    name,
                    public_key,
                })
            })
            .collect()
    }

    pub async fn pair_device(
        &self,
        owner: UserId,
        device: DeviceId,
        session: SessionId,
        key: &str,
    ) -> Result<DeviceId, DbError> {
        let conflict = || DbError::Conflict("Device or key already assigned or revoked".into());
        let txn = self.db.main().begin().await?;
        let original = devices::Entity::find_by_id(device.to_string())
            .filter(devices::Column::UserId.eq(owner.to_string()))
            .filter(devices::Column::RevokedAt.is_null())
            .one(&txn)
            .await?
            .ok_or_else(conflict)?;
        if original
            .public_key
            .as_deref()
            .is_some_and(|value| value != key)
        {
            return Err(conflict());
        }
        let existing = devices::Entity::find()
            .filter(devices::Column::PublicKey.eq(key))
            .one(&txn)
            .await?;
        let stable_id = match existing {
            Some(row) if row.revoked_at.is_none() && row.user_id == original.user_id => {
                parse_id(&row.id, "paired device id")?
            }
            Some(_) => return Err(conflict()),
            None => device,
        };
        // This update locks the same device row as revoke_device before either touches sessions.
        let registered = devices::Entity::update_many()
            .col_expr(devices::Column::PublicKey, Expr::val(key.to_owned()))
            .col_expr(devices::Column::LastSeenAt, Expr::val(now_ms()))
            .filter(devices::Column::Id.eq(stable_id.to_string()))
            .filter(devices::Column::RevokedAt.is_null())
            .filter(
                Condition::any()
                    .add(devices::Column::PublicKey.is_null())
                    .add(devices::Column::PublicKey.eq(key)),
            )
            .exec(&txn)
            .await
            .map_err(|err| {
                if is_unique_violation(&err) {
                    conflict()
                } else {
                    err.into()
                }
            })?;
        if registered.rows_affected != 1 {
            return Err(conflict());
        }
        // A normal login starts with a fresh device row. Proof of the existing key
        // attaches that login to the durable installation so revocation reaches it.
        let attached = sessions::Entity::update_many()
            .col_expr(sessions::Column::DeviceId, Expr::val(stable_id.to_string()))
            .filter(sessions::Column::Id.eq(session.to_string()))
            .filter(sessions::Column::UserId.eq(owner.to_string()))
            .filter(sessions::Column::DeviceId.eq(device.to_string()))
            .filter(sessions::Column::RevokedAt.is_null())
            .filter(
                Condition::any()
                    .add(sessions::Column::ExpiresAt.is_null())
                    .add(sessions::Column::ExpiresAt.gt(now_ms())),
            )
            .exec(&txn)
            .await?;
        if attached.rows_affected != 1 {
            return Err(conflict());
        }
        txn.commit().await?;
        Ok(stable_id)
    }

    pub async fn paired_device_key(
        &self,
        owner: UserId,
        device: DeviceId,
    ) -> Result<Option<String>, DbError> {
        Ok(devices::Entity::find_by_id(device.to_string())
            .filter(devices::Column::UserId.eq(owner.to_string()))
            .filter(devices::Column::RevokedAt.is_null())
            .filter(devices::Column::PublicKey.is_not_null())
            .select_only()
            .column(devices::Column::PublicKey)
            .into_tuple::<String>()
            .one(self.db.main())
            .await?)
    }

    /// Revokes a device and every session issued on it.
    pub async fn revoke_device(&self, device_id: DeviceId) -> Result<(), DbError> {
        let now = now_ms();
        let txn = self.db.main().begin().await?;
        devices::Entity::update_many()
            .col_expr(devices::Column::RevokedAt, Expr::val(now))
            .filter(devices::Column::Id.eq(device_id.to_string()))
            .filter(devices::Column::RevokedAt.is_null())
            .exec(&txn)
            .await?;
        sessions::Entity::update_many()
            .col_expr(sessions::Column::RevokedAt, Expr::val(now))
            .filter(sessions::Column::DeviceId.eq(device_id.to_string()))
            .filter(sessions::Column::RevokedAt.is_null())
            .exec(&txn)
            .await?;
        txn.commit().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_are_prefixed_random_hex() {
        let a = generate_token().unwrap();
        let b = generate_token().unwrap();
        assert_ne!(a, b);
        assert!(a.starts_with(TOKEN_PREFIX));
        assert_eq!(a.len(), TOKEN_PREFIX.len() + TOKEN_BYTES * 2);
        assert!(
            a[TOKEN_PREFIX.len()..]
                .chars()
                .all(|c| c.is_ascii_hexdigit())
        );
    }

    #[test]
    fn token_hash_is_stable_and_not_the_token() {
        let token = "pvt_00112233";
        let h1 = hash_token(token);
        assert_eq!(h1, hash_token(token));
        assert_ne!(h1, token);
        assert_eq!(h1.len(), 64);
        assert_ne!(h1, hash_token("pvt_00112234"));
    }
}

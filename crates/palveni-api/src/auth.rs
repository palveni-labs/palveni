//! Authentication: bearer-token extractors, the admin guard, password
//! hashing (argon2id) and credential validation rules.

use std::sync::OnceLock;
use std::time::Duration;

use argon2::Argon2;
use argon2::password_hash::phc::PasswordHash;
use argon2::password_hash::{PasswordHasher, PasswordVerifier};
use axum::extract::FromRequestParts;
use axum::http::HeaderMap;
use axum::http::header::AUTHORIZATION;
use axum::http::request::Parts;
use palveni_core::{DeviceId, SessionId, User, clock};

use crate::error::ApiError;
use crate::state::AppState;

/// Sessions record activity at most this often, keeping writes off the
/// hot path of every request.
const TOUCH_INTERVAL: Duration = Duration::from_secs(300);

pub const USERNAME_MAX_CHARS: usize = 64;
pub const PASSWORD_MIN_CHARS: usize = 8;
/// Bounds hashing cost for absurd inputs.
pub const PASSWORD_MAX_CHARS: usize = 1024;

/// The authenticated caller.
#[derive(Clone, Debug)]
pub struct CurrentUser {
    pub user: User,
    pub session_id: SessionId,
    pub device_id: Option<DeviceId>,
}

impl CurrentUser {
    pub fn is_admin(&self) -> bool {
        self.user.is_admin()
    }
}

impl FromRequestParts<AppState> for CurrentUser {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let token = bearer_token(&parts.headers).ok_or_else(ApiError::unauthorized)?;
        let session = state
            .db
            .sessions()
            .authenticate(&token)
            .await?
            .ok_or_else(ApiError::unauthorized)?;

        let idle = clock::now().signed_duration_since(session.last_used_at);
        if idle.num_seconds() >= i64::try_from(TOUCH_INTERVAL.as_secs()).unwrap_or(i64::MAX)
            && let Err(err) = state
                .db
                .sessions()
                .touch(session.session_id, session.device_id)
                .await
        {
            tracing::warn!(error = %err, "cannot record session activity");
        }

        Ok(CurrentUser {
            user: session.user,
            session_id: session.session_id,
            device_id: session.device_id,
        })
    }
}

/// An authenticated administrator; other users get `403 forbidden`.
#[derive(Clone, Debug)]
pub struct AdminUser(pub CurrentUser);

impl FromRequestParts<AppState> for AdminUser {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let user = CurrentUser::from_request_parts(parts, state).await?;
        if user.is_admin() {
            Ok(AdminUser(user))
        } else {
            Err(ApiError::forbidden())
        }
    }
}

/// Extracts the token from `Authorization: Bearer <token>`.
pub fn bearer_token(headers: &HeaderMap) -> Option<String> {
    let value = headers.get(AUTHORIZATION)?.to_str().ok()?;
    let (scheme, token) = value.trim().split_once(' ')?;
    if !scheme.eq_ignore_ascii_case("bearer") {
        return None;
    }
    let token = token.trim();
    (!token.is_empty()).then(|| token.to_owned())
}

pub fn validate_username(username: &str) -> Result<(), ApiError> {
    let length = username.chars().count();
    if length == 0 || length > USERNAME_MAX_CHARS {
        return Err(ApiError::validation(format!(
            "Username must be 1 to {USERNAME_MAX_CHARS} characters"
        ))
        .field("username"));
    }
    if username.chars().any(char::is_whitespace) {
        return Err(ApiError::validation("Username must not contain whitespace").field("username"));
    }
    Ok(())
}

pub fn validate_password(password: &str) -> Result<(), ApiError> {
    let length = password.chars().count();
    if length < PASSWORD_MIN_CHARS {
        return Err(ApiError::validation(format!(
            "Password must be at least {PASSWORD_MIN_CHARS} characters"
        ))
        .field("password"));
    }
    if length > PASSWORD_MAX_CHARS {
        return Err(ApiError::validation(format!(
            "Password must be at most {PASSWORD_MAX_CHARS} characters"
        ))
        .field("password"));
    }
    Ok(())
}

/// Hashes with argon2id (default parameters) off the async runtime.
pub async fn hash_password(password: String) -> Result<String, ApiError> {
    tokio::task::spawn_blocking(move || -> Result<String, String> {
        let hash: PasswordHash = Argon2::default()
            .hash_password(password.as_bytes())
            .map_err(|e| e.to_string())?;
        Ok(hash.to_string())
    })
    .await
    .map_err(ApiError::internal)?
    .map_err(ApiError::internal)
}

/// Verifies a password against a stored PHC string off the async runtime.
pub async fn verify_password(password: String, stored_hash: String) -> Result<bool, ApiError> {
    tokio::task::spawn_blocking(move || {
        let Ok(parsed) = PasswordHash::new(&stored_hash) else {
            return false;
        };
        Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok()
    })
    .await
    .map_err(ApiError::internal)
}

/// A real hash of a throwaway password, verified against when the
/// username does not exist so both paths take the same time.
pub async fn dummy_hash() -> Option<String> {
    static DUMMY: OnceLock<String> = OnceLock::new();
    if let Some(hash) = DUMMY.get() {
        return Some(hash.clone());
    }
    let hash = hash_password("palveni-timing-equaliser".to_owned())
        .await
        .ok()?;
    Some(DUMMY.get_or_init(|| hash).clone())
}

#[cfg(test)]
mod tests {
    use axum::http::HeaderValue;

    use super::*;

    fn headers(value: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, HeaderValue::from_str(value).unwrap());
        headers
    }

    #[test]
    fn bearer_parsing() {
        assert_eq!(
            bearer_token(&headers("Bearer pvt_abc")).as_deref(),
            Some("pvt_abc")
        );
        assert_eq!(
            bearer_token(&headers("bearer  pvt_abc ")).as_deref(),
            Some("pvt_abc")
        );
        assert_eq!(bearer_token(&headers("Basic dXNlcjpwYXNz")), None);
        assert_eq!(bearer_token(&headers("Bearer")), None);
        assert_eq!(bearer_token(&HeaderMap::new()), None);
    }

    #[test]
    fn credential_rules() {
        assert!(validate_username("alice").is_ok());
        assert!(validate_username("张三").is_ok());
        assert!(validate_username("").is_err());
        assert!(validate_username("a b").is_err());
        assert!(validate_username(&"x".repeat(65)).is_err());
        assert!(validate_password("12345678").is_ok());
        assert!(validate_password("1234567").is_err());
        // Counted in characters, not bytes.
        assert!(validate_password("密码密码密码密").is_err());
    }

    #[tokio::test]
    async fn hash_and_verify() {
        let hash = hash_password("correct horse".to_owned()).await.unwrap();
        assert!(hash.starts_with("$argon2id$"));
        assert!(
            verify_password("correct horse".to_owned(), hash.clone())
                .await
                .unwrap()
        );
        assert!(
            !verify_password("wrong horse".to_owned(), hash)
                .await
                .unwrap()
        );
        assert!(
            !verify_password("x".to_owned(), "not a hash".to_owned())
                .await
                .unwrap()
        );
    }
}

use sea_orm::{DbErr, SqlErr};

#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("{0} not found")]
    NotFound(&'static str),

    #[error("{0}")]
    Invalid(&'static str),

    #[error("{0}")]
    Conflict(String),

    #[error("invalid database configuration: {0}")]
    Config(String),

    /// A stored value could not be decoded (bad ID, unknown enum value, …).
    #[error("unexpected data in database: {0}")]
    Corrupt(String),

    #[error("{0}")]
    Internal(String),

    #[error(transparent)]
    Backend(#[from] DbErr),
}

impl From<DbError> for palveni_core::Error {
    fn from(err: DbError) -> Self {
        match err {
            DbError::NotFound(what) => palveni_core::Error::NotFound(what.to_owned()),
            DbError::Invalid(message) => palveni_core::Error::invalid(message),
            DbError::Conflict(message) => palveni_core::Error::Conflict(message),
            other => palveni_core::Error::Internal(other.to_string()),
        }
    }
}

/// True for unique/primary-key violations on any backend.
pub(crate) fn is_unique_violation(err: &DbErr) -> bool {
    matches!(err.sql_err(), Some(SqlErr::UniqueConstraintViolation(_)))
}

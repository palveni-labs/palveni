//! Crate-wide error type shared by the layers above `core`.
//!
//! Lower-level crates (database, media, jobs) keep their own error enums and
//! convert into this one at their boundary when the caller does not need the
//! detail.

/// Shorthand result alias.
pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The named thing does not exist (or is not visible to the caller).
    #[error("{0} not found")]
    NotFound(String),

    /// Input failed validation; the message is safe to show to users.
    #[error("{0}")]
    Invalid(String),

    /// The operation conflicts with existing state (duplicate name, …).
    #[error("{0}")]
    Conflict(String),

    /// A storage path resolved to a location outside its root.
    #[error("path escapes the storage root")]
    PathEscape,

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Anything else; the message is for logs, not for users.
    #[error("{0}")]
    Internal(String),
}

impl Error {
    pub fn not_found(what: impl Into<String>) -> Self {
        Self::NotFound(what.into())
    }

    pub fn invalid(message: impl Into<String>) -> Self {
        Self::Invalid(message.into())
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self::Conflict(message.into())
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal(message.into())
    }
}

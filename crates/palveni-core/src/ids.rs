//! Strongly typed identifiers.
//!
//! Every primary key is a UUIDv7: time-ordered (good B-tree locality in
//! SQLite) and safe to generate offline on clients. On the wire and in the
//! database they are lowercase, hyphenated strings.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

macro_rules! define_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(utoipa::ToSchema, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(Uuid);

        impl $name {
            /// Generates a new time-ordered (UUIDv7) identifier.
            pub fn generate() -> Self {
                Self(Uuid::now_v7())
            }

            pub const fn from_uuid(uuid: Uuid) -> Self {
                Self(uuid)
            }

            pub const fn as_uuid(&self) -> &Uuid {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Display::fmt(&self.0.hyphenated(), f)
            }
        }

        impl FromStr for $name {
            type Err = crate::Error;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Uuid::parse_str(s.trim()).map(Self).map_err(|_| {
                    crate::Error::invalid(format!("`{s}` is not a valid {}", stringify!($name)))
                })
            }
        }
    };
}

define_id!(
    /// A user account.
    UserId
);
define_id!(
    /// A login session (one bearer token).
    SessionId
);
define_id!(
    /// A registered client installation.
    DeviceId
);
define_id!(
    /// A media library.
    LibraryId
);
define_id!(
    /// One storage location of a library.
    LibraryPathId
);
define_id!(
    /// A node in a library's item tree.
    ItemId
);
define_id!(
    /// A background job.
    JobId
);
define_id!(
    /// This server installation; generated on first start and never changed.
    ServerId
);
define_id!(
    /// An audit log entry.
    AuditId
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_is_lowercase_hyphenated() {
        let id: ItemId = "0192F3A4-5B6C-7D8E-9F01-23456789ABCD".parse().unwrap();
        assert_eq!(id.to_string(), "0192f3a4-5b6c-7d8e-9f01-23456789abcd");
    }

    #[test]
    fn serde_is_a_plain_string() {
        let id = LibraryId::generate();
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, format!("\"{id}\""));
        let back: LibraryId = serde_json::from_str(&json).unwrap();
        assert_eq!(back, id);
    }

    #[test]
    fn generated_ids_are_v7_and_ordered() {
        let a = JobId::generate();
        let b = JobId::generate();
        assert_eq!(a.as_uuid().get_version_num(), 7);
        assert!(a <= b);
    }

    #[test]
    fn rejects_garbage() {
        assert!("not-a-uuid".parse::<UserId>().is_err());
    }
}

//! Strongly typed identifiers. All IDs are UUIDv7: time-ordered, which keeps
//! B-tree indexes compact and makes cursor pagination by id cheap.

use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

macro_rules! define_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub Uuid);

        impl $name {
            pub fn new() -> Self {
                Self(Uuid::now_v7())
            }

            pub fn from_uuid(id: Uuid) -> Self {
                Self(id)
            }

            pub fn as_uuid(&self) -> &Uuid {
                &self.0
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl From<Uuid> for $name {
            fn from(id: Uuid) -> Self {
                Self(id)
            }
        }
    };
}

define_id!(
    /// A person with an account (customer, master, owner...).
    UserId
);
define_id!(
    /// A beauty business: a solo master or a salon. The tenant boundary.
    BusinessId
);
define_id!(
    /// A stored refresh token (one per device session).
    RefreshTokenId
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique_and_time_ordered() {
        let a = UserId::new();
        let b = UserId::new();
        assert_ne!(a, b);
        assert!(a.as_uuid() <= b.as_uuid());
    }

    #[test]
    fn id_roundtrips_through_uuid_and_json() {
        let id = BusinessId::new();
        assert_eq!(BusinessId::from_uuid(*id.as_uuid()), id);

        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, format!("\"{}\"", id));
        let back: BusinessId = serde_json::from_str(&json).unwrap();
        assert_eq!(back, id);
    }
}

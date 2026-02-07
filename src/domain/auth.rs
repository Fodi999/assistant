use crate::shared::{RefreshTokenId, UserId};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshToken {
    pub id: RefreshTokenId,
    pub user_id: UserId,
    pub token_hash: String,
    pub expires_at: OffsetDateTime,
    pub revoked_at: Option<OffsetDateTime>,
    pub created_at: OffsetDateTime,
}

impl RefreshToken {
    pub fn new(user_id: UserId, token_hash: String, expires_at: OffsetDateTime) -> Self {
        Self {
            id: RefreshTokenId::new(),
            user_id,
            token_hash,
            expires_at,
            revoked_at: None,
            created_at: OffsetDateTime::now_utc(),
        }
    }

    pub fn is_expired(&self) -> bool {
        OffsetDateTime::now_utc() > self.expires_at
    }

    pub fn is_revoked(&self) -> bool {
        self.revoked_at.is_some()
    }

    pub fn is_valid(&self) -> bool {
        !self.is_expired() && !self.is_revoked()
    }

    pub fn revoke(&mut self) {
        self.revoked_at = Some(OffsetDateTime::now_utc());
    }
}

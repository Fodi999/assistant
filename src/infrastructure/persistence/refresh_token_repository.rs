use crate::domain::RefreshToken;
use crate::shared::{AppResult, RefreshTokenId, UserId};
use async_trait::async_trait;
use sqlx::{PgPool, Row};
use time::OffsetDateTime;
use uuid::Uuid;

#[async_trait]
pub trait RefreshTokenRepositoryTrait: Send + Sync {
    async fn create(&self, token: &RefreshToken) -> AppResult<()>;
    async fn find_by_token_hash(&self, token_hash: &str) -> AppResult<Option<RefreshToken>>;
    async fn revoke(&self, id: RefreshTokenId) -> AppResult<()>;
    async fn revoke_all_for_user(&self, user_id: UserId) -> AppResult<()>;
}

#[derive(Clone)]
pub struct RefreshTokenRepository {
    pool: PgPool,
}

impl RefreshTokenRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl RefreshTokenRepositoryTrait for RefreshTokenRepository {
    async fn create(&self, token: &RefreshToken) -> AppResult<()> {
        sqlx::query(
            r#"
            INSERT INTO refresh_tokens (id, user_id, token_hash, expires_at, revoked_at, created_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#
        )
        .bind(token.id.as_uuid())
        .bind(token.user_id.as_uuid())
        .bind(&token.token_hash)
        .bind(token.expires_at)
        .bind(token.revoked_at)
        .bind(token.created_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn find_by_token_hash(&self, token_hash: &str) -> AppResult<Option<RefreshToken>> {
        let result = sqlx::query(
            r#"
            SELECT id, user_id, token_hash, expires_at, revoked_at, created_at
            FROM refresh_tokens
            WHERE token_hash = $1
            "#
        )
        .bind(token_hash)
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.map(|row| {
            let id: Uuid = row.get("id");
            let user_id: Uuid = row.get("user_id");
            let token_hash: String = row.get("token_hash");
            let expires_at: OffsetDateTime = row.get("expires_at");
            let revoked_at: Option<OffsetDateTime> = row.get("revoked_at");
            let created_at: OffsetDateTime = row.get("created_at");

            RefreshToken {
                id: RefreshTokenId::from_uuid(id),
                user_id: UserId::from_uuid(user_id),
                token_hash,
                expires_at,
                revoked_at,
                created_at,
            }
        }))
    }

    async fn revoke(&self, id: RefreshTokenId) -> AppResult<()> {
        sqlx::query(
            r#"
            UPDATE refresh_tokens
            SET revoked_at = $1
            WHERE id = $2
            "#
        )
        .bind(OffsetDateTime::now_utc())
        .bind(id.as_uuid())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn revoke_all_for_user(&self, user_id: UserId) -> AppResult<()> {
        sqlx::query(
            r#"
            UPDATE refresh_tokens
            SET revoked_at = $1
            WHERE user_id = $2 AND revoked_at IS NULL
            "#
        )
        .bind(OffsetDateTime::now_utc())
        .bind(user_id.as_uuid())
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

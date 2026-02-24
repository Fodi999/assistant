use sqlx::PgPool;
use serde_json::Value;
use crate::shared::AppError;

#[derive(Clone)]
pub struct AiCacheRepository {
    pool: PgPool,
}

impl AiCacheRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn get(&self, key: &str) -> Result<Option<Value>, AppError> {
        let row: Option<(Value,)> = sqlx::query_as(
            "SELECT value FROM ai_cache WHERE key = $1 AND expires_at > NOW()"
        )
        .bind(key)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::from)?;

        Ok(row.map(|r| r.0))
    }

    pub async fn set(&self, key: &str, value: Value, provider: &str, model: &str, ttl_days: i32) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO ai_cache (key, value, provider, model, expires_at) 
             VALUES ($1, $2, $3, $4, NOW() + ($5 || ' days')::INTERVAL)
             ON CONFLICT (key) DO UPDATE SET 
                value = EXCLUDED.value,
                provider = EXCLUDED.provider,
                model = EXCLUDED.model,
                expires_at = EXCLUDED.expires_at,
                created_at = NOW()"
        )
        .bind(key)
        .bind(value)
        .bind(provider)
        .bind(model)
        .bind(ttl_days)
        .execute(&self.pool)
        .await
        .map_err(AppError::from)?;

        Ok(())
    }
}

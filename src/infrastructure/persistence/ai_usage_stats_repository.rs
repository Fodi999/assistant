use sqlx::PgPool;
use crate::shared::AppError;

#[derive(Clone)]
pub struct AiUsageStatsRepository {
    pool: PgPool,
}

impl AiUsageStatsRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn log_usage(
        &self,
        endpoint: &str,
        prompt_tokens: i32,
        completion_tokens: i32,
        total_tokens: i32,
        duration_ms: i32,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO ai_usage_stats (endpoint, prompt_tokens, completion_tokens, total_tokens, duration_ms) 
             VALUES ($1, $2, $3, $4, $5)"
        )
        .bind(endpoint)
        .bind(prompt_tokens)
        .bind(completion_tokens)
        .bind(total_tokens)
        .bind(duration_ms)
        .execute(&self.pool)
        .await
        .map_err(AppError::from)?;

        Ok(())
    }
}

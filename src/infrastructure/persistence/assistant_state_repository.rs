use crate::domain::assistant::{state::AssistantState, step::AssistantStep};
use crate::shared::{result::AppResult, types::{UserId, TenantId}};
use async_trait::async_trait;
use sqlx::{PgPool, Row};

#[async_trait]
pub trait AssistantStateRepositoryTrait: Send + Sync {
    async fn get_or_create(&self, user_id: UserId, tenant_id: TenantId) -> AppResult<AssistantState>;
    async fn update_step(&self, user_id: UserId, step: AssistantStep) -> AppResult<()>;
}

#[derive(Clone)]
pub struct AssistantStateRepository {
    pool: PgPool,
}

impl AssistantStateRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AssistantStateRepositoryTrait for AssistantStateRepository {
    async fn get_or_create(&self, user_id: UserId, tenant_id: TenantId) -> AppResult<AssistantState> {
        // Попытка получить существующий state
        let row = sqlx::query(
            "SELECT user_id, tenant_id, current_step, updated_at 
             FROM assistant_states 
             WHERE user_id = $1"
        )
        .bind(user_id.0)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let step_str: String = row.get("current_step");
            let step: AssistantStep = serde_json::from_str(&format!("\"{}\"", step_str))
                .unwrap_or(AssistantStep::Start);

            Ok(AssistantState {
                user_id,
                tenant_id,
                current_step: step,
                updated_at: row.get("updated_at"),
            })
        } else {
            // Создаём новый state
            sqlx::query(
                "INSERT INTO assistant_states (user_id, tenant_id, current_step) 
                 VALUES ($1, $2, $3)"
            )
            .bind(user_id.0)
            .bind(tenant_id.0)
            .bind("Start")
            .execute(&self.pool)
            .await?;

            Ok(AssistantState::new(user_id, tenant_id))
        }
    }

    async fn update_step(&self, user_id: UserId, step: AssistantStep) -> AppResult<()> {
        let step_str = format!("{:?}", step);
        
        sqlx::query(
            "UPDATE assistant_states 
             SET current_step = $1 
             WHERE user_id = $2"
        )
        .bind(step_str)
        .bind(user_id.0)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

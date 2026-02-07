use crate::domain::assistant::{
    step::AssistantStep,
    command::AssistantCommand,
    response::AssistantResponse,
    rules::next_step,
};
use crate::infrastructure::persistence::{
    AssistantStateRepository, AssistantStateRepositoryTrait,
    UserRepository, UserRepositoryTrait,
};
use crate::shared::{result::AppResult, types::{UserId, TenantId}, Language};

#[derive(Clone)]
pub struct AssistantService {
    state_repo: AssistantStateRepository,
    user_repo: UserRepository,
}

impl AssistantService {
    pub fn new(state_repo: AssistantStateRepository, user_repo: UserRepository) -> Self {
        Self { state_repo, user_repo }
    }

    /// Получить текущее состояние пользователя
    pub async fn get_state(&self, user_id: UserId, tenant_id: TenantId) -> AppResult<AssistantResponse> {
        let state = self.state_repo.get_or_create(user_id, tenant_id).await?;
        let user = self.user_repo.find_by_id(user_id).await?
            .ok_or_else(|| crate::shared::AppError::not_found("User not found"))?;
        
        Ok(state.current_step.to_response(user.language))
    }

    /// Выполнить команду и получить новое состояние
    pub async fn handle_command(
        &self,
        user_id: UserId,
        tenant_id: TenantId,
        command: AssistantCommand,
    ) -> AppResult<AssistantResponse> {
        // Получаем текущий state
        let state = self.state_repo.get_or_create(user_id, tenant_id).await?;
        let user = self.user_repo.find_by_id(user_id).await?
            .ok_or_else(|| crate::shared::AppError::not_found("User not found"))?;
        
        // Применяем правило перехода
        let next = next_step(state.current_step, &command);
        
        // Сохраняем новый step (только если изменился)
        if next != state.current_step {
            self.state_repo.update_step(user_id, next).await?;
        }
        
        Ok(next.to_response(user.language))
    }
}

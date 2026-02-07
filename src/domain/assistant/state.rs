use crate::shared::types::{UserId, TenantId};
use super::step::AssistantStep;
use time::OffsetDateTime;

#[derive(Debug, Clone)]
pub struct AssistantState {
    pub user_id: UserId,
    pub tenant_id: TenantId,
    pub current_step: AssistantStep,
    pub updated_at: OffsetDateTime,
}

impl AssistantState {
    pub fn new(user_id: UserId, tenant_id: TenantId) -> Self {
        Self {
            user_id,
            tenant_id,
            current_step: AssistantStep::Start,
            updated_at: OffsetDateTime::now_utc(),
        }
    }

    pub fn transition_to(&mut self, step: AssistantStep) {
        self.current_step = step;
        self.updated_at = OffsetDateTime::now_utc();
    }
}

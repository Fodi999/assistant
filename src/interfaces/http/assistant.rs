use axum::{Extension, Json, extract::State};
use serde::Deserialize;

use crate::application::assistant_service::AssistantService;
use crate::domain::assistant::command::AssistantCommand;
use crate::interfaces::http::middleware::AuthUser;
use crate::shared::result::AppResult;

#[derive(Deserialize)]
pub struct CommandRequest {
    pub command: AssistantCommand,
}

/// GET /api/assistant/state - получить текущее состояние пользователя
pub async fn get_state(
    Extension(auth): Extension<AuthUser>,
    State(service): State<AssistantService>,
) -> AppResult<Json<serde_json::Value>> {
    let resp = service.get_state(auth.user_id, auth.tenant_id).await?;
    Ok(Json(serde_json::to_value(resp).unwrap()))
}

/// POST /api/assistant/command - выполнить команду
pub async fn send_command(
    Extension(auth): Extension<AuthUser>,
    State(service): State<AssistantService>,
    Json(req): Json<CommandRequest>,
) -> AppResult<Json<serde_json::Value>> {
    let resp = service.handle_command(auth.user_id, auth.tenant_id, req.command).await?;
    Ok(Json(serde_json::to_value(resp).unwrap()))
}

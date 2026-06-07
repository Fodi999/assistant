//! POST /api/copilot/message    — главный endpoint Copilot-а
//! POST /api/copilot/actions/{id}/confirm — подтверждение write action
//! DELETE /api/copilot/actions/{id}       — отмена action

use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::application::copilot::{
    context::{CopilotContext, CopilotPermission, CopilotScreen},
    CopilotEngine, CopilotResponse,
};
use crate::interfaces::http::middleware::AuthUser;
use crate::shared::AppError;

/// State для Copilot routes.
#[derive(Clone)]
pub struct CopilotState {
    pub engine: Arc<CopilotEngine>,
}

// ── Request / Response DTOs ───────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CopilotMessageRequest {
    /// Текущий экран пользователя.
    #[serde(default)]
    pub screen: CopilotScreen,

    /// Выбранный объект если применимо.
    pub entity_id: Option<Uuid>,

    /// Сообщение пользователя.
    pub message: String,

    /// Явный locale из запроса (приоритет над user.language).
    /// Поддерживается: "ru", "en", "pl", "uk".
    pub locale: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CopilotMessageApiResponse {
    #[serde(flatten)]
    pub response: CopilotResponse,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

/// POST /api/copilot/message
///
/// Пример body:
/// {
///   "screen": "inventory",
///   "entity_id": null,
///   "message": "Что скоро закончится на складе?"
/// }
pub async fn handle_message(
    State(state): State<CopilotState>,
    auth: AuthUser,
    Json(req): Json<CopilotMessageRequest>,
) -> Result<impl IntoResponse, AppError> {
    let message = req.message.trim().to_string();
    if message.is_empty() {
        return Err(AppError::validation("Message cannot be empty"));
    }
    if message.len() > 2000 {
        return Err(AppError::validation("Message too long (max 2000 chars)"));
    }

    // Построить CopilotContext из JWT + request.
    // ai_actions_balance — заполним через usage service внутри engine.
    // permissions — owner получает всё (расширить через roles в следующей итерации).
    // Приоритет language: request.locale > user.language (JWT).
    let resolved_locale = req
        .locale
        .as_deref()
        .and_then(crate::shared::Language::from_code)
        .unwrap_or(auth.language);

    let ctx = CopilotContext {
        user_id: auth.user_id.clone(),
        tenant_id: auth.tenant_id.clone(),
        locale: resolved_locale,
        screen: req.screen,
        selected_entity_id: req.entity_id,
        ai_actions_balance: 999, // engine обновит через billing
        permissions: default_owner_permissions(),
    };

    tracing::info!(
        "🤖 Copilot message: user={} screen={:?} msg_len={}",
        *auth.user_id.as_uuid(),
        ctx.screen,
        message.len(),
    );

    let response = state.engine.handle_message(&ctx, &message).await;

    Ok(Json(CopilotMessageApiResponse { response }))
}

/// POST /api/copilot/actions/{action_id}/confirm
pub async fn confirm_action(
    State(state): State<CopilotState>,
    auth: AuthUser,
    Path(action_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let ctx = CopilotContext {
        user_id: auth.user_id.clone(),
        tenant_id: auth.tenant_id.clone(),
        locale: auth.language,
        screen: CopilotScreen::Dashboard, // screen не важен для confirm
        selected_entity_id: None,
        ai_actions_balance: 0,
        permissions: default_owner_permissions(),
    };

    tracing::info!(
        "✅ Copilot confirm: user={} action_id={}",
        *auth.user_id.as_uuid(),
        action_id,
    );

    let result = state.engine.confirm_action(&ctx, action_id).await?;
    Ok(Json(result))
}

/// DELETE /api/copilot/actions/{action_id}
pub async fn cancel_action(
    State(state): State<CopilotState>,
    auth: AuthUser,
    Path(action_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    tracing::info!(
        "❌ Copilot cancel: user={} action_id={}",
        *auth.user_id.as_uuid(),
        action_id,
    );

    let result = state
        .engine
        .cancel_action(auth.user_id.clone(), action_id)
        .await?;
    Ok(Json(result))
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// По умолчанию owner получает все права.
/// В следующей итерации — загружать из users.role.
fn default_owner_permissions() -> Vec<CopilotPermission> {
    vec![
        CopilotPermission::ReadInventory,
        CopilotPermission::WriteInventory,
        CopilotPermission::ReadDishes,
        CopilotPermission::WriteDishes,
        CopilotPermission::ReadRecipes,
        CopilotPermission::WriteRecipes,
        CopilotPermission::ReadLaboratory,
        CopilotPermission::WriteLaboratory,
        CopilotPermission::ManagePricing,
    ]
}

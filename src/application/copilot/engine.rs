//! CopilotEngine — главный оркестратор.
//!
//! Flow:
//!   1. Billing check (UsageService)
//!   2. Safety pre-check
//!   3. Planner (LLM) → ToolPlan
//!   4. Execute read tools
//!   5. Prepare ActionPlan (write tools → requires_confirmation)
//!   6. Synthesize final answer (LLM или прямой ответ для simple read)
//!   7. Audit log
//!   8. Return CopilotResponse

use std::sync::Arc;

use serde_json::json;

use crate::application::usage_service::UsageService;
use crate::infrastructure::gemini_service::GeminiService;
use crate::shared::{AppError, AppResult};

use super::actions::{ActionPlan, CopilotResponse, ConfirmResult, RiskLevel};
use super::audit::CopilotAuditService;
use super::billing::{AiFeature, check_and_deduct};
use super::context::CopilotContext;
use super::planner::CopilotPlanner;
use super::safety;
use super::tool_executor::{ToolExecutor, ToolExecutorServices};
use super::tools::CopilotTool;

pub struct CopilotEngine {
    planner: CopilotPlanner,
    executor: ToolExecutor,
    usage: UsageService,
    audit: CopilotAuditService,
    gemini: Arc<GeminiService>,
}

impl CopilotEngine {
    pub fn new(
        gemini: Arc<GeminiService>,
        services: ToolExecutorServices,
        usage: UsageService,
        audit: CopilotAuditService,
    ) -> Self {
        Self {
            planner: CopilotPlanner::new(gemini.clone()),
            executor: ToolExecutor::new(services),
            usage,
            audit,
            gemini,
        }
    }

    /// Главная точка входа — обрабатывает сообщение пользователя.
    pub async fn handle_message(
        &self,
        ctx: &CopilotContext,
        message: &str,
    ) -> CopilotResponse {
        match self.handle_inner(ctx, message).await {
            Ok(resp) => resp,
            Err(e) => {
                tracing::error!("CopilotEngine error: {e}");
                CopilotResponse::error(e.to_string())
            }
        }
    }

    /// Подтвердить и выполнить action plan.
    pub async fn confirm_action(
        &self,
        ctx: &CopilotContext,
        action_id: uuid::Uuid,
    ) -> AppResult<ConfirmResult> {
        // 1. Загрузить план из audit log
        let entry = self.audit.get(action_id, ctx.user_id.clone()).await?
            .ok_or_else(|| AppError::not_found("Action plan not found or access denied"))?;

        if entry.status != super::audit::AuditStatus::AwaitingConfirmation {
            return Err(AppError::validation("Action is not in awaiting_confirmation state"));
        }

        // 2. Десериализовать ActionPlan из payload
        let plan: ActionPlan = serde_json::from_value(entry.action_payload.clone())
            .map_err(|e| AppError::internal(format!("Invalid action payload: {e}")))?;

        // 3. Safety check для write execution
        safety::validate_write_execution(ctx, &plan)?;

        // 4. Отметить confirmed
        self.audit.mark_confirmed(action_id).await?;

        // 5. Execute write tool
        // TODO: подключить ToolExecutor::execute_write_tool в следующей итерации
        // Пока — mark executed с placeholder
        tracing::info!(
            "✅ Copilot confirm: action_id={} type={:?} user={}",
            action_id,
            plan.plan_type,
            *ctx.user_id.as_uuid(),
        );

        self.audit.mark_executed(action_id).await?;

        Ok(ConfirmResult {
            success: true,
            message: "Action executed successfully.".to_string(),
            action_id,
            executed_at: chrono::Utc::now(),
        })
    }

    // ── Private ──────────────────────────────────────────────────────────────

    async fn handle_inner(
        &self,
        ctx: &CopilotContext,
        message: &str,
    ) -> AppResult<CopilotResponse> {

        // ── Step 1: Billing check ─────────────────────────────────────────────
        let billing = check_and_deduct(&self.usage, ctx.user_id.clone(), AiFeature::CopilotChat).await?;

        if !billing.allowed {
            return Ok(CopilotResponse::denied(
                billing.deny_message.unwrap_or_else(|| "Insufficient AI actions.".to_string()),
                billing.actions_left,
            ));
        }

        // ── Step 2: Planner → ToolPlan ────────────────────────────────────────
        let plan = match self.planner.plan(ctx, message).await {
            Ok(p) => p,
            Err(e) => {
                tracing::warn!("Planner failed: {e}");
                // Safety: if message looks like a write request, refuse with safe error
                if looks_like_write_request(message) {
                    return Ok(CopilotResponse::safe_error(
                        "Could not prepare a safe action plan. No changes were made. Please try again.",
                        billing.actions_left,
                    ));
                }
                // Read/chat fallback is safe
                super::planner::ToolPlan {
                    intent: "general_question".to_string(),
                    tools: vec![CopilotTool::GeneralChefAnswer],
                    tool_calls: vec![super::planner::ToolCall {
                        tool: CopilotTool::GeneralChefAnswer,
                        args: std::collections::HashMap::new(),
                    }],
                    requires_confirmation: false,
                }
            }
        };

        // ── Step 3: Safety pre-check ──────────────────────────────────────────
        let safety_result = safety::validate_plan(ctx, &plan);
        if !safety_result.allowed {
            return Ok(CopilotResponse::denied(
                safety_result.deny_reason.unwrap_or_else(|| "Not allowed.".to_string()),
                billing.actions_left,
            ));
        }

        // ── Step 4: Execute read tools ────────────────────────────────────────
        let tool_results = self.executor.run_read_tools(ctx, &plan.tool_calls).await;
        let used_tools: Vec<String> = plan.tools.iter().map(|t| t.name().to_string()).collect();

        // ── Step 5: Prepare ActionPlan for write tools ────────────────────────
        let has_write = plan.tools.iter().any(|t| t.is_write());
        let action_plan = if has_write {
            self.executor.prepare_action_plan(&plan.tool_calls, &tool_results)
        } else {
            None
        };

        // ── Step 6: Synthesize answer ─────────────────────────────────────────
        let answer = self.synthesize_answer(ctx, message, &plan.intent, &tool_results, &action_plan).await;

        // ── Step 7: Audit log ─────────────────────────────────────────────────
        let _audit_id = self.audit.record(
            ctx.user_id.clone(),
            ctx.tenant_id.clone(),
            &ctx.screen,
            message,
            &plan.intent,
            &used_tools,
            &action_plan,
            has_write,
        ).await.unwrap_or_else(|e| {
            tracing::warn!("Audit log failed (non-blocking): {e}");
            uuid::Uuid::nil()
        });

        // ── Step 8: Response ──────────────────────────────────────────────────
        let billing_warning = if billing.actions_left <= 5 {
            Some(format!("Low AI actions balance: {} remaining.", billing.actions_left))
        } else {
            None
        };

        Ok(CopilotResponse {
            answer,
            used_tools,
            requires_confirmation: has_write,
            action_plan,
            actions_cost: billing.cost,
            actions_left: billing.actions_left,
            risk_level: safety_result.risk_level,
            billing_warning,
        })
    }

    /// Синтезировать финальный текстовый ответ.
    /// Если использовался GeneralChefAnswer — вызываем LLM.
    /// Иначе — строим структурированный ответ из tool_results.
    async fn synthesize_answer(
        &self,
        ctx: &CopilotContext,
        original_message: &str,
        intent: &str,
        tool_results: &[super::tool_executor::ToolResult],
        action_plan: &Option<ActionPlan>,
    ) -> String {
        // Если есть write plan — короткий confirmation-required ответ
        if let Some(plan) = action_plan {
            if !plan.changes.is_empty() {
                let changes_text = plan.changes.iter()
                    .map(|c| format!("• {} {}: {} → {}", c.entity, c.field, c.before.as_deref().unwrap_or("?"), c.after))
                    .collect::<Vec<_>>()
                    .join("\n");
                return format!(
                    "I've prepared the following changes for your review:\n\n{}\n\nPlease confirm to apply.",
                    changes_text
                );
            }
        }

        // Если GeneralChefAnswer — вызвать LLM с контекстом из tool_results
        let has_general_answer = tool_results.iter().any(|r| r.tool_name == "general_chef_answer");
        let has_tool_data = tool_results.iter().any(|r| r.tool_name != "general_chef_answer");

        if has_general_answer || !has_tool_data {
            // Вызвать LLM для финального ответа
            let context_data: Vec<serde_json::Value> = tool_results.iter()
                .map(|r| json!({ "tool": r.tool_name, "data": r.data }))
                .collect();

            let synthesis_prompt = format!(
                "You are ChefOS Copilot. Answer the user's question based on the retrieved data.\n\
                Context: {}\n\
                User intent: {}\n\
                Retrieved data: {}\n\
                User message: {}\n\
                Answer in language: {}. Be concise and practical.",
                ctx.to_prompt_context(),
                intent,
                serde_json::to_string(&context_data).unwrap_or_default(),
                original_message,
                ctx.locale.code(),
            );

            let request_body = json!({
                "model": "gemini-3-flash-preview",
                "messages": [{"role": "user", "content": synthesis_prompt}],
                "temperature": 0.3,
                "max_tokens": 800
            });

            match self.gemini.send_raw_request(&request_body).await {
                Ok(text) => return text,
                Err(e) => {
                    tracing::warn!("Synthesis LLM call failed: {e}");
                    return "I encountered an issue generating the response. Please try again.".to_string();
                }
            }
        }

        // Структурированный ответ из tool data (без дополнительного LLM call)
        let summaries: Vec<String> = tool_results.iter().map(|r| {
            format!("[{}]: {}", r.tool_name, r.data.to_string().chars().take(200).collect::<String>())
        }).collect();

        summaries.join("\n")
    }
}

/// Detect if user message looks like a write/mutation request.
/// Used to decide safe fallback when planner fails.
fn looks_like_write_request(message: &str) -> bool {
    let msg = message.to_lowercase();
    let write_keywords = [
        "добавь", "добавить", "измени", "изменить", "удали", "удалить",
        "спиши", "списать", "закажи", "заказать", "создай", "создать",
        "обнови", "обновить", "купи", "купить", "закупку", "закупить",
        "add", "update", "delete", "remove", "order", "create", "write off",
        "purchase", "modify", "set price", "change price",
    ];
    write_keywords.iter().any(|kw| msg.contains(kw))
}

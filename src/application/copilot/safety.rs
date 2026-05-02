//! Safety Layer — проверяет каждый план перед выполнением.
//!
//! Запрещено:
//!  - AI напрямую пишет SQL
//!  - AI меняет баланс actions
//!  - AI удаляет данные без подтверждения
//!  - AI отправляет реальные заказы без подтверждения
//!  - AI меняет цены без preview
//!
//! Разрешено:
//!  - AI готовит план (read + draft)
//!  - AI предлагает изменения (preview)
//!  - AI анализирует данные
//!  - AI вызывает безопасные backend tools

use crate::shared::AppError;

use super::actions::{ActionPlan, ActionPlanType, RiskLevel};
use super::context::{CopilotContext, CopilotPermission};
use super::planner::ToolPlan;
use super::tools::{CopilotTool, ToolKind};

/// Результат проверки безопасности.
#[derive(Debug)]
pub struct SafetyCheckResult {
    pub allowed: bool,
    pub risk_level: RiskLevel,
    pub deny_reason: Option<String>,
}

impl SafetyCheckResult {
    pub fn ok(risk: RiskLevel) -> Self {
        SafetyCheckResult { allowed: true, risk_level: risk, deny_reason: None }
    }
    pub fn deny(reason: &str) -> Self {
        SafetyCheckResult {
            allowed: false,
            risk_level: RiskLevel::Critical,
            deny_reason: Some(reason.to_string()),
        }
    }
}

/// Проверить ToolPlan перед выполнением read tools и построением ActionPlan.
pub fn validate_plan(ctx: &CopilotContext, plan: &ToolPlan) -> SafetyCheckResult {
    // 1. Проверить достаточно ли actions
    if ctx.ai_actions_balance <= 0 && plan.requires_ai_tools() {
        return SafetyCheckResult::deny("Insufficient AI actions balance. Please purchase more actions.");
    }

    // 2. Проверить write permissions
    for tool in &plan.tools {
        if tool.is_write() {
            if let Some(reason) = check_write_permission(ctx, tool) {
                return SafetyCheckResult::deny(&reason);
            }
        }
    }

    // 3. SendPurchaseOrder — всегда Critical, требует owner
    if plan.tools.contains(&CopilotTool::SendPurchaseOrder) && !ctx.is_owner() {
        return SafetyCheckResult::deny("Only account owner can send purchase orders.");
    }

    let risk = assess_risk(&plan.tools);
    SafetyCheckResult::ok(risk)
}

/// Проверить ActionPlan перед выполнением write action (после confirmation).
pub fn validate_write_execution(
    ctx: &CopilotContext,
    plan: &ActionPlan,
) -> Result<(), AppError> {
    // Нельзя выполнять пустые планы без write_tool
    if plan.changes.is_empty()
        && plan.write_tool.is_none()
        && !matches!(plan.plan_type, ActionPlanType::NoWriteAction)
    {
        return Err(AppError::validation("Action plan has no changes to execute."));
    }

    // Проверить write permission для типа операции
    match plan.plan_type {
        ActionPlanType::AddInventoryItems
        | ActionPlanType::UpdateInventoryItems
        | ActionPlanType::AdjustInventoryQuantity
        | ActionPlanType::WriteOffInventory => {
            if !ctx.has_permission(&CopilotPermission::WriteInventory) {
                return Err(AppError::authorization("No permission to modify inventory."));
            }
        }
        ActionPlanType::UpdateDishPrice => {
            if !ctx.has_permission(&CopilotPermission::WriteDishes) {
                return Err(AppError::authorization("No permission to modify dish prices."));
            }
        }
        ActionPlanType::CreatePurchaseDraft => {
            if !ctx.has_permission(&CopilotPermission::WriteInventory) {
                return Err(AppError::authorization("No permission to create purchase drafts."));
            }
        }
        ActionPlanType::SendPurchaseOrder => {
            if !ctx.has_permission(&CopilotPermission::WriteInventory) {
                return Err(AppError::authorization("No permission to send purchase orders."));
            }
        }
        ActionPlanType::GenerateLabRecipe
        | ActionPlanType::GenerateProductReport
        | ActionPlanType::SimulateLabProduct => {
            if !ctx.has_permission(&CopilotPermission::WriteLaboratory) {
                return Err(AppError::authorization("No permission to modify laboratory data."));
            }
        }
        ActionPlanType::CreateRecipe => {
            if !ctx.has_permission(&CopilotPermission::WriteRecipes) {
                return Err(AppError::authorization("No permission to create recipes."));
            }
        }
        ActionPlanType::NoWriteAction => {}
    }

    Ok(())
}

fn check_write_permission(ctx: &CopilotContext, tool: &CopilotTool) -> Option<String> {
    match tool {
        CopilotTool::PrepareInventoryUpdate | CopilotTool::WriteOffInventory => {
            if !ctx.has_permission(&CopilotPermission::WriteInventory) {
                return Some("No permission to modify inventory.".to_string());
            }
        }
        CopilotTool::PreparePurchaseDraft => {
            if !ctx.has_permission(&CopilotPermission::WriteInventory) {
                return Some("No permission to create purchase drafts.".to_string());
            }
        }
        CopilotTool::UpdateDishPrice => {
            if !ctx.has_permission(&CopilotPermission::WriteDishes) {
                return Some("No permission to modify dishes.".to_string());
            }
        }
        CopilotTool::CreateRecipe => {
            if !ctx.has_permission(&CopilotPermission::WriteRecipes) {
                return Some("No permission to create recipes.".to_string());
            }
        }
        CopilotTool::GenerateLabRecipe
        | CopilotTool::SimulateLabProduct
        | CopilotTool::GenerateProductReport
        | CopilotTool::Generate3DFoodModel => {
            if !ctx.has_permission(&CopilotPermission::WriteLaboratory) {
                return Some("No permission to modify laboratory.".to_string());
            }
        }
        _ => {}
    }
    None
}

fn assess_risk(tools: &[CopilotTool]) -> RiskLevel {
    let has_write = tools.iter().any(|t| t.kind() == ToolKind::Write);
    if !has_write {
        return RiskLevel::Low;
    }
    if tools.contains(&CopilotTool::SendPurchaseOrder) {
        return RiskLevel::Critical;
    }
    if tools.contains(&CopilotTool::WriteOffInventory) {
        return RiskLevel::High;
    }
    if tools.contains(&CopilotTool::UpdateDishPrice)
        || tools.contains(&CopilotTool::PrepareInventoryUpdate) {
        return RiskLevel::Medium;
    }
    RiskLevel::Low
}

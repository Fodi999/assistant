//! ActionPlan — структуры для preview изменений + CopilotResponse.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::tools::CopilotTool;

/// Одно конкретное изменение в данных (для preview пользователю).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionChange {
    /// Что меняем: "Salmon", "Rice", "Dish: Caesar Salad"
    pub entity: String,
    /// Поле которое меняется: "quantity", "price", "status"
    pub field: String,
    /// Текущее значение (строка для display).
    pub before: Option<String>,
    /// Новое значение.
    pub after: String,
    /// Единица измерения если применимо.
    pub unit: Option<String>,
}

/// Тип операции которую Copilot хочет выполнить.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionPlanType {
    AddInventoryItems,
    UpdateInventoryItems,
    AdjustInventoryQuantity,
    WriteOffInventory,
    CreatePurchaseDraft,
    SendPurchaseOrder,
    UpdateDishPrice,
    GenerateLabRecipe,
    GenerateProductReport,
    SimulateLabProduct,
    NoWriteAction,
}

/// Полный план действий — передаётся клиенту для preview.
/// После подтверждения выполняется через /api/copilot/actions/{id}/confirm.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionPlan {
    /// UUID плана — используется при confirmation.
    pub id: Uuid,
    /// Тип операции.
    pub plan_type: ActionPlanType,
    /// Список конкретных изменений для отображения пользователю.
    pub changes: Vec<ActionChange>,
    /// Write tool который будет выполнен после confirmation.
    pub write_tool: Option<CopilotTool>,
    /// Raw payload для write tool (сериализованные аргументы).
    pub payload: serde_json::Value,
}

impl ActionPlan {
    pub fn no_changes() -> Self {
        ActionPlan {
            id: Uuid::new_v4(),
            plan_type: ActionPlanType::NoWriteAction,
            changes: vec![],
            write_tool: None,
            payload: serde_json::Value::Null,
        }
    }
}

/// Уровень риска операции — используется Safety Layer.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

/// Ответ Copilot-а на сообщение пользователя.
#[derive(Debug, Serialize)]
pub struct CopilotResponse {
    /// Основной текстовый ответ для пользователя.
    pub answer: String,
    /// Список использованных tools (для transparency UI).
    pub used_tools: Vec<String>,
    /// Нужно ли подтверждение перед выполнением?
    pub requires_confirmation: bool,
    /// Plan если есть write actions (None если только read).
    pub action_plan: Option<ActionPlan>,
    /// Стоимость этого запроса в AI actions.
    pub actions_cost: i32,
    /// Сколько actions осталось у пользователя.
    pub actions_left: i32,
    /// Уровень риска (для UI индикации).
    pub risk_level: RiskLevel,
    /// Предупреждение если balance низкий.
    pub billing_warning: Option<String>,
}

impl CopilotResponse {
    pub fn denied(message: String, actions_left: i32) -> Self {
        CopilotResponse {
            answer: message,
            used_tools: vec![],
            requires_confirmation: false,
            action_plan: None,
            actions_cost: 0,
            actions_left,
            risk_level: RiskLevel::Low,
            billing_warning: None,
        }
    }

    pub fn error(message: String) -> Self {
        CopilotResponse {
            answer: format!("Error: {}", message),
            used_tools: vec![],
            requires_confirmation: false,
            action_plan: None,
            actions_cost: 0,
            actions_left: 0,
            risk_level: RiskLevel::Low,
            billing_warning: None,
        }
    }

    /// Safe error for write-command planner failures — clearly states no changes were made.
    pub fn safe_error(message: &str, actions_left: i32) -> Self {
        CopilotResponse {
            answer: message.to_string(),
            used_tools: vec![],
            requires_confirmation: false,
            action_plan: None,
            actions_cost: 1,
            actions_left,
            risk_level: RiskLevel::Low,
            billing_warning: None,
        }
    }
}

/// Результат подтверждения action plan.
#[derive(Debug, Serialize)]
pub struct ConfirmResult {
    pub success: bool,
    pub message: String,
    pub action_id: Uuid,
    pub executed_at: chrono::DateTime<chrono::Utc>,
}

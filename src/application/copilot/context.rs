//! CopilotContext — собирает всё что LLM должен знать о пользователе и экране.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::shared::{Language, TenantId, UserId};

/// Текущий экран пользователя в приложении.
/// Planner использует его чтобы понять какие tools и данные релевантны.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CopilotScreen {
    Dashboard,
    Inventory,
    Dishes,
    Recipes,
    Laboratory,
    MenuEngineering,
    Pricing,
    Profile,
    Chat,
}

impl CopilotScreen {
    /// Краткое описание экрана для LLM-промпта.
    pub fn description(&self) -> &'static str {
        match self {
            CopilotScreen::Dashboard => {
                "main dashboard with summary of inventory, dishes, and activity"
            }
            CopilotScreen::Inventory => {
                "stock management: ingredients with quantities, expiry dates, alerts"
            }
            CopilotScreen::Dishes => {
                "dish catalog: recipes with costing, allergens, nutritional info"
            }
            CopilotScreen::Recipes => "AI-generated recipes and user-saved recipes",
            CopilotScreen::Laboratory => {
                "food-tech lab: create and simulate new products and sauces"
            }
            CopilotScreen::MenuEngineering => {
                "menu engineering: dish profitability and pricing matrix"
            }
            CopilotScreen::Pricing => "action bundle purchase page",
            CopilotScreen::Profile => "user profile and preferences",
            CopilotScreen::Chat => "general AI chef chat",
        }
    }

    /// Какие read tools автоматически доступны на этом экране.
    pub fn default_context_tools(&self) -> Vec<&'static str> {
        match self {
            CopilotScreen::Inventory => vec!["get_inventory", "get_expiring_soon"],
            CopilotScreen::Dishes => vec!["get_dishes"],
            CopilotScreen::Recipes => vec!["get_recipes"],
            CopilotScreen::Laboratory => vec!["get_lab_experiment"],
            CopilotScreen::MenuEngineering => vec!["get_dishes"],
            CopilotScreen::Dashboard => vec!["get_inventory", "get_dishes"],
            _ => vec![],
        }
    }
}

impl Default for CopilotScreen {
    fn default() -> Self {
        CopilotScreen::Chat
    }
}

/// Полный контекст вызова Copilot.
/// Передаётся от HTTP handler-а в CopilotEngine::handle_message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopilotContext {
    /// ID пользователя из JWT.
    pub user_id: UserId,

    /// Tenant (ресторан / аккаунт).
    pub tenant_id: TenantId,

    /// Язык интерфейса — LLM отвечает на том же языке.
    pub locale: Language,

    /// Экран где пользователь находится сейчас.
    pub screen: CopilotScreen,

    /// Выбранный объект (блюдо, рецепт, лаб-эксперимент и т.д.).
    pub selected_entity_id: Option<Uuid>,

    /// Текущий баланс AI actions (заполняется из UsageService в engine).
    pub ai_actions_balance: i32,

    /// Права пользователя (owner | staff | viewer).
    pub permissions: Vec<CopilotPermission>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CopilotPermission {
    ReadInventory,
    WriteInventory,
    ReadDishes,
    WriteDishes,
    ReadRecipes,
    WriteRecipes,
    ReadLaboratory,
    WriteLaboratory,
    ManagePricing,
}

impl CopilotContext {
    pub fn has_permission(&self, perm: &CopilotPermission) -> bool {
        self.permissions.contains(perm)
    }

    /// Является ли пользователь owner-ом (может всё).
    pub fn is_owner(&self) -> bool {
        // Owner имеет WriteInventory + WriteDishes как минимум
        self.permissions
            .contains(&CopilotPermission::WriteInventory)
            && self.permissions.contains(&CopilotPermission::WriteDishes)
    }

    /// Краткий снапшот для вставки в LLM-промпт.
    pub fn to_prompt_context(&self) -> String {
        format!(
            "User is on the '{}' screen ({}). AI actions balance: {}. Locale: {}.",
            format!("{:?}", self.screen).to_lowercase(),
            self.screen.description(),
            self.ai_actions_balance,
            self.locale.code(),
        )
    }
}

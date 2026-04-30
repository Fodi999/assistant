//! AiFeature — стоимость каждого AI-действия в units.
//! Интегрируется с UsageService::perform_action через ActionType::AiChat (за основу)
//! и кастомный multiplier.

use serde::{Deserialize, Serialize};

use crate::application::usage_service::UsageService;
use crate::domain::usage::{ActionSource, ActionType};
use crate::shared::{AppError, AppResult, UserId};

/// Все AI-фичи с их стоимостью.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AiFeature {
    /// POST /api/copilot/message — 1 action за вызов планировщика.
    CopilotChat,
    /// Генерация рецепта.
    RecipeGeneration,
    /// Недельный план питания.
    MealPlan,
    /// Предложения блюд из инвентаря.
    CookSuggestions,
    /// Лаб-рецепт.
    LabRecipe,
    /// Симуляция продукта.
    LabSimulation,
    /// Технологический отчёт.
    ProductReport,
    /// 3D-визуализация блюда.
    Vision3D,
    /// Анализ рецепта (insights).
    RecipeInsights,
    /// Паринг ингредиентов.
    FoodPairing,
    /// Общий AI-ответ (AI Brain).
    GeneralAnswer,
}

impl AiFeature {
    /// Стоимость в AI actions.
    pub fn action_cost(&self) -> i32 {
        match self {
            AiFeature::CopilotChat      => 1,
            AiFeature::GeneralAnswer    => 1,
            AiFeature::RecipeInsights   => 1,
            AiFeature::RecipeGeneration => 2,
            AiFeature::CookSuggestions  => 2,
            AiFeature::FoodPairing      => 2,
            AiFeature::LabRecipe        => 3,
            AiFeature::LabSimulation    => 3,
            AiFeature::MealPlan         => 5,
            AiFeature::ProductReport    => 5,
            AiFeature::Vision3D         => 5,
        }
    }

    /// Человекочитаемое название для ошибок и аудит-лога.
    pub fn display_name(&self) -> &'static str {
        match self {
            AiFeature::CopilotChat      => "Copilot Chat",
            AiFeature::GeneralAnswer    => "AI Chef Answer",
            AiFeature::RecipeInsights   => "Recipe Analysis",
            AiFeature::RecipeGeneration => "Recipe Generation",
            AiFeature::CookSuggestions  => "Cook Suggestions",
            AiFeature::FoodPairing      => "Food Pairing",
            AiFeature::LabRecipe        => "Lab Recipe",
            AiFeature::LabSimulation    => "Lab Simulation",
            AiFeature::MealPlan         => "Meal Plan",
            AiFeature::ProductReport    => "Product Report",
            AiFeature::Vision3D         => "3D Food Model",
        }
    }
}

/// Результат billing check — разрешено ли выполнение и из какого источника.
#[derive(Debug)]
pub struct CopilotBillingResult {
    pub allowed: bool,
    pub is_paid: bool,
    pub actions_left: i32,
    pub cost: i32,
    pub deny_message: Option<String>,
}

/// Проверить и списать AI actions для фичи.
/// Возвращает CopilotBillingResult — engine решает продолжать или вернуть ошибку.
pub async fn check_and_deduct(
    usage: &UsageService,
    user_id: UserId,
    feature: AiFeature,
) -> AppResult<CopilotBillingResult> {
    // Все Copilot-вызовы биллятся через ActionType::AiChat (единый токен).
    // Стоимость фичи хранится здесь — UsageService дедуцирует cost_chat units.
    let result = usage.perform_action(user_id, ActionType::AiChat).await?;

    if !result.allowed {
        return Ok(CopilotBillingResult {
            allowed: false,
            is_paid: false,
            actions_left: result.purchased_actions_left,
            cost: feature.action_cost(),
            deny_message: result.message,
        });
    }

    Ok(CopilotBillingResult {
        allowed: true,
        is_paid: result.source == ActionSource::Purchased,
        actions_left: result.purchased_actions_left,
        cost: feature.action_cost(),
        deny_message: None,
    })
}

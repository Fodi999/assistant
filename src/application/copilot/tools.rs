//! CopilotTool — полный список инструментов Copilot-а.
//! Делятся на Read-only (выполняются сразу) и Write (только через confirmation).

use serde::{Deserialize, Serialize};

/// Тип tool — определяет нужна ли confirmation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolKind {
    Read,
    Write,
}

/// Все доступные инструменты Copilot-а.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum CopilotTool {
    // ── Read-only tools (выполняются немедленно) ─────────────────────────────
    /// Загрузить список товаров на складе с количеством и сроками.
    GetInventory,
    /// Товары, у которых срок годности < 3 дней.
    GetExpiringSoon,
    /// Поиск ингредиентов в каталоге по названию.
    SearchIngredients,
    /// Список блюд тенанта с ценами и себестоимостью.
    GetDishes,
    /// Список рецептов пользователя.
    GetRecipes,
    /// Полный рецепт по ID.
    GetRecipeById,
    /// Лаб-эксперимент по ID.
    GetLabExperiment,

    // ── AI Read tools (требуют AI actions, но не confirmation) ───────────────
    /// BOT 4: предложить блюда из текущего инвентаря (Cook Suggestions).
    SuggestCookFromInventory,
    /// BOT 3: сгенерировать план питания (Sous-Chef Planner).
    GenerateMealPlan,
    /// BOT 7: анализ рецепта (Recipe AI Insights).
    AnalyzeRecipe,
    /// BOT 2: общий AI-ответ (AI Brain).
    GeneralChefAnswer,
    /// BOT 8: паринг ингредиентов (Food Pairing).
    GenerateFoodPairing,

    // ── Write tools (требуют confirmation перед выполнением) ─────────────────
    /// Добавить / обновить товары на складе.
    PrepareInventoryUpdate,
    /// Создать черновик закупки.
    PreparePurchaseDraft,
    /// Изменить цену блюда.
    UpdateDishPrice,
    /// Списать товары со склада.
    WriteOffInventory,
    /// Отправить заказ поставщику (требует manual confirm).
    SendPurchaseOrder,

    // ── AI Write tools (AI генерирует + confirmation) ─────────────────────────
    /// BOT 5: сгенерировать рецепт в лаборатории.
    GenerateLabRecipe,
    /// BOT 6: 3D-модель блюда по фото.
    Generate3DFoodModel,
    /// Симуляция продукта в лаборатории.
    SimulateLabProduct,
    /// Технологический отчёт продукта.
    GenerateProductReport,
}

impl CopilotTool {
    /// Read или Write?
    pub fn kind(&self) -> ToolKind {
        match self {
            CopilotTool::GetInventory           => ToolKind::Read,
            CopilotTool::GetExpiringSoon         => ToolKind::Read,
            CopilotTool::SearchIngredients       => ToolKind::Read,
            CopilotTool::GetDishes               => ToolKind::Read,
            CopilotTool::GetRecipes              => ToolKind::Read,
            CopilotTool::GetRecipeById           => ToolKind::Read,
            CopilotTool::GetLabExperiment        => ToolKind::Read,
            CopilotTool::SuggestCookFromInventory => ToolKind::Read,
            CopilotTool::GenerateMealPlan        => ToolKind::Read,
            CopilotTool::AnalyzeRecipe           => ToolKind::Read,
            CopilotTool::GeneralChefAnswer       => ToolKind::Read,
            CopilotTool::GenerateFoodPairing     => ToolKind::Read,
            // Write
            CopilotTool::PrepareInventoryUpdate  => ToolKind::Write,
            CopilotTool::PreparePurchaseDraft    => ToolKind::Write,
            CopilotTool::UpdateDishPrice         => ToolKind::Write,
            CopilotTool::WriteOffInventory       => ToolKind::Write,
            CopilotTool::SendPurchaseOrder       => ToolKind::Write,
            CopilotTool::GenerateLabRecipe       => ToolKind::Write,
            CopilotTool::Generate3DFoodModel     => ToolKind::Write,
            CopilotTool::SimulateLabProduct      => ToolKind::Write,
            CopilotTool::GenerateProductReport   => ToolKind::Write,
        }
    }

    pub fn is_write(&self) -> bool {
        self.kind() == ToolKind::Write
    }

    pub fn is_read(&self) -> bool {
        self.kind() == ToolKind::Read
    }

    /// Человекочитаемое имя для аудит-лога и промпта.
    pub fn name(&self) -> &'static str {
        match self {
            CopilotTool::GetInventory            => "get_inventory",
            CopilotTool::GetExpiringSoon         => "get_expiring_soon",
            CopilotTool::SearchIngredients       => "search_ingredients",
            CopilotTool::GetDishes               => "get_dishes",
            CopilotTool::GetRecipes              => "get_recipes",
            CopilotTool::GetRecipeById           => "get_recipe_by_id",
            CopilotTool::GetLabExperiment        => "get_lab_experiment",
            CopilotTool::SuggestCookFromInventory => "suggest_cook_from_inventory",
            CopilotTool::GenerateMealPlan        => "generate_meal_plan",
            CopilotTool::AnalyzeRecipe           => "analyze_recipe",
            CopilotTool::GeneralChefAnswer       => "general_chef_answer",
            CopilotTool::GenerateFoodPairing     => "generate_food_pairing",
            CopilotTool::PrepareInventoryUpdate  => "prepare_inventory_update",
            CopilotTool::PreparePurchaseDraft    => "prepare_purchase_draft",
            CopilotTool::UpdateDishPrice         => "update_dish_price",
            CopilotTool::WriteOffInventory       => "write_off_inventory",
            CopilotTool::SendPurchaseOrder       => "send_purchase_order",
            CopilotTool::GenerateLabRecipe       => "generate_lab_recipe",
            CopilotTool::Generate3DFoodModel     => "generate_3d_food_model",
            CopilotTool::SimulateLabProduct      => "simulate_lab_product",
            CopilotTool::GenerateProductReport   => "generate_product_report",
        }
    }

    /// Описание для LLM-системного промпта (tool calling schema).
    pub fn description(&self) -> &'static str {
        match self {
            CopilotTool::GetInventory            => "Get current stock levels: all ingredients with quantities and expiry dates",
            CopilotTool::GetExpiringSoon         => "Get ingredients expiring within 3 days that need urgent use or write-off",
            CopilotTool::SearchIngredients       => "Search ingredient catalog by name to find IDs and details",
            CopilotTool::GetDishes               => "Get tenant's dish list with pricing and cost breakdown",
            CopilotTool::GetRecipes              => "Get user's saved recipes",
            CopilotTool::GetRecipeById           => "Get full recipe details by ID",
            CopilotTool::GetLabExperiment        => "Get lab experiment details by ID",
            CopilotTool::SuggestCookFromInventory => "AI: suggest dishes that can be made from current inventory",
            CopilotTool::GenerateMealPlan        => "AI: generate personalized weekly meal plan based on goals and inventory",
            CopilotTool::AnalyzeRecipe           => "AI: analyze recipe for nutrition, technique tips, and substitutions",
            CopilotTool::GeneralChefAnswer       => "AI: answer any culinary question using AI Brain",
            CopilotTool::GenerateFoodPairing     => "AI: generate ingredient pairing suggestions for a given product",
            CopilotTool::PrepareInventoryUpdate  => "WRITE: prepare a stock update draft (add/adjust quantities) — requires confirmation",
            CopilotTool::PreparePurchaseDraft    => "WRITE: prepare a purchase order draft for missing or low-stock items — requires confirmation",
            CopilotTool::UpdateDishPrice         => "WRITE: update dish selling price — requires confirmation",
            CopilotTool::WriteOffInventory       => "WRITE: write off expired or spoiled inventory items — requires confirmation",
            CopilotTool::SendPurchaseOrder       => "WRITE: send purchase order to supplier — requires manual confirmation",
            CopilotTool::GenerateLabRecipe       => "AI+WRITE: generate new lab recipe from ingredients — requires confirmation",
            CopilotTool::Generate3DFoodModel     => "AI+WRITE: generate 3D food model from photo — requires confirmation",
            CopilotTool::SimulateLabProduct      => "AI+WRITE: simulate lab product shelf life and nutrition — requires confirmation",
            CopilotTool::GenerateProductReport   => "AI+WRITE: generate technology report for lab product — requires confirmation",
        }
    }

    /// Построить JSON-схему всех tools для LLM Function Calling промпта.
    pub fn tool_catalog_prompt() -> String {
        let tools = vec![
            CopilotTool::GetInventory,
            CopilotTool::GetExpiringSoon,
            CopilotTool::SearchIngredients,
            CopilotTool::GetDishes,
            CopilotTool::GetRecipes,
            CopilotTool::SuggestCookFromInventory,
            CopilotTool::GenerateMealPlan,
            CopilotTool::AnalyzeRecipe,
            CopilotTool::GeneralChefAnswer,
            CopilotTool::PrepareInventoryUpdate,
            CopilotTool::PreparePurchaseDraft,
            CopilotTool::UpdateDishPrice,
            CopilotTool::WriteOffInventory,
            CopilotTool::GenerateLabRecipe,
        ];

        tools.iter().enumerate().map(|(i, t)| {
            format!("{}. {} [{}] — {}", i + 1, t.name(), if t.is_write() { "WRITE" } else { "READ" }, t.description())
        }).collect::<Vec<_>>().join("\n")
    }
}

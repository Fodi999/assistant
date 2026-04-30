//! ToolExecutor — исполняет конкретные tools через существующие backend-сервисы.
//!
//! Read tools выполняются немедленно.
//! Write tools только подготавливают ActionPlan (не выполняют).
//! Фактическое выполнение write actions — через confirm endpoint.

use std::sync::Arc;

use serde_json::json;
use uuid::Uuid;

use crate::application::cook_suggestions::CookSuggestionService;
use crate::application::dish::DishService;
use crate::application::inventory::InventoryService;
use crate::application::recipe_v2_service::RecipeV2Service;
use crate::application::sous_chef::{SousChefPlannerService, PlanRequest};
use crate::shared::PaginationParams;
use crate::shared::{AppError, AppResult};

use super::actions::{ActionChange, ActionPlan, ActionPlanType, RiskLevel};
use super::context::CopilotContext;
use super::planner::ToolCall;
use super::tools::CopilotTool;

/// Результат выполнения одного read tool.
#[derive(Debug)]
pub struct ToolResult {
    pub tool_name: String,
    /// Данные для включения в LLM response-synthesis или прямого ответа.
    pub data: serde_json::Value,
}

/// Сервисы доступные ToolExecutor-у.
pub struct ToolExecutorServices {
    pub inventory: Arc<InventoryService>,
    pub dishes: Arc<DishService>,
    pub recipes: Arc<RecipeV2Service>,
    pub cook_suggestions: Arc<CookSuggestionService>,
    pub sous_chef: Arc<SousChefPlannerService>,
}

pub struct ToolExecutor {
    services: ToolExecutorServices,
}

impl ToolExecutor {
    pub fn new(services: ToolExecutorServices) -> Self {
        Self { services }
    }

    /// Выполнить все READ tools из плана.
    /// Write tools пропускаются — они попадут в ActionPlan.
    pub async fn run_read_tools(
        &self,
        ctx: &CopilotContext,
        tool_calls: &[ToolCall],
    ) -> Vec<ToolResult> {
        let mut results = Vec::new();

        for call in tool_calls {
            if call.tool.is_write() {
                continue; // write tools выполняются только после confirmation
            }
            match self.execute_read_tool(ctx, &call.tool, &call.args).await {
                Ok(result) => results.push(result),
                Err(e) => {
                    tracing::warn!("Tool {} failed: {e}", call.tool.name());
                    results.push(ToolResult {
                        tool_name: call.tool.name().to_string(),
                        data: json!({ "error": e.to_string() }),
                    });
                }
            }
        }

        results
    }

    /// Подготовить ActionPlan для первого write tool в списке.
    /// (По одному write tool за раз — пользователь должен подтвердить каждый).
    pub fn prepare_action_plan(
        &self,
        tool_calls: &[ToolCall],
        tool_results: &[ToolResult],
    ) -> Option<ActionPlan> {
        let write_call = tool_calls.iter().find(|c| c.tool.is_write())?;

        let changes = build_preview_changes(&write_call.tool, &write_call.args, tool_results);
        let plan_type = tool_to_plan_type(&write_call.tool);

        Some(ActionPlan {
            id: Uuid::new_v4(),
            plan_type,
            changes,
            write_tool: Some(write_call.tool.clone()),
            payload: serde_json::to_value(&write_call.args).unwrap_or(json!({})),
        })
    }

    // ── Private: execute individual read tools ──────────────────────────────

    async fn execute_read_tool(
        &self,
        ctx: &CopilotContext,
        tool: &CopilotTool,
        args: &std::collections::HashMap<String, serde_json::Value>,
    ) -> AppResult<ToolResult> {
        let name = tool.name().to_string();

        match tool {
            CopilotTool::GetInventory => {
                let items = self.services.inventory
                    .list_products_with_details(
                        ctx.user_id.clone(),
                        ctx.tenant_id.clone(),
                        crate::shared::Language::En,
                    )
                    .await?;
                let summary: Vec<serde_json::Value> = items.iter().map(|i| json!({
                    "name": i.product.name,
                    "quantity": i.quantity,
                    "unit": i.product.base_unit,
                    "expires_at": i.expires_at.to_string(),
                    "severity": format!("{:?}", i.severity),
                })).collect();
                Ok(ToolResult { tool_name: name, data: json!({ "inventory": summary }) })
            }

            CopilotTool::GetExpiringSoon => {
                let items = self.services.inventory
                    .get_expiring_products(ctx.user_id.clone(), ctx.tenant_id.clone(), 3)
                    .await?;
                let summary: Vec<serde_json::Value> = items.iter().map(|b| json!({
                    "id": b.id,
                    "expires_at": b.expires_at.to_string(),
                    "quantity": b.quantity,
                })).collect();
                Ok(ToolResult { tool_name: name, data: json!({ "expiring": summary }) })
            }

            CopilotTool::SearchIngredients => {
                // Поиск по названию через каталог: пока возвращаем заглушку.
                // TODO: подключить CatalogService.search_by_name
                let query = args.get("query")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                Ok(ToolResult {
                    tool_name: name,
                    data: json!({ "search_query": query, "note": "catalog search not yet wired" }),
                })
            }

            CopilotTool::GetDishes => {
                let pagination = PaginationParams { page: Some(1), per_page: Some(50) };
                let (dishes, total) = self.services.dishes
                    .list_dishes(ctx.tenant_id.clone(), true, &pagination)
                    .await?;
                let summary: Vec<serde_json::Value> = dishes.iter().map(|d| json!({
                    "id": d.id,
                    "name": d.name,
                    "price_cents": d.selling_price.as_cents(),
                })).collect();
                Ok(ToolResult { tool_name: name, data: json!({ "dishes": summary, "total": total }) })
            }

            CopilotTool::GetRecipes => {
                let recipes = self.services.recipes
                    .list_user_recipes(ctx.user_id.clone(), ctx.tenant_id.clone(), ctx.locale)
                    .await?;
                let summary: Vec<serde_json::Value> = recipes.iter().map(|r| json!({
                    "id": r.id,
                    "name": r.name,
                })).collect();
                Ok(ToolResult { tool_name: name, data: json!({ "recipes": summary }) })
            }

            CopilotTool::GetRecipeById => {
                let id_str = args.get("id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| AppError::validation("missing id argument for get_recipe_by_id"))?;
                let recipe_id = id_str.parse::<Uuid>()
                    .map_err(|_| AppError::validation("invalid UUID for recipe id"))?;
                let recipe = self.services.recipes
                    .get_recipe(recipe_id.into(), ctx.tenant_id.clone(), ctx.locale)
                    .await?;
                Ok(ToolResult {
                    tool_name: name,
                    data: serde_json::to_value(recipe).unwrap_or(json!({})),
                })
            }

            CopilotTool::SuggestCookFromInventory => {
                let suggestions = self.services.cook_suggestions
                    .suggest(ctx.user_id.clone(), ctx.tenant_id.clone(), ctx.locale)
                    .await?;
                Ok(ToolResult {
                    tool_name: name,
                    data: serde_json::to_value(&suggestions).unwrap_or(json!({})),
                })
            }

            CopilotTool::GenerateMealPlan => {
                let query = args.get("query")
                    .and_then(|v| v.as_str())
                    .unwrap_or("balanced healthy meal plan");
                let plan = self.services.sous_chef
                    .generate_plan(PlanRequest {
                        query: query.to_string(),
                        lang: Some(ctx.locale.code().to_string()),
                    })
                    .await?;
                Ok(ToolResult {
                    tool_name: name,
                    data: serde_json::to_value(&plan).unwrap_or(json!({})),
                })
            }

            // AI Brain / GeneralChefAnswer — данные не нужны,
            // engine сам вызовет AI Brain для финального ответа
            CopilotTool::GeneralChefAnswer => {
                Ok(ToolResult {
                    tool_name: name,
                    data: json!({ "mode": "general_chef_answer" }),
                })
            }

            // GetLabExperiment — TODO: подключить LaboratoryService
            CopilotTool::GetLabExperiment => {
                let id = args.get("id").and_then(|v| v.as_str()).unwrap_or("unknown");
                Ok(ToolResult {
                    tool_name: name,
                    data: json!({ "lab_experiment_id": id, "note": "lab connector pending" }),
                })
            }

            _ => {
                // Остальные read tools — заглушка (будут подключены в следующих итерациях)
                Ok(ToolResult {
                    tool_name: name,
                    data: json!({ "note": format!("tool {} not yet implemented", tool.name()) }),
                })
            }
        }
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn tool_to_plan_type(tool: &CopilotTool) -> ActionPlanType {
    match tool {
        CopilotTool::PrepareInventoryUpdate  => ActionPlanType::AddInventoryItems,
        CopilotTool::WriteOffInventory       => ActionPlanType::WriteOffInventory,
        CopilotTool::PreparePurchaseDraft    => ActionPlanType::CreatePurchaseDraft,
        CopilotTool::UpdateDishPrice         => ActionPlanType::UpdateDishPrice,
        CopilotTool::GenerateLabRecipe       => ActionPlanType::GenerateLabRecipe,
        CopilotTool::GenerateProductReport   => ActionPlanType::GenerateProductReport,
        CopilotTool::SimulateLabProduct      => ActionPlanType::SimulateLabProduct,
        _                                    => ActionPlanType::NoWriteAction,
    }
}

/// Построить preview изменений для write tool.
fn build_preview_changes(
    tool: &CopilotTool,
    args: &std::collections::HashMap<String, serde_json::Value>,
    _tool_results: &[ToolResult],
) -> Vec<ActionChange> {
    match tool {
        CopilotTool::PrepareInventoryUpdate | CopilotTool::WriteOffInventory => {
            if let Some(items) = args.get("items").and_then(|v| v.as_array()) {
                return items.iter().filter_map(|item| {
                    let ingredient = item.get("ingredient")?.as_str()?;
                    let quantity = item.get("quantity")?.as_f64()?;
                    let unit = item.get("unit").and_then(|u| u.as_str()).unwrap_or("kg");
                    Some(ActionChange {
                        entity: ingredient.to_string(),
                        field: "quantity".to_string(),
                        before: None, // будет заполнено из inventory данных
                        after: format!("{} {}", quantity, unit),
                        unit: Some(unit.to_string()),
                    })
                }).collect();
            }
            vec![]
        }
        CopilotTool::UpdateDishPrice => {
            if let (Some(dish), Some(price)) = (
                args.get("dish_name").and_then(|v| v.as_str()),
                args.get("new_price_cents").and_then(|v| v.as_i64()),
            ) {
                return vec![ActionChange {
                    entity: format!("Dish: {}", dish),
                    field: "selling_price".to_string(),
                    before: None,
                    after: format!("€{:.2}", price as f64 / 100.0),
                    unit: None,
                }];
            }
            vec![]
        }
        _ => vec![],
    }
}

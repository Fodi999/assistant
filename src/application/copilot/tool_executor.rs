//! ToolExecutor — исполняет конкретные tools через существующие backend-сервисы.
//!
//! Read tools выполняются немедленно.
//! Write tools только подготавливают ActionPlan (не выполняют).
//! Фактическое выполнение write actions — через confirm endpoint.

use std::sync::Arc;

use serde_json::json;
use uuid::Uuid;

use crate::application::catalog::CatalogService;
use crate::application::cook_suggestions::CookSuggestionService;
use crate::application::dish::DishService;
use crate::application::inventory::InventoryService;
use crate::application::recipe_v2_service::RecipeV2Service;
use crate::application::sous_chef::{SousChefPlannerService, PlanRequest};
use crate::shared::Language;
use crate::shared::PaginationParams;
use crate::shared::{AppError, AppResult, TenantId, UserId};

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
    pub catalog: Arc<CatalogService>,
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

    /// Выполнить реальный write action после confirmation.
    /// Поддерживает: PrepareInventoryUpdate (add/update quantity).
    pub async fn execute_write_tool(
        &self,
        user_id: UserId,
        tenant_id: TenantId,
        plan: &super::actions::ActionPlan,
    ) -> AppResult<String> {
        let tool = plan.write_tool.as_ref()
            .ok_or_else(|| AppError::internal("Action plan has no write_tool"))?;

        match tool {
            CopilotTool::PrepareInventoryUpdate => {
                self.execute_inventory_add(user_id, tenant_id, &plan.payload).await
            }
            CopilotTool::WriteOffInventory => {
                self.execute_inventory_writeoff(tenant_id, &plan.payload).await
            }
            _ => {
                tracing::warn!("execute_write_tool: tool {:?} not yet implemented", tool);
                Ok(format!("Action {} logged (execution pending implementation).", tool.name()))
            }
        }
    }

    /// Извлечь {name, quantity, unit, reason?} из payload (поддерживает items[] и flat).
    fn extract_item(payload: &serde_json::Value) -> AppResult<(String, f64, String, Option<String>)> {
        let item = if let Some(items) = payload.get("items").and_then(|v| v.as_array()) {
            items.first().cloned().unwrap_or(serde_json::Value::Null)
        } else {
            payload.clone()
        };

        let name = item.get("ingredient_name")
            .or_else(|| item.get("ingredient"))
            .or_else(|| item.get("item_name"))
            .or_else(|| item.get("product_name"))
            .or_else(|| item.get("name"))
            .or_else(|| item.get("item"))
            .or_else(|| item.get("product"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| AppError::validation("ingredient_name is required in action payload"))?
            .to_string();

        let quantity = item.get("quantity")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| AppError::validation("quantity is required in action payload"))?;

        let unit = item.get("unit").and_then(|v| v.as_str()).unwrap_or("kg").to_string();

        // reason может быть на уровне item или на верхнем уровне payload
        let reason = item.get("reason")
            .or_else(|| payload.get("reason"))
            .and_then(|v| v.as_str())
            .map(String::from);

        Ok((name, quantity, unit, reason))
    }

    /// Найти catalog_ingredient_id по имени (EN → RU fallback).
    async fn find_catalog_id(&self, name: &str) -> AppResult<crate::domain::catalog::CatalogIngredientId> {
        let candidates = self.services.catalog
            .search_ingredients(name, Language::En, 5)
            .await
            .unwrap_or_default();

        if let Some(ing) = candidates.into_iter().next() {
            return Ok(ing.id);
        }

        let candidates_ru = self.services.catalog
            .search_ingredients(name, Language::Ru, 5)
            .await
            .unwrap_or_default();

        candidates_ru.into_iter().next()
            .map(|i| i.id)
            .ok_or_else(|| AppError::not_found(
                format!("Ingredient '{}' not found in catalog. Please use the exact catalog name.", name)
            ))
    }

    /// Добавить/обновить позицию в инвентаре.
    /// Ищет ингредиент в каталоге по имени, создаёт batch.
    async fn execute_inventory_add(
        &self,
        user_id: UserId,
        tenant_id: TenantId,
        payload: &serde_json::Value,
    ) -> AppResult<String> {
        let (name, quantity, unit, _reason) = Self::extract_item(payload)?;
        let catalog_id = self.find_catalog_id(&name).await?;

        // Добавить batch с разумными defaults
        let now = time::OffsetDateTime::now_utc();
        let expires_at = now + time::Duration::days(30);

        self.services.inventory
            .add_product(
                user_id,
                tenant_id,
                catalog_id,
                0, // price_per_unit_cents = 0 (неизвестно)
                quantity,
                now,
                expires_at,
            )
            .await?;

        tracing::info!("✅ Copilot write: added {} {} {} to inventory", quantity, unit, name);
        Ok(format!("Added {} {} of {} to inventory.", quantity, unit, name))
    }

    /// Списать со склада (FIFO).
    /// Поддерживает причины: expired, used_in_production, waste, correction, manual.
    async fn execute_inventory_writeoff(
        &self,
        tenant_id: TenantId,
        payload: &serde_json::Value,
    ) -> AppResult<String> {
        let (name, quantity, unit, reason_opt) = Self::extract_item(payload)?;
        let catalog_id = self.find_catalog_id(&name).await?;

        // Нормализовать reason
        let reason = reason_opt
            .as_deref()
            .map(normalize_writeoff_reason)
            .unwrap_or("manual")
            .to_string();

        self.services.inventory
            .deduct_fifo(
                tenant_id,
                catalog_id,
                quantity,
                None,
                Some(format!("copilot_writeoff:{}", reason)),
                Some(format!("Copilot write-off: {} ({})", name, reason)),
            )
            .await?;

        tracing::info!("✅ Copilot write-off: {} {} {} reason={}", quantity, unit, name, reason);
        Ok(format!("Wrote off {} {} of {} (reason: {}).", quantity, unit, name, reason))
    }
}

/// Нормализовать причину списания к одному из допустимых значений.
fn normalize_writeoff_reason(input: &str) -> &'static str {
    let s = input.to_lowercase();
    if s.contains("expir") || s.contains("просроч") || s.contains("истёк") || s.contains("истек") {
        "expired"
    } else if s.contains("product") || s.contains("производ") || s.contains("приготовл") {
        "used_in_production"
    } else if s.contains("waste") || s.contains("испорч") || s.contains("отход") {
        "waste"
    } else if s.contains("correct") || s.contains("корректир") || s.contains("исправ") {
        "correction"
    } else {
        "manual"
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
            // Вариант 1: args = { "items": [{ "ingredient", "quantity", "unit" }] }
            if let Some(items) = args.get("items").and_then(|v| v.as_array()) {
                return items.iter().filter_map(|item| {
                    let ingredient = item.get("ingredient").or_else(|| item.get("ingredient_name"))?.as_str()?;
                    let quantity = item.get("quantity")?.as_f64()?;
                    let unit = item.get("unit").and_then(|u| u.as_str()).unwrap_or("kg");
                    Some(ActionChange {
                        entity: ingredient.to_string(),
                        field: "quantity".to_string(),
                        before: None,
                        after: format!("{} {}", quantity, unit),
                        unit: Some(unit.to_string()),
                    })
                }).collect();
            }
            // Вариант 2: плоские args от Gemini = { "ingredient_name"/"ingredient"/"name", "quantity", "unit" }
            if let Some(name) = args.get("ingredient_name")
                .or_else(|| args.get("ingredient"))
                .or_else(|| args.get("name"))
                .and_then(|v| v.as_str())
            {
                let quantity = args.get("quantity").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let unit = args.get("unit").and_then(|v| v.as_str()).unwrap_or("kg");
                return vec![ActionChange {
                    entity: name.to_string(),
                    field: "quantity".to_string(),
                    before: None,
                    after: format!("{} {}", quantity, unit),
                    unit: Some(unit.to_string()),
                }];
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

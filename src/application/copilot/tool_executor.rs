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
use crate::application::purchase_draft::{
    CreatePurchaseDraftInput, PurchaseDraftItemInput, PurchaseDraftService,
};
use crate::application::recipe::RecipeService;
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
    pub recipes_v1: Arc<RecipeService>,
    pub cook_suggestions: Arc<CookSuggestionService>,
    pub sous_chef: Arc<SousChefPlannerService>,
    pub catalog: Arc<CatalogService>,
    pub purchase_drafts: Arc<PurchaseDraftService>,
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
    ///
    /// Возвращает (Option<ActionPlan>, Option<ToolResult>):
    /// - ToolResult генерируется когда write tool не нужно выполнять
    ///   (например, draft уже sent), и синтезатор ответа должен это объяснить.
    pub async fn prepare_action_plan(
        &self,
        ctx: &CopilotContext,
        tool_calls: &[ToolCall],
        tool_results: &[ToolResult],
    ) -> (Option<ActionPlan>, Option<ToolResult>) {
        let Some(write_call) = tool_calls.iter().find(|c| c.tool.is_write()) else {
            return (None, None);
        };

        // Для SendPurchaseOrder: резолвим "last" → реальный uuid и добавляем
        // позиции в preview из БД.
        let mut payload = serde_json::to_value(&write_call.args).unwrap_or(json!({}));
        let mut changes = build_preview_changes(&write_call.tool, &write_call.args, tool_results);

        if matches!(write_call.tool, CopilotTool::SendPurchaseOrder) {
            match self.resolve_send_purchase_order_preview(ctx, &write_call.args).await {
                SendPreview::Ok(resolved_id, draft_changes) => {
                    if let Some(obj) = payload.as_object_mut() {
                        obj.insert("id".to_string(), json!(resolved_id.to_string()));
                    }
                    changes = draft_changes;
                }
                SendPreview::AlreadyProcessed { id, status } => {
                    let info = ToolResult {
                        tool_name: "send_purchase_order_skipped".to_string(),
                        data: json!({
                            "skipped": true,
                            "reason": "already_processed",
                            "draft_id": id.to_string(),
                            "current_status": status,
                            "explanation": "Draft is not in 'draft' state, no action needed.",
                        }),
                    };
                    return (None, Some(info));
                }
                SendPreview::NotFound => {
                    let info = ToolResult {
                        tool_name: "send_purchase_order_skipped".to_string(),
                        data: json!({
                            "skipped": true,
                            "reason": "no_draft_found",
                            "explanation": "No purchase drafts to send.",
                        }),
                    };
                    return (None, Some(info));
                }
            }
        }

        // Для AdjustInventoryQuantity: считаем текущий остаток и формируем preview before→after
        if matches!(write_call.tool, CopilotTool::AdjustInventoryQuantity) {
            match self.resolve_adjust_quantity_preview(ctx, &write_call.args).await {
                AdjustPreview::Ok(adjust_changes) => {
                    changes = adjust_changes;
                }
                AdjustPreview::Noop { name, current } => {
                    let info = ToolResult {
                        tool_name: "adjust_inventory_quantity_skipped".to_string(),
                        data: json!({
                            "skipped": true,
                            "reason": "already_at_target",
                            "ingredient": name,
                            "current_quantity": current,
                            "explanation": "Stock is already at the target value, no change needed.",
                        }),
                    };
                    return (None, Some(info));
                }
                AdjustPreview::NotFound { name } => {
                    let info = ToolResult {
                        tool_name: "adjust_inventory_quantity_skipped".to_string(),
                        data: json!({
                            "skipped": true,
                            "reason": "ingredient_not_found",
                            "ingredient": name,
                            "explanation": "Ingredient not found in catalog or inventory.",
                        }),
                    };
                    return (None, Some(info));
                }
            }
        }

        // Для UpdateDishPrice: ищем dish, проверяем noop / multiple / not_found / invalid.
        if matches!(write_call.tool, CopilotTool::UpdateDishPrice) {
            match self.resolve_update_dish_price_preview(ctx, &write_call.args).await {
                DishPriceUpdatePreview::Ok { dish_id, dish_name, new_price_cents, changes: dish_changes } => {
                    if let Some(obj) = payload.as_object_mut() {
                        obj.insert("dish_id".to_string(), json!(dish_id.to_string()));
                        obj.insert("dish_name".to_string(), json!(dish_name));
                        obj.insert("new_price_cents".to_string(), json!(new_price_cents));
                    }
                    changes = dish_changes;
                }
                DishPriceUpdatePreview::SamePrice { name, current_cents } => {
                    let info = ToolResult {
                        tool_name: "update_dish_price_skipped".to_string(),
                        data: json!({
                            "skipped": true,
                            "reason": "already_at_target",
                            "dish": name,
                            "current_price_eur": format!("{:.2}", current_cents as f64 / 100.0),
                            "explanation": "Dish price is already at the requested value, no change needed.",
                        }),
                    };
                    return (None, Some(info));
                }
                DishPriceUpdatePreview::Multiple { query, candidates } => {
                    let list: Vec<serde_json::Value> = candidates.iter().map(|(n, c)| json!({
                        "name": n,
                        "price_eur": format!("{:.2}", *c as f64 / 100.0),
                    })).collect();
                    let info = ToolResult {
                        tool_name: "update_dish_price_skipped".to_string(),
                        data: json!({
                            "skipped": true,
                            "reason": "multiple_matches",
                            "query": query,
                            "candidates": list,
                            "explanation": "Multiple dishes match the query — ask user to specify which one.",
                        }),
                    };
                    return (None, Some(info));
                }
                DishPriceUpdatePreview::NotFound { query } => {
                    let info = ToolResult {
                        tool_name: "update_dish_price_skipped".to_string(),
                        data: json!({
                            "skipped": true,
                            "reason": "dish_not_found",
                            "query": query,
                            "explanation": "No dish found matching the query.",
                        }),
                    };
                    return (None, Some(info));
                }
                DishPriceUpdatePreview::Invalid { reason } => {
                    let info = ToolResult {
                        tool_name: "update_dish_price_skipped".to_string(),
                        data: json!({
                            "skipped": true,
                            "reason": "invalid_args",
                            "explanation": reason,
                        }),
                    };
                    return (None, Some(info));
                }
            }
        }

        // Для CreateRecipe: резолвим каждый ингредиент в catalog, валидируем servings/quantities,
        // проверяем что рецепт с таким названием ещё не существует.
        if matches!(write_call.tool, CopilotTool::CreateRecipe) {
            match self.resolve_create_recipe_preview(ctx, &write_call.args).await {
                CreateRecipePreview::Ok { recipe_name, servings, resolved, recipe_changes } => {
                    if let Some(obj) = payload.as_object_mut() {
                        obj.insert("recipe_name".to_string(), json!(recipe_name));
                        obj.insert("servings".to_string(), json!(servings));
                        obj.insert(
                            "resolved_ingredients".to_string(),
                            json!(resolved.iter().map(|r| json!({
                                "catalog_ingredient_id": r.catalog_ingredient_id.to_string(),
                                "ingredient_name": r.matched_name,
                                "quantity_in_default_unit": r.quantity_in_default_unit,
                                "default_unit": r.default_unit,
                            })).collect::<Vec<_>>()),
                        );
                    }
                    changes = recipe_changes;
                }
                CreateRecipePreview::AlreadyExists { name } => {
                    let info = ToolResult {
                        tool_name: "create_recipe_skipped".to_string(),
                        data: json!({
                            "skipped": true,
                            "reason": "name_already_exists",
                            "recipe_name": name,
                            "explanation": "A recipe with this name already exists for this tenant. Choose a different name or update the existing recipe.",
                        }),
                    };
                    return (None, Some(info));
                }
                CreateRecipePreview::IngredientNotFound { query, suggestions } => {
                    let info = ToolResult {
                        tool_name: "create_recipe_skipped".to_string(),
                        data: json!({
                            "skipped": true,
                            "reason": "ingredient_not_found",
                            "query": query,
                            "suggestions": suggestions,
                            "explanation": "One of the ingredients was not found in the catalog. Ask user to clarify or pick from suggestions.",
                        }),
                    };
                    return (None, Some(info));
                }
                CreateRecipePreview::AmbiguousIngredient { query, candidates } => {
                    let info = ToolResult {
                        tool_name: "create_recipe_skipped".to_string(),
                        data: json!({
                            "skipped": true,
                            "reason": "ambiguous_ingredient",
                            "query": query,
                            "candidates": candidates,
                            "explanation": "Multiple catalog ingredients matched the query. Ask user to be more specific.",
                        }),
                    };
                    return (None, Some(info));
                }
                CreateRecipePreview::Invalid { reason } => {
                    let info = ToolResult {
                        tool_name: "create_recipe_skipped".to_string(),
                        data: json!({
                            "skipped": true,
                            "reason": "invalid_args",
                            "explanation": reason,
                        }),
                    };
                    return (None, Some(info));
                }
            }
        }

        // ── CreateDish: резолвим recipe_name → recipe_id, валидируем цену,
        // проверяем уникальность имени блюда в рамках тенанта.
        if matches!(write_call.tool, CopilotTool::CreateDish) {
            match self.resolve_create_dish_preview(ctx, &write_call.args).await {
                CreateDishPreview::Ok { dish_name, recipe_id, recipe_name, selling_price_cents, description, dish_changes } => {
                    if let Some(obj) = payload.as_object_mut() {
                        obj.insert("dish_name".to_string(), json!(dish_name));
                        obj.insert("recipe_id".to_string(), json!(recipe_id.to_string()));
                        obj.insert("selling_price_cents".to_string(), json!(selling_price_cents));
                        if let Some(d) = description {
                            obj.insert("description".to_string(), json!(d));
                        }
                        // Keep recipe_name for display
                        obj.insert("recipe_name".to_string(), json!(recipe_name));
                    }
                    changes = dish_changes;
                }
                CreateDishPreview::RecipeNotFound { query } => {
                    let info = ToolResult {
                        tool_name: "create_dish_skipped".to_string(),
                        data: json!({
                            "skipped": true,
                            "reason": "recipe_not_found",
                            "query": query,
                            "explanation": format!("No recipe named '{}' found. Create the recipe first or check the name.", query),
                        }),
                    };
                    return (None, Some(info));
                }
                CreateDishPreview::AmbiguousRecipe { query, candidates } => {
                    let info = ToolResult {
                        tool_name: "create_dish_skipped".to_string(),
                        data: json!({
                            "skipped": true,
                            "reason": "ambiguous_recipe",
                            "query": query,
                            "candidates": candidates,
                            "explanation": "Multiple recipes matched. Ask the user to be more specific.",
                        }),
                    };
                    return (None, Some(info));
                }
                CreateDishPreview::DuplicateDishName { name } => {
                    let info = ToolResult {
                        tool_name: "create_dish_skipped".to_string(),
                        data: json!({
                            "skipped": true,
                            "reason": "dish_name_already_exists",
                            "dish_name": name,
                            "explanation": "A dish with this name already exists. Choose a different name or update the existing dish price.",
                        }),
                    };
                    return (None, Some(info));
                }
                CreateDishPreview::Invalid { reason } => {
                    let info = ToolResult {
                        tool_name: "create_dish_skipped".to_string(),
                        data: json!({
                            "skipped": true,
                            "reason": "invalid_args",
                            "explanation": reason,
                        }),
                    };
                    return (None, Some(info));
                }
            }
        }

        let plan_type = tool_to_plan_type(&write_call.tool);

        (Some(ActionPlan {
            id: Uuid::new_v4(),
            plan_type,
            changes,
            write_tool: Some(write_call.tool.clone()),
            payload,
        }), None)
    }
    async fn resolve_send_purchase_order_preview(
        &self,
        ctx: &CopilotContext,
        args: &std::collections::HashMap<String, serde_json::Value>,
    ) -> SendPreview {
        let id_arg = args.get("id").and_then(|v| v.as_str()).unwrap_or("last");
        let target_id: Uuid = if id_arg == "last" || id_arg.is_empty() {
            let drafts = match self.services.purchase_drafts.list(ctx.tenant_id.clone(), 1).await {
                Ok(d) => d,
                Err(_) => return SendPreview::NotFound,
            };
            match drafts.into_iter().next() {
                Some(d) => d.id,
                None => return SendPreview::NotFound,
            }
        } else {
            match id_arg.parse::<Uuid>() {
                Ok(u) => u,
                Err(_) => return SendPreview::NotFound,
            }
        };

        let draft = match self.services.purchase_drafts.get(target_id, ctx.user_id.clone()).await {
            Ok(Some(d)) => d,
            _ => return SendPreview::NotFound,
        };

        if draft.status != "draft" {
            return SendPreview::AlreadyProcessed {
                id: target_id,
                status: draft.status,
            };
        }

        let mut changes = vec![ActionChange {
            entity: format!("Purchase draft {}", &target_id.to_string()[..8]),
            field: "status".to_string(),
            before: Some(draft.status.clone()),
            after: "sent".to_string(),
            unit: None,
        }];
        for it in &draft.items {
            changes.push(ActionChange {
                entity: format!("Item: {}", it.ingredient_name),
                field: "quantity".to_string(),
                before: None,
                after: format!("{} {}", it.quantity, it.unit),
                unit: Some(it.unit.clone()),
            });
        }
        SendPreview::Ok(target_id, changes)
    }

    /// Считает текущий остаток ингредиента и формирует preview "before → after".
    async fn resolve_adjust_quantity_preview(
        &self,
        ctx: &CopilotContext,
        args: &std::collections::HashMap<String, serde_json::Value>,
    ) -> AdjustPreview {
        let name = match args.get("ingredient_name")
            .or_else(|| args.get("ingredient"))
            .or_else(|| args.get("name"))
            .or_else(|| args.get("item_name"))
            .or_else(|| args.get("product_name"))
            .and_then(|v| v.as_str())
        {
            Some(n) => n.to_string(),
            None => return AdjustPreview::NotFound { name: "?".into() },
        };
        let target = match args.get("target_quantity")
            .or_else(|| args.get("quantity"))
            .and_then(|v| v.as_f64())
        {
            Some(q) => q,
            None => return AdjustPreview::NotFound { name },
        };
        let unit = args.get("unit").and_then(|v| v.as_str()).unwrap_or("kg").to_string();

        let catalog_id = match self.find_catalog_id(&name).await {
            Ok(id) => id,
            Err(_) => return AdjustPreview::NotFound { name },
        };
        let cat_uuid = catalog_id.as_uuid();

        // Считаем текущий остаток по всем активным batches
        let items = self.services.inventory
            .list_products_with_details(ctx.user_id.clone(), ctx.tenant_id.clone(), Language::En)
            .await
            .unwrap_or_default();
        let current: f64 = items.iter()
            .filter(|i| i.product.id == cat_uuid)
            .map(|i| i.remaining_quantity)
            .sum();

        let diff = target - current;
        if diff.abs() < 0.0001 {
            return AdjustPreview::Noop { name, current };
        }

        let arrow = format!("{} {} → {} {} (diff {:+.3} {})",
            current, unit, target, unit, diff, unit);
        let changes = vec![ActionChange {
            entity: format!("Inventory: {}", name),
            field: "remaining_quantity".to_string(),
            before: Some(format!("{} {}", current, unit)),
            after: arrow,
            unit: Some(unit),
        }];
        AdjustPreview::Ok(changes)
    }

    /// Pred-validate UpdateDishPrice: ищет dish по имени, считает margin, проверяет noop/invalid/multi-match.
    async fn resolve_update_dish_price_preview(
        &self,
        ctx: &CopilotContext,
        args: &std::collections::HashMap<String, serde_json::Value>,
    ) -> DishPriceUpdatePreview {
        // 1. Парсим dish_name
        let query = match args.get("dish_name")
            .or_else(|| args.get("name"))
            .or_else(|| args.get("dish"))
            .and_then(|v| v.as_str())
        {
            Some(s) if !s.trim().is_empty() => s.trim().to_string(),
            _ => return DishPriceUpdatePreview::Invalid { reason: "missing dish_name".into() },
        };

        // 2. Парсим new_price_cents (приоритет) или new_price (EUR float)
        let new_price_cents: i64 = if let Some(c) = args.get("new_price_cents").and_then(|v| v.as_i64()) {
            c
        } else if let Some(eur) = args.get("new_price")
            .or_else(|| args.get("price"))
            .and_then(|v| v.as_f64())
        {
            (eur * 100.0).round() as i64
        } else {
            return DishPriceUpdatePreview::Invalid { reason: "missing new_price".into() };
        };

        if new_price_cents <= 0 {
            return DishPriceUpdatePreview::Invalid {
                reason: "new price must be greater than 0".into(),
            };
        }

        // 3. Ищем блюда по имени
        let matches = match self.services.dishes
            .find_dishes_by_name(ctx.tenant_id.clone(), &query, true, 5)
            .await
        {
            Ok(m) => m,
            Err(_) => return DishPriceUpdatePreview::NotFound { query },
        };

        if matches.is_empty() {
            return DishPriceUpdatePreview::NotFound { query };
        }

        if matches.len() > 1 {
            let candidates = matches.iter()
                .map(|d| (d.name().as_str().to_string(), d.selling_price().as_cents()))
                .collect();
            return DishPriceUpdatePreview::Multiple { query, candidates };
        }

        // 4. Ровно одно совпадение
        let dish = &matches[0];
        let current_cents = dish.selling_price().as_cents();
        let dish_name = dish.name().as_str().to_string();

        if current_cents == new_price_cents {
            return DishPriceUpdatePreview::SamePrice { name: dish_name, current_cents };
        }

        // 5. Строим preview changes (price + опциональный margin)
        let mut changes = vec![ActionChange {
            entity: format!("Dish: {}", dish_name),
            field: "selling_price".to_string(),
            before: Some(format!("€{:.2}", current_cents as f64 / 100.0)),
            after: format!("€{:.2}", new_price_cents as f64 / 100.0),
            unit: Some("EUR".to_string()),
        }];

        // Margin preview если есть recipe_cost
        if let Some(cost_cents) = dish.recipe_cost_cents() {
            let old_margin_pct = if current_cents > 0 {
                (current_cents - cost_cents) as f64 * 100.0 / current_cents as f64
            } else { 0.0 };
            let new_margin_pct = (new_price_cents - cost_cents) as f64 * 100.0 / new_price_cents as f64;

            changes.push(ActionChange {
                entity: format!("Dish: {}", dish_name),
                field: "profit_margin".to_string(),
                before: Some(format!("{:.1}%", old_margin_pct)),
                after: format!("{:.1}%", new_margin_pct),
                unit: Some("%".to_string()),
            });

            // Warning row если новая цена ниже food cost
            if new_price_cents < cost_cents {
                changes.push(ActionChange {
                    entity: format!("⚠️ Dish: {}", dish_name),
                    field: "warning".to_string(),
                    before: Some(format!("food_cost €{:.2}", cost_cents as f64 / 100.0)),
                    after: format!("new price €{:.2} is BELOW food cost", new_price_cents as f64 / 100.0),
                    unit: None,
                });
            }
        }

        DishPriceUpdatePreview::Ok {
            dish_id: dish.id().as_uuid(),
            dish_name,
            new_price_cents,
            changes,
        }
    }

    /// Pre-validate args для CreateRecipe: разрешает ингредиенты по каталогу,
    /// конвертирует количества в default_unit ингредиента, проверяет уникальность имени.
    async fn resolve_create_recipe_preview(
        &self,
        ctx: &CopilotContext,
        args: &std::collections::HashMap<String, serde_json::Value>,
    ) -> CreateRecipePreview {
        use crate::domain::catalog::Unit;

        // 1. recipe_name
        let recipe_name = match args.get("recipe_name")
            .or_else(|| args.get("name"))
            .and_then(|v| v.as_str())
        {
            Some(s) if !s.trim().is_empty() => s.trim().to_string(),
            _ => return CreateRecipePreview::Invalid { reason: "missing recipe_name".into() },
        };

        // 2. servings (default 1, must be > 0 and <= 1000)
        let servings_n: u32 = match args.get("servings").and_then(|v| v.as_u64()) {
            Some(n) if n > 0 && n <= 1000 => n as u32,
            Some(_) => return CreateRecipePreview::Invalid {
                reason: "servings must be between 1 and 1000".into(),
            },
            None => 1,
        };

        // 3. ingredients[] (required, non-empty)
        let ingredients_arr = match args.get("ingredients").and_then(|v| v.as_array()) {
            Some(arr) if !arr.is_empty() => arr.clone(),
            _ => return CreateRecipePreview::Invalid {
                reason: "ingredients[] is required and must be non-empty".into(),
            },
        };

        // 4. Проверка дубликата по имени (case-insensitive)
        let pagination = crate::shared::pagination::PaginationParams {
            page: Some(1),
            per_page: Some(100),
        };
        if let Ok(page) = self.services.recipes_v1.list_recipes(ctx.tenant_id.clone(), &pagination).await {
            let target = recipe_name.to_lowercase();
            if page.items.iter().any(|r| r.name().as_str().to_lowercase() == target) {
                return CreateRecipePreview::AlreadyExists { name: recipe_name };
            }
        }

        // 5. Резолвим каждый ингредиент
        let mut resolved: Vec<ResolvedRecipeIngredient> = Vec::with_capacity(ingredients_arr.len());
        let mut recipe_changes: Vec<ActionChange> = Vec::new();

        // Header row
        recipe_changes.push(ActionChange {
            entity: format!("Recipe: {}", recipe_name),
            field: "servings".to_string(),
            before: None,
            after: servings_n.to_string(),
            unit: None,
        });

        for (idx, item) in ingredients_arr.iter().enumerate() {
            let query = match item.get("ingredient_name")
                .or_else(|| item.get("name"))
                .and_then(|v| v.as_str())
            {
                Some(s) if !s.trim().is_empty() => s.trim().to_string(),
                _ => return CreateRecipePreview::Invalid {
                    reason: format!("ingredients[{}].ingredient_name is required", idx),
                },
            };

            let qty_raw = match item.get("quantity").and_then(|v| v.as_f64()) {
                Some(q) if q > 0.0 => q,
                Some(_) => return CreateRecipePreview::Invalid {
                    reason: format!("ingredients[{}].quantity must be > 0", idx),
                },
                None => return CreateRecipePreview::Invalid {
                    reason: format!("ingredients[{}].quantity is required", idx),
                },
            };

            let unit_str = item.get("unit").and_then(|v| v.as_str()).unwrap_or("g").trim();
            let user_unit = match Unit::from_str(unit_str) {
                Ok(u) => u,
                Err(_) => return CreateRecipePreview::Invalid {
                    reason: format!("ingredients[{}].unit '{}' is not a valid unit", idx, unit_str),
                },
            };

            // Каталог: EN → RU fallback. Берём список — если >1 без точного совпадения, ambiguous.
            let mut candidates = self.services.catalog
                .search_ingredients(&query, Language::En, 5)
                .await
                .unwrap_or_default();
            if candidates.is_empty() {
                candidates = self.services.catalog
                    .search_ingredients(&query, Language::Ru, 5)
                    .await
                    .unwrap_or_default();
            }

            if candidates.is_empty() {
                return CreateRecipePreview::IngredientNotFound {
                    query,
                    suggestions: vec![],
                };
            }

            // Точное совпадение по name_en/name_ru (case-insensitive)
            let q_low = query.to_lowercase();
            let exact_idx = candidates.iter().position(|c| {
                c.name_en.to_lowercase() == q_low || c.name_ru.to_lowercase() == q_low
            });

            let chosen = if let Some(i) = exact_idx {
                candidates.remove(i)
            } else if candidates.len() == 1 {
                candidates.remove(0)
            } else {
                let names: Vec<String> = candidates.iter()
                    .map(|c| c.name_en.clone())
                    .collect();
                return CreateRecipePreview::AmbiguousIngredient {
                    query,
                    candidates: names,
                };
            };

            // Конвертируем qty_raw из user_unit в chosen.default_unit
            let qty_in_default = match convert_quantity(qty_raw, user_unit, chosen.default_unit) {
                Ok(q) => q,
                Err(msg) => return CreateRecipePreview::Invalid {
                    reason: format!("ingredients[{}] ('{}'): {}", idx, query, msg),
                },
            };

            recipe_changes.push(ActionChange {
                entity: format!("Ingredient: {}", chosen.name_en),
                field: "quantity".to_string(),
                before: None,
                after: format!("{} {} (= {:.3} {})", qty_raw, user_unit.as_str(), qty_in_default, chosen.default_unit.as_str()),
                unit: Some(chosen.default_unit.as_str().to_string()),
            });

            resolved.push(ResolvedRecipeIngredient {
                catalog_ingredient_id: chosen.id.as_uuid(),
                matched_name: chosen.name_en,
                quantity_in_default_unit: qty_in_default,
                default_unit: chosen.default_unit.as_str().to_string(),
            });
        }

        CreateRecipePreview::Ok {
            recipe_name,
            servings: servings_n,
            resolved,
            recipe_changes,
        }
    }

    /// Создать рецепт из уже резолвнутого payload (после confirmation).
    async fn execute_create_recipe(
        &self,
        user_id: UserId,
        tenant_id: TenantId,
        payload: &serde_json::Value,
    ) -> AppResult<String> {
        let name_str = payload.get("recipe_name").and_then(|v| v.as_str())
            .ok_or_else(|| AppError::validation("recipe_name is required"))?;
        let servings_n = payload.get("servings").and_then(|v| v.as_u64()).unwrap_or(1) as u32;

        let resolved = payload.get("resolved_ingredients").and_then(|v| v.as_array())
            .ok_or_else(|| AppError::validation(
                "resolved_ingredients is required (resolver must run before execution)"
            ))?;

        let recipe_name = crate::domain::RecipeName::new(name_str)?;
        let servings = crate::domain::Servings::new(servings_n)?;

        let mut ingredients: Vec<crate::domain::RecipeIngredient> = Vec::with_capacity(resolved.len());
        for r in resolved {
            let cat_id_str = r.get("catalog_ingredient_id").and_then(|v| v.as_str())
                .ok_or_else(|| AppError::validation("catalog_ingredient_id is required in resolved_ingredients"))?;
            let cat_uuid = Uuid::parse_str(cat_id_str)
                .map_err(|_| AppError::validation("invalid catalog_ingredient_id"))?;
            let qty_val = r.get("quantity_in_default_unit").and_then(|v| v.as_f64())
                .ok_or_else(|| AppError::validation("quantity_in_default_unit is required"))?;

            ingredients.push(crate::domain::RecipeIngredient::new(
                crate::domain::catalog::CatalogIngredientId::from_uuid(cat_uuid),
                crate::domain::inventory::Quantity::new(qty_val)?,
            ));
        }

        let recipe = self.services.recipes_v1
            .create_recipe(recipe_name, servings, ingredients, vec![], user_id, tenant_id)
            .await?;

        Ok(format!(
            "Created recipe '{}' (id: {}) with {} ingredient(s), {} serving(s).",
            recipe.name().as_str(),
            recipe.id().as_uuid(),
            recipe.ingredients().len(),
            recipe.servings().count(),
        ))
    }

    /// Pre-validate args для CreateDish: резолвим recipe_name → recipe_id,
    /// проверяем selling_price > 0, проверяем уникальность имени блюда.
    async fn resolve_create_dish_preview(
        &self,
        ctx: &CopilotContext,
        args: &std::collections::HashMap<String, serde_json::Value>,
    ) -> CreateDishPreview {
        // 1. dish_name (required)
        let dish_name = match args.get("dish_name")
            .or_else(|| args.get("name"))
            .and_then(|v| v.as_str())
        {
            Some(s) if !s.trim().is_empty() => s.trim().to_string(),
            _ => return CreateDishPreview::Invalid { reason: "missing dish_name".into() },
        };

        // 2. selling_price_cents (required, > 0)
        let selling_price_cents: i64 = if let Some(c) = args.get("selling_price_cents").and_then(|v| v.as_i64()) {
            c
        } else if let Some(eur) = args.get("selling_price").and_then(|v| v.as_f64()) {
            (eur * 100.0).round() as i64
        } else {
            return CreateDishPreview::Invalid { reason: "missing selling_price_cents (or selling_price in EUR)".into() };
        };
        if selling_price_cents <= 0 {
            return CreateDishPreview::Invalid { reason: "selling_price must be greater than 0".into() };
        }

        // 3. recipe_name (required) → resolve to recipe_id
        let recipe_query = match args.get("recipe_name")
            .or_else(|| args.get("recipe"))
            .and_then(|v| v.as_str())
        {
            Some(s) if !s.trim().is_empty() => s.trim().to_string(),
            _ => return CreateDishPreview::Invalid { reason: "missing recipe_name — which recipe should this dish be based on?".into() },
        };

        let description = args.get("description").and_then(|v| v.as_str()).map(str::to_string);

        // 4. Search recipes by name (list all, filter case-insensitive)
        let pagination = crate::shared::pagination::PaginationParams { page: Some(1), per_page: Some(200) };
        let recipes = match self.services.recipes_v1.list_recipes(ctx.tenant_id.clone(), &pagination).await {
            Ok(p) => p.items,
            Err(_) => return CreateDishPreview::Invalid { reason: "could not load recipes".into() },
        };

        let q_low = recipe_query.to_lowercase();
        let exact: Vec<_> = recipes.iter().filter(|r| r.name().as_str().to_lowercase() == q_low).collect();
        let partial: Vec<_> = recipes.iter().filter(|r| r.name().as_str().to_lowercase().contains(&q_low)).collect();

        let chosen_recipe = if exact.len() == 1 {
            exact[0]
        } else if exact.is_empty() && partial.len() == 1 {
            partial[0]
        } else if exact.is_empty() && partial.is_empty() {
            return CreateDishPreview::RecipeNotFound { query: recipe_query };
        } else {
            let candidates: Vec<String> = if !exact.is_empty() {
                exact.iter().map(|r| r.name().as_str().to_string()).collect()
            } else {
                partial.iter().map(|r| r.name().as_str().to_string()).collect()
            };
            return CreateDishPreview::AmbiguousRecipe { query: recipe_query, candidates };
        };

        let recipe_id = chosen_recipe.id();
        let recipe_name = chosen_recipe.name().as_str().to_string();

        // 5. Check dish name uniqueness (case-insensitive)
        let dish_pagination = crate::shared::pagination::PaginationParams { page: Some(1), per_page: Some(200) };
        if let Ok((existing_dishes, _)) = self.services.dishes.list_dishes(ctx.tenant_id.clone(), false, &dish_pagination).await {
            let dname_low = dish_name.to_lowercase();
            if existing_dishes.iter().any(|d| d.name().as_str().to_lowercase() == dname_low) {
                return CreateDishPreview::DuplicateDishName { name: dish_name };
            }
        }

        // 6. Build preview changes
        let dish_changes = vec![
            ActionChange {
                entity: format!("Dish: {}", dish_name),
                field: "recipe".to_string(),
                before: None,
                after: recipe_name.clone(),
                unit: None,
            },
            ActionChange {
                entity: format!("Dish: {}", dish_name),
                field: "selling_price".to_string(),
                before: None,
                after: format!("€{:.2}", selling_price_cents as f64 / 100.0),
                unit: Some("EUR".to_string()),
            },
        ];

        CreateDishPreview::Ok {
            dish_name,
            recipe_id: recipe_id.as_uuid(),
            recipe_name,
            selling_price_cents,
            description,
            dish_changes,
        }
    }

    /// Создать блюдо из уже резолвнутого payload (после confirmation).
    async fn execute_create_dish(
        &self,
        tenant_id: TenantId,
        payload: &serde_json::Value,
    ) -> AppResult<String> {
        let dish_name_str = payload.get("dish_name").and_then(|v| v.as_str())
            .ok_or_else(|| AppError::validation("dish_name is required"))?;
        let recipe_id_str = payload.get("recipe_id").and_then(|v| v.as_str())
            .ok_or_else(|| AppError::validation("recipe_id is required (resolver must run first)"))?;
        let selling_price_cents = payload.get("selling_price_cents").and_then(|v| v.as_i64())
            .ok_or_else(|| AppError::validation("selling_price_cents is required"))?;
        let description = payload.get("description").and_then(|v| v.as_str()).map(str::to_string);

        let recipe_uuid = Uuid::parse_str(recipe_id_str)
            .map_err(|_| AppError::validation("invalid recipe_id UUID"))?;
        let recipe_id = crate::domain::RecipeId::from_uuid(recipe_uuid);
        let dish_name = crate::domain::DishName::new(dish_name_str)?;
        let selling_price = crate::domain::Money::from_cents(selling_price_cents)?;

        let dish = self.services.dishes
            .create_dish(tenant_id, recipe_id, dish_name, description, selling_price, None)
            .await?;

        let margin_info = match dish.profit_margin_percent() {
            Some(m) => format!(", margin {:.1}%", m),
            None => String::new(),
        };
        let cost_info = match dish.recipe_cost_cents() {
            Some(c) => format!(", food cost €{:.2}", c as f64 / 100.0),
            None => String::new(),
        };

        Ok(format!(
            "Created dish '{}' (id: {}) linked to recipe. Price: €{:.2}{}{} .",
            dish.name().as_str(),
            dish.id().as_uuid(),
            selling_price_cents as f64 / 100.0,
            cost_info,
            margin_info,
        ))
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

            CopilotTool::ListPurchaseDrafts => {
                let limit = args.get("limit").and_then(|v| v.as_i64()).unwrap_or(10).clamp(1, 50);
                let drafts = self.services.purchase_drafts
                    .list(ctx.tenant_id.clone(), limit)
                    .await?;
                let summary: Vec<serde_json::Value> = drafts.iter().map(|d| json!({
                    "id": d.id,
                    "supplier_name": d.supplier_name,
                    "delivery_date": d.delivery_date.map(|x| x.to_string()),
                    "status": d.status,
                    "items_count": d.items.len(),
                    "total_cost_cents": d.total_cost_cents,
                    "created_at": d.created_at.to_string(),
                })).collect();
                Ok(ToolResult {
                    tool_name: name,
                    data: json!({ "drafts": summary, "total": drafts.len() }),
                })
            }

            CopilotTool::GetPurchaseDraft => {
                let id_arg = args.get("id").and_then(|v| v.as_str()).unwrap_or("last");
                let target_id: Uuid = if id_arg == "last" || id_arg.is_empty() {
                    let drafts = self.services.purchase_drafts
                        .list(ctx.tenant_id.clone(), 1)
                        .await?;
                    let Some(first) = drafts.into_iter().next() else {
                        return Ok(ToolResult {
                            tool_name: name,
                            data: json!({ "draft": null, "note": "no drafts found" }),
                        });
                    };
                    first.id
                } else {
                    id_arg.parse::<Uuid>()
                        .map_err(|_| AppError::validation("invalid UUID for purchase draft id"))?
                };

                let draft = self.services.purchase_drafts
                    .get(target_id, ctx.user_id.clone())
                    .await?;

                let data = match draft {
                    Some(d) => json!({
                        "draft": {
                            "id": d.id,
                            "supplier_name": d.supplier_name,
                            "delivery_date": d.delivery_date.map(|x| x.to_string()),
                            "status": d.status,
                            "note": d.note,
                            "total_cost_cents": d.total_cost_cents,
                            "created_at": d.created_at.to_string(),
                            "items": d.items.iter().map(|i| json!({
                                "ingredient_name": i.ingredient_name,
                                "quantity": i.quantity,
                                "unit": i.unit,
                                "price_per_unit_cents": i.price_per_unit_cents,
                            })).collect::<Vec<_>>(),
                        }
                    }),
                    None => json!({ "draft": null, "note": "draft not found" }),
                };
                Ok(ToolResult { tool_name: name, data })
            }

            CopilotTool::GetDailyBriefing => {
                let expiring_days = args.get("expiring_days").and_then(|v| v.as_i64()).unwrap_or(3).clamp(1, 14);
                let low_stock_threshold = args.get("low_stock_threshold").and_then(|v| v.as_f64()).unwrap_or(1.0);

                // 1. Inventory snapshot (one query, used for expiring + low stock)
                let inventory = self.services.inventory
                    .list_products_with_details(ctx.user_id.clone(), ctx.tenant_id.clone(), Language::En)
                    .await
                    .unwrap_or_default();

                let now = time::OffsetDateTime::now_utc();
                let limit = now + time::Duration::days(expiring_days);

                let expiring: Vec<serde_json::Value> = inventory.iter()
                    .filter(|i| i.expires_at <= limit && i.remaining_quantity > 0.0)
                    .take(10)
                    .map(|i| {
                        let days_left = (i.expires_at - now).whole_days();
                        json!({
                            "name": i.product.name,
                            "remaining": i.remaining_quantity,
                            "unit": i.product.base_unit,
                            "days_left": days_left,
                            "severity": format!("{:?}", i.severity),
                            "expires_at": i.expires_at.date().to_string(),
                        })
                    })
                    .collect();

                // 2. Low stock — сгруппируем по catalog_ingredient_id и сравним с min_threshold
                use std::collections::HashMap;
                let mut totals: HashMap<uuid::Uuid, (String, String, f64, f64)> = HashMap::new();
                for i in inventory.iter() {
                    let entry = totals.entry(i.product.id)
                        .or_insert((i.product.name.clone(), i.product.base_unit.clone(), 0.0, i.product.min_stock_threshold));
                    entry.2 += i.remaining_quantity;
                }
                let mut low_stock: Vec<serde_json::Value> = totals.values()
                    .filter(|(_, _, total, min_thr)| {
                        let threshold = if *min_thr > 0.0 { *min_thr } else { low_stock_threshold };
                        *total < threshold
                    })
                    .map(|(name, unit, total, min_thr)| json!({
                        "name": name,
                        "remaining": total,
                        "unit": unit,
                        "threshold": if *min_thr > 0.0 { *min_thr } else { low_stock_threshold },
                    }))
                    .collect();
                low_stock.sort_by(|a, b| {
                    a["remaining"].as_f64().unwrap_or(0.0)
                        .partial_cmp(&b["remaining"].as_f64().unwrap_or(0.0))
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
                low_stock.truncate(10);

                // 3. Purchase drafts (split by status)
                let drafts = self.services.purchase_drafts
                    .list(ctx.tenant_id.clone(), 20)
                    .await
                    .unwrap_or_default();
                let open_drafts: Vec<serde_json::Value> = drafts.iter()
                    .filter(|d| d.status == "draft")
                    .take(5)
                    .map(|d| json!({
                        "id": d.id.to_string()[..8].to_string(),
                        "supplier": d.supplier_name.clone().unwrap_or_else(|| "—".into()),
                        "delivery_date": d.delivery_date.map(|x| x.to_string()),
                        "items_count": d.items.len(),
                        "total_cost_eur": format!("{:.2}", d.total_cost_cents as f64 / 100.0),
                    }))
                    .collect();
                let recent_sent: Vec<serde_json::Value> = drafts.iter()
                    .filter(|d| d.status == "sent")
                    .take(5)
                    .map(|d| json!({
                        "id": d.id.to_string()[..8].to_string(),
                        "supplier": d.supplier_name.clone().unwrap_or_else(|| "—".into()),
                        "items_count": d.items.len(),
                        "total_cost_eur": format!("{:.2}", d.total_cost_cents as f64 / 100.0),
                        "sent_at": d.created_at.date().to_string(),
                    }))
                    .collect();

                // 4. Dish margin warnings (food_cost_percent > 35% или цена ниже cost)
                let pagination = PaginationParams { page: Some(1), per_page: Some(100) };
                let dish_warnings: Vec<serde_json::Value> = match self.services.dishes
                    .list_dishes(ctx.tenant_id.clone(), true, &pagination)
                    .await
                {
                    Ok((dishes, _)) => {
                        let mut warnings: Vec<serde_json::Value> = dishes.iter()
                            .filter_map(|d| {
                                let fc_pct = d.food_cost_percent()?;
                                let margin_pct = d.profit_margin_percent().unwrap_or(0.0);
                                let level = if fc_pct >= 100.0 {
                                    "below_cost"
                                } else if fc_pct > 40.0 {
                                    "low_margin"
                                } else if fc_pct > 35.0 {
                                    "watch"
                                } else {
                                    return None;
                                };
                                Some(json!({
                                    "name": d.name().as_str(),
                                    "selling_price_eur": format!("{:.2}", d.selling_price().as_cents() as f64 / 100.0),
                                    "food_cost_percent": format!("{:.1}", fc_pct),
                                    "margin_percent": format!("{:.1}", margin_pct),
                                    "level": level,
                                }))
                            })
                            .collect();
                        // Сначала самые проблемные
                        warnings.sort_by(|a, b| {
                            let pa = a["food_cost_percent"].as_str().and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);
                            let pb = b["food_cost_percent"].as_str().and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);
                            pb.partial_cmp(&pa).unwrap_or(std::cmp::Ordering::Equal)
                        });
                        warnings.truncate(5);
                        warnings
                    }
                    Err(_) => vec![],
                };

                // 5. Сводка: подсчёт alerts
                let total_alerts = expiring.len() + low_stock.len() + open_drafts.len() + dish_warnings.len();

                Ok(ToolResult {
                    tool_name: name,
                    data: json!({
                        "briefing": {
                            "date": now.date().to_string(),
                            "total_alerts": total_alerts,
                            "expiring_soon": {
                                "count": expiring.len(),
                                "items": expiring,
                                "days_window": expiring_days,
                            },
                            "low_stock": {
                                "count": low_stock.len(),
                                "items": low_stock,
                            },
                            "open_purchase_drafts": {
                                "count": open_drafts.len(),
                                "items": open_drafts,
                            },
                            "recent_sent_purchases": {
                                "count": recent_sent.len(),
                                "items": recent_sent,
                            },
                            "dish_margin_warnings": {
                                "count": dish_warnings.len(),
                                "items": dish_warnings,
                            },
                        }
                    }),
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
            CopilotTool::AdjustInventoryQuantity => {
                self.execute_adjust_quantity(user_id, tenant_id, &plan.payload).await
            }
            CopilotTool::PreparePurchaseDraft => {
                self.execute_purchase_draft(user_id, tenant_id, &plan.payload).await
            }
            CopilotTool::SendPurchaseOrder => {
                self.execute_send_purchase_order(user_id, &plan.payload).await
            }
            CopilotTool::UpdateDishPrice => {
                self.execute_update_dish_price(tenant_id, &plan.payload).await
            }
            CopilotTool::CreateRecipe => {
                self.execute_create_recipe(user_id, tenant_id, &plan.payload).await
            }
            CopilotTool::CreateDish => {
                self.execute_create_dish(tenant_id, &plan.payload).await
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

    /// Создать purchase draft.
    /// Payload: { supplier_name?, delivery_date? (YYYY-MM-DD), note?, items: [{ ingredient_name, quantity, unit, price_per_unit_cents? }] }
    async fn execute_purchase_draft(
        &self,
        user_id: UserId,
        tenant_id: TenantId,
        payload: &serde_json::Value,
    ) -> AppResult<String> {
        let supplier = payload.get("supplier_name")
            .or_else(|| payload.get("supplier"))
            .and_then(|v| v.as_str())
            .map(String::from);

        let delivery_date = payload.get("delivery_date")
            .or_else(|| payload.get("date"))
            .and_then(|v| v.as_str())
            .and_then(|s| {
                let fmt = time::macros::format_description!("[year]-[month]-[day]");
                time::Date::parse(s, &fmt).ok()
            });

        let note = payload.get("note")
            .and_then(|v| v.as_str())
            .map(String::from)
            .or_else(|| Some("Created by Copilot".to_string()));

        let items_arr = payload.get("items")
            .and_then(|v| v.as_array())
            .ok_or_else(|| AppError::validation("purchase draft requires 'items' array"))?;

        if items_arr.is_empty() {
            return Err(AppError::validation("purchase draft must have at least one item"));
        }

        let mut items_input = Vec::with_capacity(items_arr.len());
        for raw in items_arr {
            let name = raw.get("ingredient_name")
                .or_else(|| raw.get("ingredient"))
                .or_else(|| raw.get("item_name"))
                .or_else(|| raw.get("product_name"))
                .or_else(|| raw.get("name"))
                .or_else(|| raw.get("item"))
                .or_else(|| raw.get("product"))
                .and_then(|v| v.as_str())
                .ok_or_else(|| AppError::validation("each item requires ingredient_name"))?
                .to_string();

            let quantity = raw.get("quantity")
                .and_then(|v| v.as_f64())
                .ok_or_else(|| AppError::validation("each item requires quantity"))?;

            let unit = raw.get("unit").and_then(|v| v.as_str()).unwrap_or("kg").to_string();

            let price_per_unit_cents = raw.get("price_per_unit_cents")
                .and_then(|v| v.as_i64());

            // Поиск catalog_id (опционально)
            let catalog_ingredient_id = self.find_catalog_id(&name).await.ok().map(|c| c.as_uuid());

            items_input.push(PurchaseDraftItemInput {
                catalog_ingredient_id,
                ingredient_name: name,
                quantity,
                unit,
                price_per_unit_cents,
            });
        }

        let count = items_input.len();
        let draft_id = self.services.purchase_drafts
            .create(user_id, tenant_id, CreatePurchaseDraftInput {
                supplier_name: supplier.clone(),
                delivery_date,
                note,
                items: items_input,
            })
            .await?;

        tracing::info!("✅ Copilot purchase draft: id={} items={} supplier={:?}", draft_id, count, supplier);

        Ok(format!(
            "Purchase draft created with {} item(s){}{}.",
            count,
            supplier.map(|s| format!(" for supplier {}", s)).unwrap_or_default(),
            delivery_date.map(|d| format!(" (delivery {})", d)).unwrap_or_default(),
        ))
    }

    /// Перевести purchase draft в статус 'sent'.
    /// Payload: { id: "<uuid>" } — резолвится в prepare_action_plan.
    async fn execute_send_purchase_order(
        &self,
        user_id: UserId,
        payload: &serde_json::Value,
    ) -> AppResult<String> {
        let id_str = payload.get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AppError::validation("send_purchase_order requires 'id'"))?;
        let draft_id: Uuid = id_str.parse()
            .map_err(|_| AppError::validation("invalid UUID for purchase draft id"))?;

        let (id, supplier, items_count) = self.services.purchase_drafts
            .mark_sent(draft_id, user_id)
            .await?;

        tracing::info!("✅ Copilot send_purchase_order: draft {} sent ({} items)", id, items_count);

        Ok(format!(
            "Purchase draft {} marked as sent ({} item(s){}). Status: draft → sent.",
            &id.to_string()[..8],
            items_count,
            supplier.map(|s| format!(", supplier {}", s)).unwrap_or_default(),
        ))
    }

    /// Скорректировать остаток до целевого значения.
    /// Payload: { ingredient_name, target_quantity, unit?, reason? }
    /// diff = target - current. Если diff > 0 — добавляем batch. Если diff < 0 — deduct_fifo.
    async fn execute_adjust_quantity(
        &self,
        user_id: UserId,
        tenant_id: TenantId,
        payload: &serde_json::Value,
    ) -> AppResult<String> {
        let name = payload.get("ingredient_name")
            .or_else(|| payload.get("ingredient"))
            .or_else(|| payload.get("name"))
            .or_else(|| payload.get("item_name"))
            .or_else(|| payload.get("product_name"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| AppError::validation("adjust_inventory_quantity requires 'ingredient_name'"))?;

        let target = payload.get("target_quantity")
            .or_else(|| payload.get("quantity"))
            .and_then(|v| v.as_f64())
            .ok_or_else(|| AppError::validation("adjust_inventory_quantity requires 'target_quantity'"))?;

        if target < 0.0 {
            return Err(AppError::validation("target_quantity must be non-negative"));
        }

        let unit = payload.get("unit").and_then(|v| v.as_str()).unwrap_or("kg").to_string();
        let reason = payload.get("reason").and_then(|v| v.as_str())
            .map(normalize_correction_reason)
            .unwrap_or("correction");

        let catalog_id = self.find_catalog_id(name).await?;
        let cat_uuid = catalog_id.as_uuid();

        // Текущий остаток
        let items = self.services.inventory
            .list_products_with_details(user_id.clone(), tenant_id.clone(), Language::En)
            .await?;
        let current: f64 = items.iter()
            .filter(|i| i.product.id == cat_uuid)
            .map(|i| i.remaining_quantity)
            .sum();

        let diff = target - current;
        if diff.abs() < 0.0001 {
            return Ok(format!("No change: {} already at {} {}.", name, current, unit));
        }

        if diff > 0.0 {
            // Добавляем batch с положительной корректировкой.
            // price=0 (correction), expires_at = +365 days.
            let now = time::OffsetDateTime::now_utc();
            let expires = now + time::Duration::days(365);
            self.services.inventory
                .add_product(
                    user_id.clone(),
                    tenant_id.clone(),
                    catalog_id,
                    0,
                    diff,
                    now,
                    expires,
                )
                .await?;
            tracing::info!("✅ Copilot adjust +{} {} of {} (reason={})", diff, unit, name, reason);
        } else {
            // FIFO write-off на |diff|
            self.services.inventory
                .deduct_fifo(
                    tenant_id.clone(),
                    catalog_id,
                    diff.abs(),
                    None,
                    Some(reason.to_string()),
                    Some(format!("Copilot correction: {} ({})", name, reason)),
                )
                .await?;
            tracing::info!("✅ Copilot adjust {} {} of {} (reason={})", diff, unit, name, reason);
        }

        Ok(format!(
            "Adjusted {}: {} {} → {} {} (diff {:+.3} {}, reason: {}).",
            name, current, unit, target, unit, diff, unit, reason
        ))
    }

    /// Update dish selling price.
    /// Payload (resolved by prepare_action_plan): { dish_id: "<uuid>", dish_name, new_price_cents }
    async fn execute_update_dish_price(
        &self,
        tenant_id: TenantId,
        payload: &serde_json::Value,
    ) -> AppResult<String> {
        let dish_id_str = payload.get("dish_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AppError::validation("update_dish_price requires resolved 'dish_id'"))?;
        let dish_uuid: Uuid = dish_id_str.parse()
            .map_err(|_| AppError::validation("invalid UUID for dish_id"))?;

        let new_price_cents = payload.get("new_price_cents")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| AppError::validation("update_dish_price requires 'new_price_cents'"))?;

        if new_price_cents <= 0 {
            return Err(AppError::validation("new_price_cents must be greater than 0"));
        }

        let dish_name = payload.get("dish_name")
            .and_then(|v| v.as_str())
            .unwrap_or("dish")
            .to_string();

        let new_price = crate::domain::Money::from_cents(new_price_cents)?;
        let dish_id = crate::domain::DishId::from_uuid(dish_uuid);

        let updated = self.services.dishes
            .set_selling_price(dish_id, tenant_id, new_price)
            .await?;

        let margin_str = updated.profit_margin_percent()
            .map(|m| format!(", new margin {:.1}%", m))
            .unwrap_or_default();

        tracing::info!(
            "✅ Copilot update_dish_price: {} → €{:.2}{}",
            dish_name,
            new_price_cents as f64 / 100.0,
            margin_str,
        );

        Ok(format!(
            "Updated price of '{}' to €{:.2}{}.",
            dish_name,
            new_price_cents as f64 / 100.0,
            margin_str,
        ))
    }
}

/// Нормализовать причину correction к одному из допустимых значений.
fn normalize_correction_reason(input: &str) -> &'static str {
    let s = input.to_lowercase();
    if s.contains("inventory") || s.contains("инвентар") || s.contains("check") || s.contains("audit") {
        "inventory_check"
    } else if s.contains("manual") || s.contains("count") || s.contains("ручн") || s.contains("пересчёт") || s.contains("пересчет") {
        "manual_count"
    } else {
        "correction"
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

/// Внутренний результат пред-валидации для SendPurchaseOrder.
enum SendPreview {
    /// Готов к подтверждению — есть актуальный draft + сформированный preview.
    Ok(Uuid, Vec<ActionChange>),
    /// Draft уже не в статусе 'draft' (sent/cancelled/etc) — write tool пропускается.
    AlreadyProcessed { id: Uuid, status: String },
    /// Draft по id или 'last' не найден.
    NotFound,
}

/// Внутренний результат пред-валидации для AdjustInventoryQuantity.
enum AdjustPreview {
    Ok(Vec<ActionChange>),
    /// Целевой остаток уже равен текущему — диффа нет, действие не нужно.
    Noop { name: String, current: f64 },
    NotFound { name: String },
}

/// Внутренний результат пред-валидации для UpdateDishPrice.
enum DishPriceUpdatePreview {
    /// Ровно один matching dish, цена отличается, валидна — готов к подтверждению.
    Ok {
        dish_id: Uuid,
        dish_name: String,
        new_price_cents: i64,
        changes: Vec<ActionChange>,
    },
    /// Цена уже равна целевой — действие не нужно.
    SamePrice { name: String, current_cents: i64 },
    /// Несколько блюд подходят под запрос — нужна disambiguation.
    Multiple { query: String, candidates: Vec<(String, i64)> },
    /// Не найдено ни одного блюда.
    NotFound { query: String },
    /// new_price ≤ 0 или некорректные args.
    Invalid { reason: String },
}

/// Резолвнутый ингредиент рецепта: catalog_id + конвертированное в default_unit количество.
struct ResolvedRecipeIngredient {
    catalog_ingredient_id: Uuid,
    matched_name: String,
    quantity_in_default_unit: f64,
    default_unit: String,
}

/// Внутренний результат пред-валидации для CreateRecipe.
enum CreateRecipePreview {
    /// Все ингредиенты резолвлены, имя уникально, готов к подтверждению.
    Ok {
        recipe_name: String,
        servings: u32,
        resolved: Vec<ResolvedRecipeIngredient>,
        recipe_changes: Vec<ActionChange>,
    },
    /// Рецепт с таким именем уже существует (case-insensitive).
    AlreadyExists { name: String },
    /// Один из ингредиентов не найден в каталоге.
    IngredientNotFound { query: String, suggestions: Vec<String> },
    /// Несколько вариантов в каталоге — нужна disambiguation.
    AmbiguousIngredient { query: String, candidates: Vec<String> },
    /// Некорректные args (servings/quantity ≤ 0, missing fields, bad unit).
    Invalid { reason: String },
}

/// Внутренний результат пред-валидации для CreateDish.
enum CreateDishPreview {
    /// Recipe найден, имя блюда уникально, цена валидна — готов к подтверждению.
    Ok {
        dish_name: String,
        recipe_id: Uuid,
        recipe_name: String,
        selling_price_cents: i64,
        description: Option<String>,
        dish_changes: Vec<ActionChange>,
    },
    /// Рецепт с таким именем не найден.
    RecipeNotFound { query: String },
    /// Несколько рецептов подошли под запрос.
    AmbiguousRecipe { query: String, candidates: Vec<String> },
    /// Блюдо с таким именем уже существует.
    DuplicateDishName { name: String },
    /// Некорректные args (цена ≤ 0, missing fields).
    Invalid { reason: String },
}

/// Конвертирует quantity из user_unit в target_unit (default_unit ингредиента).
/// Поддерживает g↔kg и l↔ml. Прочие комбинации требуют точного совпадения юнитов.
fn convert_quantity(
    qty: f64,
    from: crate::domain::catalog::Unit,
    to: crate::domain::catalog::Unit,
) -> Result<f64, String> {
    use crate::domain::catalog::Unit;
    if from == to {
        return Ok(qty);
    }
    match (from, to) {
        (Unit::Kilogram, Unit::Gram) => Ok(qty * 1000.0),
        (Unit::Gram, Unit::Kilogram) => Ok(qty / 1000.0),
        (Unit::Liter, Unit::Milliliter) => Ok(qty * 1000.0),
        (Unit::Milliliter, Unit::Liter) => Ok(qty / 1000.0),
        _ => Err(format!(
            "cannot convert {} to {} (incompatible unit families)",
            from.as_str(), to.as_str()
        )),
    }
}

fn tool_to_plan_type(tool: &CopilotTool) -> ActionPlanType {
    match tool {
        CopilotTool::PrepareInventoryUpdate  => ActionPlanType::AddInventoryItems,
        CopilotTool::AdjustInventoryQuantity => ActionPlanType::AdjustInventoryQuantity,
        CopilotTool::WriteOffInventory       => ActionPlanType::WriteOffInventory,
        CopilotTool::PreparePurchaseDraft    => ActionPlanType::CreatePurchaseDraft,
        CopilotTool::SendPurchaseOrder       => ActionPlanType::SendPurchaseOrder,
        CopilotTool::UpdateDishPrice         => ActionPlanType::UpdateDishPrice,
        CopilotTool::CreateRecipe            => ActionPlanType::CreateRecipe,
        CopilotTool::CreateDish              => ActionPlanType::CreateDish,
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
        CopilotTool::PreparePurchaseDraft => {
            let supplier = args.get("supplier_name")
                .or_else(|| args.get("supplier"))
                .and_then(|v| v.as_str())
                .unwrap_or("—");
            let delivery = args.get("delivery_date")
                .or_else(|| args.get("date"))
                .and_then(|v| v.as_str())
                .unwrap_or("—");

            let items = match args.get("items").and_then(|v| v.as_array()) {
                Some(arr) => arr.clone(),
                None => return vec![],
            };

            let mut changes: Vec<ActionChange> = items.iter().filter_map(|it| {
                let name = it.get("ingredient_name")
                    .or_else(|| it.get("ingredient"))
                    .or_else(|| it.get("name"))
                    .and_then(|v| v.as_str())?;
                let qty = it.get("quantity").and_then(|v| v.as_f64())?;
                let unit = it.get("unit").and_then(|v| v.as_str()).unwrap_or("kg");
                Some(ActionChange {
                    entity: format!("Purchase: {}", name),
                    field: "quantity".to_string(),
                    before: None,
                    after: format!("{} {}", qty, unit),
                    unit: Some(unit.to_string()),
                })
            }).collect();

            // Header row
            changes.insert(0, ActionChange {
                entity: "Purchase draft".to_string(),
                field: "supplier / delivery".to_string(),
                before: None,
                after: format!("{} / {}", supplier, delivery),
                unit: None,
            });
            changes
        }
        _ => vec![],
    }
}

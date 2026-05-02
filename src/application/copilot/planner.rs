//! Planner — главный LLM решает что делать.
//!
//! Берёт: CopilotContext + user message + tool catalog
//! Возвращает: ToolPlan (список tools + args + risk level)
//!
//! Structured output — Gemini отвечает строго JSON.
//! Backend валидирует и нормализует — никогда не доверяет сырому LLM output.

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::collections::HashMap;

use crate::infrastructure::gemini_service::GeminiService;
use crate::shared::AppError;

use super::context::CopilotContext;
use super::tools::CopilotTool;

/// Аргументы для одного tool call (произвольный JSON map).
pub type ToolArgs = HashMap<String, serde_json::Value>;

/// Один вызов tool в плане.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub tool: CopilotTool,
    pub args: ToolArgs,
}

/// Полный план от LLM Planner.
#[derive(Debug, Clone)]
pub struct ToolPlan {
    /// Намерение пользователя (для аудит-лога).
    pub intent: String,
    /// Список tools в порядке выполнения.
    pub tools: Vec<CopilotTool>,
    /// Аргументы для каждого tool (по имени).
    pub tool_calls: Vec<ToolCall>,
    /// Нужна ли confirmation (если есть write tools — всегда true).
    pub requires_confirmation: bool,
}

impl ToolPlan {
    /// Есть ли AI tools требующие Gemini вызова?
    pub fn requires_ai_tools(&self) -> bool {
        use CopilotTool::*;
        self.tools.iter().any(|t| matches!(t,
            SuggestCookFromInventory | GenerateMealPlan | AnalyzeRecipe |
            GeneralChefAnswer | GenerateFoodPairing | GenerateLabRecipe |
            SimulateLabProduct | GenerateProductReport | Generate3DFoodModel
        ))
    }
}

/// Raw структура которую возвращает LLM.
#[derive(Debug, Deserialize)]
struct PlannerLlmResponse {
    intent: String,
    tools: Vec<String>,
    args: Option<HashMap<String, serde_json::Value>>,
    requires_confirmation: Option<bool>,
}

pub struct CopilotPlanner {
    gemini: Arc<GeminiService>,
}

impl CopilotPlanner {
    pub fn new(gemini: Arc<GeminiService>) -> Self {
        Self { gemini }
    }

    /// Вызвать LLM и получить ToolPlan.
    pub async fn plan(
        &self,
        ctx: &CopilotContext,
        message: &str,
    ) -> Result<ToolPlan, AppError> {
        let system_prompt = self.build_system_prompt(ctx);
        let request_body = serde_json::json!({
            "model": "gemini-3-flash-preview",
            "messages": [
                {
                    "role": "system",
                    "content": system_prompt
                },
                {
                    "role": "user",
                    "content": message
                }
            ],
            "temperature": 0.1,
            "max_tokens": 2048,
            "response_format": { "type": "json_object" }
        });

        let raw = self.gemini.send_raw_request(&request_body).await?;
        self.parse_plan(&raw)
    }

    fn build_system_prompt(&self, ctx: &CopilotContext) -> String {
        let tool_catalog = CopilotTool::tool_catalog_prompt();
        format!(
r#"You are ChefOS Copilot. Output ONLY compact valid JSON. No markdown. No explanations.

CONTEXT: {context}

TOOLS: {tools}

RULES:
- Pick minimal tools needed.
- WRITE tools MUST have requires_confirmation=true.
- intent: short snake_case label (e.g. "inventory_add", "inventory_view", "inventory_writeoff", "dish_price_update").
- args: short keys, English values only.
- Unknown request → use "general_chef_answer".
- Keep total response under 400 tokens.

INVENTORY ARGS SCHEMA (use exactly these keys):
- prepare_inventory_update: {{ "ingredient_name": "<en>", "quantity": <number>, "unit": "<kg|l|pcs>" }}
- adjust_inventory_quantity:{{ "ingredient_name": "<en>", "target_quantity": <number>, "unit": "<kg|l|pcs>", "reason": "<correction|manual_count|inventory_check>" }}
- write_off_inventory:      {{ "ingredient_name": "<en>", "quantity": <number>, "unit": "<kg|l|pcs>", "reason": "<expired|waste|used_in_production|correction|manual>" }}
- update_dish_price:        {{ "dish_name": "<English keyword from dish, partial ok>", "new_price_cents": <int>, "currency": "EUR" }}
- prepare_purchase_draft:   {{ "supplier_name": "<str|null>", "delivery_date": "<YYYY-MM-DD|null>", "note": "<str|null>", "items": [{{ "ingredient_name": "<en>", "quantity": <number>, "unit": "<kg|l|pcs>" }}] }}
- list_purchase_drafts:     {{ "limit": <int, optional, default 10> }}
- get_purchase_draft:       {{ "id": "<uuid|'last'>" }}
- send_purchase_order:      {{ "id": "<uuid|'last'>" }}
- get_daily_briefing:       {{ "expiring_days": <int, optional, default 3>, "low_stock_threshold": <number, optional, default 1.0> }}

DAILY BRIEFING INTENT HINTS (HIGHEST PRIORITY for these phrases):
- "Что сегодня важно / что важно сегодня / brief me / brief me on today / дневной отчёт / дневной отчет / daily briefing / daily report / что нужно сделать / what should I do today / итоги дня / today summary / overview / обзор / morning briefing / утренний обзор" → get_daily_briefing
- This is the PRIMARY tool for any request that asks for an operational overview without specifying a domain.

PURCHASE DRAFT INTENT HINTS:
- "show / list / which drafts / мои черновики / какие закупки" → list_purchase_drafts
- "last / latest / последняя / на завтра" without explicit id → get_purchase_draft with id='last'
- "создай / сделай / order / create purchase" → prepare_purchase_draft
- "отправь / подтверди закупку / переведи в sent / send / mark as sent" → send_purchase_order (requires_confirmation=true)

DISH PRICE INTENT HINTS:
- "Поменяй цену / измени цену / change price / set price / update price / переставь цену / make ... cost" → update_dish_price (requires_confirmation=true)
- Always pass new_price_cents as int (e.g. "18 евро" → 1800, "9.50€" → 950).
- ALWAYS translate dish_name to English keyword(s) (e.g. "Цезарь" → "Caesar", "салат" → "Salad", "борщ" → "Borscht"). Use the most distinctive English keyword. Backend matches by case-insensitive substring against English dish names.
- dish_name can be partial; backend will resolve and ask for clarification if multiple matches.

INVENTORY ADJUSTMENT INTENT HINTS:
- "Исправь / поставь / set / должно быть / fix / correct / adjust / должно стать / make it" with a TARGET quantity → adjust_inventory_quantity
- "After inventory check / после инвентаризации" → adjust_inventory_quantity with reason=inventory_check
- "Add / Buy / Купи / приехало" (delta, not target) → prepare_inventory_update
- "Spoiled / expired / списать / выкинь" → write_off_inventory

OUTPUT FORMAT (exactly):
{{"intent":"<snake_case>","tools":["tool_name"],"args":{{"tool_name":{{"key":"value"}}}},"requires_confirmation":false}}"#,
            context = ctx.to_prompt_context(),
            tools = tool_catalog,
        )
    }

    fn parse_plan(&self, raw: &str) -> Result<ToolPlan, AppError> {
        // Извлечь JSON из ответа (Gemini иногда оборачивает в markdown)
        let json_str = extract_json(raw);

        let parsed: PlannerLlmResponse = serde_json::from_str(&json_str)
            .map_err(|e| {
                tracing::warn!("Planner JSON parse failed: {e}\nRaw: {raw}");
                AppError::internal(format!("Planner response parse error: {e}"))
            })?;

        // Парсить tool names → CopilotTool enum
        let tools: Vec<CopilotTool> = parsed.tools.iter()
            .filter_map(|name| parse_tool_name(name))
            .collect();

        if tools.is_empty() {
            // Fallback — общий ответ
            return Ok(ToolPlan {
                intent: parsed.intent,
                tools: vec![CopilotTool::GeneralChefAnswer],
                tool_calls: vec![ToolCall {
                    tool: CopilotTool::GeneralChefAnswer,
                    args: HashMap::new(),
                }],
                requires_confirmation: false,
            });
        }

        let has_write = tools.iter().any(|t| t.is_write());
        let requires_confirmation = has_write || parsed.requires_confirmation.unwrap_or(false);

        let args_map = parsed.args.unwrap_or_default();
        let tool_calls: Vec<ToolCall> = tools.iter().map(|t| {
            let args = args_map.get(t.name())
                .and_then(|v| v.as_object())
                .map(|m| m.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
                .unwrap_or_default();
            ToolCall { tool: t.clone(), args }
        }).collect();

        Ok(ToolPlan {
            intent: parsed.intent,
            tools,
            tool_calls,
            requires_confirmation,
        })
    }
}

/// Парсить строковое имя tool → CopilotTool.
fn parse_tool_name(name: &str) -> Option<CopilotTool> {
    match name {
        "get_inventory"              => Some(CopilotTool::GetInventory),
        "get_expiring_soon"          => Some(CopilotTool::GetExpiringSoon),
        "search_ingredients"         => Some(CopilotTool::SearchIngredients),
        "get_dishes"                 => Some(CopilotTool::GetDishes),
        "get_recipes"                => Some(CopilotTool::GetRecipes),
        "get_recipe_by_id"           => Some(CopilotTool::GetRecipeById),
        "get_lab_experiment"         => Some(CopilotTool::GetLabExperiment),
        "list_purchase_drafts"       => Some(CopilotTool::ListPurchaseDrafts),
        "get_purchase_draft"         => Some(CopilotTool::GetPurchaseDraft),
        "get_daily_briefing"         => Some(CopilotTool::GetDailyBriefing),
        "suggest_cook_from_inventory" => Some(CopilotTool::SuggestCookFromInventory),
        "generate_meal_plan"         => Some(CopilotTool::GenerateMealPlan),
        "analyze_recipe"             => Some(CopilotTool::AnalyzeRecipe),
        "general_chef_answer"        => Some(CopilotTool::GeneralChefAnswer),
        "generate_food_pairing"      => Some(CopilotTool::GenerateFoodPairing),
        "prepare_inventory_update"   => Some(CopilotTool::PrepareInventoryUpdate),
        "adjust_inventory_quantity"  => Some(CopilotTool::AdjustInventoryQuantity),
        "prepare_purchase_draft"     => Some(CopilotTool::PreparePurchaseDraft),
        "update_dish_price"          => Some(CopilotTool::UpdateDishPrice),
        "write_off_inventory"        => Some(CopilotTool::WriteOffInventory),
        "send_purchase_order"        => Some(CopilotTool::SendPurchaseOrder),
        "generate_lab_recipe"        => Some(CopilotTool::GenerateLabRecipe),
        "generate_3d_food_model"     => Some(CopilotTool::Generate3DFoodModel),
        "simulate_lab_product"       => Some(CopilotTool::SimulateLabProduct),
        "generate_product_report"    => Some(CopilotTool::GenerateProductReport),
        unknown => {
            tracing::warn!("Planner returned unknown tool: {unknown}");
            None
        }
    }
}

/// Извлечь JSON из raw ответа (убрать ```json ... ``` если есть).
fn extract_json(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.starts_with("```") {
        trimmed
            .lines()
            .skip(1)
            .take_while(|l| !l.starts_with("```"))
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        trimmed.to_string()
    }
}

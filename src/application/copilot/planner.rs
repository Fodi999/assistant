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
    /// Immediate workspace/scene commands (geometry_op, spawn_shape, switch_lab, …).
    /// Executed by the frontend immediately without confirmation.
    pub workspace_commands: Vec<serde_json::Value>,
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
    workspace_commands: Option<Vec<serde_json::Value>>,
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
r##"You are ChefOS Copilot. Output ONLY compact valid JSON. No markdown. No explanations.

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
- create_recipe:            {{ "recipe_name": "<English title>", "servings": <positive int>, "ingredients": [{{ "ingredient_name": "<English catalog name>", "quantity": <number>, "unit": "<g|kg|ml|l|pcs>" }}] }}
- create_dish:              {{ "dish_name": "<English name of the dish>", "recipe_name": "<English name of existing recipe>", "selling_price_cents": <positive int>, "description": "<optional str>" }}

DAILY BRIEFING INTENT HINTS (HIGHEST PRIORITY for these phrases):
- "что сегодня важно / brief me / daily briefing / daily report / что нужно сделать / today summary / overview / morning briefing" → get_daily_briefing
- This is the PRIMARY tool for any request that asks for an operational overview without specifying a domain.

PURCHASE DRAFT INTENT HINTS:
- "show / list / which drafts / my drafts / list purchases" → list_purchase_drafts
- "last / latest / last draft" without explicit id → get_purchase_draft with id='last'
- "create / order / make purchase" → prepare_purchase_draft
- "send / confirm purchase / mark as sent" → send_purchase_order (requires_confirmation=true)

DISH PRICE INTENT HINTS:
- "change price / set price / update price / make cost" → update_dish_price (requires_confirmation=true)
- Always pass new_price_cents as int (e.g. "18 euro" → 1800, "9.50 euro" → 950).
- ALWAYS translate dish_name to English keyword(s). Backend matches by case-insensitive substring.

CREATE RECIPE INTENT HINTS:
- "create recipe / new recipe / add recipe / make a recipe" + LIST OF INGREDIENTS → create_recipe (requires_confirmation=true)
- ALWAYS translate recipe_name AND every ingredient_name to English.
- servings: parse from context, default to 1.

CREATE DISH INTENT HINTS:
- "create dish / add dish / add to menu / put on menu" → create_dish (requires_confirmation=true)
- dish_name: translate to English. recipe_name: existing recipe, translate to English.
- selling_price_cents: convert to cents.

INVENTORY ADJUSTMENT INTENT HINTS:
- "fix / set / correct / adjust / should be / make it" with TARGET quantity → adjust_inventory_quantity
- "after inventory check" → adjust_inventory_quantity with reason=inventory_check
- "add / buy / arrived" (delta) → prepare_inventory_update
- "spoiled / expired / write off" → write_off_inventory

LAB 3D GEOMETRY (spawn_shape / geometry_op):
These are FRONTEND-ONLY workspace commands — requires_confirmation=false always.
Emit them under "workspace_commands" array key alongside the normal plan.

spawn_shape — simple primitives (sharp, no bevel):
  shapes: "cube","sphere","circle","square","triangle","rectangle","line"
  {{ "type": "spawn_shape", "shape": "<shape>", "label": "<English>", "color": "<hex>" }}

geometry_op — CSG boolean or bevel shaping:
  {{ "type": "geometry_op", "op": {{
      "operation": "subtract",
      "target": {{ "type": "shape_cube", "color": "#38BDF8", "subdivisions": 4, "bevel": 0.0 }},
      "cutter": {{ "type": "cylinder", "radius": 0.2, "height": 1.2, "center": [0,0,0], "cap_color": "#1E40AF" }},
      "quality": "high",
      "label": "Cube with hole"
  }} }}

BEVEL / ROUNDED CORNERS RULES:
- "round corners / bevel / chamfer / rounded cube / pill shape" → geometry_op with bevel > 0
  light=0.15 subs=3 | medium=0.35 subs=4 | strong=0.65 subs=5 | sphere=0.95 subs=5
- ALWAYS set subdivisions >= 3 when bevel > 0 (otherwise corners look jagged).
- Rounded cube without hole: use operation="union" with tiny dummy cutter:
  {{ "type": "geometry_op", "op": {{ "operation": "union",
    "target": {{ "type": "shape_cube", "color": "#38BDF8", "subdivisions": 4, "bevel": 0.35 }},
    "cutter": {{ "type": "box", "half_extents": [0.001,0.001,0.001] }},
    "quality": "high", "label": "Rounded cube" }} }}
- Vision feedback loop shows you the result — if not rounded enough, increase bevel in correction.
- Plain cube (no rounding) → spawn_shape is sufficient.

OUTPUT FORMAT (exactly):
{{"intent":"<snake_case>","tools":["tool_name"],"args":{{"tool_name":{{"key":"value"}}}},"requires_confirmation":false,"workspace_commands":[]}}"##,
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
                workspace_commands: parsed.workspace_commands.unwrap_or_default(),
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
            workspace_commands: parsed.workspace_commands.unwrap_or_default(),
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
        "create_recipe"              => Some(CopilotTool::CreateRecipe),
        "create_dish"                => Some(CopilotTool::CreateDish),
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

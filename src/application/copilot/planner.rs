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
            "model": "gemini-2.0-flash",
            "contents": [{
                "role": "user",
                "parts": [{ "text": format!("{}\n\nUser message: {}", system_prompt, message) }]
            }],
            "generationConfig": {
                "temperature": 0.1,
                "maxOutputTokens": 1024,
                "responseMimeType": "application/json"
            }
        });

        let raw = self.gemini.send_raw_request(&request_body).await?;
        self.parse_plan(&raw)
    }

    fn build_system_prompt(&self, ctx: &CopilotContext) -> String {
        let tool_catalog = CopilotTool::tool_catalog_prompt();
        format!(
r#"You are ChefOS Copilot — an AI assistant for restaurant and food-tech professionals.

CONTEXT:
{context}

AVAILABLE TOOLS:
{tools}

INSTRUCTIONS:
- Analyze the user message and select the minimal set of tools needed.
- For WRITE tools, always set requires_confirmation to true.
- READ tools can be used without confirmation.
- Respond ONLY with valid JSON in this exact format:
{{
  "intent": "<short description of what the user wants>",
  "tools": ["tool_name_1", "tool_name_2"],
  "args": {{
    "tool_name_1": {{ "key": "value" }},
    "tool_name_2": {{ "key": "value" }}
  }},
  "requires_confirmation": false
}}
- If you don't know which tool to use, use "general_chef_answer".
- Never invent tool names. Use only the tools listed above.
- Language for your answer: {locale}"#,
            context = ctx.to_prompt_context(),
            tools = tool_catalog,
            locale = ctx.locale.code(),
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
        "suggest_cook_from_inventory" => Some(CopilotTool::SuggestCookFromInventory),
        "generate_meal_plan"         => Some(CopilotTool::GenerateMealPlan),
        "analyze_recipe"             => Some(CopilotTool::AnalyzeRecipe),
        "general_chef_answer"        => Some(CopilotTool::GeneralChefAnswer),
        "generate_food_pairing"      => Some(CopilotTool::GenerateFoodPairing),
        "prepare_inventory_update"   => Some(CopilotTool::PrepareInventoryUpdate),
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

//! AI Brain — Layer 2 intelligence for ChefOS Chat.
//!
//! Activated ONLY when Layer 1 (rule-based intent_router) returns `Intent::Unknown`.
//! This means 60-70% of requests never touch LLM — pure speed.
//!
//! Architecture:
//!   Layer 0 (Off-Topic Gate):  3-tier keyword filter → 0ms, $0
//!   Layer 1 (Fast System):     intent_router → hardcoded handlers → 0ms, $0
//!   Layer 2 (AI Brain):        LLM reasoning → tool selection → data fetch → response
//!
//! Module structure (DDD):
//!   - tool_types       — AiAction, ToolChoice enum, defaults
//!   - off_topic_gate   — 3-tier filter: LowQuality / OutOfScope / Borderline
//!   - tool_executors   — execute_search, execute_nutrition, execute_conversion, execute_meal_plan
//!   - response_helpers — text formatting, suggestions, fallback messages
//!   - parsing          — JSON extraction from raw LLM output
//!
//! Cost: ~$0.001-0.003 per request (Groq llama-3.3-70b = practically free)
//! Latency: ~200-500ms (Groq is fast)

mod tool_types;
mod off_topic_gate;
mod tool_executors;
mod response_helpers;
mod parsing;

use std::sync::Arc;

use crate::infrastructure::IngredientCache;
use crate::infrastructure::llm_adapter::LlmAdapter;
use super::intent_router::ChatLang;
use super::chat_response::ChatResponse;
use super::session_context::SessionContext;

use tool_types::{ToolChoice, action_name};
use off_topic_gate::{classify_off_topic, OffTopicTier, respond_low_quality, respond_out_of_scope, respond_borderline};
use response_helpers::{truncate, build_context_hint};
use parsing::parse_ai_action;

// ── AI Brain ─────────────────────────────────────────────────────────────────

pub struct AiBrain {
    ingredient_cache: Arc<IngredientCache>,
    llm_adapter: Arc<LlmAdapter>,
}

impl AiBrain {
    pub fn new(ingredient_cache: Arc<IngredientCache>, llm_adapter: Arc<LlmAdapter>) -> Self {
        Self { ingredient_cache, llm_adapter }
    }

    /// Main entry point — called when Layer 1 returns Intent::Unknown.
    ///
    /// Flow:
    ///   1. Off-topic gate (3 tiers, 0ms, $0)
    ///   2. LLM decides tool → execute tool → LLM formats response
    pub async fn handle(&self, input: &str, lang: ChatLang, ctx: &SessionContext) -> ChatResponse {
        // ── Step 0: Off-Topic Gate — filter garbage/irrelevant BEFORE LLM ──
        match classify_off_topic(input) {
            OffTopicTier::LowQuality => {
                tracing::info!("🚫 Off-topic gate: LowQuality → {:?}", &input[..input.len().min(40)]);
                return respond_low_quality(lang);
            }
            OffTopicTier::OutOfScope => {
                tracing::info!("🚫 Off-topic gate: OutOfScope → {:?}", &input[..input.len().min(40)]);
                return respond_out_of_scope(lang);
            }
            OffTopicTier::Borderline => {
                tracing::info!("🔀 Off-topic gate: Borderline → {:?}", &input[..input.len().min(40)]);
                return respond_borderline(input, lang);
            }
            OffTopicTier::Pass => {
                // Continue to AI Brain
            }
        }

        // ── Step 1: Ask LLM which tool to use ──
        let action = match self.decide_action(input, lang, ctx).await {
            Ok(action) => action,
            Err(e) => {
                tracing::warn!("🧠 AI Brain decision failed: {}", e);
                return tool_executors::fallback_response(&self.llm_adapter, input, lang).await;
            }
        };

        tracing::info!(
            "🧠 AI Brain decided: tool={}, reasoning='{}'",
            action_name(&action.tool),
            truncate(&action.reasoning, 80),
        );

        // ── Step 2: Execute the chosen tool ──
        match action.tool {
            ToolChoice::SearchProducts { query, goal, limit } => {
                tool_executors::execute_search(
                    &self.ingredient_cache, &self.llm_adapter,
                    &query, &goal, limit.min(5), lang,
                ).await
            }
            ToolChoice::GetNutrition { product } => {
                tool_executors::execute_nutrition(
                    &self.ingredient_cache, &self.llm_adapter,
                    &product, lang,
                ).await
            }
            ToolChoice::ConvertUnits { value, from, to } => {
                tool_executors::execute_conversion(value, &from, &to, lang)
            }
            ToolChoice::GeneralAnswer { answer } => {
                ChatResponse::text_only(answer, super::intent_router::Intent::Unknown, lang, 0)
            }
            ToolChoice::MealPlan { goal, meals } => {
                tool_executors::execute_meal_plan(
                    &self.ingredient_cache, &self.llm_adapter,
                    input, &goal, meals.min(5), lang,
                ).await
            }
        }
    }

    // ── Step 1: LLM Decision ─────────────────────────────────────────────────

    async fn decide_action(
        &self,
        input: &str,
        lang: ChatLang,
        ctx: &SessionContext,
    ) -> Result<tool_types::AiAction, String> {
        let catalog_summary = self.build_catalog_summary().await;
        let context_hint = build_context_hint(ctx);

        let prompt = format!(
            r#"You are ChefOS AI Brain — a culinary assistant with access to tools.

USER INPUT: "{input}"
USER LANGUAGE: {lang}
{context_hint}

AVAILABLE TOOLS:

1. search_products — Search our ingredient database ({catalog_count} products).
   params: {{ "query": "...", "goal": "high_protein|low_calorie|balanced", "limit": 1-5 }}
   USE FOR: "что есть для похудения", "high protein foods", "healthy snacks"

2. get_nutrition — Get nutrition data for a specific product.
   params: {{ "product": "salmon" }}
   USE FOR: "what's in chicken", "nutrition of rice", specific product questions

3. convert_units — Convert between culinary units.
   params: {{ "value": 200, "from": "g", "to": "tbsp" }}
   USE FOR: "200g to tablespoons", "cups to ml", unit conversion

4. general_answer — Answer from your own knowledge (no database needed).
   params: {{ "answer": "Your answer here in the USER LANGUAGE" }}
   USE FOR: cooking tips, technique questions, "how to make dough", food science
   RULE: ONLY about food/cooking/nutrition. If NOT food-related, politely decline.

5. meal_plan — Create a meal plan with products from our database.
   params: {{ "goal": "lose weight", "meals": 3 }}
   USE FOR: "make me a diet plan", "menu for the week", "what to eat today"

TOP PRODUCTS IN DATABASE:
{catalog_summary}

RULES:
- Pick EXACTLY ONE tool
- If the question is about a specific product from our database → get_nutrition
- If the question is about finding products by goal → search_products
- If the question is about meal planning / menu → meal_plan
- If the question is about unit conversion → convert_units
- If the question is general cooking knowledge → general_answer
- If NOT about food/cooking → general_answer with a polite decline
- Answer in the USER LANGUAGE ({lang})

OUTPUT FORMAT (strict JSON, no other text):
{{
  "tool": {{
    "name": "search_products|get_nutrition|convert_units|general_answer|meal_plan",
    "params": {{ ... }}
  }},
  "reasoning": "short explanation why this tool"
}}"#,
            input = input,
            lang = lang.code(),
            context_hint = context_hint,
            catalog_count = self.ingredient_cache.len().await,
            catalog_summary = catalog_summary,
        );

        let raw = self.llm_adapter
            .groq_raw_request_with_model(&prompt, 500, "llama-3.3-70b-versatile")
            .await
            .map_err(|e| format!("LLM error: {}", e))?;

        parse_ai_action(&raw)
    }

    // ── Helpers ──────────────────────────────────────────────────────────────

    /// Build a compact catalog summary for the LLM (top products per category).
    async fn build_catalog_summary(&self) -> String {
        let all = self.ingredient_cache.all().await;
        if all.is_empty() {
            return "No products loaded.".to_string();
        }

        let mut by_protein: Vec<_> = all.iter()
            .filter(|p| p.protein_per_100g > 15.0)
            .collect();
        by_protein.sort_by(|a, b| b.protein_per_100g.partial_cmp(&a.protein_per_100g).unwrap_or(std::cmp::Ordering::Equal));

        let mut low_cal: Vec<_> = all.iter()
            .filter(|p| p.calories_per_100g > 0.0 && p.calories_per_100g < 80.0)
            .collect();
        low_cal.sort_by(|a, b| a.calories_per_100g.partial_cmp(&b.calories_per_100g).unwrap_or(std::cmp::Ordering::Equal));

        let mut lines = Vec::new();
        lines.push("High protein:".to_string());
        for p in by_protein.iter().take(5) {
            lines.push(format!("  {} — {}g protein, {}kcal", p.name_en, p.protein_per_100g, p.calories_per_100g as i32));
        }
        lines.push("Low calorie:".to_string());
        for p in low_cal.iter().take(5) {
            lines.push(format!("  {} — {}kcal, {}g protein", p.name_en, p.calories_per_100g as i32, p.protein_per_100g));
        }

        lines.join("\n")
    }
}

//! SousChefPlannerService — orchestrator.
//!
//! Flow: cache check -> build_variants (0 SQL) -> Gemini (text only) -> store cache -> return

use std::sync::Arc;

use crate::infrastructure::ingredient_cache::IngredientCache;
use crate::infrastructure::llm_adapter::LlmAdapter;
use crate::infrastructure::persistence::AiCacheRepository;
use crate::shared::AppError;

use super::goal::{build_cache_key, Goal};
use super::gemini::{
    build_gemini_prompt, fallback_explanation, fallback_intro, fallback_motivation,
    parse_gemini_response,
};
use super::resolver::build_variants;
use super::types::{MealPlan, PlanRequest};

const CACHE_TTL_DAYS: i32 = 7;

pub struct SousChefPlannerService {
    llm: Arc<LlmAdapter>,
    ai_cache: AiCacheRepository,
    ingredients: IngredientCache,
}

impl SousChefPlannerService {
    pub fn new(
        llm: Arc<LlmAdapter>,
        ai_cache: AiCacheRepository,
        ingredients: IngredientCache,
    ) -> Self {
        Self { llm, ai_cache, ingredients }
    }

    pub async fn generate_plan(&self, req: PlanRequest) -> Result<MealPlan, AppError> {
        let lang = req.lang.as_deref().unwrap_or("en");
        let goal = Goal::detect(&req.query);
        let cache_key = build_cache_key(goal, lang);

        // ── 1. DB cache check ──
        if let Ok(Some(cached)) = self.ai_cache.get(&cache_key).await {
            if let Ok(mut plan) = serde_json::from_value::<MealPlan>(cached) {
                plan.cached = true;
                tracing::debug!("�� Sous-chef DB cache HIT: {}", cache_key);
                return Ok(plan);
            }
        }

        // ── 2. Build variants from IngredientCache (0 SQL) ──
        let variants = build_variants(goal, lang, &self.ingredients).await;

        // ── 3. Ask LLM for personality text only ──
        let prompt = build_gemini_prompt(goal, lang, &variants);

        let (chef_intro, explanation, motivation) = match self
            .llm
            .groq_raw_request_with_model(&prompt, 800, "gemini-3-flash-preview")
            .await
        {
            Ok(raw) => match parse_gemini_response(&raw) {
                Some(r) => (r.chef_intro, r.explanation, r.motivation),
                None => {
                    tracing::warn!("⚠️ Failed to parse Gemini response, using fallback");
                    (
                        fallback_intro(goal, lang),
                        fallback_explanation(goal, lang),
                        fallback_motivation(lang),
                    )
                }
            },
            Err(e) => {
                tracing::warn!("⚠️ LLM call failed: {}, using fallback", e);
                (
                    fallback_intro(goal, lang),
                    fallback_explanation(goal, lang),
                    fallback_motivation(lang),
                )
            }
        };

        // ── 4. Assemble plan ──
        let plan = MealPlan {
            cache_key: cache_key.clone(),
            cached: false,
            chef_intro,
            variants,
            explanation,
            motivation,
            goal: goal.slug().to_string(),
            lang: lang.to_string(),
        };

        // ── 5. Store in DB cache ──
        if let Ok(val) = serde_json::to_value(&plan) {
            if let Err(e) = self
                .ai_cache
                .set(&cache_key, val, "gemini", "gemini-3-flash-preview", CACHE_TTL_DAYS)
                .await
            {
                tracing::warn!("Failed to cache sous-chef plan: {}", e);
            }
        }

        tracing::info!(
            "✅ Sous-chef plan generated: goal={}, lang={}, variants={}",
            goal.slug(),
            lang,
            plan.variants.len()
        );

        Ok(plan)
    }
}

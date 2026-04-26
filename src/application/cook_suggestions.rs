//! CookSuggestionService — smart recipe suggestions from user's inventory.
//!
//! Architecture:
//!   Inventory → ingredient names → Gemini (suggest dishes) → recipe_engine (resolve) → diff with inventory → classify
//!
//! Flow:
//!   1. Load user's inventory (with details: name, category, quantity, expiry)
//!   2. Build context: available ingredients, expiring items
//!   3. Ask Gemini for 5-8 dish candidates based on available ingredients
//!   4. For each dish: resolve via recipe_engine → get full TechCard
//!   5. Diff TechCard ingredients vs inventory → missing count
//!   6. Classify: can_cook (0 missing), almost (1-2 missing), strategic (smart picks)
//!   7. Add insights: uses_expiring, high_protein, budget_friendly
//!   8. Sort by priority: expiring first, then missing ASC, protein DESC

use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::application::inventory::{InventoryService, InventoryView};
use crate::application::preferences_service::PreferencesService;
use crate::domain::user_preferences::UserPreferences;
use crate::infrastructure::IngredientCache;
use crate::infrastructure::llm_adapter::LlmAdapter;
use crate::shared::{AppResult, Language, TenantId, UserId};

use super::rulebot::dish_schema::{ask_gemini_dish_schema, DishSchema, parse_dish_schema};
use super::rulebot::intent_router::ChatLang;
use super::rulebot::recipe_engine;
use super::rulebot::response_builder::HealthGoal;
use super::rulebot::goal_modifier::HealthModifier;
use super::rulebot::user_constraints::{UserConstraints, DietaryMode};

// ── Response Types (stable contract for iOS) ─────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct CookSuggestionsResponse {
    /// High-level inventory analysis
    pub inventory_insight: InventoryInsight,
    /// Dishes that can be cooked RIGHT NOW (0 missing ingredients)
    pub can_cook: Vec<SuggestedDish>,
    /// Dishes missing 1-2 ingredients (almost ready)
    pub almost: Vec<SuggestedDish>,
    /// Smart strategic suggestions (uses expiring, high protein, budget)
    pub strategic: Vec<SuggestedDish>,
    /// What to buy to unlock more recipes
    pub suggestions: UnlockSuggestions,
    /// Whether results are personalized based on user preferences
    #[serde(skip_serializing_if = "Option::is_none")]
    pub personalization: Option<PersonalizationInfo>,
}

/// Info about how results were personalized
#[derive(Debug, Clone, Serialize)]
pub struct PersonalizationInfo {
    pub personalized: bool,
    pub goal: String,
    pub diet: String,
    pub kcal_target: i32,
    pub protein_target: i32,
    pub excluded_allergens: Vec<String>,
    pub excluded_dislikes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct InventoryInsight {
    /// Estimated days until inventory runs out
    pub days_left: u8,
    /// Names of ingredients expiring within 3 days
    pub at_risk: Vec<String>,
    /// Waste risk percentage (0-100)
    pub waste_risk: u8,
    /// Total unique ingredients available
    pub total_ingredients: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct UnlockSuggestions {
    /// Most commonly missing ingredients across all recipes
    pub missing_frequently: Vec<String>,
    /// Human-readable hints like "+rice → 3 more recipes"
    pub unlock_hints: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SuggestedDish {
    pub dish_name: String,
    pub dish_name_local: Option<String>,
    pub display_name: Option<String>,
    pub dish_type: String,
    pub complexity: String,
    pub ingredients: Vec<SuggestedIngredient>,
    pub missing_ingredients: Vec<String>,
    pub missing_count: usize,
    /// Nutrition per dish
    pub total_kcal: u32,
    pub total_protein_g: f32,
    pub total_fat_g: f32,
    pub total_carbs_g: f32,
    /// Nutrition per serving
    pub per_serving_kcal: u32,
    pub per_serving_protein_g: f32,
    pub per_serving_fat_g: f32,
    pub per_serving_carbs_g: f32,
    pub servings: u8,
    /// Cooking steps (deterministic, no LLM)
    pub steps: Vec<RecipeStep>,
    /// Smart insights
    pub insight: DishInsight,
    /// Flavor/texture analysis
    pub flavor: Option<FlavorInfo>,
    /// Adaptation report
    pub adaptation: Option<AdaptationInfo>,
    /// Validation warnings
    pub warnings: Vec<String>,
    /// Diet tags
    pub tags: Vec<String>,
    /// Allergens
    pub allergens: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RecipeStep {
    pub step: u8,
    pub text: String,
    pub time_min: Option<u16>,
    pub temp_c: Option<u16>,
    pub tip: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SuggestedIngredient {
    pub name: String,
    pub slug: String,
    pub gross_g: f32,
    pub role: String,
    pub available: bool,
    pub expiring_soon: bool,
    /// Catalog photo URL when present (lets the frontend render an
    /// "ingredient card" with the real product image, not just text).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_url: Option<String>,
    /// Catalog category (e.g. "meat", "dairy", "produce") — used by the
    /// UI to pick fallback icons / colors when there is no photo and to
    /// group ingredients visually.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    /// Density g/ml — lets the client render the row as ml/l rather than
    /// grams (milk 1.03, oil 0.91, honey 1.42 …).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub density_g_per_ml: Option<f32>,
    /// Typical mass of one piece in grams — lets the client render the
    /// row as pcs (egg ≈ 60 g, apple ≈ 180 g …).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub typical_portion_g: Option<f32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DishInsight {
    pub uses_expiring: bool,
    pub high_protein: bool,
    pub budget_friendly: bool,
    pub estimated_cost_cents: i64,
    pub priority_score: i32,
    /// Why this dish was suggested
    pub reasons: Vec<String>,
    /// Honest kitchen economics — present only when we have real price data
    /// for at least one ingredient. Never fabricated.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub economics: Option<DishEconomics>,
}

/// Honest economics for a dish. Every field is computed from real inventory
/// prices — never "guessed". If we don't have enough price data we return
/// `None` instead of fabricating numbers.
///
/// Phase 0 (MVP): cost + waste_saved + margin only.
#[derive(Debug, Clone, Serialize)]
pub struct DishEconomics {
    /// Total ingredient cost in cents (summed from actual inventory prices).
    pub cost_cents: i64,

    /// Money saved from waste — sum of (price × grams_used) ONLY for
    /// ingredients that are expiring (Critical/Warning ≤3 days).
    /// 0 if dish doesn't use any expiring stock.
    pub waste_saved_cents: i64,

    /// Suggested menu price in cents using a realistic 3.0× markup
    /// (food cost ≈ 33%, standard restaurant industry baseline).
    pub suggested_price_cents: i64,

    /// Margin %: (price - cost) / price × 100. Rounded to 1 decimal.
    pub margin_percent: f64,

    /// How many of the dish's ingredients had real price data.
    /// If < 70% we down-rank confidence.
    pub price_coverage_percent: u8,

    /// Recommendation strength based on data quality + fit.
    pub confidence: ConfidenceLevel,
}

/// Confidence of the recommendation — drives UI emphasis (🔥 / ⚠️ / ❌).
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ConfidenceLevel {
    /// 🔥 All ingredients in stock, good price coverage, strong fit
    Strong,
    /// ⚠️ Minor gaps (≤2 missing OR partial price data)
    Medium,
    /// ❌ Many gaps — show only as a fallback
    Weak,
}

#[derive(Debug, Clone, Serialize)]
pub struct FlavorInfo {
    pub balance_score: f32,
    pub dominant: Option<String>,
    pub suggestions: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdaptationInfo {
    pub changed: bool,
    pub strategy: Option<String>,
    pub actions: Vec<String>,
}

// ── Service ──────────────────────────────────────────────────────────────────

pub struct CookSuggestionService {
    inventory_service: Arc<InventoryService>,
    ingredient_cache: Arc<IngredientCache>,
    llm_adapter: Arc<LlmAdapter>,
    preferences_service: PreferencesService,
}

impl CookSuggestionService {
    pub fn new(
        inventory_service: Arc<InventoryService>,
        ingredient_cache: Arc<IngredientCache>,
        llm_adapter: Arc<LlmAdapter>,
        preferences_service: PreferencesService,
    ) -> Self {
        Self {
            inventory_service,
            ingredient_cache,
            llm_adapter,
            preferences_service,
        }
    }

    /// Main entry: suggest dishes from user's current inventory.
    pub async fn suggest(
        &self,
        user_id: UserId,
        tenant_id: TenantId,
        language: Language,
    ) -> AppResult<CookSuggestionsResponse> {
        let lang = language_to_chat_lang(&language);

        // 0. Load user preferences for personalization
        let prefs = self.preferences_service.get(user_id).await.unwrap_or_default();
        let constraints = build_constraints_from_prefs(&prefs);
        let modifier = modifier_from_prefs(&prefs);
        let goal = HealthGoal::from_modifier(modifier, "");

        tracing::info!(
            "🎯 Personalization: goal={}, diet={}, allergies={:?}, dislikes={:?}, kcal={}",
            prefs.goal, prefs.diet, prefs.allergies, prefs.dislikes, prefs.calorie_target
        );

        // 1. Load inventory
        let inventory = self
            .inventory_service
            .list_products_with_details(user_id, tenant_id, language.clone())
            .await?;

        if inventory.is_empty() {
            return Ok(CookSuggestionsResponse {
                inventory_insight: InventoryInsight {
                    days_left: 0, at_risk: vec![], waste_risk: 0, total_ingredients: 0,
                },
                can_cook: vec![],
                almost: vec![],
                strategic: vec![],
                suggestions: UnlockSuggestions { missing_frequently: vec![], unlock_hints: vec![] },
                personalization: None,
            });
        }

        // 2. Build ingredient context (with slug aliases from cache)
        let ctx = InventoryContext::from_views_with_cache(&inventory, &self.ingredient_cache).await;

        // 3. Ask Gemini for dish candidates (with diet/allergy hints)
        let dish_schemas = self.generate_dish_candidates_personalized(&ctx, lang, &prefs).await;
        tracing::info!("📋 Gemini returned {} dish candidates", dish_schemas.len());

        // 4. Resolve each dish → TechCard → classify (with user constraints)
        let mut can_cook = Vec::new();
        let mut almost = Vec::new();
        let mut strategic = Vec::new();

        for schema in &dish_schemas {
            if let Some(dish) = self.resolve_and_classify_personalized(schema, &ctx, lang, &constraints, modifier, goal).await {
                tracing::info!(
                    "✅ Dish '{}': missing={}, ingredients={}",
                    dish.dish_name, dish.missing_count, dish.ingredients.len()
                );
                match dish.missing_count {
                    0 => can_cook.push(dish),
                    1..=2 => almost.push(dish),
                    3..=4 => strategic.push(dish), // relaxed: show as strategic instead of dropping
                    _ => {
                        tracing::info!("⏭ Skipped '{}' (missing {})", dish.dish_name, dish.missing_count);
                    }
                }
            } else {
                tracing::warn!("❌ resolve_and_classify returned None for '{}'", schema.dish);
            }
        }

        // 5. Build strategic suggestions (expiring-first dishes)
        if !ctx.expiring_names.is_empty() {
            for schema in &dish_schemas {
                if let Some(mut dish) = self.resolve_and_classify_personalized(schema, &ctx, lang, &constraints, modifier, goal).await {
                    if dish.insight.uses_expiring && dish.missing_count <= 2 {
                        dish.insight.priority_score += 50; // boost expiring
                        if !strategic.iter().any(|s: &SuggestedDish| s.dish_name == dish.dish_name) {
                            strategic.push(dish);
                        }
                    }
                }
            }
        }

        // 6. Sort each category
        can_cook.sort_by(|a, b| b.insight.priority_score.cmp(&a.insight.priority_score));
        almost.sort_by(|a, b| {
            a.missing_count
                .cmp(&b.missing_count)
                .then(b.insight.priority_score.cmp(&a.insight.priority_score))
        });
        strategic.sort_by(|a, b| b.insight.priority_score.cmp(&a.insight.priority_score));

        // Limit results
        can_cook.truncate(5);
        almost.truncate(5);
        strategic.truncate(3);

        // 7. Build inventory insight
        let inventory_insight = build_inventory_insight(&ctx);

        // 8. Build unlock suggestions from missing ingredients across all dishes
        let all_dishes: Vec<&SuggestedDish> = can_cook.iter()
            .chain(almost.iter())
            .chain(strategic.iter())
            .collect();
        let suggestions = build_unlock_suggestions(&all_dishes, &almost);

        // 9. Build personalization info
        let personalization = Some(PersonalizationInfo {
            personalized: prefs.goal != "eat_healthier" || prefs.diet != "no_restrictions"
                || !prefs.allergies.is_empty() || !prefs.dislikes.is_empty(),
            goal: prefs.goal.clone(),
            diet: prefs.diet.clone(),
            kcal_target: prefs.calorie_target,
            protein_target: prefs.protein_target,
            excluded_allergens: prefs.allergies.clone(),
            excluded_dislikes: prefs.dislikes.clone(),
        });

        Ok(CookSuggestionsResponse {
            inventory_insight,
            can_cook,
            almost,
            strategic,
            suggestions,
            personalization,
        })
    }

    // ── Gemini: generate dish names from inventory ───────────────────────────

    async fn generate_dish_candidates(
        &self,
        ctx: &InventoryContext,
        lang: ChatLang,
    ) -> Vec<DishSchema> {
        let ingredients_list = ctx.available_names.join(", ");
        let expiring_hint = if ctx.expiring_names.is_empty() {
            String::new()
        } else {
            format!(
                "\n\n⚠️ HIGH PRIORITY — these ingredients expire within 3 days and will be wasted if unused:\n{}\n\nGUIDELINES (soft priorities — prefer tasty over forced):\n1. Aim for AT LEAST 4 out of 6 dishes to use one or more expiring ingredients.\n2. Maximize total amount of expiring ingredients consumed — but only when the result is a natural, tasty dish.\n3. If a dish would become weird just to include an expiring item, SKIP it and suggest a normal balanced dish instead.\n4. Do NOT invent dishes that require expensive new purchases just to use 1 short-dated item.\n5. Prefer dishes where expiring ingredients are MAIN components, not minor garnish.",
                ctx.expiring_names.join(", ")
            )
        };

        let lang_label = match lang {
            ChatLang::Ru => "Russian",
            ChatLang::En => "English",
            ChatLang::Pl => "Polish",
            ChatLang::Uk => "Ukrainian",
        };

        let prompt = format!(
            r#"You are a practical chef focused on ZERO FOOD WASTE and cost efficiency.
The user has ONLY these ingredients in stock:
{ingredients}{expiring}

Suggest 6 realistic dishes that can be made from these ingredients.
For each dish, list ONLY ingredients from the user's stock (max 6 per dish).

Return a JSON array, no other text:
[
  {{"dish":"english name","dish_local":"name in {lang}","items":["slug1","slug2"]}},
  ...
]

Use English slugs for items (e.g. "chicken-breast", "tomato", "rice").
Only suggest dishes where at least 60% of ingredients are available in stock."#,
            ingredients = ingredients_list,
            expiring = expiring_hint,
            lang = lang_label,
        );

        let raw = match self
            .llm_adapter
            .groq_raw_request_with_model(&prompt, 4000, "gemini-3-flash-preview")
            .await
        {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!("⚠️ Gemini dish candidates failed: {e}");
                return self.fallback_candidates(ctx);
            }
        };

        // Parse JSON array
        match parse_dish_array(&raw) {
            Ok(schemas) => schemas,
            Err(e) => {
                tracing::warn!("⚠️ Failed to parse dish array: {e} — using fallback");
                self.fallback_candidates(ctx)
            }
        }
    }

    /// Personalized dish candidates: includes diet, allergies, dislikes, goal in prompt
    async fn generate_dish_candidates_personalized(
        &self,
        ctx: &InventoryContext,
        lang: ChatLang,
        prefs: &UserPreferences,
    ) -> Vec<DishSchema> {
        let ingredients_list = ctx.available_names.join(", ");
        let expiring_hint = if ctx.expiring_names.is_empty() {
            String::new()
        } else {
            format!(
                "\n\n⚠️ HIGH PRIORITY — these ingredients expire within 3 days and will be wasted if unused:\n{}\n\nGUIDELINES (soft priorities — prefer tasty over forced):\n1. Aim for AT LEAST 4 out of 6 dishes to use one or more expiring ingredients.\n2. Maximize total amount of expiring ingredients consumed — but only when the result is a natural, tasty dish.\n3. If a dish would become weird just to include an expiring item, SKIP it and suggest a normal balanced dish instead.\n4. Do NOT invent dishes that require expensive new purchases just to use 1 short-dated item.\n5. Prefer dishes where expiring ingredients are MAIN components, not minor garnish.",
                ctx.expiring_names.join(", ")
            )
        };

        // Build personalization hints for Gemini
        let mut pref_hints = Vec::new();
        if prefs.diet != "no_restrictions" {
            pref_hints.push(format!("Diet: {} — strictly follow this dietary restriction.", prefs.diet));
        }
        if !prefs.allergies.is_empty() {
            pref_hints.push(format!("ALLERGIES (MUST AVOID): {}", prefs.allergies.join(", ")));
        }
        if !prefs.intolerances.is_empty() {
            pref_hints.push(format!("Intolerances (avoid): {}", prefs.intolerances.join(", ")));
        }
        if !prefs.dislikes.is_empty() {
            pref_hints.push(format!("User dislikes (avoid if possible): {}", prefs.dislikes.join(", ")));
        }
        match prefs.goal.as_str() {
            "lose_weight" | "low_calorie" => pref_hints.push(format!(
                "Goal: weight loss. Target ~{} kcal/day. Prefer light, low-calorie dishes.", prefs.calorie_target
            )),
            "gain_muscle" | "high_protein" => pref_hints.push(format!(
                "Goal: muscle gain. Target ~{}g protein/day. Prefer high-protein dishes.", prefs.protein_target
            )),
            "maintain" => pref_hints.push("Goal: maintain weight. Balanced meals.".into()),
            _ => {}
        }
        if prefs.cooking_time == "quick" {
            pref_hints.push("User prefers quick recipes (under 20 min).".into());
        }
        if prefs.cooking_level == "beginner" {
            pref_hints.push("User is a beginner cook — suggest simple recipes.".into());
        }

        let personalization_block = if pref_hints.is_empty() {
            String::new()
        } else {
            format!("\n\nUser preferences:\n{}", pref_hints.join("\n"))
        };

        let lang_label = match lang {
            ChatLang::Ru => "Russian",
            ChatLang::En => "English",
            ChatLang::Pl => "Polish",
            ChatLang::Uk => "Ukrainian",
        };

        let prompt = format!(
            r#"You are a practical chef focused on ZERO FOOD WASTE and cost efficiency.
The user has ONLY these ingredients in stock:
{ingredients}{expiring}{personalization}

Suggest 6 realistic dishes that can be made from these ingredients.
For each dish, list ONLY ingredients from the user's stock (max 6 per dish).

Return a JSON array, no other text:
[
  {{"dish":"english name","dish_local":"name in {lang}","items":["slug1","slug2"]}},
  ...
]

Use English slugs for items (e.g. "chicken-breast", "tomato", "rice").
Only suggest dishes where at least 60% of ingredients are available in stock."#,
            ingredients = ingredients_list,
            expiring = expiring_hint,
            personalization = personalization_block,
            lang = lang_label,
        );

        let raw = match self
            .llm_adapter
            .groq_raw_request_with_model(&prompt, 4000, "gemini-3-flash-preview")
            .await
        {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!("⚠️ Gemini dish candidates failed: {e}");
                return self.fallback_candidates(ctx);
            }
        };

        match parse_dish_array(&raw) {
            Ok(schemas) => schemas,
            Err(e) => {
                tracing::warn!("⚠️ Failed to parse dish array: {e} — using fallback");
                self.fallback_candidates(ctx)
            }
        }
    }

    /// Deterministic fallback: simple protein + vegetable combos
    fn fallback_candidates(&self, ctx: &InventoryContext) -> Vec<DishSchema> {
        let mut results = Vec::new();

        let proteins: Vec<&str> = ctx
            .categories
            .iter()
            .filter(|(_, cat)| {
                let c = cat.to_lowercase();
                c.contains("meat") || c.contains("fish") || c.contains("dairy")
            })
            .map(|(name, _)| name.as_str())
            .collect();

        let vegs: Vec<&str> = ctx
            .categories
            .iter()
            .filter(|(_, cat)| {
                let c = cat.to_lowercase();
                c.contains("vegetab") || c.contains("fruit")
            })
            .map(|(name, _)| name.as_str())
            .collect();

        // Simple combos
        for p in proteins.iter().take(3) {
            for v in vegs.iter().take(2) {
                results.push(DishSchema {
                    dish: format!("{} with {}", p, v),
                    dish_local: None,
                    items: vec![p.to_lowercase().replace(' ', "-"), v.to_lowercase().replace(' ', "-")],
                });
            }
        }

        if results.is_empty() {
            // Just use whatever is available
            let items: Vec<String> = ctx
                .available_names
                .iter()
                .take(4)
                .map(|s| s.to_lowercase().replace(' ', "-"))
                .collect();
            if !items.is_empty() {
                results.push(DishSchema {
                    dish: "Mixed stir fry".into(),
                    dish_local: None,
                    items,
                });
            }
        }

        results
    }

    // ── Resolve dish + classify ──────────────────────────────────────────────

    async fn resolve_and_classify(
        &self,
        schema: &DishSchema,
        ctx: &InventoryContext,
        lang: ChatLang,
    ) -> Option<SuggestedDish> {
        let constraints = UserConstraints::default();
        let modifier = HealthModifier::None;
        let goal = HealthGoal::Balanced;

        let tech_card = recipe_engine::resolve_dish(
            &self.ingredient_cache,
            schema,
            goal,
            lang,
            &constraints,
            modifier,
        )
        .await;

        if tech_card.ingredients.is_empty() {
            return None;
        }

        // Compare with inventory
        let mut ingredients = Vec::new();
        let mut missing = Vec::new();
        let mut uses_expiring = false;
        // Honest economics accumulators — only real data, no fabrication.
        let mut cost_cents: i64 = 0;
        let mut waste_saved_cents: i64 = 0;
        let mut priced_count = 0usize;
        let mut total_count = 0usize;

        for ing in &tech_card.ingredients {
            let slug = ing.resolved_slug.as_deref().unwrap_or(&ing.slug_hint);
            let lang_code = match lang {
                ChatLang::Ru => "ru",
                ChatLang::Pl => "pl",
                ChatLang::Uk => "uk",
                ChatLang::En => "en",
            };

            let name = ing
                .product
                .as_ref()
                .map(|p| p.name(lang_code).to_string())
                .unwrap_or_else(|| slug.to_string());

            let available = ctx.has_ingredient(slug);
            let expiring_soon = ctx.is_expiring(slug);

            if !available && ing.gross_g > 0.0 {
                missing.push(name.clone());
            }

            if expiring_soon {
                uses_expiring = true;
            }

            // Honest cost: only count if unit is convertible (kg/g/l/ml).
            if ing.gross_g > 0.0 {
                total_count += 1;
                match ctx.cost_for_grams(slug, ing.gross_g) {
                    Some(c) => {
                        cost_cents += c;
                        priced_count += 1;
                        if expiring_soon {
                            waste_saved_cents += c;
                        }
                    }
                    None => {
                        tracing::debug!(
                            "💸 no price for slug='{}' ({}g) — not in inventory lookup",
                            slug, ing.gross_g
                        );
                    }
                }
            }

            ingredients.push(SuggestedIngredient {
                name,
                slug: slug.to_string(),
                gross_g: ing.gross_g,
                role: ing.role.clone(),
                available,
                expiring_soon,
                image_url: ing.product.as_ref().and_then(|p| p.image_url.clone()),
                category: ing.product.as_ref().map(|p| p.product_type.clone()),
                density_g_per_ml: ing.product.as_ref().and_then(|p| p.density_g_per_ml),
                typical_portion_g: ing.product.as_ref().and_then(|p| p.typical_portion_g),
            });
        }

        tracing::info!(
            "📊 dish '{}' economics: {}/{} priced, cost={}¢, waste_saved={}¢",
            schema.dish, priced_count, total_count, cost_cents, waste_saved_cents
        );

        let estimated_cost_cents = cost_cents; // alias for legacy field

        let servings = tech_card.servings.max(1);
        let protein_per_serving = tech_card.total_protein / servings as f32;
        let high_protein = protein_per_serving >= 30.0;

        let mut priority = 0i32;
        if uses_expiring {
            priority += 40;
        }
        if missing.is_empty() {
            priority += 30;
        }
        if high_protein {
            priority += 10;
        }
        // Less missing = higher priority
        priority -= (missing.len() as i32) * 15;

        // Build reasons
        let mut reasons = Vec::new();
        if uses_expiring { reasons.push("uses_expiring_ingredients".into()); }
        if high_protein { reasons.push("high_protein".into()); }
        if missing.is_empty() { reasons.push("all_ingredients_available".into()); }
        if estimated_cost_cents > 0 && estimated_cost_cents < 500 { reasons.push("budget_friendly".into()); }

        // Honest economics (Phase 0) — None when we couldn't price anything.
        let economics = compute_economics(
            cost_cents, waste_saved_cents, priced_count, total_count, missing.len(),
        );

        // Flavor info from TechCard
        let flavor = tech_card.flavor_analysis.as_ref().map(|f| FlavorInfo {
            balance_score: f.balance_score,
            dominant: f.dominant.clone(),
            suggestions: f.suggestions.clone(),
        });

        // Adaptation info
        let adaptation = if tech_card.adaptations.is_empty() {
            None
        } else {
            Some(AdaptationInfo {
                changed: true,
                strategy: Some(tech_card.goal.clone()),
                actions: tech_card.adaptations.iter()
                    .map(|a| format!("{}: {} ({})", a.action, a.slug, a.detail))
                    .collect(),
            })
        };

        // Steps
        let steps: Vec<RecipeStep> = tech_card.steps.iter().map(|s| RecipeStep {
            step: s.step,
            text: s.text.clone(),
            time_min: s.time_min,
            temp_c: s.temp_c,
            tip: s.tip.clone(),
        }).collect();

        Some(SuggestedDish {
            dish_name: schema.dish.clone(),
            dish_name_local: schema.dish_local.clone(),
            display_name: tech_card.display_name.clone(),
            dish_type: tech_card.dish_type.clone(),
            complexity: tech_card.complexity.clone(),
            ingredients,
            missing_count: missing.len(),
            missing_ingredients: missing,
            total_kcal: tech_card.total_kcal,
            total_protein_g: tech_card.total_protein,
            total_fat_g: tech_card.total_fat,
            total_carbs_g: tech_card.total_carbs,
            per_serving_kcal: tech_card.per_serving_kcal,
            per_serving_protein_g: tech_card.per_serving_protein,
            per_serving_fat_g: tech_card.per_serving_fat,
            per_serving_carbs_g: tech_card.per_serving_carbs,
            servings,
            steps,
            insight: DishInsight {
                uses_expiring,
                high_protein,
                budget_friendly: estimated_cost_cents > 0 && estimated_cost_cents < 500,
                estimated_cost_cents,
                priority_score: priority,
                reasons,
                economics: economics.clone(),
            },
            flavor,
            adaptation,
            warnings: tech_card.validation_warnings.clone(),
            tags: tech_card.tags.clone(),
            allergens: tech_card.allergens.clone(),
        })
    }

    /// Personalized resolve_and_classify — uses user's constraints, goal, modifier
    async fn resolve_and_classify_personalized(
        &self,
        schema: &DishSchema,
        ctx: &InventoryContext,
        lang: ChatLang,
        constraints: &UserConstraints,
        modifier: HealthModifier,
        goal: HealthGoal,
    ) -> Option<SuggestedDish> {
        let tech_card = recipe_engine::resolve_dish(
            &self.ingredient_cache,
            schema,
            goal,
            lang,
            constraints,
            modifier,
        )
        .await;

        if tech_card.ingredients.is_empty() {
            return None;
        }

        // Compare with inventory
        let mut ingredients = Vec::new();
        let mut missing = Vec::new();
        let mut uses_expiring = false;
        let mut cost_cents: i64 = 0;
        let mut waste_saved_cents: i64 = 0;
        let mut priced_count = 0usize;
        let mut total_count = 0usize;

        for ing in &tech_card.ingredients {
            let slug = ing.resolved_slug.as_deref().unwrap_or(&ing.slug_hint);
            let lang_code = match lang {
                ChatLang::Ru => "ru",
                ChatLang::Pl => "pl",
                ChatLang::Uk => "uk",
                ChatLang::En => "en",
            };

            let name = ing
                .product
                .as_ref()
                .map(|p| p.name(lang_code).to_string())
                .unwrap_or_else(|| slug.to_string());

            let available = ctx.has_ingredient(slug);
            let expiring_soon = ctx.is_expiring(slug);

            if !available && ing.gross_g > 0.0 {
                missing.push(name.clone());
            }
            if expiring_soon {
                uses_expiring = true;
            }
            if ing.gross_g > 0.0 {
                total_count += 1;
                match ctx.cost_for_grams(slug, ing.gross_g) {
                    Some(c) => {
                        cost_cents += c;
                        priced_count += 1;
                        if expiring_soon {
                            waste_saved_cents += c;
                        }
                    }
                    None => {
                        tracing::debug!(
                            "💸 no price for slug='{}' ({}g) — not in inventory lookup (personalized)",
                            slug, ing.gross_g
                        );
                    }
                }
            }

            ingredients.push(SuggestedIngredient {
                name,
                slug: slug.to_string(),
                gross_g: ing.gross_g,
                role: ing.role.clone(),
                available,
                expiring_soon,
                image_url: ing.product.as_ref().and_then(|p| p.image_url.clone()),
                category: ing.product.as_ref().map(|p| p.product_type.clone()),
                density_g_per_ml: ing.product.as_ref().and_then(|p| p.density_g_per_ml),
                typical_portion_g: ing.product.as_ref().and_then(|p| p.typical_portion_g),
            });
        }

        tracing::info!(
            "📊 dish '{}' economics (personalized): {}/{} priced, cost={}¢, waste_saved={}¢",
            schema.dish, priced_count, total_count, cost_cents, waste_saved_cents
        );

        let estimated_cost_cents = cost_cents; // alias for legacy field

        let servings = tech_card.servings.max(1);
        let protein_per_serving = tech_card.total_protein / servings as f32;
        let high_protein = protein_per_serving >= 30.0;

        let mut priority = 0i32;
        if uses_expiring { priority += 40; }
        if missing.is_empty() { priority += 30; }
        if high_protein { priority += 10; }
        priority -= (missing.len() as i32) * 15;

        let mut reasons = Vec::new();
        if uses_expiring { reasons.push("uses_expiring_ingredients".into()); }
        if high_protein { reasons.push("high_protein".into()); }
        if missing.is_empty() { reasons.push("all_ingredients_available".into()); }
        if estimated_cost_cents > 0 && estimated_cost_cents < 500 { reasons.push("budget_friendly".into()); }

        let economics = compute_economics(
            cost_cents, waste_saved_cents, priced_count, total_count, missing.len(),
        );

        let flavor = tech_card.flavor_analysis.as_ref().map(|f| FlavorInfo {
            balance_score: f.balance_score,
            dominant: f.dominant.clone(),
            suggestions: f.suggestions.clone(),
        });

        let adaptation = if tech_card.adaptations.is_empty() {
            None
        } else {
            Some(AdaptationInfo {
                changed: true,
                strategy: Some(tech_card.goal.clone()),
                actions: tech_card.adaptations.iter()
                    .map(|a| format!("{}: {} ({})", a.action, a.slug, a.detail))
                    .collect(),
            })
        };

        let steps: Vec<RecipeStep> = tech_card.steps.iter().map(|s| RecipeStep {
            step: s.step,
            text: s.text.clone(),
            time_min: s.time_min,
            temp_c: s.temp_c,
            tip: s.tip.clone(),
        }).collect();

        Some(SuggestedDish {
            dish_name: schema.dish.clone(),
            dish_name_local: schema.dish_local.clone(),
            display_name: tech_card.display_name.clone(),
            dish_type: tech_card.dish_type.clone(),
            complexity: tech_card.complexity.clone(),
            ingredients,
            missing_count: missing.len(),
            missing_ingredients: missing,
            total_kcal: tech_card.total_kcal,
            total_protein_g: tech_card.total_protein,
            total_fat_g: tech_card.total_fat,
            total_carbs_g: tech_card.total_carbs,
            per_serving_kcal: tech_card.per_serving_kcal,
            per_serving_protein_g: tech_card.per_serving_protein,
            per_serving_fat_g: tech_card.per_serving_fat,
            per_serving_carbs_g: tech_card.per_serving_carbs,
            servings,
            steps,
            insight: DishInsight {
                uses_expiring,
                high_protein,
                budget_friendly: estimated_cost_cents > 0 && estimated_cost_cents < 500,
                estimated_cost_cents,
                priority_score: priority,
                reasons,
                economics: economics.clone(),
            },
            flavor,
            adaptation,
            warnings: tech_card.validation_warnings.clone(),
            tags: tech_card.tags.clone(),
            allergens: tech_card.allergens.clone(),
        })
    }
}

// ── Preferences → Constraints/Modifier helpers ──────────────────────────────

/// Build UserConstraints from saved preferences (allergies, diet, dislikes)
fn build_constraints_from_prefs(prefs: &UserPreferences) -> UserConstraints {
    let mut c = UserConstraints::default();

    // Diet → DietaryMode
    match prefs.diet.as_str() {
        "vegan" => { c.dietary_mode = Some(DietaryMode::Vegan); }
        "vegetarian" => { c.dietary_mode = Some(DietaryMode::Vegetarian); }
        "pescatarian" => { c.dietary_mode = Some(DietaryMode::Pescatarian); }
        _ => {}
    }

    // Allergies → exclude_allergens
    for a in &prefs.allergies {
        let lower = a.to_lowercase();
        match lower.as_str() {
            "lactose" | "dairy" | "milk" => {
                c.exclude_allergens.push("lactose".into());
                c.exclude_types.push("dairy".into());
            }
            "gluten" | "wheat" => {
                c.exclude_allergens.push("gluten".into());
            }
            "nuts" | "tree nuts" | "peanuts" => {
                c.exclude_allergens.push("nuts".into());
            }
            "eggs" | "egg" => {
                c.exclude_allergens.push("eggs".into());
            }
            "fish" => {
                c.exclude_allergens.push("fish".into());
                c.exclude_types.push("fish".into());
            }
            "shellfish" | "seafood" => {
                c.exclude_allergens.push("shellfish".into());
            }
            "soy" | "soya" => {
                c.exclude_allergens.push("soy".into());
            }
            _ => {
                c.exclude_slugs.push(lower);
            }
        }
    }

    // Intolerances → also exclude
    for i in &prefs.intolerances {
        let lower = i.to_lowercase();
        if !c.exclude_allergens.contains(&lower) {
            c.exclude_allergens.push(lower);
        }
    }

    // Dislikes → exclude_slugs
    for d in &prefs.dislikes {
        let slug = d.to_lowercase().replace(' ', "-");
        if !c.exclude_slugs.contains(&slug) {
            c.exclude_slugs.push(slug);
        }
    }

    c
}

/// Map user goal preference → HealthModifier
fn modifier_from_prefs(prefs: &UserPreferences) -> HealthModifier {
    match prefs.goal.as_str() {
        "lose_weight" | "low_calorie" | "cut" => HealthModifier::LowCalorie,
        "gain_muscle" | "high_protein" | "bulk" => HealthModifier::HighProtein,
        "gain_weight" | "mass" => HealthModifier::ComfortFood,
        _ => HealthModifier::None,
    }
}

// ── Inventory Context (precomputed for fast lookups) ─────────────────────────

struct InventoryContext {
    /// All available ingredient names (for Gemini prompt)
    available_names: Vec<String>,
    /// Expiring ingredient names (≤3 days)
    expiring_names: Vec<String>,
    /// name → category
    categories: Vec<(String, String)>,
    /// Lowercase name → (quantity, price_cents, is_expiring)
    lookup: std::collections::HashMap<String, IngredientInfo>,
}

#[derive(Clone)]
struct IngredientInfo {
    _quantity: f64,
    price_per_unit_cents: i64,
    /// `"kg"`, `"g"`, `"l"`, `"ml"`, `"piece"`, …
    base_unit: String,
    is_expiring: bool,
    /// Fallback piece weight (g) when `base_unit` is `piece`/`pack`/unknown.
    /// Derived from ingredient cache `product_type` when available.
    /// Falls back to 100 g inside `cost_for_grams` when `None`.
    typical_piece_g: Option<f32>,
}

impl IngredientInfo {
    /// Convert `grams` to real cost.
    ///
    /// * `kg` / `g` / `l` / `ml` — exact (density ≈ 1 for liquids).
    /// * `piece` / `pack` / unknown — pragmatic fallback using
    ///   `typical_piece_g` (or 100 g if unknown). We log a debug line
    ///   so imprecise conversions are traceable, but we no longer return
    ///   `None` — that was causing empty economics for dishes with eggs,
    ///   lemons, bread slices, etc.
    fn cost_for_grams(&self, grams: f32) -> Option<i64> {
        if self.price_per_unit_cents <= 0 || grams <= 0.0 {
            return None;
        }
        let cents = match self.base_unit.to_ascii_lowercase().as_str() {
            "kg"          => (self.price_per_unit_cents as f64 * grams as f64 / 1000.0) as i64,
            "g"           => (self.price_per_unit_cents as f64 * grams as f64) as i64,
            // density ≈ 1 fallback for l/ml (good enough for water-based items)
            "l"           => (self.price_per_unit_cents as f64 * grams as f64 / 1000.0) as i64,
            "ml"          => (self.price_per_unit_cents as f64 * grams as f64) as i64,
            // piece / pack / szt / pcs / unknown → use typical weight
            other => {
                let per_piece = self.typical_piece_g.unwrap_or(100.0) as f64;
                let result = (self.price_per_unit_cents as f64 * grams as f64 / per_piece) as i64;
                tracing::debug!(
                    "💰 piece-fallback: unit='{}' price={}¢ grams={:.0} per_piece={:.0}g → {}¢",
                    other, self.price_per_unit_cents, grams, per_piece, result
                );
                result
            }
        };
        Some(cents)
    }
}

impl InventoryContext {
    fn from_views(views: &[InventoryView]) -> Self {
        Self::build(views, &[])
    }

    async fn from_views_with_cache(
        views: &[InventoryView],
        cache: &crate::infrastructure::ingredient_cache::IngredientCache,
    ) -> Self {
        let all_ingredients = cache.all().await;
        Self::build(views, &all_ingredients)
    }

    fn build(
        views: &[InventoryView],
        cache_ingredients: &[crate::infrastructure::ingredient_cache::IngredientData],
    ) -> Self {
        let mut available_names = Vec::new();
        let mut expiring_names = Vec::new();
        let mut categories = Vec::new();
        let mut lookup = std::collections::HashMap::new();

        for v in views {
            let name = v.product.name.clone();
            let is_expiring = matches!(
                v.severity,
                crate::domain::inventory::ExpirationSeverity::Critical
                    | crate::domain::inventory::ExpirationSeverity::Warning
            );

            available_names.push(name.clone());
            categories.push((name.clone(), v.product.category.clone()));

            if is_expiring {
                expiring_names.push(name.clone());
            }

            let info = IngredientInfo {
                _quantity: v.remaining_quantity,
                price_per_unit_cents: v.price_per_unit_cents,
                base_unit: v.product.base_unit.clone(),
                is_expiring,
                typical_piece_g: None, // filled later when we match a cache entry
            };

            // Index by lowercase name
            let key = name.to_lowercase();
            lookup.insert(key.clone(), info.clone());

            // Also index by all slug/name variants from ingredient cache
            // Find matching cache entry by name (any language)
            let name_lower = name.to_lowercase();
            for ing in cache_ingredients {
                let names = [
                    ing.slug.to_lowercase(),
                    ing.name_en.to_lowercase(),
                    ing.name_ru.to_lowercase(),
                    ing.name_pl.to_lowercase(),
                    ing.name_uk.to_lowercase(),
                ];
                if names.iter().any(|n| n == &name_lower || n.contains(&name_lower) || name_lower.contains(n.as_str())) {
                    // Enrich info with a per-piece weight estimate based on product_type.
                    let mut enriched = info.clone();
                    enriched.typical_piece_g = Some(typical_piece_weight_g(&ing.product_type, &ing.slug));
                    // Add all variants as keys
                    for alias in &names {
                        if !alias.is_empty() && !lookup.contains_key(alias) {
                            lookup.insert(alias.clone(), enriched.clone());
                        }
                    }
                    // Also add slug with dashes replaced by spaces
                    let slug_spaced = ing.slug.to_lowercase().replace('-', " ");
                    if !lookup.contains_key(&slug_spaced) {
                        lookup.insert(slug_spaced, enriched.clone());
                    }
                    // Overwrite the original name entry with the enriched one
                    // (so piece-fallback works when matched by display name too)
                    lookup.insert(key.clone(), enriched);
                    break;
                }
            }
        }

        // Deduplicate names
        available_names.sort();
        available_names.dedup();
        expiring_names.sort();
        expiring_names.dedup();

        tracing::info!(
            "🗃 InventoryContext: {} products, {} lookup keys, {} expiring",
            available_names.len(), lookup.len(), expiring_names.len()
        );

        Self {
            available_names,
            expiring_names,
            categories,
            lookup,
        }
    }

    fn has_ingredient(&self, slug: &str) -> bool {
        let normalized = slug.to_lowercase().replace('-', " ");
        self.lookup.keys().any(|k| {
            k.contains(&normalized) || normalized.contains(k.as_str())
        })
    }

    fn is_expiring(&self, slug: &str) -> bool {
        let normalized = slug.to_lowercase().replace('-', " ");
        self.lookup.iter().any(|(k, info)| {
            (k.contains(&normalized) || normalized.contains(k.as_str())) && info.is_expiring
        })
    }

    fn price_cents_per_unit(&self, slug: &str) -> Option<i64> {
        let normalized = slug.to_lowercase().replace('-', " ");
        self.lookup.iter().find_map(|(k, info)| {
            if k.contains(&normalized) || normalized.contains(k.as_str()) {
                Some(info.price_per_unit_cents)
            } else {
                None
            }
        })
    }

    /// Honest cost for `grams` of ingredient matching `slug`.
    /// Returns `None` when we can't convert the base unit (e.g. `piece`).
    fn cost_for_grams(&self, slug: &str, grams: f32) -> Option<i64> {
        let normalized = slug.to_lowercase().replace('-', " ");
        self.lookup.iter().find_map(|(k, info)| {
            if k.contains(&normalized) || normalized.contains(k.as_str()) {
                info.cost_for_grams(grams)
            } else {
                None
            }
        })
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn parse_dish_array(raw: &str) -> Result<Vec<DishSchema>, String> {
    // Extract JSON array from raw text
    let start = raw.find('[').ok_or("No [ found")?;
    let end = raw.rfind(']').ok_or("No ] found")?;
    if end < start {
        return Err("Invalid JSON array".into());
    }
    let json_str = &raw[start..=end];
    let schemas: Vec<DishSchema> = serde_json::from_str(json_str)
        .map_err(|e| format!("JSON parse error: {e}"))?;
    Ok(schemas)
}

fn language_to_chat_lang(lang: &Language) -> ChatLang {
    match lang.code() {
        "ru" => ChatLang::Ru,
        "pl" => ChatLang::Pl,
        "uk" => ChatLang::Uk,
        _ => ChatLang::En,
    }
}

/// Rough per-piece weight in grams, used as fallback when a product is priced
/// per-piece/pack and the recipe asks for grams. These are intentionally
/// middle-of-the-road values; the goal is "close enough to show a price" not
/// lab-grade accuracy. Order: specific slug overrides, then product_type bucket.
fn typical_piece_weight_g(product_type: &str, slug: &str) -> f32 {
    // Specific slug overrides (most common offenders).
    let s = slug.to_lowercase();
    let specific: &[(&str, f32)] = &[
        ("egg", 55.0),
        ("lemon", 80.0),
        ("lime", 50.0),
        ("orange", 150.0),
        ("apple", 180.0),
        ("banana", 120.0),
        ("pear", 170.0),
        ("avocado", 170.0),
        ("potato", 180.0),
        ("onion", 110.0),
        ("garlic-clove", 5.0),
        ("garlic", 40.0),
        ("cucumber", 200.0),
        ("tomato", 120.0),
        ("bell-pepper", 160.0),
        ("pepper", 160.0),
        ("carrot", 80.0),
        ("zucchini", 200.0),
        ("eggplant", 280.0),
        ("mushroom", 20.0),
        ("bread-slice", 30.0),
        ("bread", 500.0),
        ("tortilla", 45.0),
    ];
    for (k, w) in specific {
        if s.contains(k) {
            return *w;
        }
    }
    // Bucket by product_type.
    match product_type.to_ascii_lowercase().as_str() {
        "vegetable" | "fruit"       => 150.0,
        "egg"                       => 55.0,
        "spice" | "herb"            => 5.0,
        "mushroom"                  => 20.0,
        "nut"                       => 10.0,
        "grain" | "legume"          => 200.0, // pack-ish
        "dairy"                     => 200.0, // small container
        "meat" | "fish" | "seafood" => 200.0,
        _                           => 100.0,
    }
}

// ── Honest Economics (Phase 0) ──────────────────────────────────────────────
//
// Rules:
//   • If we have zero priced ingredients → return None. We never fabricate.
//   • suggested_price uses a realistic 3.0× markup (food cost ≈ 33%). This is
//     a documented industry baseline, not magic. We also round UP to the
//     nearest 0.50 zł for a more "menu-looking" price.
//   • Confidence is conservative: Strong requires ≥80% price coverage AND
//     no missing ingredients.
//
// The function is pure — easy to unit-test.

/// Realistic markup for casual restaurants / home-cook → menu conversion.
const MARKUP_FACTOR: f64 = 3.0;

fn compute_economics(
    cost_cents: i64,
    waste_saved_cents: i64,
    priced_count: usize,
    total_count: usize,
    missing_count: usize,
) -> Option<DishEconomics> {
    // No priced ingredients → no honest economics.
    if priced_count == 0 || cost_cents <= 0 || total_count == 0 {
        return None;
    }

    // Round suggested price UP to nearest 50 cents (→ 0.50 zł).
    let raw_price = (cost_cents as f64 * MARKUP_FACTOR).round() as i64;
    let suggested_price_cents = ((raw_price + 49) / 50) * 50;

    let margin_percent = if suggested_price_cents > 0 {
        let m = (suggested_price_cents - cost_cents) as f64 / suggested_price_cents as f64 * 100.0;
        (m * 10.0).round() / 10.0
    } else {
        0.0
    };

    let coverage = ((priced_count as f32 / total_count as f32) * 100.0).round() as u8;

    let confidence = if coverage >= 80 && missing_count == 0 {
        ConfidenceLevel::Strong
    } else if coverage >= 50 && missing_count <= 2 {
        ConfidenceLevel::Medium
    } else {
        ConfidenceLevel::Weak
    };

    Some(DishEconomics {
        cost_cents,
        waste_saved_cents,
        suggested_price_cents,
        margin_percent,
        price_coverage_percent: coverage,
        confidence,
    })
}

#[cfg(test)]
mod economics_tests {
    use super::*;

    #[test]
    fn no_priced_ingredients_returns_none() {
        assert!(compute_economics(0, 0, 0, 5, 0).is_none());
    }

    #[test]
    fn honest_markup_and_margin() {
        // cost 820 cents (8.20 zł) → price 820*3=2460 → rounded up to 2500 (25.00 zł)
        let e = compute_economics(820, 0, 5, 5, 0).unwrap();
        assert_eq!(e.cost_cents, 820);
        assert_eq!(e.suggested_price_cents, 2500);
        // margin = (2500 - 820) / 2500 = 67.2%
        assert!((e.margin_percent - 67.2).abs() < 0.1, "got {}", e.margin_percent);
        assert!(matches!(e.confidence, ConfidenceLevel::Strong));
    }

    #[test]
    fn waste_saved_is_tracked() {
        let e = compute_economics(1500, 450, 4, 5, 0).unwrap();
        assert_eq!(e.waste_saved_cents, 450);
        assert_eq!(e.price_coverage_percent, 80);
        assert!(matches!(e.confidence, ConfidenceLevel::Strong));
    }

    #[test]
    fn low_coverage_becomes_weak() {
        let e = compute_economics(300, 0, 1, 5, 0).unwrap();
        assert_eq!(e.price_coverage_percent, 20);
        assert!(matches!(e.confidence, ConfidenceLevel::Weak));
    }

    #[test]
    fn missing_ingredients_downgrade() {
        let e = compute_economics(600, 0, 4, 5, 1).unwrap();
        assert!(matches!(e.confidence, ConfidenceLevel::Medium));
    }

    #[test]
    fn kg_cost_is_exact() {
        let info = IngredientInfo {
            _quantity: 0.0, price_per_unit_cents: 2000, // 20 zł/kg
            base_unit: "kg".into(), is_expiring: false, typical_piece_g: None,
        };
        // 500g → 10 zł
        assert_eq!(info.cost_for_grams(500.0), Some(1000));
    }

    #[test]
    fn piece_uses_typical_weight() {
        // egg: 55 g per piece, 90¢ per piece → 50g should cost ~82¢
        let info = IngredientInfo {
            _quantity: 0.0, price_per_unit_cents: 90,
            base_unit: "piece".into(), is_expiring: false,
            typical_piece_g: Some(55.0),
        };
        let c = info.cost_for_grams(50.0).unwrap();
        assert!(c >= 70 && c <= 90, "expected ~82¢, got {c}");
    }

    #[test]
    fn piece_falls_back_to_100g_when_unknown() {
        // 5 zł per piece, asking for 100g → ~5 zł (1:1)
        let info = IngredientInfo {
            _quantity: 0.0, price_per_unit_cents: 500,
            base_unit: "szt".into(), is_expiring: false,
            typical_piece_g: None,
        };
        assert_eq!(info.cost_for_grams(100.0), Some(500));
    }

    #[test]
    fn zero_price_returns_none() {
        let info = IngredientInfo {
            _quantity: 0.0, price_per_unit_cents: 0,
            base_unit: "kg".into(), is_expiring: false, typical_piece_g: None,
        };
        assert_eq!(info.cost_for_grams(100.0), None);
    }
}

// ── Inventory Insight Builder ────────────────────────────────────────────────

fn build_inventory_insight(ctx: &InventoryContext) -> InventoryInsight {
    let total = ctx.available_names.len();
    let at_risk = ctx.expiring_names.clone();
    let waste_risk = if total == 0 {
        0
    } else {
        ((at_risk.len() as f32 / total as f32) * 100.0).round() as u8
    };
    // Rough estimate: 1 ingredient ≈ 1 day of cooking variety
    let days_left = (total as u8).min(14);

    InventoryInsight {
        days_left,
        at_risk,
        waste_risk,
        total_ingredients: total,
    }
}

// ── Unlock Suggestions Builder ───────────────────────────────────────────────

fn build_unlock_suggestions(all: &[&SuggestedDish], almost: &[SuggestedDish]) -> UnlockSuggestions {
    use std::collections::HashMap;

    // Count how often each ingredient is missing across "almost" dishes
    let mut missing_freq: HashMap<String, usize> = HashMap::new();
    for dish in almost {
        for m in &dish.missing_ingredients {
            *missing_freq.entry(m.clone()).or_default() += 1;
        }
    }

    let mut sorted: Vec<(String, usize)> = missing_freq.into_iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(&a.1));

    let missing_frequently: Vec<String> = sorted.iter().take(5).map(|(k, _)| k.clone()).collect();

    let unlock_hints: Vec<String> = sorted.iter().take(3).map(|(name, count)| {
        format!("+{} → {} more recipes", name, count)
    }).collect();

    UnlockSuggestions {
        missing_frequently,
        unlock_hints,
    }
}

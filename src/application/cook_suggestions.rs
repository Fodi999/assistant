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
use crate::infrastructure::IngredientCache;
use crate::infrastructure::llm_adapter::LlmAdapter;
use crate::shared::{AppResult, Language, TenantId, UserId};

use super::rulebot::dish_schema::{ask_gemini_dish_schema, DishSchema, parse_dish_schema};
use super::rulebot::intent_router::ChatLang;
use super::rulebot::recipe_engine;
use super::rulebot::response_builder::HealthGoal;
use super::rulebot::goal_modifier::HealthModifier;
use super::rulebot::user_constraints::UserConstraints;

// ── Response Types (stable contract for iOS) ─────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct CookSuggestionsResponse {
    /// Dishes that can be cooked RIGHT NOW (0 missing ingredients)
    pub can_cook: Vec<SuggestedDish>,
    /// Dishes missing 1-2 ingredients (almost ready)
    pub almost: Vec<SuggestedDish>,
    /// Smart strategic suggestions (uses expiring, high protein, budget)
    pub strategic: Vec<SuggestedDish>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SuggestedDish {
    pub dish_name: String,
    pub dish_name_local: Option<String>,
    pub ingredients: Vec<SuggestedIngredient>,
    pub missing_ingredients: Vec<String>,
    pub missing_count: usize,
    /// Nutrition summary
    pub total_kcal: u32,
    pub total_protein_g: f32,
    pub total_fat_g: f32,
    pub total_carbs_g: f32,
    pub servings: u8,
    /// Smart insights
    pub insight: DishInsight,
}

#[derive(Debug, Clone, Serialize)]
pub struct SuggestedIngredient {
    pub name: String,
    pub slug: String,
    pub gross_g: f32,
    pub available: bool,
    pub expiring_soon: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct DishInsight {
    /// Uses ingredients that are about to expire
    pub uses_expiring: bool,
    /// High protein dish (>30g per serving)
    pub high_protein: bool,
    /// Budget-friendly (uses cheap/available ingredients)
    pub budget_friendly: bool,
    /// Estimated cost from inventory (cents)
    pub estimated_cost_cents: i64,
    /// Priority score (higher = suggest first)
    pub priority_score: i32,
}

// ── Service ──────────────────────────────────────────────────────────────────

pub struct CookSuggestionService {
    inventory_service: Arc<InventoryService>,
    ingredient_cache: Arc<IngredientCache>,
    llm_adapter: Arc<LlmAdapter>,
}

impl CookSuggestionService {
    pub fn new(
        inventory_service: Arc<InventoryService>,
        ingredient_cache: Arc<IngredientCache>,
        llm_adapter: Arc<LlmAdapter>,
    ) -> Self {
        Self {
            inventory_service,
            ingredient_cache,
            llm_adapter,
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

        // 1. Load inventory
        let inventory = self
            .inventory_service
            .list_products_with_details(user_id, tenant_id, language.clone())
            .await?;

        if inventory.is_empty() {
            return Ok(CookSuggestionsResponse {
                can_cook: vec![],
                almost: vec![],
                strategic: vec![],
            });
        }

        // 2. Build ingredient context
        let ctx = InventoryContext::from_views(&inventory);

        // 3. Ask Gemini for dish candidates
        let dish_schemas = self.generate_dish_candidates(&ctx, lang).await;

        // 4. Resolve each dish → TechCard → classify
        let mut can_cook = Vec::new();
        let mut almost = Vec::new();
        let mut strategic = Vec::new();

        for schema in &dish_schemas {
            if let Some(dish) = self.resolve_and_classify(schema, &ctx, lang).await {
                match dish.missing_count {
                    0 => can_cook.push(dish),
                    1..=2 => almost.push(dish),
                    _ => {} // skip dishes needing 3+ missing
                }
            }
        }

        // 5. Build strategic suggestions (expiring-first dishes)
        if !ctx.expiring_names.is_empty() {
            for schema in &dish_schemas {
                if let Some(mut dish) = self.resolve_and_classify(schema, &ctx, lang).await {
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

        Ok(CookSuggestionsResponse {
            can_cook,
            almost,
            strategic,
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
                "\nURGENT — these expire soon, prioritize them: {}",
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
            r#"You are a practical chef. The user has ONLY these ingredients in stock:
{ingredients}{expiring}

Suggest 6 realistic dishes that can be made from these ingredients.
For each dish, list ONLY ingredients from the user's stock (max 6 per dish).
Prefer dishes that use expiring ingredients first.

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
        let mut estimated_cost_cents: i64 = 0;

        for ing in &tech_card.ingredients {
            let slug = ing.resolved_slug.as_deref().unwrap_or(&ing.slug_hint);
            let name = ing
                .product
                .as_ref()
                .map(|p| p.name_en.clone())
                .unwrap_or_else(|| slug.to_string());

            let available = ctx.has_ingredient(slug);
            let expiring_soon = ctx.is_expiring(slug);

            if !available && ing.gross_g > 0.0 {
                missing.push(name.clone());
            }

            if expiring_soon {
                uses_expiring = true;
            }

            // Estimate cost from inventory prices
            if let Some(price_cents) = ctx.price_cents_per_unit(slug) {
                estimated_cost_cents += (price_cents as f64 * ing.gross_g as f64 / 1000.0) as i64;
            }

            ingredients.push(SuggestedIngredient {
                name,
                slug: slug.to_string(),
                gross_g: ing.gross_g,
                available,
                expiring_soon,
            });
        }

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

        Some(SuggestedDish {
            dish_name: schema.dish.clone(),
            dish_name_local: schema.dish_local.clone(),
            ingredients,
            missing_count: missing.len(),
            missing_ingredients: missing,
            total_kcal: tech_card.total_kcal,
            total_protein_g: tech_card.total_protein,
            total_fat_g: tech_card.total_fat,
            total_carbs_g: tech_card.total_carbs,
            servings,
            insight: DishInsight {
                uses_expiring,
                high_protein,
                budget_friendly: estimated_cost_cents < 500, // < 5 currency units
                estimated_cost_cents,
                priority_score: priority,
            },
        })
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

struct IngredientInfo {
    _quantity: f64,
    price_per_unit_cents: i64,
    is_expiring: bool,
}

impl InventoryContext {
    fn from_views(views: &[InventoryView]) -> Self {
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

            // Use lowercase name as key for fuzzy matching
            let key = name.to_lowercase();
            lookup.insert(
                key,
                IngredientInfo {
                    _quantity: v.remaining_quantity,
                    price_per_unit_cents: v.price_per_unit_cents,
                    is_expiring,
                },
            );
        }

        // Deduplicate names
        available_names.sort();
        available_names.dedup();
        expiring_names.sort();
        expiring_names.dedup();

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

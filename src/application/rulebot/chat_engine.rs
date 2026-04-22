//! ChefOS Chat Engine — routing layer.
//!
//! Entry point: `POST /public/chat`
//!
//! Architecture (3-layer):
//! ```text
//! chat_engine.rs       → WHAT to answer  (intent routing + data fetching)
//! response_builder.rs  → HOW to build    (card assembly + suggestions)
//! response_templates.rs → HOW it sounds  (localized text generation)
//! ```
//!
//! Flow:
//! 1. parse_input(input)          — intent + multi-intents + goal modifier
//! 2. detect_language(input)      — which language?
//! 3. session follow-up check     — "а сколько в нём калорий?" → last product
//! 4. dispatch to handler(s)      — data fetching + builder call
//! 5. return ChatResponse         — text + card + reason + intents + context

use std::sync::Arc;
use std::time::Instant;

use crate::infrastructure::IngredientCache;
use crate::infrastructure::llm_adapter::LlmAdapter;
use crate::application::preferences_service::PreferencesService;
use crate::domain::user_preferences::UserPreferences;
use crate::domain::tools::unit_converter as uc;
use super::intent_router::{detect_language, parse_input_with_context, DialogContext, ChatLang, Intent};
use super::chat_response::ChatResponse;
use super::session_context::SessionContext;
use super::chat_response::{Action, Card};
use super::category_filter::{detect_category, ProductCategory};
use super::ai_brain::AiBrain;
use super::response_builder::{self as rb, HealthGoal};
use super::chef_coach;
use super::meal_builder;
use super::recipe_engine;

// ── Engine ───────────────────────────────────────────────────────────────────

pub struct ChatEngine {
    ingredient_cache: Arc<IngredientCache>,
    llm_adapter: Arc<LlmAdapter>,
    ai_brain: AiBrain,
    preferences_service: Option<PreferencesService>,
}

impl ChatEngine {
    pub fn new(ingredient_cache: Arc<IngredientCache>, llm_adapter: Arc<LlmAdapter>) -> Self {
        let ai_brain = AiBrain::new(Arc::clone(&ingredient_cache), Arc::clone(&llm_adapter));
        Self { ingredient_cache, llm_adapter, ai_brain, preferences_service: None }
    }

    /// Create with preferences support for personalized chat
    pub fn with_preferences(mut self, prefs_service: PreferencesService) -> Self {
        self.preferences_service = Some(prefs_service);
        self
    }

    /// Main entry point — takes free-text + optional session context, returns ChatResponse.
    pub async fn handle_chat(&self, input: &str) -> ChatResponse {
        self.handle_chat_with_context(input, &SessionContext::new()).await
    }

    /// Extended entry with optional user_id for personalization.
    pub async fn handle_chat_with_user(
        &self,
        input: &str,
        ctx: &SessionContext,
        user_id: Option<crate::shared::UserId>,
    ) -> ChatResponse {
        // Load preferences if user_id provided and service available
        let prefs = if let (Some(uid), Some(svc)) = (user_id, &self.preferences_service) {
            match svc.get(uid).await {
                Ok(p) => {
                    tracing::debug!("🎯 Chat personalization: goal={}, diet={}, allergies={:?}", p.goal, p.diet, p.allergies);
                    Some(p)
                }
                Err(e) => {
                    tracing::warn!("⚠️ Failed to load preferences for chat: {e}");
                    None
                }
            }
        } else {
            None
        };

        self.handle_chat_personalized(input, ctx, prefs.as_ref()).await
    }

    /// Extended entry with session context — enables follow-ups and modifier persistence.
    pub async fn handle_chat_with_context(
        &self,
        input: &str,
        ctx: &SessionContext,
    ) -> ChatResponse {
        self.handle_chat_personalized(input, ctx, None).await
    }

    /// Core chat handler with optional preferences.
    async fn handle_chat_personalized(
        &self,
        input: &str,
        ctx: &SessionContext,
        prefs: Option<&UserPreferences>,
    ) -> ChatResponse {
        let start = Instant::now();
        let lang = detect_language(input);

        // ── Personalization: resolve prefs → slugs, enrich ctx ────────────
        // Allergies / intolerances / dislikes become a HARD exclusion list
        // that flows through `ctx.excluded_slugs()` into every product picker.
        // Likes become a soft-boost hint (reserved for future ranking tweak).
        // We clone ctx here so downstream handlers see a uniform view —
        // non-personalized callers simply get an empty list.
        let ctx_owned: SessionContext;
        let ctx: &SessionContext = if let Some(p) = prefs {
            let hints = super::preference_resolver::resolve(p, &self.ingredient_cache).await;
            if !hints.excludes.is_empty() || !hints.likes.is_empty() {
                tracing::info!(
                    "🎯 chat personalization: {} hard-excludes, {} likes",
                    hints.excludes.len(), hints.likes.len()
                );
                let mut enriched = ctx.clone();
                enriched.preference_excludes = hints.excludes;
                enriched.preference_likes = hints.likes;
                ctx_owned = enriched;
                &ctx_owned
            } else {
                ctx
            }
        } else {
            ctx
        };

        // Build dialog context from session for context-aware intent scoring
        let dialog_ctx = DialogContext {
            last_intent: ctx.last_intent,
            last_modifier: ctx.effective_modifier_opt(),
            turn_count: ctx.turn_count,
        };
        let parsed = parse_input_with_context(input, &dialog_ctx);

        // Effective modifier: current OR remembered from last turn OR from preferences
        let modifier = ctx.effective_modifier(parsed.modifier);
        let goal = if modifier != super::goal_modifier::HealthModifier::None {
            HealthGoal::from_modifier(modifier, input)
        } else if let Some(p) = prefs {
            // Use saved goal from preferences as fallback
            match p.goal.as_str() {
                "lose_weight" | "low_calorie" | "cut" => HealthGoal::LowCalorie,
                "gain_muscle" | "high_protein" | "bulk" => HealthGoal::HighProtein,
                _ => HealthGoal::Balanced,
            }
        } else {
            HealthGoal::from_modifier(modifier, input)
        };

        tracing::debug!(
            "💬 chat: intent={:?} intents={:?} modifier={:?} lang={:?} turn={}",
            parsed.intent, parsed.intents, modifier, lang, ctx.turn_count
        );

        // ── Follow-up resolution ──────────────────────────────────────────────
        // "а сколько в нём калорий?" refers to last product in context
        if ctx.is_followup(input) {
            if let Some(slug) = &ctx.last_product_slug {
                if let Some(p) = self.ingredient_cache.get(slug).await {
                    let mut resp = rb::build_followup_nutrition(&p, lang);
                    resp.timing_ms = start.elapsed().as_millis() as u64;
                    return resp;
                }
            }
        }

        // ── Primary dispatch ──────────────────────────────────────────────────
        let mut response = match parsed.intent {
            Intent::Greeting       => self.handle_greeting(lang),
            Intent::HealthyProduct => self.handle_healthy_product(input, lang, goal, ctx).await,
            Intent::Conversion     => self.handle_conversion_with_density(input, lang).await,
            Intent::NutritionInfo  => self.handle_nutrition(input, lang).await,
            Intent::Seasonality    => self.handle_seasonality(input, lang),
            Intent::RecipeHelp     => self.handle_recipe(input, lang, goal, modifier).await,
            Intent::MealIdea       => self.handle_meal_idea(lang, goal, input, ctx).await,
            Intent::ProductInfo    => self.handle_product_info(input, lang).await,
            Intent::CookingLoss    => self.handle_cooking_loss(input, lang).await,
            // ── Layer 2: AI Brain ── LLM with tool calling for complex queries
            Intent::Unknown        => {
                tracing::info!("🧠 Escalating to AI Brain (Layer 2) for: {:?}", &input[..input.len().min(60)]);
                self.ai_brain.handle(input, lang, ctx).await
            }
        };

        // Attach multi-intent list to response (frontend can use for multi-card UI)
        if parsed.intents.len() > 1 {
            response.intents = parsed.intents;
        }

        // ── Coach motivation ──────────────────────────────────────────────
        // Suppress coach on precise factual intents — they are clean data answers
        // where a motivational line is noise ("сколько калорий" + "ты уже профи!").
        let coach_suppressed = matches!(
            parsed.intent,
            Intent::NutritionInfo | Intent::CookingLoss | Intent::Conversion | Intent::ProductInfo | Intent::Seasonality
        );
        if !coach_suppressed {
            response.coach_message = chef_coach::pick_message(ctx, goal, lang);
        }

        // ── Action Layer: enrich cards with user-invokable actions ───────
        enrich_with_actions(&mut response, ctx);

        response.timing_ms = start.elapsed().as_millis() as u64;
        response
    }

    // ── Handlers (thin — fetch data, delegate to builder) ───────────────────

    fn handle_greeting(&self, lang: ChatLang) -> ChatResponse {
        rb::build_greeting(lang)
    }

    async fn handle_healthy_product(&self, input: &str, lang: ChatLang, goal: HealthGoal, ctx: &SessionContext) -> ChatResponse {
        // ── Step 1: Does the user mention a SPECIFIC product? ────────────
        if let Some(product) = self.find_ingredient_in_text(input).await {
            let already_seen = ctx.last_cards.contains(&product.slug)
                || ctx.last_product_slug.as_deref() == Some(&product.slug);

            if already_seen {
                // Product was already shown → give a DIFFERENT angle
                tracing::debug!("🔁 healthy_product: {} already seen → alternative response", product.slug);
                let alternatives = self.select_top_products(goal, 2, &[product.slug.clone()], None).await;
                return rb::build_already_seen_product(&product, &alternatives, lang, goal);
            }

            tracing::debug!("🎯 healthy_product: specific product found — {}", product.slug);
            return rb::build_specific_healthy_product(&product, lang, goal);
        }

        // ── Step 2: Category filter (vegetables, fruit, fish, ...) ──────
        // If the user asked "какой овощ полезный" we MUST NOT return tuna.
        let category = detect_category(input);
        if let Some(c) = category {
            tracing::debug!("🥦 healthy_product: category filter = {}", c.as_str());
        }

        // ── Step 3: No specific product → generic top-N by goal+category ─
        let exclude = ctx.excluded_slugs();
        let products = self.select_top_products(goal, 3, &exclude, category).await;

        // ── Step 4: Graceful fallback when category has zero matches ────
        if products.is_empty() {
            if let Some(c) = category {
                tracing::warn!("🥦 no products for category={} goal={:?} — widening to any", c.as_str(), goal);
                let wider = self.select_top_products(goal, 3, &exclude, None).await;
                return rb::build_healthy_response(&wider, lang, goal);
            }
        }
        let mut response = rb::build_healthy_response(&products, lang, goal);

        // ── Step 5 (Guidance): complementary category block ─────────────
        // After main cards — attach a side block (e.g. Fish → Vegetable).
        // Skipped when: no category detected, no complement mapped, or
        // no products found for the complement.
        if let Some(c) = category {
            if let Some(complement) = c.complement() {
                // Exclude main products too so the side doesn't repeat them.
                let mut side_exclude = exclude.clone();
                for (p, _, _) in &products {
                    side_exclude.push(p.slug.clone());
                }
                let side = self.select_top_products(goal, 2, &side_exclude, Some(complement)).await;
                if !side.is_empty() {
                    tracing::debug!(
                        "🍽️ complement: {} → {} ({} items)",
                        c.as_str(), complement.as_str(), side.len()
                    );
                    response.suggestion_block = Some(
                        rb::build_suggestion_block(complement, &side, lang, goal)
                    );
                }
            }
        }
        response
    }

    /// Scan ALL ingredients from cache, rank by weighted normalized score.
    /// Returns up to `n` diverse products, excluding `exclude_slugs`.
    /// Uses 80/20 exploration: 20% of turns pick random from top-10.
    async fn select_top_products(
        &self,
        goal: HealthGoal,
        n: usize,
        exclude_slugs: &[String],
        category: Option<ProductCategory>,
    ) -> Vec<(crate::infrastructure::ingredient_cache::IngredientData, &'static str, String)> {
        let all = self.ingredient_cache.all().await;
        if all.is_empty() {
            return vec![];
        }

        // Filter valid + non-excluded products
        let mut candidates: Vec<_> = all.into_iter()
            .filter(|p| (p.calories_per_100g > 0.0 || p.protein_per_100g > 0.0)
                && !exclude_slugs.contains(&p.slug))
            .collect();

        // ── Category hard filter (vegetables, fruit, fish, ...) ──────────
        // Must run BEFORE goal filters so we don't e.g. drop all veggies
        // for HighProtein and then look empty.
        if let Some(cat) = category {
            let allowed = cat.product_types();
            let before = candidates.len();
            candidates.retain(|p| allowed.iter().any(|t| p.product_type.eq_ignore_ascii_case(t)));
            tracing::debug!(
                "🥦 category={} filter: {} → {} candidates",
                cat.as_str(), before, candidates.len()
            );
        }

        // ── Goal-specific hard filters ───────────────────────────────────
        // LowCalorie: remove products >250 kcal/100g (nuts, grains, oils etc.)
        // HighProtein: require at least 10g protein/100g
        match goal {
            HealthGoal::LowCalorie => {
                candidates.retain(|p| p.calories_per_100g <= 250.0);
            }
            HealthGoal::HighProtein => {
                let high_prot: Vec<_> = candidates.iter()
                    .filter(|p| p.protein_per_100g >= 10.0)
                    .cloned()
                    .collect();
                if !high_prot.is_empty() {
                    candidates = high_prot;
                }
            }
            HealthGoal::Balanced => {}
        }

        if candidates.is_empty() {
            return vec![];
        }

        // ── Step 1: Compute min/max for normalization ──────────────────────
        let max_protein = candidates.iter().map(|p| p.protein_per_100g).fold(0.0_f32, f32::max);
        let min_protein = candidates.iter().map(|p| p.protein_per_100g).fold(f32::MAX, f32::min);
        let max_cal = candidates.iter().map(|p| p.calories_per_100g).fold(0.0_f32, f32::max);
        let min_cal = candidates.iter().map(|p| p.calories_per_100g).fold(f32::MAX, f32::min);
        let max_fat = candidates.iter().map(|p| p.fat_per_100g).fold(0.0_f32, f32::max);
        let min_fat = candidates.iter().map(|p| p.fat_per_100g).fold(f32::MAX, f32::min);

        let norm = |x: f32, lo: f32, hi: f32| -> f64 {
            let range = (hi - lo) as f64;
            if range < 1e-6 { 0.5 } else { ((x - lo) as f64) / range }
        };

        // ── Step 2: Score each candidate ──────────────────────────────────
        let mut scored: Vec<(f64, crate::infrastructure::ingredient_cache::IngredientData)> = candidates
            .into_iter()
            .map(|p| {
                let np = norm(p.protein_per_100g, min_protein, max_protein);
                let nc = norm(p.calories_per_100g, min_cal, max_cal);
                let nf = norm(p.fat_per_100g, min_fat, max_fat);
                // density bonus: high protein AND low cal
                let density_bonus: f64 = if p.protein_per_100g > 15.0 && p.calories_per_100g < 200.0 { 0.1 } else { 0.0 };

                let score: f64 = match goal {
                    HealthGoal::HighProtein => 0.60 * np + 0.20 * (1.0 - nc) + 0.10 * (1.0 - nf) + density_bonus,
                    HealthGoal::LowCalorie  => 0.15 * np + 0.55 * (1.0 - nc) + 0.20 * (1.0 - nf) + density_bonus,
                    HealthGoal::Balanced    => 0.35 * np + 0.35 * (1.0 - nc) + 0.20 * (1.0 - nf) + density_bonus,
                };
                (score, p)
            })
            .collect();

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        // ── Step 3: 80/20 exploration on the top pick ─────────────────────
        // Use turn_count proxy: day-of-second mod 5 == 0 triggers exploration
        let top10_len = scored.len().min(10);
        let explore = {
            use std::time::{SystemTime, UNIX_EPOCH};
            let secs = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
            // 20% of turns = every 5th second bucket
            secs % 5 == 0
        };

        if explore && top10_len > 1 {
            // Swap a random top-10 product to position 0
            use std::time::{SystemTime, UNIX_EPOCH};
            let secs = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
            let pick = (secs as usize % top10_len).max(1); // never pick 0 (already top)
            scored.swap(0, pick);
        }

        // ── Step 4: Diversity — pick top-n with no repeated category ──────
        let mut result = Vec::with_capacity(n);
        let mut seen_categories: Vec<&'static str> = Vec::new();

        // Apply diversity penalty (0.5x) to same-category items already seen,
        // then re-rank and pick greedily
        let category_of = |slug: &str| -> &'static str {
            let fish = ["salmon", "tuna", "cod", "herring", "mackerel", "trout", "sardine"];
            let veg  = ["spinach", "broccoli", "kale", "carrot", "tomato", "cucumber", "beet", "celery", "asparagus", "zucchini", "peppers"];
            let meat = ["chicken", "beef", "pork", "lamb", "turkey", "duck"];
            let dairy= ["milk", "cheese", "butter", "yogurt", "cream", "eggs"];
            let grain= ["rice", "oat", "quinoa", "wheat", "pasta", "bread", "corn"];
            let nut  = ["almond", "walnut", "cashew", "pecan", "hazelnut", "peanut", "pistachio"];
            let fruit= ["apple", "banana", "orange", "mango", "berry", "blueberr", "strawberr", "lemon", "avocado"];
            let slug_l = slug.to_lowercase();
            if fish.iter().any(|k| slug_l.contains(k)) { return "fish"; }
            if veg.iter().any(|k| slug_l.contains(k))  { return "vegetable"; }
            if meat.iter().any(|k| slug_l.contains(k)) { return "meat"; }
            if dairy.iter().any(|k| slug_l.contains(k)){ return "dairy"; }
            if grain.iter().any(|k| slug_l.contains(k)){ return "grain"; }
            if nut.iter().any(|k| slug_l.contains(k))  { return "nut"; }
            if fruit.iter().any(|k| slug_l.contains(k)){ return "fruit"; }
            "other"
        };

        // Re-score with diversity penalty applied, then pick greedily
        let mut pool = scored;
        while result.len() < n && !pool.is_empty() {
            // Apply current diversity penalties
            let best_idx = pool.iter().enumerate().map(|(i, (raw_score, p))| {
                let cat = category_of(&p.slug);
                let penalty = if seen_categories.contains(&cat) { 0.5 } else { 1.0 };
                (i, raw_score * penalty)
            }).max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
                .map(|(i, _)| i)
                .unwrap_or(0);

            let (score, p) = pool.remove(best_idx);
            let cat = category_of(&p.slug);
            if !seen_categories.contains(&cat) {
                seen_categories.push(cat);
            }

            // Build explainability
            let reason_idx = result.len() + 1;
            let (reason_tag, reason_text) = match goal {
                HealthGoal::HighProtein => (
                    "high_protein",
                    format!("protein: {:.1}g/100g, score {:.2} (rank #{})", p.protein_per_100g, score, reason_idx),
                ),
                HealthGoal::LowCalorie => (
                    "low_calorie",
                    format!("only {} kcal/100g, score {:.2} (rank #{})", p.calories_per_100g as i32, score, reason_idx),
                ),
                HealthGoal::Balanced => (
                    "balanced",
                    format!("balanced score {:.2}: {:.1}g protein, {} kcal/100g (rank #{})", score, p.protein_per_100g, p.calories_per_100g as i32, reason_idx),
                ),
            };

            result.push((p, reason_tag, reason_text));
        }

        result
    }

    /// Kept as single-product convenience wrapper (used by meal idea + follow-up).
    async fn select_healthy_product(
        &self,
        goal: HealthGoal,
    ) -> Option<(crate::infrastructure::ingredient_cache::IngredientData, &'static str, String)> {
        self.select_top_products(goal, 1, &[], None).await.into_iter().next()
    }

    fn handle_conversion(&self, input: &str, lang: ChatLang) -> ChatResponse {
        if let Some((value, from, to)) = extract_conversion(input) {
            let result_raw = uc::convert_units(value, &from, &to);
            let supported = result_raw.is_some();
            let result = uc::display_round(result_raw.unwrap_or(0.0));
            rb::build_conversion(value, from, to, result, supported, lang)
        } else {
            rb::build_conversion_hint(lang)
        }
    }

    /// Density-aware conversion: "100г лосося в мл" uses product's density_g_per_ml
    async fn handle_conversion_with_density(&self, input: &str, lang: ChatLang) -> ChatResponse {
        // Try to find a product mentioned in the conversion request
        let product = self.find_ingredient_in_text(input).await;

        if let Some((value, from, to)) = extract_conversion(input) {
            // If converting between g ↔ ml (or tbsp/cup) and product has density → use it
            let density_result = product.as_ref().and_then(|p| p.density_g_per_ml).and_then(|density| {
                density_convert(value, &from, &to, density)
            });

            if let Some(result) = density_result {
                let pname = product.as_ref().map(|p| p.name(lang.code()).to_string()).unwrap_or_default();
                return rb::build_conversion_with_product(value, from, to, result, &pname, lang);
            }

            // Check if this is a volume↔weight conversion without a product — ask "for what?"
            let needs_density = is_volume_weight_pair(&from, &to);
            if needs_density && product.is_none() {
                return rb::build_conversion_ask_product(value, from, to, lang);
            }

            // Fallback to standard unit conversion
            let result_raw = uc::convert_units(value, &from, &to);
            let supported = result_raw.is_some();
            let result = uc::display_round(result_raw.unwrap_or(0.0));
            rb::build_conversion(value, from, to, result, supported, lang)
        } else {
            rb::build_conversion_hint(lang)
        }
    }

    async fn handle_nutrition(&self, input: &str, lang: ChatLang) -> ChatResponse {
        if let Some(p) = self.find_ingredient_in_text(input).await {
            rb::build_nutrition(&p, lang)
        } else {
            rb::build_nutrition_hint(lang)
        }
    }

    async fn handle_cooking_loss(&self, input: &str, lang: ChatLang) -> ChatResponse {
        match self.find_ingredient_in_text(input).await {
            Some(p) => {
                tracing::info!("🔥 cooking_loss: {} ({} states)", p.slug, p.states.len());
                rb::build_cooking_loss(&p, lang)
            }
            None => rb::build_cooking_loss_no_product(lang),
        }
    }

    fn handle_seasonality(&self, input: &str, lang: ChatLang) -> ChatResponse {
        let text_lower = input.to_lowercase();
        let product = detect_season_product(&text_lower);
        rb::build_seasonality(product, lang)
    }

    async fn handle_recipe(&self, input: &str, lang: ChatLang, goal: HealthGoal, modifier: super::goal_modifier::HealthModifier) -> ChatResponse {
        // ── Step 0: Parse dietary constraints from user text ──
        let constraints = super::user_constraints::parse_constraints(input, lang);
        if !constraints.is_empty() {
            tracing::info!("🥗 Dietary constraints: {:?}", constraints.raw_exclusions);
        }

        // ── Step 1: Ask Gemini for dish name + ingredient list (50-100 tokens) ──
        match recipe_engine::ask_gemini_dish_schema(&self.llm_adapter, input, lang, goal).await {
            Ok(schema) => {
                tracing::info!(
                    "🍽 recipe_engine: dish={} items=[{}]",
                    schema.dish,
                    schema.items.join(", ")
                );

                // ── Step 2: Backend resolves everything: roles, states, grams, yield, КБЖУ ──
                let mut tech_card = recipe_engine::resolve_dish(&self.ingredient_cache, &schema, goal, lang, &constraints, modifier).await;

                tracing::info!(
                    "📊 tech_card: output={:.0}g kcal={} resolved={}/{} unresolved=[{}]",
                    tech_card.total_output_g,
                    tech_card.total_kcal,
                    tech_card.ingredients.len() - tech_card.unresolved.len(),
                    tech_card.ingredients.len(),
                    tech_card.unresolved.join(", ")
                );

                // ── Step 2b: Generate dish photo with gemini-2.5-flash-image ──
                let dish_slug = schema.dish.to_lowercase().replace(' ', "-");
                let ingredient_names: Vec<String> = schema.items.iter().take(5).cloned().collect();
                match self.llm_adapter.generate_dish_image(&dish_slug, &schema.dish, &ingredient_names).await {
                    Ok(base64) => {
                        tracing::info!("🖼 Dish image generated for '{}'", schema.dish);
                        tech_card.dish_image_url = Some(format!("data:image/png;base64,{}", base64));
                    }
                    Err(e) => {
                        tracing::warn!("⚠️ Dish image generation failed for '{}': {}", schema.dish, e);
                        // Non-fatal — recipe response continues without image
                    }
                }

                // ── Step 3: Build text + card response ──
                let text = recipe_engine::format_recipe_text(&tech_card, lang);
                rb::build_recipe_card(&tech_card, text, lang)
            }
            Err(e) => {
                tracing::warn!("⚠️ recipe_engine failed: {} — falling back to static hint", e);
                let text_lower = input.to_lowercase();
                let dish = detect_dish_keyword(&text_lower);
                rb::build_recipe(dish, lang)
            }
        }
    }

    async fn handle_meal_idea(&self, lang: ChatLang, goal: HealthGoal, input: &str, ctx: &SessionContext) -> ChatResponse {
        let text_lower = input.to_lowercase();
        let is_meal_plan = text_lower.contains("план")
            || text_lower.contains("plan")
            || text_lower.contains("рацион")
            || text_lower.contains("собрать день")
            || text_lower.contains("build my day")
            || text_lower.contains("ułóż dzień");

        if is_meal_plan {
            return self.handle_meal_plan(lang, goal).await;
        }

        // ── Dynamic Meal Combo (primary path) ────────────────────────────────
        // Build a smart combo from live cache: protein + side [+ base]
        let all = self.ingredient_cache.all().await;
        let exclude = ctx.excluded_slugs();
        if let Some(combo) = meal_builder::build_combo(&all, goal, &exclude) {
            tracing::info!(
                "🍽 meal_combo: {} + {} [+ {}] → {}kcal {:.0}g protein",
                combo.protein.slug,
                combo.side.slug,
                combo.base.as_ref().map(|b| b.slug.as_str()).unwrap_or("-"),
                combo.total_kcal,
                combo.total_protein,
            );
            return rb::build_meal_combo(&combo, lang, goal);
        }

        // ── Fallback: static meal idea tables ────────────────────────────────
        tracing::debug!("⚠️ meal_builder returned None → static table fallback");
        let ideas_ru: &[(&str, &str, &str)] = match goal {
            HealthGoal::HighProtein => &[
                ("Куриная грудка с киноа", "chicken-breast", "Высокобелковое блюдо: ~50г белка на порцию."),
                ("Тунец с яйцами", "tuna", "Силовой завтрак. Тунец + яйца = ~40г белка за 10 минут."),
                ("Лосось с брокколи", "salmon", "Омега-3 + белок. Запекается 20 мин."),
            ],
            HealthGoal::LowCalorie => &[
                ("Шпинатный салат", "spinach", "Всего ~25 ккал на 100г."),
                ("Суп из брокколи", "broccoli", "Сытный и лёгкий — около 120 ккал."),
            ],
            HealthGoal::Balanced => &[
                ("Паста с курицей и шпинатом", "chicken-breast", "Быстрый и сытный ужин."),
                ("Омлет с овощами", "eggs", "Идеальный завтрак."),
                ("Запечённый лосось с овощами", "salmon", "Лосось богат омега-3."),
            ],
        };
        let ideas_en: &[(&str, &str, &str)] = match goal {
            HealthGoal::HighProtein => &[
                ("Chicken & Quinoa Bowl", "chicken-breast", "~50g protein per serving."),
                ("Baked Salmon with Broccoli", "salmon", "Omega-3 + protein."),
            ],
            HealthGoal::LowCalorie => &[
                ("Spinach Salad", "spinach", "Only ~25 kcal/100g."),
                ("Broccoli Soup", "broccoli", "Filling and light."),
            ],
            HealthGoal::Balanced => &[
                ("Chicken & Spinach Pasta", "chicken-breast", "Ready in 20 minutes."),
                ("Baked Salmon with Vegetables", "salmon", "Rich in omega-3."),
            ],
        };

        let hour = chrono::Utc::now().hour() as usize;
        let ideas = match lang {
            ChatLang::Ru | ChatLang::Uk => ideas_ru,
            ChatLang::En | ChatLang::Pl => ideas_en,
        };
        let (meal_name, slug, description) = ideas[hour % ideas.len()];

        if let Some(p) = self.ingredient_cache.get(slug).await {
            return rb::build_meal_idea(meal_name, description, slug, &p, lang, goal);
        }

        rb::build_meal_idea_text_only(meal_name, description, lang)
    }

    /// Full day meal plan: breakfast + lunch + dinner with diverse product cards.
    /// Enforces different product categories per meal slot.
    async fn handle_meal_plan(&self, lang: ChatLang, goal: HealthGoal) -> ChatResponse {
        let all = self.ingredient_cache.all().await;
        if all.is_empty() {
            return rb::build_meal_plan(&[], lang, goal);
        }

        // Define preferred protein categories per meal slot for maximum diversity
        let breakfast_types = ["dairy", "legume", "grain", "eggs"];
        let lunch_types = ["fish", "seafood", "poultry"];
        let dinner_types = ["meat", "poultry", "fish"];

        // ── Calorie caps per goal (per 100g) ────────────────────────────────
        // Prevents absurd picks like bacon (541 kcal) or cheese (374 kcal)
        // in a "balanced" or "low calorie" plan.
        let max_cal_per_100g: f32 = match goal {
            HealthGoal::LowCalorie  => 200.0,
            HealthGoal::Balanced    => 300.0,
            HealthGoal::HighProtein => 500.0,
        };

        let find_for_types = |preferred: &[&str], exclude_slugs: &[String]| -> Option<crate::infrastructure::ingredient_cache::IngredientData> {
            let mut candidates: Vec<_> = all.iter()
                .filter(|p| {
                    !exclude_slugs.contains(&p.slug)
                    && (p.calories_per_100g > 0.0 || p.protein_per_100g > 0.0)
                    && p.calories_per_100g <= max_cal_per_100g
                    && preferred.iter().any(|t| p.product_type == *t)
                    && !matches!(p.product_type.as_str(), "spice" | "herb" | "condiment" | "oil" | "beverage" | "other")
                })
                .collect();

            if candidates.is_empty() {
                // Fallback: any protein-role product not yet used (with calorie cap)
                candidates = all.iter()
                    .filter(|p| {
                        !exclude_slugs.contains(&p.slug)
                        && p.protein_per_100g >= 5.0
                        && p.calories_per_100g <= max_cal_per_100g
                        && !matches!(p.product_type.as_str(), "spice" | "herb" | "condiment" | "oil" | "beverage" | "other")
                    })
                    .collect();
            }

            if candidates.is_empty() { return None; }

            // Score by goal — stronger calorie penalty for Balanced
            candidates.sort_by(|a, b| {
                let score = |p: &crate::infrastructure::ingredient_cache::IngredientData| -> f64 {
                    match goal {
                        HealthGoal::HighProtein => {
                            p.protein_per_100g as f64 * 2.0 - p.fat_per_100g as f64 * 0.3
                        }
                        HealthGoal::LowCalorie => {
                            (300.0 - p.calories_per_100g as f64) * 0.05
                            + p.protein_per_100g as f64 * 0.5
                        }
                        HealthGoal::Balanced => {
                            // Reward protein, strongly penalize high calorie & fat
                            p.protein_per_100g as f64 * 1.5
                            - p.calories_per_100g as f64 * 0.02
                            - p.fat_per_100g as f64 * 0.3
                        }
                    }
                };
                score(b).partial_cmp(&score(a)).unwrap_or(std::cmp::Ordering::Equal)
            });

            // 80/20 exploration: pick from top 3
            let explore_idx = {
                use std::time::{SystemTime, UNIX_EPOCH};
                let secs = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
                (secs % 3) as usize
            };
            let idx = explore_idx.min(candidates.len() - 1);
            Some(candidates[idx].clone())
        };

        let mut used_slugs: Vec<String> = Vec::new();
        let mut products = Vec::new();

        // Breakfast
        if let Some(p) = find_for_types(&breakfast_types, &used_slugs) {
            let reason = format!("breakfast pick ({})", p.product_type);
            used_slugs.push(p.slug.clone());
            products.push((p, "balanced" as &'static str, reason));
        }

        // Lunch
        if let Some(p) = find_for_types(&lunch_types, &used_slugs) {
            let reason = format!("lunch pick ({})", p.product_type);
            used_slugs.push(p.slug.clone());
            products.push((p, "balanced" as &'static str, reason));
        }

        // Dinner
        if let Some(p) = find_for_types(&dinner_types, &used_slugs) {
            let reason = format!("dinner pick ({})", p.product_type);
            used_slugs.push(p.slug.clone());
            products.push((p, "balanced" as &'static str, reason));
        }

        // Fallback: if any slot missed, fill with top products
        if products.len() < 3 {
            let extra = self.select_top_products(goal, 3 - products.len(), &used_slugs, None).await;
            products.extend(extra);
        }

        rb::build_meal_plan(&products, lang, goal)
    }

    fn handle_unknown_static(&self, lang: ChatLang) -> ChatResponse {
        rb::build_unknown(lang)
    }

    /// ProductInfo — "что такое лосось", "расскажи о шпинате", "what is chicken"
    async fn handle_product_info(&self, input: &str, lang: ChatLang) -> ChatResponse {
        if let Some(p) = self.find_ingredient_in_text(input).await {
            return rb::build_product_info(&p, lang);
        }

        // Product not in cache → try LLM for a short description
        let lang_name = match lang {
            ChatLang::Ru => "Russian",
            ChatLang::En => "English",
            ChatLang::Pl => "Polish",
            ChatLang::Uk => "Ukrainian",
        };
        let prompt = format!(
            "You are ChefOS, a culinary assistant. The user asked: \"{}\". \
            Answer in {} in 2-3 sentences only about the food product mentioned. \
            Include: what it is, key nutrients, typical culinary use. No markdown.",
            input, lang_name
        );
        match self.llm_adapter.groq_raw_request(&prompt, 200).await {
            Ok(text) => rb::build_product_info_llm(text, lang),
            Err(_) => rb::build_product_not_found(lang),
        }
    }

    /// Legacy LLM fallback — replaced by AI Brain (Layer 2).
    #[allow(dead_code)]
    async fn handle_fallback_llm(&self, input: &str, lang: ChatLang) -> ChatResponse {
        let lang_name = match lang {
            ChatLang::Ru => "Russian",
            ChatLang::En => "English",
            ChatLang::Pl => "Polish",
            ChatLang::Uk => "Ukrainian",
        };
        let prompt = format!(
            "You are ChefOS, a culinary assistant. The user asked: \"{}\". \
            Reply in {} in 1-3 sentences. \
            ONLY answer if the question is about food, cooking, nutrition, ingredients, or recipes. \
            If it's unrelated to food/cooking, politely say you only help with culinary topics. \
            No markdown, no bullet points.",
            input, lang_name
        );
        match self.llm_adapter.groq_raw_request(&prompt, 250).await {
            Ok(text) => {
                tracing::debug!("🤖 LLM fallback answered: {:?}", &text[..text.len().min(100)]);
                ChatResponse::text_only(text, Intent::Unknown, lang, 0)
            }
            Err(e) => {
                tracing::warn!("⚠️ LLM fallback failed: {}", e);
                self.handle_unknown_static(lang)
            }
        }
    }

    // ── Internal helpers ─────────────────────────────────────────────────────

    /// Scan ingredient cache for any product slug/name mentioned in the input.
    ///
    /// Two-pass strategy:
    ///   Pass 1: Stem-based keyword map — covers declensions (лосось/лососе/лососём)
    ///   Pass 2: Full-cache scan — match any product whose name appears in the input
    async fn find_ingredient_in_text(
        &self,
        input: &str,
    ) -> Option<crate::infrastructure::ingredient_cache::IngredientData> {
        let text_lower = input.to_lowercase();

        // ── Pass 1: Stem → slug mapping (covers word forms / declensions) ────
        // Use word STEMS (shortest unambiguous root) instead of full words.
        // "лосос" matches: лосось, лосося, лососе, лососём, лососи
        // "куриц" matches: курица, курицы, курицу, курице, курицей
        let stem_slugs: &[(&str, &str)] = &[
            // Fish & seafood
            ("лосос",    "salmon"),    ("salmon",    "salmon"),    ("łosoś",    "salmon"),    ("łosos",    "salmon"),
            ("тунц",     "tuna"),      ("tuna",      "tuna"),      ("tuńczyk",  "tuna"),
            ("треск",    "cod"),       ("cod",       "cod"),       ("dorsz",    "cod"),
            ("форел",    "trout"),     ("trout",     "trout"),     ("pstrąg",   "trout"),
            ("скумбри",  "mackerel"),  ("mackerel",  "mackerel"),  ("makrela",  "mackerel"),
            ("сардин",   "sardines"),  ("sardine",   "sardines"),
            ("креветк",  "shrimp"),    ("shrimp",    "shrimp"),    ("krewetk",  "shrimp"),
            // Poultry & meat
            ("куриц",    "chicken-breast"), ("курятин",  "chicken-breast"), ("chicken",  "chicken-breast"),
            ("kurczak",  "chicken-breast"), ("курк",     "chicken-breast"),
            ("индейк",   "turkey"),    ("turkey",    "turkey"),    ("indyk",    "turkey"),
            ("говядин",  "beef"),      ("beef",      "beef"),      ("wołowin",  "beef"),
            ("свинин",   "pork"),      ("pork",      "pork"),      ("wieprzow", "pork"),
            // Eggs & dairy
            ("яйц",     "eggs"),      ("яйк",      "eggs"),      ("egg",      "eggs"),      ("jajk",     "eggs"),
            ("молок",    "milk"),      ("milk",      "milk"),      ("mlek",     "milk"),
            ("масл",     "butter"),    ("butter",    "butter"),    ("masł",     "butter"),
            ("сыр",      "cheese"),    ("cheese",    "cheese"),    ("ser ",     "cheese"),
            ("творог",   "cottage-cheese"), ("cottage",  "cottage-cheese"), ("twaróg", "cottage-cheese"),
            // Vegetables
            ("шпинат",   "spinach"),   ("spinach",   "spinach"),   ("szpinak",  "spinach"),
            ("брокколи", "broccoli"),  ("broccoli",  "broccoli"),  ("brokuł",   "broccoli"),
            ("помидор",  "tomato"),    ("томат",     "tomato"),    ("tomato",   "tomato"),    ("pomidor",  "tomato"),
            ("картофел", "potato"),    ("картошк",   "potato"),    ("potato",   "potato"),    ("ziemniak", "potato"),
            ("морков",   "carrot"),    ("carrot",    "carrot"),    ("marchew",  "carrot"),    ("морквин",  "carrot"),
            ("лук",      "onion"),     ("onion",     "onion"),     ("cebul",    "onion"),
            ("чеснок",   "garlic"),    ("часник",    "garlic"),    ("garlic",   "garlic"),    ("czosn",    "garlic"),
            ("огурц",    "cucumber"),  ("cucumber",  "cucumber"),  ("ogórek",   "cucumber"),  ("огірк",    "cucumber"),
            ("капуст",   "cabbage"),   ("cabbage",   "cabbage"),   ("kapust",   "cabbage"),
            ("перц",     "bell-pepper"), ("pepper",  "bell-pepper"), ("papryk",  "bell-pepper"),
            ("авокадо",  "avocado"),   ("avocado",   "avocado"),
            ("батат",    "sweet-potato"), ("sweet potato", "sweet-potato"),
            // Fruits & berries
            ("яблок",    "apple"),     ("apple",     "apple"),     ("jabłk",    "apple"),
            ("банан",    "banana"),    ("banana",    "banana"),
            ("черник",   "blueberry"), ("blueberr",  "blueberry"), ("borówk",   "blueberry"),
            ("клубник",  "strawberry"),("strawberr", "strawberry"),("truskawk", "strawberry"),
            ("лимон",    "lemon"),     ("lemon",     "lemon"),     ("cytryn",   "lemon"),
            // Nuts & seeds
            ("миндал",   "almonds"),   ("almond",    "almonds"),   ("migdał",   "almonds"),
            ("грецк",    "walnuts"),   ("walnut",    "walnuts"),   ("orzech włosk", "walnuts"),
            ("орех",     "walnuts"),
            ("фисташк",  "pistachios"), ("pistachio", "pistachios"),
            // Grains & legumes
            ("рис",      "rice"),      ("rice",      "rice"),      ("ryż",      "rice"),
            ("гречк",    "buckwheat"), ("buckwheat", "buckwheat"), ("gryczana", "buckwheat"),
            ("овсянк",   "oats"),      ("oat",       "oats"),      ("owsian",   "oats"),
            ("чечевиц",  "lentils"),   ("lentil",    "lentils"),   ("soczewic", "lentils"),
            ("киноа",    "quinoa"),    ("quinoa",    "quinoa"),
            // Other
            ("мёд",      "honey"),     ("мед",       "honey"),     ("honey",    "honey"),     ("miód",     "honey"),
        ];

        for (stem, slug) in stem_slugs {
            if text_lower.contains(stem) {
                // Try the mapped slug first, then common singular/plural variants
                // (seed-data inconsistency safety net).
                let candidates: [String; 3] = [
                    (*slug).to_string(),
                    if slug.ends_with('s') { slug.trim_end_matches('s').to_string() } else { format!("{}s", slug) },
                    if slug.ends_with("es") { slug.trim_end_matches("es").to_string() } else { format!("{}es", slug) },
                ];
                for cand in &candidates {
                    if let Some(p) = self.ingredient_cache.get(cand).await {
                        return Some(p);
                    }
                }
            }
        }

        // ── Pass 2: Full-cache scan — fuzzy match by product names ───────────
        // For products not in the stem map, check if any cached product's name
        // (in any language) appears in the user input, or vice versa.
        let all = self.ingredient_cache.all().await;

        // Score each product: longer name match = higher confidence
        let mut best: Option<(usize, crate::infrastructure::ingredient_cache::IngredientData)> = None;

        for p in &all {
            let names = [
                p.name_en.to_lowercase(),
                p.name_ru.to_lowercase(),
                p.name_pl.to_lowercase(),
                p.name_uk.to_lowercase(),
                p.slug.replace('-', " "),
            ];

            for name in &names {
                // Skip very short names that could false-match (e.g. "рис" in "рисунок")
                if name.len() < 3 { continue; }

                if text_lower.contains(name.as_str()) {
                    let score = name.len();
                    if best.as_ref().map_or(true, |(s, _)| score > *s) {
                        best = Some((score, p.clone()));
                    }
                }
            }
        }

        best.map(|(_, p)| p)
    }
}

// ── Simple NLP helpers ────────────────────────────────────────────────────────

/// Try to extract (value, from_unit, to_unit) from text like "200 грамм в ложках".
fn extract_conversion(input: &str) -> Option<(f64, String, String)> {
    let text = input.to_lowercase();

    // Find a number
    let value = text
        .split_whitespace()
        .find_map(|w| w.parse::<f64>().ok())?;

    // Map Russian/Polish keywords to standard unit codes
    let unit_map: &[(&str, &str)] = &[
        ("грамм", "g"),  ("граммов", "g"), ("грам", "g"), ("г ", "g"),
        ("килограмм", "kg"), ("кг", "kg"), ("кило", "kg"),
        ("миллилитр", "ml"), ("мл", "ml"),
        ("литр", "l"), ("л ", "l"),
        ("унци", "oz"), ("унц", "oz"), ("oz", "oz"),
        ("ложк", "tbsp"), ("tbsp", "tbsp"), ("ст.л", "tbsp"),
        ("чайн", "tsp"), ("tsp", "tsp"), ("ч.л", "tsp"),
        ("стакан", "cup"), ("cup", "cup"),
        // Polish
        ("gram", "g"), ("kilogram", "kg"), ("mililitr", "ml"),
        ("litr", "l"), ("łyżk", "tbsp"), ("szklank", "cup"),
        // English
        ("gram", "g"), ("kilogram", "kg"), ("milliliter", "ml"),
        ("liter", "l"), ("tablespoon", "tbsp"), ("teaspoon", "tsp"),
    ];

    let from_unit = unit_map.iter()
        .find(|(kw, _)| text.contains(kw) && text.find(kw) < text.rfind(|c: char| c.is_ascii_digit()).map(|p| p + 10).or(Some(1000)))
        .map(|(_, u)| u.to_string())?;

    // Find "to" unit — look after "в ", "in ", "na "
    let after_markers = ["в ", "in ", "na ", "to ", "на "];
    let search_start = after_markers.iter()
        .find_map(|m| text.find(m).map(|p| p + m.len()))
        .unwrap_or(0);
    let rest = &text[search_start..];

    let to_unit = unit_map.iter()
        .find(|(kw, u)| rest.contains(kw) && *u != from_unit)
        .map(|(_, u)| u.to_string())?;

    Some((value, from_unit, to_unit))
}

fn detect_season_product(text: &str) -> Option<&'static str> {
    let products = [
        ("salmon", &["лосось", "salmon", "łosoś"][..]),
        ("strawberry", &["клубника", "strawberry", "truskawka"][..]),
        ("herring", &["сельдь", "селёдка", "herring", "śledź"][..]),
        ("mushrooms", &["гриб", "mushroom", "grzyb"][..]),
    ];
    for (slug, keywords) in &products {
        if keywords.iter().any(|kw| text.contains(kw)) {
            return Some(slug);
        }
    }
    None
}

fn detect_dish_keyword(text: &str) -> Option<&'static str> {
    let dishes = [
        ("борщ", "borscht"), ("pasta", "pasta"), ("паст", "pasta"),
        ("суп", "soup"), ("soup", "soup"), ("zup", "soup"),
        ("салат", "salad"), ("salad", "salad"), ("sałatk", "salad"),
        ("омлет", "omelette"), ("omelette", "omelette"),
        ("пицца", "pizza"), ("pizza", "pizza"),
        ("котлет", "cutlet"), ("стейк", "steak"), ("steak", "steak"),
    ];
    for (keyword, dish_name) in &dishes {
        if text.contains(keyword) {
            return Some(dish_name);
        }
    }
    None
}

/// Density-based g ↔ ml conversion for a specific product.
/// 1 tbsp ≈ 15 ml, 1 cup ≈ 240 ml, 1 tsp ≈ 5 ml.
fn density_convert(value: f64, from: &str, to: &str, density: f32) -> Option<f64> {
    let d = density as f64;
    // Convert everything to grams first, then to target
    let grams = match from {
        "g"    => value,
        "kg"   => value * 1000.0,
        "ml"   => value * d,
        "l"    => value * 1000.0 * d,
        "tbsp" => value * 15.0 * d,
        "tsp"  => value * 5.0 * d,
        "cup"  => value * 240.0 * d,
        _ => return None,
    };
    let result = match to {
        "g"    => grams,
        "kg"   => grams / 1000.0,
        "ml"   => grams / d,
        "l"    => grams / (1000.0 * d),
        "tbsp" => grams / (15.0 * d),
        "tsp"  => grams / (5.0 * d),
        "cup"  => grams / (240.0 * d),
        _ => return None,
    };
    Some(uc::display_round(result))
}

/// Check if the from/to pair involves volume↔weight (needs density).
fn is_volume_weight_pair(from: &str, to: &str) -> bool {
    let weight = ["g", "kg", "oz"];
    let volume = ["ml", "l", "tbsp", "tsp", "cup"];
    (weight.contains(&from) && volume.contains(&to))
        || (volume.contains(&from) && weight.contains(&to))
}

// needed for hour/day helpers
use chrono::Timelike;

// ── Action Layer ─────────────────────────────────────────────────────────────
//
// Centralizes decision of "which buttons should appear on each card".
// The frontend (iOS) simply renders whatever actions the backend attaches.
//
// This lets us:
//   - A/B test button sets
//   - Hide "Add to plan" if already added (future: session-aware)
//   - Keep the contract in one place instead of duplicated across 10+ card
//     construction sites.

/// Attach user-invokable actions to every card in the response.
/// Idempotent — if a card already has actions, they're preserved.
///
/// Step 3 (stateful): drops actions that no longer make sense given the
/// current session state. A recipe already in the user's plan loses
/// AddToPlan; a product already in the shopping list loses AddToShopping.
/// StartCooking and ShowRecipesFor are always kept — they remain useful
/// even after the item is added.
fn enrich_with_actions(response: &mut ChatResponse, ctx: &SessionContext) {
    for card in response.cards.iter_mut() {
        match card {
            Card::Recipe(r) if r.actions.is_empty() => {
                // Stable slug is the single source of truth for recipe identity.
                // display_name/dish_name_local are localized/LLM-rephrased
                // and MUST NOT be used as an id.
                let recipe_id = r.slug.clone();
                let already_planned = ctx.added_recipes.iter().any(|id| id == &recipe_id);
                let mut acts = Vec::with_capacity(2);
                if !already_planned {
                    acts.push(Action::AddToPlan { recipe_id: recipe_id.clone() });
                }
                acts.push(Action::StartCooking { recipe_id });
                r.actions = acts;
            }
            Card::Product(p) if p.actions.is_empty() => {
                let already_in_list = ctx.added_products.iter().any(|s| s == &p.slug);
                let mut acts = Vec::with_capacity(2);
                if !already_in_list {
                    acts.push(Action::AddToShopping { product_slug: p.slug.clone() });
                }
                acts.push(Action::ShowRecipesFor { product_slug: p.slug.clone() });
                p.actions = acts;
            }
            _ => {}
        }
    }
}


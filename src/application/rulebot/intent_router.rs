//! Intent Router — score-based intent detection from natural language input.
//!
//! DDD Architecture:
//!   `intent_keywords.rs`  → WHAT words mean (data tables)
//!   `goal_modifier.rs`    → WHICH goal the user wants (modifier detection)
//!   `intent_router.rs`    → HOW to score/route (this file — the BRAIN)
//!
//! NOT simple `text.contains()` — uses weighted keyword scoring per intent.
//! Supports:
//!   - Multi-intent: "что-то полезное и быстрое" → [HealthyProduct, MealIdea]
//!   - Context modifiers: "на массу" → HighProtein, "хочу сушиться" → LowCalorie
//!   - Score threshold: min 2 pts to activate an intent
//!
//! ```text
//! "что полезного поесть"        → [HealthyProduct]  score=5
//! "200 грамм в ложках"          → [Conversion]      score=6
//! "что поесть полезное быстрое" → [HealthyProduct, MealIdea]
//! "хочу на массу"               → modifier=HighProtein
//! "хочу сушиться"               → modifier=LowCalorie
//! "кето рецепт"                 → modifier=LowCarb
//! "что-то сытное"               → modifier=ComfortFood
//! ```

use serde::{Deserialize, Serialize};

// Re-export HealthModifier from goal_modifier (single source of truth)
pub use super::goal_modifier::HealthModifier;
pub use super::goal_modifier::detect_modifier;

// Import keyword data tables
use super::intent_keywords as kw;
// Category detection — used as a fallback signal when keyword scoring fails.
use super::category_filter::detect_category;

// ── Dialog Context (minimal, passed by caller) ───────────────────────────────

/// Minimal context from previous turn — passed into `parse_input_with_context`.
/// Keeps intent_router free from session_context dependency (DDD boundary).
#[derive(Debug, Clone, Default)]
pub struct DialogContext {
    /// What was the previous turn's intent?
    pub last_intent: Option<Intent>,
    /// Remembered goal modifier (e.g. HighProtein from "на массу")
    pub last_modifier: Option<HealthModifier>,
    /// How many turns into the session (0 = first turn)
    pub turn_count: u32,
}

// ── Intent Enum ──────────────────────────────────────────────────────────────

/// Detected user intent from free-text input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Intent {
    /// "привет", "hello", "cześć"
    Greeting,
    /// "полезный продукт", "healthy food", "zdrowy produkt"
    HealthyProduct,
    /// "грамм в ложках", "convert 200g", "ile to łyżek"
    Conversion,
    /// "рецепт борща", "recipe for soup", "przepis na zupę"
    RecipeHelp,
    /// "сезон лосося", "when is salmon in season"
    Seasonality,
    /// "что приготовить", "meal idea", "co ugotować"
    MealIdea,
    /// "калории шпината", "nutrition info", "wartości odżywcze"
    NutritionInfo,
    /// "что такое лосось", "tell me about chicken", "co to jest szpinak"
    ProductInfo,
    /// Could not determine intent — triggers controlled LLM fallback
    Unknown,
}

impl Intent {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Greeting       => "greeting",
            Self::HealthyProduct => "healthy_product",
            Self::Conversion     => "conversion",
            Self::RecipeHelp     => "recipe_help",
            Self::Seasonality    => "seasonality",
            Self::MealIdea       => "meal_idea",
            Self::NutritionInfo  => "nutrition_info",
            Self::ProductInfo    => "product_info",
            Self::Unknown        => "unknown",
        }
    }
}

// ── Language Detection ───────────────────────────────────────────────────────

/// Detected language of the user input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChatLang {
    Ru,
    En,
    Pl,
    Uk,
}

impl ChatLang {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Ru => "ru",
            Self::En => "en",
            Self::Pl => "pl",
            Self::Uk => "uk",
        }
    }
}

// ── Score-based Intent Detection ─────────────────────────────────────────────

/// Single intent detection result with score.
#[derive(Debug, Clone)]
pub struct IntentScore {
    pub intent: Intent,
    pub score: i32,
}

/// Full parsed result: primary intent + all active intents + modifier.
#[derive(Debug, Clone)]
pub struct ParsedInput {
    /// Highest-scoring intent (primary).
    pub intent: Intent,
    /// All intents above threshold — enables multi-intent dispatch.
    pub intents: Vec<Intent>,
    /// Goal/context modifier detected from input.
    pub modifier: HealthModifier,
}

/// Detect primary intent (backward compatible).
pub fn detect_intent(input: &str) -> Intent {
    parse_input(input).intent
}

/// Full scored detection — returns intent + its score (for debug/logging).
pub fn detect_intent_scored(input: &str) -> IntentScore {
    const MIN_THRESHOLD: i32 = 2;
    let text = input.to_lowercase();
    let mut scores = score_all_intents(&text);
    apply_context_boosts(&text, &mut scores);
    let best = scores.iter().max_by_key(|(_, s)| *s).unwrap();
    if best.1 >= MIN_THRESHOLD {
        IntentScore { intent: best.0, score: best.1 }
    } else {
        IntentScore { intent: Intent::Unknown, score: 0 }
    }
}

/// Full parse: primary intent + multi-intents + goal modifier.
/// Use this in ChatEngine for full context.
pub fn parse_input(input: &str) -> ParsedInput {
    const MIN_THRESHOLD: i32 = 2;
    const SECONDARY_THRESHOLD: i32 = 2;

    let text = input.to_lowercase();
    let mut all_scores = score_all_intents(&text);
    apply_context_boosts(&text, &mut all_scores);

    // Best (primary)
    let best = all_scores.iter().max_by_key(|(_, s)| *s).unwrap();
    let mut primary = if best.1 >= MIN_THRESHOLD { best.0 } else { Intent::Unknown };

    // Category fallback (same logic as parse_input_with_context).
    if primary == Intent::Unknown {
        if let Some(cat) = detect_category(&text) {
            tracing::debug!("🥗 category fallback (no ctx): Unknown → HealthyProduct (category={})", cat.as_str());
            primary = Intent::HealthyProduct;
            for (intent, score) in all_scores.iter_mut() {
                if *intent == Intent::HealthyProduct {
                    *score = (*score).max(MIN_THRESHOLD);
                }
            }
        }
    }

    // All intents above secondary threshold (multi-intent)
    let mut intents: Vec<Intent> = all_scores
        .iter()
        .filter(|(i, s)| *s >= SECONDARY_THRESHOLD && *i != primary)
        .map(|(i, _)| *i)
        .collect();

    if primary != Intent::Unknown {
        intents.insert(0, primary);
    }

    let modifier = detect_modifier(&text);

    ParsedInput {
        intent: primary,
        intents,
        modifier,
    }
}

/// Context-aware parse: uses dialog history to resolve ambiguous/short inputs.
///
/// This is the MAIN entry point for ChatEngine. It handles:
///
/// 1. **Follow-up "ещё"** — "ещё" / "more" / "другое" → repeat `last_intent`
/// 2. **Short recipe refs** — "а борщ?" / "а с курицей?" when last = RecipeHelp → RecipeHelp
/// 3. **Confirmation** — "давай" / "да" / "ok" after HealthyProduct → repeat intent
/// 4. **Context continuity** — short ambiguous input inherits last_intent if no strong signal
/// 5. **Modifier → intent boost** — LowCalorie modifier boosts RecipeHelp/HealthyProduct
///
/// ```text
/// Turn 1: "хочу похудеть"       → HealthyProduct
/// Turn 2: "ещё"                  → HealthyProduct (from context)
/// Turn 3: "приготовь лёгкое"     → RecipeHelp (keywords)
/// Turn 4: "а борщ?"              → RecipeHelp (short + context)
/// Turn 5: "давай"                → RecipeHelp (confirmation + context)
/// ```
pub fn parse_input_with_context(input: &str, ctx: &DialogContext) -> ParsedInput {
    const MIN_THRESHOLD: i32 = 2;
    const SECONDARY_THRESHOLD: i32 = 2;

    let text = input.to_lowercase();
    let word_count = text.split_whitespace().count();

    // ── 1. Follow-up "ещё" / "more" / "другое" ─────────────────────────
    if let Some(last) = ctx.last_intent {
        if is_followup_more(&text) && last != Intent::Greeting && last != Intent::Unknown {
            tracing::debug!("🔄 follow-up 'more': repeating {:?}", last);
            return ParsedInput {
                intent: last,
                intents: vec![last],
                modifier: detect_modifier(&text),
            };
        }
    }

    // ── 2. Confirmation "давай" / "да" / "ok" ───────────────────────────
    if let Some(last) = ctx.last_intent {
        if is_confirmation(&text) && word_count <= 3 && last != Intent::Greeting {
            tracing::debug!("✅ confirmation: repeating {:?}", last);
            return ParsedInput {
                intent: last,
                intents: vec![last],
                modifier: detect_modifier(&text),
            };
        }
    }

    // ── 3. Regular scoring ──────────────────────────────────────────────
    let mut all_scores = score_all_intents(&text);
    apply_context_boosts(&text, &mut all_scores);

    // ── 4. Context boosts ───────────────────────────────────────────────

    // 4a. Short recipe references: "а борщ?", "а с курицей?" → RecipeHelp
    if let Some(Intent::RecipeHelp) = ctx.last_intent {
        if is_short_recipe_ref(&text) {
            for (intent, score) in all_scores.iter_mut() {
                if *intent == Intent::RecipeHelp {
                    *score += 5;
                }
            }
        }
    }

    // 4b. Short ambiguous input (1-2 words, no strong signal) → inherit last intent
    if word_count <= 2 && ctx.turn_count > 0 {
        let best_score = all_scores.iter().map(|(_, s)| *s).max().unwrap_or(0);
        if best_score < MIN_THRESHOLD {
            if let Some(last) = ctx.last_intent {
                if last != Intent::Greeting && last != Intent::Unknown {
                    tracing::debug!("📎 short input '{}' → inheriting {:?}", text, last);
                    for (intent, score) in all_scores.iter_mut() {
                        if *intent == last {
                            *score += 3;
                        }
                    }
                }
            }
        }
    }

    // 4c. Modifier → intent affinity boost
    // If user has an active goal modifier, slightly boost relevant intents
    let effective_modifier = detect_modifier(&text);
    let active_modifier = if effective_modifier != HealthModifier::None {
        effective_modifier
    } else {
        ctx.last_modifier.unwrap_or(HealthModifier::None)
    };

    if active_modifier != HealthModifier::None {
        apply_modifier_boost(active_modifier, &mut all_scores);
    }

    // ── 5. Resolve ──────────────────────────────────────────────────────
    let best = all_scores.iter().max_by_key(|(_, s)| *s).unwrap();
    let mut primary = if best.1 >= MIN_THRESHOLD { best.0 } else { Intent::Unknown };

    // ── 5b. Category-based fallback ────────────────────────────────────
    // If keyword scoring failed (Unknown) but the user mentioned a food
    // category (vegetables/fish/meat/fruit/...), treat it as an implicit
    // HealthyProduct request. This catches "какое мясо лучше", "what fruit",
    // "co jeść na obiad z ryb", etc. — phrasings that don't contain
    // "полезн/healthy" keywords but clearly ask for a food recommendation.
    if primary == Intent::Unknown {
        if let Some(cat) = detect_category(&text) {
            tracing::debug!("🥗 category fallback: Unknown → HealthyProduct (category={})", cat.as_str());
            primary = Intent::HealthyProduct;
            // Raise the score so multi-intent logic below sees it too.
            for (intent, score) in all_scores.iter_mut() {
                if *intent == Intent::HealthyProduct {
                    *score = (*score).max(MIN_THRESHOLD);
                }
            }
        }
    }

    let mut intents: Vec<Intent> = all_scores
        .iter()
        .filter(|(i, s)| *s >= SECONDARY_THRESHOLD && *i != primary)
        .map(|(i, _)| *i)
        .collect();

    if primary != Intent::Unknown {
        intents.insert(0, primary);
    }

    let modifier = detect_modifier(&text);

    ParsedInput {
        intent: primary,
        intents,
        modifier,
    }
}

// ── Context Helpers ──────────────────────────────────────────────────────────

/// Check if input is a "more" / "ещё" follow-up.
fn is_followup_more(text: &str) -> bool {
    kw::FOLLOWUP_MORE.iter().any(|kw| text.contains(kw))
}

/// Check if input is a confirmation ("давай", "да", "ok").
fn is_confirmation(text: &str) -> bool {
    kw::CONFIRM_SIGNALS.iter().any(|kw| text == *kw || text.starts_with(&format!("{} ", kw)))
}

/// Check if input is a short recipe reference: "а борщ?", "а с курицей?"
fn is_short_recipe_ref(text: &str) -> bool {
    // "а борщ?", "а если стейк?", "а с рыбой?"
    kw::SHORT_RECIPE_TRIGGERS.iter().any(|kw| text.starts_with(kw))
}

/// Boost scores based on active modifier → intent affinity.
///
/// LowCalorie/LowCarb/HighFiber  → slight boost to RecipeHelp + HealthyProduct
/// HighProtein                     → slight boost to RecipeHelp + HealthyProduct
/// Quick                           → slight boost to RecipeHelp
/// Budget                          → slight boost to HealthyProduct
/// ComfortFood                     → slight boost to RecipeHelp
fn apply_modifier_boost(modifier: HealthModifier, scores: &mut [(Intent, i32); 8]) {
    let (recipe_boost, healthy_boost) = match modifier {
        HealthModifier::LowCalorie  => (1, 1),
        HealthModifier::HighProtein => (1, 1),
        HealthModifier::LowCarb     => (1, 1),
        HealthModifier::HighFiber   => (1, 1),
        HealthModifier::Quick       => (1, 0),
        HealthModifier::Budget      => (0, 1),
        HealthModifier::ComfortFood => (1, 0),
        HealthModifier::None        => (0, 0),
    };

    for (intent, score) in scores.iter_mut() {
        match intent {
            Intent::RecipeHelp     => *score += recipe_boost,
            Intent::HealthyProduct => *score += healthy_boost,
            _ => {}
        }
    }
}

// ── Scoring Engine ────────────────────────────────────────────────────────────

/// Score all intents in one pass using keyword tables from `intent_keywords`.
fn score_all_intents(text: &str) -> [(Intent, i32); 8] {
    let hi_base = if text.trim() == "hi" { 3 } else { 0 };
    [
        (Intent::Greeting,       hi_base + sum_scores(text, kw::GREETING)),
        (Intent::Conversion,     sum_scores(text, kw::CONVERSION)),
        (Intent::NutritionInfo,  sum_scores(text, kw::NUTRITION)),
        (Intent::ProductInfo,    sum_scores(text, kw::PRODUCT_INFO)),
        (Intent::HealthyProduct, sum_scores(text, kw::HEALTHY)),
        (Intent::Seasonality,    sum_scores(text, kw::SEASONALITY)),
        (Intent::RecipeHelp,     sum_scores(text, kw::RECIPE)),
        (Intent::MealIdea,       sum_scores(text, kw::MEAL_IDEA)),
    ]
}

/// Sum weights of all matching keywords.
fn sum_scores(text: &str, keywords: &[kw::ScoredKeyword]) -> i32 {
    keywords.iter()
        .filter(|(kw, _)| text.contains(kw))
        .map(|(_, weight)| weight)
        .sum()
}

/// Context-aware score adjustments applied AFTER all individual scorers run.
///
/// Fixes:
/// - "хочу на массу, что поесть?" → MealIdea (goal + action verb = meal, not product list)
/// - "low calorie dinner for diet" → MealIdea (meal-time + diet = meal, not nutrition_info)
/// - "хочу похудеть, приготовь томатный суп" → RecipeHelp (explicit cook verb wins over goal)
fn apply_context_boosts(text: &str, scores: &mut [(Intent, i32); 8]) {
    let has_goal = kw::GOAL_SIGNALS.iter().any(|kw| text.contains(kw));
    let has_action_verb = kw::ACTION_SIGNALS.iter().any(|kw| text.contains(kw));
    let has_meal_time = kw::MEAL_TIME_SIGNALS.iter().any(|kw| text.contains(kw));
    let has_recipe_imperative = kw::RECIPE_IMPERATIVE.iter().any(|kw| text.contains(kw));
    let has_need = kw::NEED_SIGNALS.iter().any(|kw| text.contains(kw));

    // ── RULE 1: Explicit cook command always wins ────────────────────────
    // "приготовь томатный суп" / "сделай борщ для похудения" / "рецепт салата на сушку"
    // The user explicitly asked to cook → RecipeHelp must win regardless of goal keywords.
    if has_recipe_imperative {
        for (intent, score) in scores.iter_mut() {
            if *intent == Intent::RecipeHelp {
                *score += 6;
            }
            // Penalize HealthyProduct so goal keywords ("похуд", "на массу") don't steal the intent
            if *intent == Intent::HealthyProduct && *score > 0 {
                *score = (*score - 4).max(0);
            }
        }
    }

    // ── RULE 2: Goal + action/meal-time → MealIdea (no specific dish) ───
    // "что приготовить на ужин на массу?" / "diet lunch ideas"
    // Only if no recipe imperative already grabbed it.
    if has_goal && (has_action_verb || has_meal_time) && !has_recipe_imperative {
        for (intent, score) in scores.iter_mut() {
            if *intent == Intent::MealIdea {
                *score += 5;
            }
            // Penalize HealthyProduct to avoid it stealing the intent
            if *intent == Intent::HealthyProduct && *score > 0 {
                *score = (*score - 3).max(0);
            }
        }
    }

    // ── RULE 3: Goal + need/constraint → MealIdea (complex dietary request) ──
    // "chcę schudnąć, ale potrzebuję dużo białka" → user wants a meal suggestion
    // "хочу похудеть, но нужно много белка" → same pattern
    // The "need" signal indicates the user has requirements → they want a solution (meal),
    // not a product list. Strong boost needed because goal keywords dominate HealthyProduct.
    if has_goal && has_need && !has_recipe_imperative {
        for (intent, score) in scores.iter_mut() {
            if *intent == Intent::MealIdea {
                *score += 8;
            }
            if *intent == Intent::HealthyProduct && *score > 0 {
                *score = (*score / 2).max(0); // halve, not just -3
            }
        }
    }

    // Penalize NutritionInfo when meal time + goal (user wants meal, not facts)
    if has_meal_time && has_goal {
        for (intent, score) in scores.iter_mut() {
            if *intent == Intent::NutritionInfo && *score > 0 {
                *score = (*score - 2).max(0);
            }
        }
    }
}

/// Detect language from input text.
pub fn detect_language(input: &str) -> ChatLang {
    let text = input.to_lowercase();

    // Polish-specific characters
    if text.contains('ą') || text.contains('ę') || text.contains('ś')
        || text.contains('ć') || text.contains('ź') || text.contains('ż')
        || text.contains('ł') || text.contains('ń') || text.contains('ó')
    {
        return ChatLang::Pl;
    }

    // Ukrainian-specific characters (ї, і, є, ґ)
    // Note: 'і' is U+0456 (Cyrillic і), distinct from Latin 'i' (U+0069)
    if text.contains('ї') || text.contains('ґ') || text.contains('є')
        || text.contains('\u{0456}') // Ukrainian 'і'
    {
        return ChatLang::Uk;
    }

    // Cyrillic = Russian (default cyrillic)
    if text.chars().any(|c| matches!(c, 'а'..='я' | 'А'..='Я')) {
        return ChatLang::Ru;
    }

    // Default: English
    ChatLang::En
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Greeting ──
    #[test] fn greeting_ru()     { assert_eq!(detect_intent("Привет!"),                Intent::Greeting); }
    #[test] fn greeting_en()     { assert_eq!(detect_intent("Hello there"),            Intent::Greeting); }
    #[test] fn greeting_hi()     { assert_eq!(detect_intent("hi"),                     Intent::Greeting); }
    #[test] fn greeting_pl()     { assert_eq!(detect_intent("Cześć!"),                 Intent::Greeting); }

    // ── Healthy Product ──
    #[test] fn healthy_ru()      { assert_eq!(detect_intent("какой продукт полезный"), Intent::HealthyProduct); }
    #[test] fn healthy_ru2()     { assert_eq!(detect_intent("что полезного поесть"),   Intent::HealthyProduct); }
    #[test] fn healthy_en()      { assert_eq!(detect_intent("healthy food please"),    Intent::HealthyProduct); }
    #[test] fn healthy_super()   { assert_eq!(detect_intent("superfood рекомендации"),Intent::HealthyProduct); }

    // ── Category fallback (no explicit "healthy" keyword) ──
    #[test] fn fallback_meat_best()   { assert_eq!(detect_intent("какое мясо лучше"),     Intent::HealthyProduct); }
    #[test] fn fallback_fruit_plain() { assert_eq!(detect_intent("посоветуй фрукты"),     Intent::HealthyProduct); }
    #[test] fn fallback_fish()        { assert_eq!(detect_intent("топ рыбы"),             Intent::HealthyProduct); }
    #[test] fn fallback_en_veg()      { assert_eq!(detect_intent("best vegetables"),      Intent::HealthyProduct); }
    #[test] fn fallback_pl_meat()     { assert_eq!(detect_intent("jakie mięso najlepsze"),Intent::HealthyProduct); }
    // Guard: generic query without category should stay Unknown (no false positive).
    #[test] fn no_fallback_empty()    { assert_eq!(detect_intent("asdf qwerty"),          Intent::Unknown); }

    // ── Conversion ──
    #[test] fn conv_ru()         { assert_eq!(detect_intent("200 грамм в ложках"),     Intent::Conversion); }
    #[test] fn conv_en()         { assert_eq!(detect_intent("convert 100g to ounces"),Intent::Conversion); }
    #[test] fn conv_how()        { assert_eq!(detect_intent("сколько ложек в стакане"),Intent::Conversion); }

    // ── Nutrition ──
    #[test] fn nutr_ru()         { assert_eq!(detect_intent("сколько калорий в яблоке"),Intent::NutritionInfo); }
    #[test] fn nutr_en()         { assert_eq!(detect_intent("protein in chicken"),     Intent::NutritionInfo); }
    #[test] fn nutr_bju()        { assert_eq!(detect_intent("бжу шпината"),            Intent::NutritionInfo); }

    // ── Product Info ──
    #[test] fn prod_info_ru()    { assert_eq!(detect_intent("что такое лосось"),        Intent::ProductInfo); }
    #[test] fn prod_info_en()    { assert_eq!(detect_intent("what is spinach"),         Intent::ProductInfo); }
    #[test] fn prod_info_pl()    { assert_eq!(detect_intent("co to jest brokuł"),      Intent::ProductInfo); }
    #[test] fn prod_info_tell()  { assert_eq!(detect_intent("расскажи о шпинате"),     Intent::ProductInfo); }

    // ── Recipe ──
    #[test] fn recipe_ru()       { assert_eq!(detect_intent("рецепт борща"),            Intent::RecipeHelp); }
    #[test] fn recipe_en()       { assert_eq!(detect_intent("how to cook pasta"),       Intent::RecipeHelp); }

    // ── Imperative recipe triggers ──
    #[test] fn recipe_imperative_ru()  { assert_eq!(detect_intent("приготовь борщ с говядиной"), Intent::RecipeHelp); }
    #[test] fn recipe_imperative_ru2() { assert_eq!(detect_intent("сделай салат из помидоров"),  Intent::RecipeHelp); }
    #[test] fn recipe_imperative_ru3() { assert_eq!(detect_intent("свари суп из курицы"),        Intent::RecipeHelp); }
    #[test] fn recipe_imperative_ru4() { assert_eq!(detect_intent("пожарь стейк из говядины"),   Intent::RecipeHelp); }
    #[test] fn recipe_imperative_ru5() { assert_eq!(detect_intent("потуши картошку с курицей"),  Intent::RecipeHelp); }
    #[test] fn recipe_imperative_ru6() { assert_eq!(detect_intent("запеки рыбу в духовке"),      Intent::RecipeHelp); }
    #[test] fn recipe_imperative_en()  { assert_eq!(detect_intent("cook pasta carbonara"),       Intent::RecipeHelp); }
    #[test] fn recipe_imperative_en2() { assert_eq!(detect_intent("make chicken stir fry"),      Intent::RecipeHelp); }
    #[test] fn recipe_imperative_pl()  { assert_eq!(detect_intent("ugotuj barszcz"),             Intent::RecipeHelp); }
    #[test] fn recipe_imperative_uk()  { assert_eq!(detect_intent("приготуй борщ з яловичиною"), Intent::RecipeHelp); }

    // ── Goal-based recipe queries (from suggestion buttons) ──
    #[test] fn recipe_goal_light_ru()  { assert_eq!(detect_intent("приготовь лёгкое блюдо с треска"), Intent::RecipeHelp); }
    #[test] fn recipe_goal_diet_ru()   { assert_eq!(detect_intent("диетический рецепт борща"),       Intent::RecipeHelp); }
    #[test] fn recipe_goal_light_en()  { assert_eq!(detect_intent("cook a light dish with cod"),     Intent::RecipeHelp); }
    #[test] fn recipe_goal_light_uk()  { assert_eq!(detect_intent("приготуй легку страву з тріски"), Intent::RecipeHelp); }

    // ── New modifier-aware recipe tests ──
    #[test] fn recipe_quick_ru()       { assert_eq!(detect_intent("быстрый рецепт салата"),          Intent::RecipeHelp); }
    #[test] fn recipe_keto_en()        { assert_eq!(detect_intent("keto recipe for chicken"),       Intent::RecipeHelp); }
    #[test] fn recipe_comfort_ru()     { assert_eq!(detect_intent("приготовь сытное блюдо"),         Intent::RecipeHelp); }

    // ── Meal Idea ──
    #[test] fn meal_ru()         { assert_eq!(detect_intent("что приготовить на ужин"), Intent::MealIdea); }
    #[test] fn meal_en()         { assert_eq!(detect_intent("dinner idea"),             Intent::MealIdea); }

    // ── Seasonality ──
    #[test] fn season_ru()       { assert_eq!(detect_intent("когда сезон лосося"),      Intent::Seasonality); }
    #[test] fn season_en()       { assert_eq!(detect_intent("is salmon in season"),     Intent::Seasonality); }

    // ── Goal phrases → HealthyProduct ──
    #[test] fn goal_lose_weight_ru()  { assert_eq!(detect_intent("хочу похудеть"),         Intent::HealthyProduct); }
    #[test] fn goal_lose_weight_pl()  { assert_eq!(detect_intent("chcę schudnąć"),         Intent::HealthyProduct); }
    #[test] fn goal_lose_weight_en()  { assert_eq!(detect_intent("I want to lose weight"), Intent::HealthyProduct); }
    #[test] fn goal_muscle_ru()       { assert_eq!(detect_intent("хочу набрать мышечную массу"), Intent::HealthyProduct); }
    #[test] fn goal_na_massu_ru()     { assert_eq!(detect_intent("что есть на массу"),     Intent::HealthyProduct); }
    #[test] fn goal_chto_est_ru()     { assert_eq!(detect_intent("что поесть"),             Intent::HealthyProduct); }
    #[test] fn goal_chto_est_pl()     { assert_eq!(detect_intent("co zjeść"),               Intent::HealthyProduct); }
    #[test] fn goal_sushka_ru()       { assert_eq!(detect_intent("питание на сушку"),       Intent::HealthyProduct); }

    // ── New goal → HealthyProduct tests ──
    #[test] fn goal_keto_ru()         { assert_eq!(detect_intent("кето продукты"),          Intent::HealthyProduct); }
    #[test] fn goal_fiber_ru()        { assert_eq!(detect_intent("продукты с клетчаткой"),  Intent::HealthyProduct); }
    #[test] fn goal_comfort_ru()      { assert_eq!(detect_intent("что-нибудь сытное"),       Intent::HealthyProduct); }

    #[test] fn unknown_garbage() { assert_eq!(detect_intent("asdfghjkl"),               Intent::Unknown); }

    // ── Explicit cook verb + goal → RecipeHelp (NOT HealthyProduct) ──
    #[test]
    fn recipe_with_goal_cook_tomato_soup() {
        assert_eq!(detect_intent("хочу похудеть, приготовь томатный суп"), Intent::RecipeHelp);
    }
    #[test]
    fn recipe_with_goal_make_salad() {
        assert_eq!(detect_intent("сделай салат для похудения"), Intent::RecipeHelp);
    }
    #[test]
    fn recipe_with_goal_cook_borscht_muscle() {
        assert_eq!(detect_intent("рецепт борща на массу"), Intent::RecipeHelp);
    }
    #[test]
    fn recipe_with_goal_bake_fish_diet() {
        assert_eq!(detect_intent("запеки рыбу диетическую"), Intent::RecipeHelp);
    }
    #[test]
    fn recipe_with_goal_cook_keto_en() {
        assert_eq!(detect_intent("cook a keto chicken dinner"), Intent::RecipeHelp);
    }
    #[test]
    fn recipe_with_goal_make_light_soup_pl() {
        assert_eq!(detect_intent("ugotuj lekką zupę pomidorową"), Intent::RecipeHelp);
    }

    // ── Score-based wins ──
    #[test] fn ambiguous_healthy_wins() {
        let r = detect_intent_scored("полезный белок в шпинате");
        assert_eq!(r.intent, Intent::HealthyProduct);
        assert!(r.score >= 3);
    }

    // ── Language detection ──
    #[test] fn lang_ru()  { assert_eq!(detect_language("привет мир"),       ChatLang::Ru); }
    #[test] fn lang_en()  { assert_eq!(detect_language("hello world"),      ChatLang::En); }
    #[test] fn lang_pl()  { assert_eq!(detect_language("cześć świat"),      ChatLang::Pl); }
    #[test] fn lang_uk()  { assert_eq!(detect_language("привіт світ їжа"), ChatLang::Uk); }

    #[test] fn score_threshold() {
        assert_eq!(detect_intent("xyz abc def"), Intent::Unknown);
    }

    // ── Context boost tests ──
    #[test] fn goal_action_meal_ru() {
        assert_eq!(detect_intent("хочу на массу, что приготовить?"), Intent::MealIdea);
    }
    #[test] fn goal_action_meal_en() {
        assert_eq!(detect_intent("low calorie dinner for diet"), Intent::MealIdea);
    }
    #[test] fn goal_action_eat_ru() {
        assert_eq!(detect_intent("хочу похудеть, что поесть на ужин?"), Intent::MealIdea);
    }
    #[test] fn goal_action_muscle_en() {
        assert_eq!(detect_intent("high protein meal for dinner"), Intent::MealIdea);
    }
    #[test] fn goal_action_diet_lunch() {
        assert_eq!(detect_intent("diet lunch ideas"), Intent::MealIdea);
    }

    // ═══ Goal + need → MealIdea (not HealthyProduct) ═════════════════════
    #[test] fn goal_need_pl() {
        assert_eq!(detect_intent("chcę schudnąć, ale potrzebuję dużo białka"), Intent::MealIdea);
    }
    #[test] fn goal_need_ru() {
        assert_eq!(detect_intent("хочу похудеть, но нужно много белка"), Intent::MealIdea);
    }
    #[test] fn goal_need_en() {
        assert_eq!(detect_intent("want to lose weight but need high protein"), Intent::MealIdea);
    }
    #[test] fn goal_need_uk() {
        assert_eq!(detect_intent("хочу схуднути, але потрібно багато білка"), Intent::MealIdea);
    }

    // ═══ Context-aware routing tests ═════════════════════════════════════

    fn ctx_with(intent: Intent) -> DialogContext {
        DialogContext { last_intent: Some(intent), last_modifier: None, turn_count: 1 }
    }

    fn ctx_with_mod(intent: Intent, modifier: HealthModifier) -> DialogContext {
        DialogContext { last_intent: Some(intent), last_modifier: Some(modifier), turn_count: 1 }
    }

    fn empty_ctx() -> DialogContext {
        DialogContext { last_intent: None, last_modifier: None, turn_count: 0 }
    }

    // ── Follow-up "ещё" ──
    #[test]
    fn ctx_followup_more_healthy() {
        let ctx = ctx_with(Intent::HealthyProduct);
        assert_eq!(parse_input_with_context("ещё", &ctx).intent, Intent::HealthyProduct);
    }

    #[test]
    fn ctx_followup_more_recipe() {
        let ctx = ctx_with(Intent::RecipeHelp);
        assert_eq!(parse_input_with_context("more", &ctx).intent, Intent::RecipeHelp);
    }

    #[test]
    fn ctx_followup_drugoe_meal() {
        let ctx = ctx_with(Intent::MealIdea);
        assert_eq!(parse_input_with_context("другое", &ctx).intent, Intent::MealIdea);
    }

    #[test]
    fn ctx_followup_no_context_stays_unknown() {
        let ctx = empty_ctx();
        // Without context, "ещё" alone should NOT resolve to anything meaningful
        let r = parse_input_with_context("ещё", &ctx);
        // No last_intent → falls through to scoring, likely Unknown
        assert_ne!(r.intent, Intent::RecipeHelp);
    }

    // ── Confirmation ──
    #[test]
    fn ctx_confirm_after_recipe() {
        let ctx = ctx_with(Intent::RecipeHelp);
        assert_eq!(parse_input_with_context("давай", &ctx).intent, Intent::RecipeHelp);
    }

    #[test]
    fn ctx_confirm_yes_after_healthy() {
        let ctx = ctx_with(Intent::HealthyProduct);
        assert_eq!(parse_input_with_context("да", &ctx).intent, Intent::HealthyProduct);
    }

    #[test]
    fn ctx_confirm_ok_after_meal() {
        let ctx = ctx_with(Intent::MealIdea);
        assert_eq!(parse_input_with_context("ok", &ctx).intent, Intent::MealIdea);
    }

    // ── Short recipe ref ──
    #[test]
    fn ctx_short_recipe_ref_borsch() {
        let ctx = ctx_with(Intent::RecipeHelp);
        assert_eq!(parse_input_with_context("а борщ?", &ctx).intent, Intent::RecipeHelp);
    }

    #[test]
    fn ctx_short_recipe_ref_with_chicken() {
        let ctx = ctx_with(Intent::RecipeHelp);
        assert_eq!(parse_input_with_context("а с курицей?", &ctx).intent, Intent::RecipeHelp);
    }

    #[test]
    fn ctx_short_recipe_ref_what_about() {
        let ctx = ctx_with(Intent::RecipeHelp);
        assert_eq!(parse_input_with_context("what about steak?", &ctx).intent, Intent::RecipeHelp);
    }

    // ── Short ambiguous inherits context ──
    #[test]
    fn ctx_short_inherits_recipe() {
        let ctx = ctx_with(Intent::RecipeHelp);
        // "борщ" alone is ambiguous (1 word, no strong signal)
        let r = parse_input_with_context("борщ", &ctx);
        assert_eq!(r.intent, Intent::RecipeHelp);
    }

    #[test]
    fn ctx_short_inherits_healthy() {
        let ctx = ctx_with(Intent::HealthyProduct);
        let r = parse_input_with_context("лосось", &ctx);
        assert_eq!(r.intent, Intent::HealthyProduct);
    }

    // ── Modifier boost ──
    #[test]
    fn ctx_modifier_boost_recipe() {
        let ctx = ctx_with_mod(Intent::RecipeHelp, HealthModifier::LowCalorie);
        // "треска" alone + LowCalorie context → should lean toward RecipeHelp
        let r = parse_input_with_context("треска", &ctx);
        assert_eq!(r.intent, Intent::RecipeHelp);
    }

    // ── No context = normal scoring ──
    #[test]
    fn ctx_no_context_normal_scoring() {
        let ctx = empty_ctx();
        assert_eq!(parse_input_with_context("привет", &ctx).intent, Intent::Greeting);
    }

    #[test]
    fn ctx_no_context_recipe() {
        let ctx = empty_ctx();
        assert_eq!(parse_input_with_context("приготовь борщ", &ctx).intent, Intent::RecipeHelp);
    }

    #[test]
    fn ctx_no_context_healthy() {
        let ctx = empty_ctx();
        assert_eq!(parse_input_with_context("хочу похудеть", &ctx).intent, Intent::HealthyProduct);
    }
}

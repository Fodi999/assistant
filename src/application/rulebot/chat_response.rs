//! Chat Response — unified response format for ChefOS Chat Interface.
//!
//! Every chat response contains:
//! - `text`        — human-readable message (always present)
//! - `cards`       — zero or more rich cards (product, conversion, recipe, etc.)
//! - `intents`     — ALL detected intents (multi-intent: ["healthy_product","quick"])
//! - `reason`      — WHY this result (explainability: "high protein — 31.0g/100g")
//! - `lang`        — detected language
//! - `timing_ms`   — processing time

use serde::Serialize;
use super::intent_router::{ChatLang, Intent};

// ── Chat Response ────────────────────────────────────────────────────────────

/// Unified response from `POST /public/chat`.
#[derive(Debug, Serialize)]
pub struct ChatResponse {
    /// Human-readable answer text.
    pub text: String,
    /// Rich cards — zero to many (product, conversion, nutrition, etc.).
    /// Empty array = text-only response.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub cards: Vec<Card>,
    /// Primary detected intent (backward compat).
    pub intent: Intent,
    /// All detected intents — enables multi-intent responses.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub intents: Vec<Intent>,
    /// Explainability reason — WHY this product/result was chosen.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Next-step action suggestions — "Zrób plan dnia", "Pokaż przepisy"
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub suggestions: Vec<Suggestion>,
    /// Complementary recommendation block (Step 3.5 "Guidance"):
    /// e.g. after serving Fish cards → a side block of Vegetable cards.
    /// Rendered as a SEPARATE section below main cards — never replaces them.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion_block: Option<SuggestionBlock>,
    /// Chef tip — cooking insight from the "chef mode".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chef_tip: Option<String>,
    /// Motivational message from the sous-chef coach.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub coach_message: Option<String>,
    /// Detected language.
    pub lang: ChatLang,
    /// Processing time in milliseconds.
    pub timing_ms: u64,
}

/// A follow-up action the user can tap (rendered as a button).
#[derive(Debug, Serialize, Clone)]
pub struct Suggestion {
    /// Button label shown to user.
    pub label: String,
    /// The exact query to send when tapped.
    pub query: String,
    /// Optional emoji icon.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emoji: Option<&'static str>,
}

/// Complementary recommendation block — Step 3.5 "Guidance" layer.
///
/// Appears BELOW main cards as a separate section. Example: user asked
/// for fish, we show fish cards, then attach a SuggestionBlock with
/// `title = "Add a side"` and 2 vegetable `ProductCard`s.
///
/// Never replaces main `cards[]` — always an addition.
#[derive(Debug, Serialize)]
pub struct SuggestionBlock {
    /// Human-readable heading: "Add a side", "Добавь гарнир", etc.
    pub title: String,
    /// The suggested category slug ("vegetable", "fruit", …) —
    /// clients can use this for icons / deep links.
    pub category: String,
    /// Complementary cards. Same shape as `ChatResponse.cards`.
    pub items: Vec<Card>,
}

impl ChatResponse {
    /// Quick text-only response (no cards).
    pub fn text_only(text: impl Into<String>, intent: Intent, lang: ChatLang, timing_ms: u64) -> Self {
        Self {
            text: text.into(),
            cards: vec![],
            intent,
            intents: vec![],
            reason: None,
            suggestions: vec![],
            suggestion_block: None,
            chef_tip: None,
            coach_message: None,
            lang,
            timing_ms,
        }
    }

    /// Response with a single card.
    pub fn with_card(
        text: impl Into<String>,
        card: Card,
        intent: Intent,
        lang: ChatLang,
        timing_ms: u64,
    ) -> Self {
        Self {
            text: text.into(),
            cards: vec![card],
            intent,
            intents: vec![],
            reason: None,
            suggestions: vec![],
            suggestion_block: None,
            chef_tip: None,
            coach_message: None,
            lang,
            timing_ms,
        }
    }

    /// Response with multiple cards + explainability reason + multi-intents.
    pub fn with_cards(
        text: impl Into<String>,
        cards: Vec<Card>,
        intent: Intent,
        intents: Vec<Intent>,
        reason: impl Into<String>,
        lang: ChatLang,
        timing_ms: u64,
    ) -> Self {
        Self {
            text: text.into(),
            cards,
            intent,
            intents,
            reason: Some(reason.into()),
            suggestions: vec![],
            suggestion_block: None,
            chef_tip: None,
            coach_message: None,
            lang,
            timing_ms,
        }
    }

    /// Response with a single card + explainability reason + multi-intents.
    /// Convenience wrapper around `with_cards`.
    pub fn with_full(
        text: impl Into<String>,
        card: Card,
        intent: Intent,
        intents: Vec<Intent>,
        reason: impl Into<String>,
        lang: ChatLang,
        timing_ms: u64,
    ) -> Self {
        Self::with_cards(text, vec![card], intent, intents, reason, lang, timing_ms)
    }
}

// ── Actions ──────────────────────────────────────────────────────────────────

/// User-invokable action attached to a card — rendered as a button on the frontend.
///
/// Centralizing actions here (vs generated on iOS) gives us:
///   - single source of truth for what the user can do with a card
///   - ability to A/B test button sets
///   - session-aware actions (e.g. don't offer "add to plan" if already added)
#[derive(Debug, Serialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Action {
    /// Add a recipe to today's meal plan.
    AddToPlan {
        recipe_id: String,
    },
    /// Start guided cooking flow for this recipe.
    StartCooking {
        recipe_id: String,
    },
    /// Suggest an alternative for a single ingredient in a recipe.
    SwapIngredient {
        recipe_id: String,
        ingredient_slug: String,
    },
    /// Add product to the shopping list.
    AddToShopping {
        product_slug: String,
    },
    /// Trigger a new chat query asking for recipes that use this product.
    ShowRecipesFor {
        product_slug: String,
    },
}

// ── Card Types ───────────────────────────────────────────────────────────────

/// Rich card attached to a chat response.
#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Card {
    /// Product/ingredient info card.
    Product(ProductCard),
    /// Unit conversion result card.
    Conversion(ConversionCard),
    /// Nutrition breakdown card.
    Nutrition(NutritionCard),
    /// Tech-card / recipe card — full dish breakdown.
    Recipe(RecipeCard),
}

/// Product card — name, nutrition, image.
#[derive(Debug, Serialize)]
pub struct ProductCard {
    pub slug: String,
    pub name: String,
    pub calories_per_100g: f32,
    pub protein_per_100g: f32,
    pub fat_per_100g: f32,
    pub carbs_per_100g: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_url: Option<String>,
    /// Short human label for why this product was highlighted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub highlight: Option<String>,
    /// Machine-readable reason tag: "high_protein" | "low_calorie" | "balanced"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason_tag: Option<&'static str>,
    /// User-invokable actions — populated centrally in `chat_engine::enrich_with_actions`.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub actions: Vec<Action>,
}

/// Conversion result card.
#[derive(Debug, Serialize)]
pub struct ConversionCard {
    pub value: f64,
    pub from: String,
    pub to: String,
    pub result: f64,
    pub supported: bool,
}

/// Nutrition breakdown card.
#[derive(Debug, Serialize)]
pub struct NutritionCard {
    pub name: String,
    pub calories_per_100g: f32,
    pub protein_per_100g: f32,
    pub fat_per_100g: f32,
    pub carbs_per_100g: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_url: Option<String>,
}

/// Recipe/tech-card — full dish breakdown with gross/net calculations.
#[derive(Debug, Serialize)]
pub struct RecipeCard {
    /// Stable identifier — slugified `dish_name` (canonical English).
    /// MUST be used for state tracking (added_recipes), NOT display_name
    /// or dish_name_local because those vary by language and AI rephrasing.
    pub slug: String,
    pub dish_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dish_name_local: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dish_type: Option<String>,
    pub servings: u8,
    pub ingredients: Vec<RecipeIngredientRow>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub steps: Vec<RecipeStepRow>,
    pub total_output_g: f32,
    pub total_gross_g: f32,
    pub total_loss_g: f32,
    pub loss_percent: f32,
    pub kcal_per_100g: f32,
    pub total_kcal: u32,
    pub total_protein: f32,
    pub total_fat: f32,
    pub total_carbs: f32,
    pub per_serving_kcal: u32,
    pub per_serving_protein: f32,
    pub per_serving_fat: f32,
    pub per_serving_carbs: f32,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub unresolved: Vec<String>,
    /// Ingredients removed by food pairing (slug, reason)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub removed_ingredients: Vec<RemovedIngredient>,
    // ── Dish context (v2) ──
    /// "easy" | "medium" | "hard"
    pub complexity: String,
    /// "balanced" | "high_protein" | "low_calorie"
    pub goal: String,
    /// Allergens/intolerances present: ["gluten", "lactose", "nuts", "eggs", "fish", "shellfish", "soy"]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub allergens: Vec<String>,
    /// Diet tags: ["vegan", "vegetarian", "pescatarian"]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,

    // ── Goal Engine v2 fields ──
    /// Dietary constraints applied, e.g. ["lactose-free", "vegan diet"]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub applied_constraints: Vec<String>,
    /// Adaptation actions taken by the engine (added / reduced / substituted)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub adaptations: Vec<AdaptationActionRow>,
    /// Post-build validation warnings
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub validation_warnings: Vec<String>,
    /// Auto-fix actions taken, e.g. ["Added 2 eggs as protein source"]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub auto_fixes: Vec<String>,
    /// User-invokable actions — populated centrally in `chat_engine::enrich_with_actions`.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub actions: Vec<Action>,
}

/// A single adaptation action surfaced to the frontend.
#[derive(Debug, Serialize)]
pub struct AdaptationActionRow {
    /// "added" | "removed" | "increased" | "reduced" | "substituted"
    pub action: String,
    /// The ingredient slug affected
    pub slug: String,
    /// Human-readable detail
    pub detail: String,
}

/// A cooking step in a recipe.
#[derive(Debug, Serialize)]
pub struct RecipeStepRow {
    pub step: u8,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_min: Option<u16>,
    /// Cooking temperature in °C
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temp_c: Option<u16>,
    /// Localized chef tip for this step
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tip: Option<String>,
}

/// Single ingredient row in a recipe card.
#[derive(Debug, Serialize)]
pub struct RecipeIngredientRow {
    pub name: String,
    pub slug: Option<String>,
    pub state: String,
    pub role: String,
    pub gross_g: f32,
    pub net_g: f32,
    pub kcal: u32,
    pub protein_g: f32,
    pub fat_g: f32,
    pub carbs_g: f32,
}

/// An ingredient removed by the food pairing filter.
#[derive(Debug, Serialize)]
pub struct RemovedIngredient {
    pub slug: String,
    pub reason: String,
}

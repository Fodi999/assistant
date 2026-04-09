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

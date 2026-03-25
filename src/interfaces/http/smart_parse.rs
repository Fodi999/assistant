//! HTTP handlers for SmartParse endpoints.
//!
//! - `POST /api/smart/parse`      — text → ingredient list
//! - `POST /api/smart/from-text`  — text → full SmartService analysis (one-click)

use axum::{extract::State, Json};
use serde::Deserialize;
use std::sync::Arc;

use crate::application::smart_parse::{SmartParseResponse, SmartParseService};
use crate::application::smart_service::{CulinaryContext, SmartResponse, SmartService};
use crate::shared::{AppError, Language};

// ── POST /api/smart/parse ────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct SmartParseRequest {
    /// Free-form ingredient text.
    pub text: String,
    /// Language code: en, ru, uk, pl.
    #[serde(default = "default_lang")]
    pub lang: String,
}

fn default_lang() -> String {
    "en".to_string()
}

/// POST /api/smart/parse
pub async fn smart_parse(
    State(service): State<SmartParseService>,
    Json(req): Json<SmartParseRequest>,
) -> Result<Json<SmartParseResponse>, AppError> {
    let text = req.text.trim();
    if text.is_empty() {
        return Err(AppError::validation("text is required"));
    }
    if text.len() > 500 {
        return Err(AppError::validation("text must be ≤ 500 characters"));
    }

    let lang = Language::from_code(&req.lang).unwrap_or(Language::En);
    let response = service.parse(text, lang).await?;
    Ok(Json(response))
}

// ── POST /api/smart/from-text ────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct FromTextRequest {
    /// Free-form ingredient text.
    pub text: String,
    /// Optional goal: balanced, high_protein, low_calorie, keto, etc.
    #[serde(default)]
    pub goal: Option<String>,
    /// Meal occasion: breakfast, lunch, dinner, snack, dessert.
    #[serde(default)]
    pub meal_type: Option<String>,
    /// Dietary restriction: vegan, vegetarian, gluten_free, etc.
    #[serde(default)]
    pub diet: Option<String>,
    /// Language code: en, ru, uk, pl.
    #[serde(default = "default_lang")]
    pub lang: String,
    /// Optional cooking state for the primary ingredient.
    #[serde(default)]
    pub state: Option<String>,
    /// Optional session_id for continuity.
    #[serde(default)]
    pub session_id: Option<String>,
}

/// Shared state for /api/smart/from-text (needs both services).
#[derive(Clone)]
pub struct FromTextState {
    pub parse_service: SmartParseService,
    pub smart_service: Arc<SmartService>,
}

/// POST /api/smart/from-text
///
/// 1. Parse text → ingredient slugs
/// 2. First slug → main ingredient; rest → additional_ingredients
/// 3. Call SmartService → full analysis
///
/// One request, full analysis. Zero AI.
pub async fn smart_from_text(
    State(state): State<FromTextState>,
    Json(req): Json<FromTextRequest>,
) -> Result<Json<SmartResponse>, AppError> {
    let text = req.text.trim();
    if text.is_empty() {
        return Err(AppError::validation("text is required"));
    }
    if text.len() > 500 {
        return Err(AppError::validation("text must be ≤ 500 characters"));
    }

    let lang = Language::from_code(&req.lang).unwrap_or(Language::En);

    // Step 1: parse text → ingredient slugs
    let parsed = state.parse_service.parse(text, lang).await?;

    if parsed.found.is_empty() {
        return Err(AppError::validation(
            "no ingredients found in the provided text",
        ));
    }

    // Step 2: first = main, rest = additional
    let main_slug = parsed.found[0].slug.clone();
    let additional: Vec<String> = parsed.found[1..].iter().map(|i| i.slug.clone()).collect();

    // Step 3: build CulinaryContext and call SmartService
    let ctx = CulinaryContext {
        ingredient: main_slug,
        state: req.state,
        additional_ingredients: additional,
        goal: req.goal,
        meal_type: req.meal_type,
        diet: req.diet,
        lang: req.lang,
        session_id: req.session_id,
    };

    let response = state.smart_service.get_smart_ingredient(ctx).await?;
    Ok(Json(response))
}

//! Public HTTP handler for Sous-Chef Planner.
//!
//! POST /public/sous-chef/plan
//! { "query": "Хочу похудеть — меню на 1 день", "lang": "ru" }
//!
//! Also serves pre-defined suggestion chips:
//! GET /public/sous-chef/suggestions?lang=ru

use axum::{
    extract::{Extension, Query, State},
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::application::sous_chef::{MealPlan, PlanRequest, SousChefPlannerService};
use crate::infrastructure::cache::{self, AppCache};

// ── POST /public/sous-chef/plan ───────────────────────────────────────────────

pub async fn generate_plan(
    State(service): State<Arc<SousChefPlannerService>>,
    cache: Option<Extension<AppCache>>,
    Json(req): Json<PlanRequest>,
) -> Result<Json<MealPlan>, (StatusCode, Json<serde_json::Value>)> {
    // Quick validation
    let query = req.query.trim().to_string();
    if query.is_empty() || query.len() < 3 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Query too short (min 3 characters)" })),
        ));
    }
    if query.len() > 500 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Query too long (max 500 characters)" })),
        ));
    }

    // Check in-memory cache first (5 min)
    let lang = req.lang.as_deref().unwrap_or("en");
    let mem_key = format!("sous_chef:plan:{}:{}", 
        crate::application::sous_chef::normalize_for_cache(&query), lang);

    if let Some(Extension(ref c)) = cache {
        if let Some(cached) = c.get(&mem_key) {
            if let Ok(mut plan) = serde_json::from_value::<MealPlan>(cached) {
                plan.cached = true;
                tracing::debug!("⚡ In-memory cache HIT: {}", mem_key);
                return Ok(Json(plan));
            }
        }
    }

    let plan_req = PlanRequest { query, lang: Some(lang.to_string()) };

    match service.generate_plan(plan_req).await {
        Ok(plan) => {
            // Store in in-memory cache
            if let Some(Extension(c)) = cache {
                if let Ok(val) = serde_json::to_value(&plan) {
                    c.set(mem_key, val);
                }
            }
            Ok(Json(plan))
        }
        Err(e) => {
            tracing::error!("❌ Sous-chef plan error: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "Failed to generate meal plan" })),
            ))
        }
    }
}

// ── GET /public/sous-chef/suggestions?lang=ru ─────────────────────────────────

#[derive(Deserialize)]
pub struct SuggestionsQuery {
    pub lang: Option<String>,
}

#[derive(Serialize)]
pub struct Suggestion {
    pub text: String,
    pub goal: String,
}

pub async fn get_suggestions(
    Query(params): Query<SuggestionsQuery>,
) -> Json<Vec<Suggestion>> {
    let lang = params.lang.as_deref().unwrap_or("en");

    let suggestions = match lang {
        "ru" => vec![
            Suggestion { text: "Хочу похудеть — меню на 1 день".into(), goal: "weight_loss".into() },
            Suggestion { text: "Быстрый завтрак из простых продуктов".into(), goal: "quick_breakfast".into() },
            Suggestion { text: "Меню на день с высоким белком".into(), goal: "high_protein".into() },
            Suggestion { text: "Что приготовить из: курица, рис, овощи".into(), goal: "from_ingredients".into() },
            Suggestion { text: "Здоровое меню на весь день".into(), goal: "healthy_day".into() },
        ],
        "pl" => vec![
            Suggestion { text: "Chcę schudnąć — menu na 1 dzień".into(), goal: "weight_loss".into() },
            Suggestion { text: "Szybkie śniadanie z prostych produktów".into(), goal: "quick_breakfast".into() },
            Suggestion { text: "Menu na dzień z dużą ilością białka".into(), goal: "high_protein".into() },
            Suggestion { text: "Co ugotować z: kurczak, ryż, warzywa".into(), goal: "from_ingredients".into() },
            Suggestion { text: "Zdrowe menu na cały dzień".into(), goal: "healthy_day".into() },
        ],
        "uk" => vec![
            Suggestion { text: "Хочу схуднути — меню на 1 день".into(), goal: "weight_loss".into() },
            Suggestion { text: "Швидкий сніданок з простих продуктів".into(), goal: "quick_breakfast".into() },
            Suggestion { text: "Меню на день з високим білком".into(), goal: "high_protein".into() },
            Suggestion { text: "Що приготувати з: курка, рис, овочі".into(), goal: "from_ingredients".into() },
            Suggestion { text: "Здорове меню на цілий день".into(), goal: "healthy_day".into() },
        ],
        _ => vec![
            Suggestion { text: "I want to lose weight — 1-day menu".into(), goal: "weight_loss".into() },
            Suggestion { text: "Quick breakfast from simple ingredients".into(), goal: "quick_breakfast".into() },
            Suggestion { text: "High protein day menu".into(), goal: "high_protein".into() },
            Suggestion { text: "What to cook from: chicken, rice, vegetables".into(), goal: "from_ingredients".into() },
            Suggestion { text: "Healthy menu for the whole day".into(), goal: "healthy_day".into() },
        ],
    };

    Json(suggestions)
}

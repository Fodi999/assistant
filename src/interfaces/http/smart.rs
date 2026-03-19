//! HTTP handlers for SmartService v2.
//!
//! - `POST /api/smart/ingredient`  — full analysis
//! - `GET  /api/smart/autocomplete` — fast slug/name/image search

use axum::{
    extract::{Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::sync::Arc;

use crate::application::smart_service::{CulinaryContext, SmartResponse, SmartService};
use crate::shared::{AppError, Language};

/// POST /api/smart/ingredient
///
/// Request body: CulinaryContext (JSON)
/// Response: SmartResponse (JSON)
pub async fn smart_ingredient(
    State(service): State<Arc<SmartService>>,
    Json(ctx): Json<CulinaryContext>,
) -> Result<Json<SmartResponse>, AppError> {
    // Validate input
    if ctx.ingredient.is_empty() {
        return Err(AppError::validation("ingredient slug is required"));
    }
    if ctx.ingredient.len() > 100 {
        return Err(AppError::validation("ingredient slug too long"));
    }
    if ctx.additional_ingredients.len() > 20 {
        return Err(AppError::validation("too many additional ingredients (max 20)"));
    }

    let response = service.get_smart_ingredient(ctx).await?;
    Ok(Json(response))
}

// ── Autocomplete (Step 5) ────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct AutocompleteQuery {
    /// Search query (min 1 char)
    pub q: String,
    /// Language: en, ru, pl, uk
    #[serde(default = "default_lang")]
    pub lang: String,
    /// Max results (default 10, max 30)
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn default_lang() -> String { "en".to_string() }
fn default_limit() -> i64 { 10 }

#[derive(Serialize, sqlx::FromRow)]
pub struct AutocompleteItem {
    pub slug: String,
    pub name: String,
    pub image_url: Option<String>,
}

/// GET /api/smart/autocomplete?q=sal&lang=en&limit=10
///
/// Fast lightweight endpoint for search-as-you-type.
/// Searches slug + localized name with ILIKE.
/// Target: <20ms.
pub async fn smart_autocomplete(
    State(pool): State<PgPool>,
    Query(params): Query<AutocompleteQuery>,
) -> Result<Json<Vec<AutocompleteItem>>, AppError> {
    let q = params.q.trim().to_lowercase();
    if q.is_empty() || q.len() > 100 {
        return Ok(Json(vec![]));
    }

    let limit = params.limit.min(30).max(1);
    let lang = Language::from_code(&params.lang).unwrap_or(Language::En);
    let pattern = format!("%{}%", q);

    // Pick the right name column based on language
    let name_col = match lang {
        Language::En => "name_en",
        Language::Ru => "name_ru",
        Language::Pl => "name_pl",
        Language::Uk => "name_uk",
    };

    // Search slug + localized name, prioritize exact prefix match
    let sql = format!(
        r#"SELECT slug, {name} as name, image_url
           FROM catalog_ingredients
           WHERE COALESCE(is_active, true) = true
             AND (slug ILIKE $1 OR {name} ILIKE $1)
           ORDER BY
             (slug = $2 OR LOWER({name}) = $2) DESC,
             (slug LIKE $3 OR LOWER({name}) LIKE $3) DESC,
             {name}
           LIMIT $4"#,
        name = name_col,
    );

    let items: Vec<AutocompleteItem> = sqlx::query_as(&sql)
        .bind(&pattern)          // $1: %query%
        .bind(&q)                // $2: exact match
        .bind(format!("{}%", q)) // $3: prefix match
        .bind(limit)             // $4: limit
        .fetch_all(&pool)
        .await
        .map_err(|e| {
            tracing::error!("Autocomplete DB error: {}", e);
            AppError::internal("autocomplete search failed")
        })?;

    Ok(Json(items))
}

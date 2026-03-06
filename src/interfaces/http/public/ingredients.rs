use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::catalog::{IngredientMeasures, IngredientNutrition, IngredientReference};
use crate::domain::tools::unit_converter as uc;
use crate::infrastructure::persistence::catalog_ingredient_repository::find_ingredient_ref_by_slug;
use crate::shared::i18n::{translate_allergens, translate_seasons};
use crate::shared::language::Language;

// ── Query params ──────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct ListQuery {
    pub lang: Option<String>,
    /// Full-text search: ?q=salmon
    pub q: Option<String>,
}

#[derive(Deserialize)]
pub struct LangQuery {
    pub lang: Option<String>,
}

fn parse_lang(s: &Option<String>) -> Language {
    s.as_deref()
        .and_then(Language::from_code)
        .unwrap_or_default()
}

// ── Response types ────────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct IngredientListItem {
    pub slug: String,
    pub name_en: String,
    pub name_ru: String,
    pub name_pl: String,
    pub name_uk: String,
    pub image_url: Option<String>,
    pub category_id: Option<Uuid>,
    pub calories_per_100g: Option<i32>,
    pub seasons: Vec<String>,
}

#[derive(Serialize)]
pub struct ListResponse {
    pub items: Vec<IngredientListItem>,
    pub total: i64,
}

// ── DB row structs ────────────────────────────────────────────────────────────

#[derive(sqlx::FromRow)]
struct IngredientRow {
    slug: Option<String>,
    name_en: String,
    name_ru: String,
    name_pl: String,
    name_uk: String,
    image_url: Option<String>,
    category_id: Option<Uuid>,
    calories_per_100g: Option<i32>,
    seasons: Vec<String>,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

/// GET /public/ingredients?lang=pl&q=salmon
///
/// Returns all active ingredients, optionally filtered by full-text search.
/// Supports ?q= for searching across all language name columns and slug.
pub async fn list_ingredients(
    State(pool): State<PgPool>,
    Query(params): Query<ListQuery>,
) -> Result<Json<ListResponse>, (StatusCode, Json<serde_json::Value>)> {
    let rows: Vec<IngredientRow> = if let Some(q) = params.q.as_deref().filter(|s| !s.is_empty()) {
        let pattern = format!("%{}%", q.to_lowercase());
        sqlx::query_as(
            r#"
            SELECT
                slug, name_en, name_ru, name_pl, name_uk,
                image_url, category_id, calories_per_100g,
                ARRAY(SELECT unnest(seasons::text[])) AS seasons
            FROM catalog_ingredients
            WHERE is_active = true AND slug IS NOT NULL AND slug != ''
              AND (
                LOWER(name_en) LIKE $1 OR
                LOWER(name_ru) LIKE $1 OR
                LOWER(name_pl) LIKE $1 OR
                LOWER(name_uk) LIKE $1 OR
                slug LIKE $1
              )
            ORDER BY
                CASE WHEN LOWER(name_en) = $2 OR slug = $2 THEN 0 ELSE 1 END,
                name_en ASC
            "#,
        )
        .bind(&pattern)
        .bind(&q.to_lowercase())
        .fetch_all(&pool)
        .await
    } else {
        sqlx::query_as(
            r#"
            SELECT
                slug, name_en, name_ru, name_pl, name_uk,
                image_url, category_id, calories_per_100g,
                ARRAY(SELECT unnest(seasons::text[])) AS seasons
            FROM catalog_ingredients
            WHERE is_active = true AND slug IS NOT NULL AND slug != ''
            ORDER BY name_en ASC
            "#,
        )
        .fetch_all(&pool)
        .await
    }
    .map_err(|e| {
        tracing::error!("DB error listing public ingredients: {e}");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": "Database error" })),
        )
    })?;

    let total = rows.len() as i64;
    let items = rows
        .into_iter()
        .map(|r| IngredientListItem {
            slug: r.slug.unwrap_or_default(),
            name_en: r.name_en,
            name_ru: r.name_ru,
            name_pl: r.name_pl,
            name_uk: r.name_uk,
            image_url: r.image_url,
            category_id: r.category_id,
            calories_per_100g: r.calories_per_100g,
            seasons: r.seasons,
        })
        .collect();

    Ok(Json(ListResponse { items, total }))
}

/// GET /public/ingredients/:slug?lang=pl
///
/// Returns the full structured ingredient reference:
/// - nutrition (calories, protein, fat, carbs)
/// - density + pre-computed kitchen measures (cup / tbsp / tsp in grams)
/// - description in the requested language (falls back to English)
/// - seasons and allergens translated to the requested language
///
/// Ideal for SEO ingredient pages: /pl/ingredients/salmon
pub async fn get_ingredient_by_slug(
    State(pool): State<PgPool>,
    Path(slug): Path<String>,
    Query(params): Query<LangQuery>,
) -> Result<Json<IngredientReference>, (StatusCode, Json<serde_json::Value>)> {
    let lang = parse_lang(&params.lang);

    let row = find_ingredient_ref_by_slug(&pool, &slug)
        .await
        .map_err(|e| {
            tracing::error!("DB error fetching ingredient slug={slug}: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "Database error" })),
            )
        })?;

    match row {
        None => Err((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": format!("Ingredient '{}' not found", slug)
            })),
        )),
        Some(r) => {
            let to_f64 = |d: Option<rust_decimal::Decimal>| -> Option<f64> {
                d.and_then(|v| rust_decimal::prelude::ToPrimitive::to_f64(&v))
            };

            let density = to_f64(r.density_g_per_ml);

            let measures = IngredientMeasures {
                grams_per_cup: density.map(uc::grams_per_cup),
                grams_per_tbsp: density.map(uc::grams_per_tbsp),
                grams_per_tsp: density.map(uc::grams_per_tsp),
            };

            let nutrition = IngredientNutrition {
                calories_per_100g: r.calories_per_100g.map(|c| c as f64),
                protein_per_100g: to_f64(r.protein_per_100g),
                fat_per_100g: to_f64(r.fat_per_100g),
                carbs_per_100g: to_f64(r.carbs_per_100g),
            };

            // Pick description for the requested language, fall back to English
            let description = match lang {
                Language::Pl => r.description_pl.as_ref().or(r.description_en.as_ref()),
                Language::Ru => r.description_ru.as_ref().or(r.description_en.as_ref()),
                Language::Uk => r.description_uk.as_ref().or(r.description_en.as_ref()),
                Language::En => r.description_en.as_ref(),
            }
            .cloned();

            let localized_seasons = translate_seasons(&r.seasons, lang);
            let localized_allergens = translate_allergens(&r.allergens, lang);

            Ok(Json(IngredientReference {
                slug: r.slug.unwrap_or_default(),
                name_en: r.name_en,
                name_pl: r.name_pl,
                name_ru: r.name_ru,
                name_uk: r.name_uk,
                description,
                description_en: r.description_en,
                description_pl: r.description_pl,
                description_ru: r.description_ru,
                description_uk: r.description_uk,
                image_url: r.image_url,
                nutrition,
                density_g_per_ml: density,
                measures,
                seasons: r.seasons,
                localized_seasons,
                allergens: r.allergens,
                localized_allergens,
            }))
        }
    }
}

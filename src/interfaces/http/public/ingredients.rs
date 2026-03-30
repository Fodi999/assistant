use axum::{
    extract::{Extension, Path, Query, State},
    http::{header, StatusCode},
    response::{IntoResponse, Json, Response},
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::catalog::{IngredientMeasures, IngredientNutrition, IngredientReference};
use crate::domain::tools::unit_converter as uc;
use crate::infrastructure::cache::{self, AppCache};
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

#[derive(Deserialize)]
pub struct StateQuery {
    pub lang: Option<String>,
}

fn parse_lang(s: &Option<String>) -> Language {
    s.as_deref()
        .and_then(Language::from_code)
        .unwrap_or_default()
}

// ── Response types ────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize)]
pub struct IngredientListItem {
    pub slug: String,
    pub name_en: String,
    pub name_ru: String,
    pub name_pl: String,
    pub name_uk: String,
    pub image_url: Option<String>,
    pub category_id: Option<Uuid>,
    pub category_name_en: Option<String>,
    pub category_name_ru: Option<String>,
    pub category_name_pl: Option<String>,
    pub category_name_uk: Option<String>,
    pub calories_per_100g: Option<i32>,
    pub seasons: Vec<String>,
    pub updated_at: String,
}

#[derive(Serialize, Deserialize)]
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
    category_name_en: Option<String>,
    category_name_ru: Option<String>,
    category_name_pl: Option<String>,
    category_name_uk: Option<String>,
    calories_per_100g: Option<i32>,
    seasons: Vec<String>,
    updated_at: sqlx::types::time::OffsetDateTime,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

/// GET /public/ingredients?lang=pl&q=salmon
///
/// Returns all active ingredients, optionally filtered by full-text search.
/// Supports ?q= for searching across all language name columns and slug.
///
/// 🚀 In-memory cached: list without ?q= served from AppCache (0 DB).
pub async fn list_ingredients(
    State(pool): State<PgPool>,
    Query(params): Query<ListQuery>,
    cache: Option<Extension<AppCache>>,
) -> Result<Json<ListResponse>, (StatusCode, Json<serde_json::Value>)> {
    // Cache: only for no-query (full list) — the hottest endpoint
    let cache_key = if params.q.is_none() {
        Some(cache::keys::ingredients_list())
    } else {
        None
    };

    // Try cache first
    if let (Some(ref key), Some(Extension(ref c))) = (&cache_key, &cache) {
        if let Some(cached) = c.get(key) {
            tracing::debug!("⚡ Cache HIT: {}", key);
            let resp: ListResponse = serde_json::from_value(cached).unwrap_or_else(|_| ListResponse { items: vec![], total: 0 });
            return Ok(Json(resp));
        }
    }

    let rows: Vec<IngredientRow> = if let Some(q) = params.q.as_deref().filter(|s| !s.is_empty()) {
        let pattern = format!("%{}%", q.to_lowercase());
        sqlx::query_as(
            r#"
            SELECT
                ci.slug, ci.name_en, ci.name_ru, ci.name_pl, ci.name_uk,
                ci.image_url, ci.category_id, ci.calories_per_100g,
                ARRAY(SELECT unnest(ci.seasons::text[])) AS seasons,
                cc.name_en AS category_name_en,
                cc.name_ru AS category_name_ru,
                cc.name_pl AS category_name_pl,
                cc.name_uk AS category_name_uk,
                ci.updated_at
            FROM catalog_ingredients ci
            LEFT JOIN catalog_categories cc ON cc.id = ci.category_id
            WHERE ci.is_active = true AND COALESCE(ci.is_published, false) = true
              AND ci.slug IS NOT NULL AND ci.slug != ''
              AND (
                LOWER(ci.name_en) LIKE $1 OR
                LOWER(ci.name_ru) LIKE $1 OR
                LOWER(ci.name_pl) LIKE $1 OR
                LOWER(ci.name_uk) LIKE $1 OR
                ci.slug LIKE $1
              )
            ORDER BY
                CASE WHEN LOWER(ci.name_en) = $2 OR ci.slug = $2 THEN 0 ELSE 1 END,
                ci.name_en ASC
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
                ci.slug, ci.name_en, ci.name_ru, ci.name_pl, ci.name_uk,
                ci.image_url, ci.category_id, ci.calories_per_100g,
                ARRAY(SELECT unnest(ci.seasons::text[])) AS seasons,
                cc.name_en AS category_name_en,
                cc.name_ru AS category_name_ru,
                cc.name_pl AS category_name_pl,
                cc.name_uk AS category_name_uk,
                ci.updated_at
            FROM catalog_ingredients ci
            LEFT JOIN catalog_categories cc ON cc.id = ci.category_id
            WHERE ci.is_active = true AND COALESCE(ci.is_published, false) = true
              AND ci.slug IS NOT NULL AND ci.slug != ''
            ORDER BY ci.name_en ASC
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
            category_name_en: r.category_name_en,
            category_name_ru: r.category_name_ru,
            category_name_pl: r.category_name_pl,
            category_name_uk: r.category_name_uk,
            calories_per_100g: r.calories_per_100g,
            seasons: r.seasons,
            updated_at: r.updated_at.to_string(),
        })
        .collect();

    let response = ListResponse { items, total };

    // Store in cache
    if let (Some(key), Some(Extension(c))) = (cache_key, cache) {
        if let Ok(val) = serde_json::to_value(&response) {
            c.set(key, val);
        }
    }

    Ok(Json(response))
}

// ── Full bulk endpoint (sitemap + SSG — eliminates N+1) ───────────────────────

/// Response item with macros included — no need for per-slug detail fetch.
#[derive(Serialize, Deserialize)]
pub struct IngredientFullItem {
    pub slug: String,
    pub name_en: String,
    pub name_ru: String,
    pub name_pl: String,
    pub name_uk: String,
    pub image_url: Option<String>,
    pub category_id: Option<Uuid>,
    pub category_name_en: Option<String>,
    pub calories_per_100g: Option<i32>,
    pub protein_per_100g: Option<f64>,
    pub fat_per_100g: Option<f64>,
    pub carbs_per_100g: Option<f64>,
    pub seasons: Vec<String>,
    pub updated_at: String,
}

#[derive(Serialize, Deserialize)]
pub struct FullListResponse {
    pub items: Vec<IngredientFullItem>,
    pub total: i64,
}

#[derive(sqlx::FromRow)]
struct FullRow {
    slug: Option<String>,
    name_en: String,
    name_ru: String,
    name_pl: String,
    name_uk: String,
    image_url: Option<String>,
    category_id: Option<Uuid>,
    category_name_en: Option<String>,
    calories_per_100g: Option<i32>,
    protein_per_100g: Option<rust_decimal::Decimal>,
    fat_per_100g: Option<rust_decimal::Decimal>,
    carbs_per_100g: Option<rust_decimal::Decimal>,
    seasons: Vec<String>,
    updated_at: sqlx::types::time::OffsetDateTime,
}

/// GET /public/ingredients-full
///
/// Returns ALL published ingredients with macros in a single query.
/// Designed for sitemap generation and SSG — eliminates N+1 fetches.
/// No limit by default (returns everything published).
///
/// 🚀 In-memory cached: 0 DB on repeat hits.
pub async fn list_ingredients_full(
    State(pool): State<PgPool>,
    cache: Option<Extension<AppCache>>,
) -> Result<Json<FullListResponse>, (StatusCode, Json<serde_json::Value>)> {
    let cache_key = cache::keys::ingredients_full();

    // Try cache
    if let Some(Extension(ref c)) = cache {
        if let Some(cached) = c.get(&cache_key) {
            tracing::debug!("⚡ Cache HIT: {}", cache_key);
            if let Ok(resp) = serde_json::from_value::<FullListResponse>(cached) {
                return Ok(Json(resp));
            }
        }
    }

    let rows: Vec<FullRow> = sqlx::query_as(
        r#"
        SELECT
            ci.slug, ci.name_en, ci.name_ru, ci.name_pl, ci.name_uk,
            ci.image_url, ci.category_id,
            cc.name_en AS category_name_en,
            ci.calories_per_100g,
            ci.protein_per_100g,
            ci.fat_per_100g,
            ci.carbs_per_100g,
            ARRAY(SELECT unnest(ci.seasons::text[])) AS seasons,
            ci.updated_at
        FROM catalog_ingredients ci
        LEFT JOIN catalog_categories cc ON cc.id = ci.category_id
        WHERE ci.is_active = true AND COALESCE(ci.is_published, false) = true
          AND ci.slug IS NOT NULL AND ci.slug != ''
        ORDER BY ci.name_en ASC
        "#,
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| {
        tracing::error!("DB error listing ingredients-full: {e}");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": "Database error" })),
        )
    })?;

    let total = rows.len() as i64;
    let items = rows
        .into_iter()
        .map(|r| {
            use rust_decimal::prelude::ToPrimitive;
            IngredientFullItem {
                slug: r.slug.unwrap_or_default(),
                name_en: r.name_en,
                name_ru: r.name_ru,
                name_pl: r.name_pl,
                name_uk: r.name_uk,
                image_url: r.image_url,
                category_id: r.category_id,
                category_name_en: r.category_name_en,
                calories_per_100g: r.calories_per_100g,
                protein_per_100g: r.protein_per_100g.and_then(|d| d.to_f64()),
                fat_per_100g: r.fat_per_100g.and_then(|d| d.to_f64()),
                carbs_per_100g: r.carbs_per_100g.and_then(|d| d.to_f64()),
                seasons: r.seasons,
                updated_at: r.updated_at.to_string(),
            }
        })
        .collect();

    let response = FullListResponse { items, total };

    // Store in cache
    if let Some(Extension(c)) = cache {
        if let Ok(val) = serde_json::to_value(&response) {
            c.set(cache_key, val);
        }
    }

    Ok(Json(response))
}

/// GET /public/ingredients/:slug?lang=pl
///
/// Returns the full structured ingredient reference:
/// - nutrition (calories, protein, fat, carbs)
/// - density + pre-computed kitchen measures (cup / tbsp / tsp in grams)
/// - description in the requested language (falls back to English)
/// - seasons and allergens translated to the requested language
///
/// 🚀 In-memory cached per slug+lang: 0 DB on repeat hits.
pub async fn get_ingredient_by_slug(
    State(pool): State<PgPool>,
    Path(slug): Path<String>,
    Query(params): Query<LangQuery>,
    cache: Option<Extension<AppCache>>,
) -> Result<Response, (StatusCode, Json<serde_json::Value>)> {
    let lang = parse_lang(&params.lang);
    let lang_code = params.lang.as_deref().unwrap_or("en");
    let cache_key = cache::keys::ingredient_by_slug(&slug, lang_code);

    // Try cache
    if let Some(Extension(ref c)) = cache {
        if let Some(cached) = c.get(&cache_key) {
            tracing::debug!("⚡ Cache HIT: {}", cache_key);
            return Ok(Json(cached).into_response());
        }
    }

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
        None => {
            // Check slug_aliases for 301 redirect
            let alias: Option<(String,)> = sqlx::query_as(
                r#"SELECT ci.slug
                   FROM slug_aliases sa
                   JOIN catalog_ingredients ci ON ci.id = sa.ingredient_id
                   WHERE sa.old_slug = $1 AND ci.is_active = true
                   LIMIT 1"#,
            )
            .bind(&slug)
            .fetch_optional(&pool)
            .await
            .ok()
            .flatten();

            if let Some((new_slug,)) = alias {
                let lang_param = params.lang.as_deref().unwrap_or("en");
                let location = format!("/public/ingredients/{}?lang={}", new_slug, lang_param);
                tracing::info!("🔀 301 redirect: {} → {}", slug, new_slug);
                Ok(Response::builder()
                    .status(StatusCode::MOVED_PERMANENTLY)
                    .header(header::LOCATION, &location)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(axum::body::Body::from(
                        serde_json::to_string(&serde_json::json!({
                            "redirect": true,
                            "old_slug": slug,
                            "new_slug": new_slug,
                            "location": location,
                        })).unwrap_or_default()
                    ))
                    .unwrap()
                    .into_response())
            } else {
                Err((
                    StatusCode::NOT_FOUND,
                    Json(serde_json::json!({
                        "error": format!("Ingredient '{}' not found", slug)
                    })),
                ))
            }
        }
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

            let result = IngredientReference {
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
                seo_title: r.seo_title,
                seo_description: r.seo_description,
                seo_h1: r.seo_h1,
                canonical_url: r.canonical_url,
                og_title: r.og_title,
                og_description: r.og_description,
                og_image: r.og_image,
            };

            // Store in cache
            if let Some(Extension(c)) = cache {
                if let Ok(val) = serde_json::to_value(&result) {
                    c.set(cache_key, val);
                }
            }

            Ok(Json(result).into_response())
        }
    }
}

// ── Public States Endpoint ────────────────────────────────────────────────────

/// Single processing state for public API
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct PublicStateRow {
    pub state: String,
    pub calories_per_100g: Option<f64>,
    pub protein_per_100g: Option<f64>,
    pub fat_per_100g: Option<f64>,
    pub carbs_per_100g: Option<f64>,
    pub fiber_per_100g: Option<f64>,
    pub water_percent: Option<f64>,
    pub shelf_life_hours: Option<i32>,
    pub storage_temp_c: Option<i32>,
    pub texture: Option<String>,
    pub weight_change_percent: Option<f64>,
    pub state_type: Option<String>,
    pub oil_absorption_g: Option<f64>,
    pub water_loss_percent: Option<f64>,
    pub glycemic_index: Option<i16>,
    pub cooking_method: Option<String>,
    pub name_suffix_en: Option<String>,
    pub name_suffix_pl: Option<String>,
    pub name_suffix_ru: Option<String>,
    pub name_suffix_uk: Option<String>,
    pub notes_en: Option<String>,
    pub notes_pl: Option<String>,
    pub notes_ru: Option<String>,
    pub notes_uk: Option<String>,
    pub data_score: Option<f64>,
}

/// GET /public/ingredients/:slug/states?lang=pl
///
/// Returns all processing states for an ingredient (raw, boiled, fried, etc.)
/// Each state includes recalculated nutrition, storage rules, and translations.
/// Perfect for SEO pages: /ingredients/almonds/fried
///
/// 🚀 In-memory cached per slug.
pub async fn get_ingredient_states(
    State(pool): State<PgPool>,
    Path(slug): Path<String>,
    Query(_params): Query<StateQuery>,
    cache: Option<Extension<AppCache>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let cache_key = cache::keys::ingredient_states(&slug);

    // Try cache
    if let Some(Extension(ref c)) = cache {
        if let Some(cached) = c.get(&cache_key) {
            tracing::debug!("⚡ Cache HIT: {}", cache_key);
            return Ok(Json(cached));
        }
    }

    // 1. Resolve slug → ingredient_id
    let ingredient: Option<(Uuid, String, String, String, String, String, Option<String>)> =
        sqlx::query_as(
            r#"SELECT id, name_en, name_pl, name_ru, name_uk, COALESCE(slug, '') as slug, image_url
               FROM catalog_ingredients
               WHERE slug = $1 AND is_active = true"#,
        )
        .bind(&slug)
        .fetch_optional(&pool)
        .await
        .map_err(|e| {
            tracing::error!("DB error resolving slug {}: {}", slug, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "Database error" })),
            )
        })?;

    let (ingredient_id, name_en, name_pl, name_ru, name_uk, resolved_slug, image_url) =
        match ingredient {
            Some(row) => row,
            None => {
                return Err((
                    StatusCode::NOT_FOUND,
                    Json(serde_json::json!({
                        "error": format!("Ingredient '{}' not found", slug)
                    })),
                ))
            }
        };

    // 2. Fetch all states
    let states = sqlx::query_as::<_, PublicStateRow>(
        r#"SELECT
            state::text as state,
            calories_per_100g::float8, protein_per_100g::float8,
            fat_per_100g::float8, carbs_per_100g::float8,
            fiber_per_100g::float8, water_percent::float8,
            shelf_life_hours, storage_temp_c, texture,
            weight_change_percent::float8, state_type, oil_absorption_g::float8, water_loss_percent::float8,
            glycemic_index, cooking_method::text as cooking_method,
            name_suffix_en, name_suffix_pl, name_suffix_ru, name_suffix_uk,
            notes_en, notes_pl, notes_ru, notes_uk,
            data_score::float8
        FROM ingredient_states
        WHERE ingredient_id = $1
        ORDER BY
            CASE state
                WHEN 'raw' THEN 0
                WHEN 'boiled' THEN 1
                WHEN 'steamed' THEN 2
                WHEN 'baked' THEN 3
                WHEN 'grilled' THEN 4
                WHEN 'fried' THEN 5
                WHEN 'smoked' THEN 6
                WHEN 'frozen' THEN 7
                WHEN 'dried' THEN 8
                WHEN 'pickled' THEN 9
            END"#,
    )
    .bind(ingredient_id)
    .fetch_all(&pool)
    .await
    .map_err(|e| {
        tracing::error!("DB error fetching states for {}: {}", slug, e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": "Database error" })),
        )
    })?;

    let result = serde_json::json!({
        "slug": resolved_slug,
        "ingredient_id": ingredient_id,
        "name_en": name_en,
        "name_pl": name_pl,
        "name_ru": name_ru,
        "name_uk": name_uk,
        "image_url": image_url,
        "states_count": states.len(),
        "states": states,
    });

    // Store in cache
    if let Some(Extension(c)) = cache {
        c.set(cache_key, result.clone());
    }

    Ok(Json(result))
}

/// GET /public/ingredients/:slug/states/:state?lang=pl
///
/// Returns ONE specific processing state for an ingredient.
/// Perfect for individual SEO pages: /ingredients/almonds/fried
///
/// 🚀 In-memory cached per slug+state.
pub async fn get_ingredient_state(
    State(pool): State<PgPool>,
    Path((slug, state)): Path<(String, String)>,
    Query(_params): Query<StateQuery>,
    cache: Option<Extension<AppCache>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let cache_key = cache::keys::ingredient_state(&slug, &state);

    // Try cache
    if let Some(Extension(ref c)) = cache {
        if let Some(cached) = c.get(&cache_key) {
            tracing::debug!("⚡ Cache HIT: {}", cache_key);
            return Ok(Json(cached));
        }
    }

    // 1. Resolve slug → ingredient
    let ingredient: Option<(Uuid, String, String, String, String, String, Option<String>)> =
        sqlx::query_as(
            r#"SELECT id, name_en, name_pl, name_ru, name_uk, COALESCE(slug, '') as slug, image_url
               FROM catalog_ingredients
               WHERE slug = $1 AND is_active = true"#,
        )
        .bind(&slug)
        .fetch_optional(&pool)
        .await
        .map_err(|e| {
            tracing::error!("DB error resolving slug {}: {}", slug, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "Database error" })),
            )
        })?;

    let (ingredient_id, name_en, name_pl, name_ru, name_uk, resolved_slug, image_url) =
        match ingredient {
            Some(row) => row,
            None => {
                return Err((
                    StatusCode::NOT_FOUND,
                    Json(serde_json::json!({
                        "error": format!("Ingredient '{}' not found", slug)
                    })),
                ))
            }
        };

    // 2. Fetch specific state
    let state_row = sqlx::query_as::<_, PublicStateRow>(
        r#"SELECT
            state::text as state,
            calories_per_100g::float8, protein_per_100g::float8,
            fat_per_100g::float8, carbs_per_100g::float8,
            fiber_per_100g::float8, water_percent::float8,
            shelf_life_hours, storage_temp_c, texture,
            weight_change_percent::float8, state_type, oil_absorption_g::float8, water_loss_percent::float8,
            glycemic_index, cooking_method::text as cooking_method,
            name_suffix_en, name_suffix_pl, name_suffix_ru, name_suffix_uk,
            notes_en, notes_pl, notes_ru, notes_uk,
            data_score::float8
        FROM ingredient_states
        WHERE ingredient_id = $1 AND state::text = $2"#,
    )
    .bind(ingredient_id)
    .bind(&state)
    .fetch_optional(&pool)
    .await
    .map_err(|e| {
        tracing::error!("DB error fetching state {}/{}: {}", slug, state, e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": "Database error" })),
        )
    })?;

    match state_row {
        None => Err((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": format!("State '{}' not found for ingredient '{}'", state, slug)
            })),
        )),
        Some(s) => {
            let result = serde_json::json!({
                "slug": resolved_slug,
                "ingredient_id": ingredient_id,
                "name_en": name_en,
                "name_pl": name_pl,
                "name_ru": name_ru,
                "name_uk": name_uk,
                "image_url": image_url,
                "state": s.state,
                "state_type": s.state_type,
                "cooking_method": s.cooking_method,
                "glycemic_index": s.glycemic_index,
                "name_suffix_en": s.name_suffix_en,
                "name_suffix_pl": s.name_suffix_pl,
                "name_suffix_ru": s.name_suffix_ru,
                "name_suffix_uk": s.name_suffix_uk,
                "nutrition": {
                    "calories_per_100g": s.calories_per_100g,
                    "protein_per_100g": s.protein_per_100g,
                    "fat_per_100g": s.fat_per_100g,
                    "carbs_per_100g": s.carbs_per_100g,
                    "fiber_per_100g": s.fiber_per_100g,
                    "water_percent": s.water_percent,
                },
                "cooking": {
                    "weight_change_percent": s.weight_change_percent,
                    "oil_absorption_g": s.oil_absorption_g,
                    "water_loss_percent": s.water_loss_percent,
                },
                "storage": {
                    "shelf_life_hours": s.shelf_life_hours,
                    "storage_temp_c": s.storage_temp_c,
                    "texture": s.texture,
                },
                "notes_en": s.notes_en,
                "notes_pl": s.notes_pl,
                "notes_ru": s.notes_ru,
                "notes_uk": s.notes_uk,
                "data_score": s.data_score,
            });

            // Store in cache
            if let Some(Extension(c)) = cache {
                c.set(cache_key, result.clone());
            }

            Ok(Json(result))
        },
    }
}

// ── Autocomplete ──────────────────────────────────────────────────────────────

/// GET /public/ingredients/autocomplete?q=lo&lang=ru&limit=10
///
/// Lightweight typeahead search: returns only slug + localised name + image.
/// Minimum query: 2 characters.  Starts-with search (`ILIKE 'q%'`) first,
/// falls back to contains (`ILIKE '%q%'`) so prefix hits come first.
#[derive(Deserialize)]
pub struct AutocompleteQuery {
    pub q: Option<String>,
    pub lang: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct AutocompleteItem {
    pub slug: String,
    pub name: String,       // localised display name
    pub name_en: String,    // always English for entity_a in generation
    pub image_url: Option<String>,
    pub category: Option<String>, // localised category name
}

pub async fn autocomplete_ingredients(
    State(pool): State<PgPool>,
    Query(params): Query<AutocompleteQuery>,
) -> Result<Json<Vec<AutocompleteItem>>, (StatusCode, Json<serde_json::Value>)> {
    let q = params.q.unwrap_or_default();
    let q = q.trim();

    if q.chars().count() < 2 {
        return Ok(Json(vec![]));
    }

    let lang = params
        .lang
        .as_deref()
        .and_then(Language::from_code)
        .unwrap_or_default();

    let limit = params.limit.unwrap_or(10).clamp(1, 50);

    // Column to display and sort by depends on requested language.
    // But we ALWAYS search all 4 language columns so that e.g.
    // typing "ло" finds "Лосось" (name_ru) even when lang=en.
    let (name_col, cat_col) = match lang {
        Language::Ru => ("ci.name_ru", "cc.name_ru"),
        Language::Pl => ("ci.name_pl", "cc.name_pl"),
        Language::Uk => ("ci.name_uk", "cc.name_uk"),
        Language::En => ("ci.name_en", "cc.name_en"),
    };

    // $1 = prefix pattern  ('q%')
    // $2 = contains pattern ('%q%')
    // $3 = limit
    //
    // Priority ordering:
    //   0 – display-lang prefix match  (best: "лос" → "Лосось" in ru)
    //   1 – any-lang prefix match
    //   2 – display-lang contains match
    //   3 – any-lang contains match
    let sql = format!(
        r#"
        SELECT
            COALESCE(ci.slug, LOWER(REPLACE(ci.name_en, ' ', '-'))) AS slug,
            {name_col}                          AS name,
            ci.name_en,
            ci.image_url,
            {cat_col}                           AS category
        FROM catalog_ingredients ci
        LEFT JOIN catalog_categories cc ON cc.id = ci.category_id
        WHERE COALESCE(ci.is_active, true) = true
          AND (
                -- search in ALL four language columns
                ci.name_en ILIKE $2
             OR ci.name_ru ILIKE $2
             OR ci.name_pl ILIKE $2
             OR ci.name_uk ILIKE $2
             OR ci.slug    ILIKE $2
          )
        ORDER BY
            CASE
                WHEN {name_col} ILIKE $1 THEN 0          -- display-lang prefix
                WHEN ci.name_en ILIKE $1
                  OR ci.name_ru ILIKE $1
                  OR ci.name_pl ILIKE $1
                  OR ci.name_uk ILIKE $1 THEN 1           -- any-lang prefix
                WHEN {name_col} ILIKE $2 THEN 2           -- display-lang contains
                ELSE 3                                     -- any-lang contains
            END,
            {name_col} ASC
        LIMIT $3
        "#,
        name_col = name_col,
        cat_col = cat_col,
    );

    let prefix = format!("{}%", q);
    let contains = format!("%{}%", q);

    #[derive(sqlx::FromRow)]
    struct Row {
        slug: String,
        name: String,
        name_en: String,
        image_url: Option<String>,
        category: Option<String>,
    }

    let rows: Vec<Row> = sqlx::query_as(&sql)
        .bind(&prefix)
        .bind(&contains)
        .bind(limit)
        .fetch_all(&pool)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
        })?;

    let items = rows
        .into_iter()
        .map(|r| AutocompleteItem {
            slug: r.slug,
            name: r.name,
            name_en: r.name_en,
            image_url: r.image_url,
            category: r.category,
        })
        .collect();

    Ok(Json(items))
}

// ── Bulk states map (sitemap use — eliminates N+1) ────────────────────────────

/// GET /public/ingredients-states-map
///
/// Returns { "slug": ["raw", "boiled", ...] } for ALL active ingredients that
/// have at least one processing state in the DB.
///
/// Used by the sitemap to emit ONLY state URLs that actually exist, avoiding
/// 404s for ingredients that haven't had states generated yet.
/// Single query — O(1) instead of O(N) for the sitemap.
/// 🚀 In-memory cached.
pub async fn get_ingredients_states_map(
    State(pool): State<PgPool>,
    cache: Option<Extension<AppCache>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let cache_key = cache::keys::ingredients_states_map();

    // Try cache
    if let Some(Extension(ref c)) = cache {
        if let Some(cached) = c.get(&cache_key) {
            tracing::debug!("⚡ Cache HIT: {}", cache_key);
            return Ok(Json(cached));
        }
    }

    #[derive(sqlx::FromRow)]
    struct Row {
        slug: String,
        state: String,
    }

    let rows: Vec<Row> = sqlx::query_as(
        r#"
        SELECT ci.slug, s.state::text AS state
        FROM ingredient_states s
        JOIN catalog_ingredients ci ON ci.id = s.ingredient_id
        WHERE ci.is_active = true
          AND COALESCE(ci.is_published, false) = true
          AND ci.slug IS NOT NULL AND ci.slug != ''
        ORDER BY ci.slug, s.state
        "#,
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| {
        tracing::error!("DB error fetching states map: {e}");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": "Database error" })),
        )
    })?;

    // Group by slug → [states]
    let mut map: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    for row in rows {
        map.entry(row.slug).or_default().push(row.state);
    }

    let result = serde_json::json!(map);

    // Store in cache
    if let Some(Extension(c)) = cache {
        c.set(cache_key, result.clone());
    }

    Ok(Json(result))
}

// ── Sitemap data (single query, all quality filters built in) ─────────────────

/// Sitemap-ready ingredient entry with quality flags.
/// Frontend builds sitemap.xml from this — no additional filtering needed.
#[derive(Debug, Serialize, Deserialize)]
pub struct SitemapIngredient {
    pub slug: String,
    pub updated_at: String,
    /// true if density_g_per_ml is set → how-many pages are valid
    pub has_conversions: bool,
    /// true if calories_per_100g is set → nutrition pages are valid
    pub has_nutrition: bool,
    /// List of actual processing states in DB (only these get sitemap URLs)
    pub states: Vec<String>,
}

/// GET /public/ingredients-sitemap-data
///
/// Returns ALL published ingredients with quality flags + actual states.
/// **Single query** — frontend builds sitemap.xml directly from this.
///
/// Quality rules (built into response, frontend just trusts them):
/// - `has_conversions: true` → emit how-many pages for this ingredient
/// - `has_nutrition: true` → emit nutrition/calorie pages
/// - `states: [...]` → emit ONLY these state pages (no guessing)
///
/// This eliminates:
/// - 404s from how-many pages without conversion data
/// - 404s from state pages that don't exist
/// - Thin/empty pages that hurt Google rankings
/// 🚀 In-memory cached.
pub async fn get_ingredients_sitemap_data(
    State(pool): State<PgPool>,
    cache: Option<Extension<AppCache>>,
) -> Result<Json<Vec<SitemapIngredient>>, (StatusCode, Json<serde_json::Value>)> {
    let cache_key = cache::keys::ingredients_sitemap();

    // Try cache
    if let Some(Extension(ref c)) = cache {
        if let Some(cached) = c.get(&cache_key) {
            tracing::debug!("⚡ Cache HIT: {}", cache_key);
            if let Ok(resp) = serde_json::from_value::<Vec<SitemapIngredient>>(cached) {
                return Ok(Json(resp));
            }
        }
    }

    // 1. Fetch all published ingredients with quality flags
    #[derive(sqlx::FromRow)]
    struct IngRow {
        slug: String,
        updated_at: sqlx::types::time::OffsetDateTime,
        has_conversions: bool,
        has_nutrition: bool,
    }

    let ingredients: Vec<IngRow> = sqlx::query_as(
        r#"
        SELECT
            ci.slug,
            ci.updated_at,
            (ci.density_g_per_ml IS NOT NULL) AS has_conversions,
            (ci.calories_per_100g IS NOT NULL) AS has_nutrition
        FROM catalog_ingredients ci
        WHERE ci.is_active = true
          AND COALESCE(ci.is_published, false) = true
          AND ci.slug IS NOT NULL AND ci.slug != ''
        ORDER BY ci.slug
        "#,
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| {
        tracing::error!("DB error fetching sitemap data: {e}");
        (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": "Database error" })))
    })?;

    // 2. Fetch all states in one query
    #[derive(sqlx::FromRow)]
    struct StateRow {
        slug: String,
        state: String,
    }

    let state_rows: Vec<StateRow> = sqlx::query_as(
        r#"
        SELECT ci.slug, s.state::text AS state
        FROM ingredient_states s
        JOIN catalog_ingredients ci ON ci.id = s.ingredient_id
        WHERE ci.is_active = true
          AND COALESCE(ci.is_published, false) = true
          AND ci.slug IS NOT NULL AND ci.slug != ''
        ORDER BY ci.slug, s.state
        "#,
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| {
        tracing::error!("DB error fetching states for sitemap: {e}");
        (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": "Database error" })))
    })?;

    // 3. Group states by slug
    let mut states_map: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    for row in state_rows {
        states_map.entry(row.slug).or_default().push(row.state);
    }

    // 4. Merge into response
    let result: Vec<SitemapIngredient> = ingredients
        .into_iter()
        .map(|ing| {
            let states = states_map.remove(&ing.slug).unwrap_or_default();
            SitemapIngredient {
                slug: ing.slug,
                updated_at: ing.updated_at.to_string(),
                has_conversions: ing.has_conversions,
                has_nutrition: ing.has_nutrition,
                states,
            }
        })
        .collect();

    tracing::info!(
        "📊 Sitemap data: {} ingredients, {} with conversions, {} with states",
        result.len(),
        result.iter().filter(|i| i.has_conversions).count(),
        result.iter().filter(|i| !i.states.is_empty()).count(),
    );

    // Store in cache
    if let Some(Extension(c)) = cache {
        if let Ok(val) = serde_json::to_value(&result) {
            c.set(cache_key, val);
        }
    }

    Ok(Json(result))
}
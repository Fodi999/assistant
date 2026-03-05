use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

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
pub struct IngredientDetail {
    pub slug: String,
    pub name_en: String,
    pub name_ru: String,
    pub name_pl: String,
    pub name_uk: String,
    pub image_url: Option<String>,
    pub category_id: Option<Uuid>,
    pub default_unit: String,
    pub default_shelf_life_days: Option<i32>,
    pub allergens: Vec<String>,
    pub calories_per_100g: Option<i32>,
    pub seasons: Vec<String>,
    pub description: Option<String>,
    pub min_stock_threshold: Option<f64>,
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

#[derive(sqlx::FromRow)]
struct IngredientDetailRow {
    slug: Option<String>,
    name_en: String,
    name_ru: String,
    name_pl: String,
    name_uk: String,
    image_url: Option<String>,
    category_id: Option<Uuid>,
    default_unit: String,
    default_shelf_life_days: Option<i32>,
    allergens: Vec<String>,
    calories_per_100g: Option<i32>,
    seasons: Vec<String>,
    description: Option<String>,
    min_stock_threshold: Option<rust_decimal::Decimal>,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

/// GET /public/ingredients
/// Returns all active ingredients (slug, name, image, calories, seasons)
pub async fn list_ingredients(
    State(pool): State<PgPool>,
) -> Result<Json<ListResponse>, (StatusCode, Json<serde_json::Value>)> {
    let rows: Vec<IngredientRow> = sqlx::query_as(
        r#"
        SELECT
            slug,
            name_en,
            name_ru,
            name_pl,
            name_uk,
            image_url,
            category_id,
            calories_per_100g,
            ARRAY(SELECT unnest(seasons::text[])) AS seasons
        FROM catalog_ingredients
        WHERE is_active = true AND slug IS NOT NULL AND slug != ''
        ORDER BY name_en ASC
        "#,
    )
    .fetch_all(&pool)
    .await
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

/// GET /public/ingredients/:slug
/// Returns full ingredient detail by slug
pub async fn get_ingredient_by_slug(
    State(pool): State<PgPool>,
    Path(slug): Path<String>,
) -> Result<Json<IngredientDetail>, (StatusCode, Json<serde_json::Value>)> {
    let row: Option<IngredientDetailRow> = sqlx::query_as(
        r#"
        SELECT
            slug,
            name_en,
            name_ru,
            name_pl,
            name_uk,
            image_url,
            category_id,
            default_unit::text AS default_unit,
            default_shelf_life_days,
            ARRAY(SELECT unnest(allergens::text[])) AS allergens,
            calories_per_100g,
            ARRAY(SELECT unnest(seasons::text[])) AS seasons,
            description,
            min_stock_threshold
        FROM catalog_ingredients
        WHERE slug = $1 AND is_active = true
        "#,
    )
    .bind(&slug)
    .fetch_optional(&pool)
    .await
    .map_err(|e| {
        tracing::error!("DB error fetching ingredient slug={slug}: {e}");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": "Database error" })),
        )
    })?;

    match row {
        Some(r) => Ok(Json(IngredientDetail {
            slug: r.slug.unwrap_or_default(),
            name_en: r.name_en,
            name_ru: r.name_ru,
            name_pl: r.name_pl,
            name_uk: r.name_uk,
            image_url: r.image_url,
            category_id: r.category_id,
            default_unit: r.default_unit,
            default_shelf_life_days: r.default_shelf_life_days,
            allergens: r.allergens,
            calories_per_100g: r.calories_per_100g,
            seasons: r.seasons,
            description: r.description,
            min_stock_threshold: r.min_stock_threshold.map(|d| {
                rust_decimal::prelude::ToPrimitive::to_f64(&d).unwrap_or(0.0)
            }),
        })),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": format!("Ingredient '{}' not found", slug)
            })),
        )),
    }
}

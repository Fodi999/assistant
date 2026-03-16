//! Share recipe endpoint — save & retrieve recipe configs via short slugs.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use rand::Rng;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

// ── Request / Response types ────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ShareRecipeRequest {
    pub ingredients: Vec<ShareIngredient>,
    pub portions: Option<u32>,
    pub lang: Option<String>,
    pub title: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ShareIngredient {
    pub slug: String,
    pub grams: f64,
}

#[derive(Debug, Serialize)]
pub struct ShareRecipeResponse {
    pub slug: String,
    pub url: String,
}

#[derive(Debug, Serialize)]
pub struct SharedRecipeData {
    pub slug: String,
    pub ingredients: Vec<ShareIngredient>,
    pub portions: u32,
    pub lang: String,
    pub title: Option<String>,
}

// ── Slug generator ─────────────────────────────────────────────────────────

/// Generate a short URL-safe slug like "aB3kX7mQ"
fn generate_slug(len: usize) -> String {
    const CHARSET: &[u8] = b"abcdefghijkmnopqrstuvwxyzABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    let mut rng = rand::thread_rng();
    (0..len)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

// ── POST /tools/share-recipe ───────────────────────────────────────────────

pub async fn share_recipe(
    State(pool): State<PgPool>,
    Json(body): Json<ShareRecipeRequest>,
) -> Result<Json<ShareRecipeResponse>, (StatusCode, String)> {
    // Validate
    if body.ingredients.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "ingredients required".into()));
    }
    if body.ingredients.len() > 50 {
        return Err((StatusCode::BAD_REQUEST, "max 50 ingredients".into()));
    }

    let portions = body.portions.unwrap_or(1).max(1).min(100);
    let lang = body.lang.clone().unwrap_or_else(|| "en".to_string());
    let title = body.title.clone();

    let ingredients_json = serde_json::to_value(&body.ingredients)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Try up to 5 times to generate a unique slug
    for _ in 0..5 {
        let slug = generate_slug(8);
        let result = sqlx::query_scalar::<_, String>(
            r#"
            INSERT INTO shared_recipes (slug, ingredients_json, portions, lang, title)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (slug) DO NOTHING
            RETURNING slug
            "#,
        )
        .bind(&slug)
        .bind(&ingredients_json)
        .bind(portions as i32)
        .bind(&lang)
        .bind(&title)
        .fetch_optional(&pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        if let Some(created_slug) = result {
            return Ok(Json(ShareRecipeResponse {
                url: format!("/chef-tools/lab/r/{}", created_slug),
                slug: created_slug,
            }));
        }
    }

    Err((
        StatusCode::INTERNAL_SERVER_ERROR,
        "could not generate unique slug".into(),
    ))
}

// ── GET /tools/shared-recipe/:slug ─────────────────────────────────────────

pub async fn get_shared_recipe(
    State(pool): State<PgPool>,
    Path(slug): Path<String>,
) -> Result<Json<SharedRecipeData>, (StatusCode, String)> {
    let row = sqlx::query_as::<_, SharedRecipeRow>(
        "SELECT slug, ingredients_json, portions, lang, title FROM shared_recipes WHERE slug = $1",
    )
    .bind(&slug)
    .fetch_optional(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    match row {
        Some(r) => {
            let ingredients: Vec<ShareIngredient> =
                serde_json::from_value(r.ingredients_json)
                    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            Ok(Json(SharedRecipeData {
                slug: r.slug,
                ingredients,
                portions: r.portions as u32,
                lang: r.lang,
                title: r.title,
            }))
        }
        None => Err((StatusCode::NOT_FOUND, "recipe not found".into())),
    }
}

// ── DB row ─────────────────────────────────────────────────────────────────

#[derive(Debug, sqlx::FromRow)]
struct SharedRecipeRow {
    slug: String,
    ingredients_json: serde_json::Value,
    portions: i32,
    lang: String,
    title: Option<String>,
}

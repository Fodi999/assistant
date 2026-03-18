//! Handlers for nutrition endpoints:
//! - GET /public/tools/nutrition
//! - GET /public/tools/ingredients
//! - GET /public/tools/compare

use axum::extract::{Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::domain::tools::catalog_row::{CatalogNutritionRow, CATALOG_NUTRITION_COLS};
use crate::domain::tools::nutrition::{
    breakdown_per_100g_nullable, macros_ratio, nutrition_score, vitamins_for,
    MacrosRatio, NutritionBreakdownNullable, VitaminData,
};
use crate::domain::tools::unit_converter as uc;
use crate::shared::Language;

use super::shared::{label, parse_lang};

// ── 1. GET /public/tools/nutrition ───────────────────────────────────────────

#[derive(Deserialize)]
pub struct NutritionQuery {
    pub name:   Option<String>,
    pub slug:   Option<String>,
    pub amount: Option<f64>,
    pub unit:   Option<String>,
    pub lang:   Option<String>,
}

#[derive(Serialize)]
pub struct NutritionResponse {
    pub query:             String,
    pub slug:              Option<String>,
    /// If the queried slug was an old alias, this contains the canonical slug.
    /// Frontend should 301-redirect to the new URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redirect_slug:     Option<String>,
    pub name:              String,
    pub product_type:      Option<String>,
    pub image_url:         Option<String>,
    pub water_type:        Option<String>,
    pub wild_farmed:       Option<String>,
    pub sushi_grade:       Option<bool>,
    pub amount_g:          f64,
    pub unit:              String,
    pub unit_label:        String,
    pub per_100g:          NutritionBreakdownNullable,
    pub for_amount:        NutritionBreakdownNullable,
    pub macros_ratio:      MacrosRatio,
    pub nutrition_score:   u8,
    pub vitamins:          VitaminData,
    pub typical_portion_g: Option<f64>,
    pub found_in_db:       bool,
    pub lang:              String,
}

pub async fn nutrition(
    State(pool): State<PgPool>,
    Query(params): Query<NutritionQuery>,
) -> Json<NutritionResponse> {
    let lang     = parse_lang(&params.lang);
    let lang_str = params.lang.clone().unwrap_or_else(|| "en".to_string());
    let raw_amount = params.amount.unwrap_or(100.0);
    let unit_str   = params.unit.clone().unwrap_or_else(|| "g".to_string());
    let unit       = unit_str.as_str();

    let query_str = params.slug.clone()
        .or_else(|| params.name.clone())
        .unwrap_or_else(|| "unknown".to_string());

    let lookup = if let Some(ref s) = params.slug {
        s.to_lowercase()
    } else if let Some(ref n) = params.name {
        n.to_lowercase()
    } else {
        "unknown".to_string()
    };

    let db_row: Option<CatalogNutritionRow> = sqlx::query_as(&format!(
        r#"SELECT {CATALOG_NUTRITION_COLS}
           FROM catalog_ingredients
           LEFT JOIN slug_aliases sa ON sa.ingredient_id = catalog_ingredients.id AND sa.old_slug = $1
           WHERE is_active = true
             AND (slug = $1 OR sa.old_slug = $1 OR LOWER(name_en) = $1 OR LOWER(name_en) LIKE '%' || $1 || '%')
           ORDER BY (slug = $1)::int DESC, (sa.old_slug = $1)::int DESC, (LOWER(name_en) = $1)::int DESC, LENGTH(name_en) ASC
           LIMIT 1"#,
    ))
    .bind(&lookup)
    .fetch_optional(&pool)
    .await
    .ok()
    .flatten();

    let found = db_row.is_some();

    let (localized, slug_val, image, product_type, water_type, wild_farmed, sushi_grade,
         typical_g, density,
         cal, prot, fat, carbs, fiber, sugar, salt,
         cal_opt, prot_opt, fat_opt, carbs_opt, fiber_opt, sugar_opt, salt_opt) =
        if let Some(ref r) = db_row {
            (r.localized_name(lang).to_string(), r.slug.clone(), r.image_url.clone(),
             r.product_type.clone(), r.water_type.clone(), r.wild_farmed.clone(), r.sushi_grade,
             r.typical_g(), r.density(),
             r.cal(), r.prot(), r.fat(), r.carbs(), r.fiber(), r.sugar(), r.salt(),
             r.cal_opt(), r.prot_opt(), r.fat_opt(), r.carbs_opt(), r.fiber_opt(), r.sugar_opt(), r.salt_opt())
        } else {
            (query_str.clone(), None, None, None, None, None, None, None, 1.0,
             0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
             None, None, None, None, None, None, None)
        };

    let amount_g = if unit == "g" {
        raw_amount
    } else if let Some(g) = uc::mass_to_grams(unit) {
        raw_amount * g
    } else if let Some(ml) = uc::volume_to_ml(unit) {
        raw_amount * ml * density
    } else {
        raw_amount
    };

    // Nullable breakdown for API response (null = no data, NOT 0)
    let per_100g  = breakdown_per_100g_nullable(cal_opt, prot_opt, fat_opt, carbs_opt, fiber_opt, sugar_opt, salt_opt);
    let factor    = amount_g / 100.0;
    let for_amount = per_100g.scale(factor);

    // If the queried slug was an old alias, signal a redirect to the canonical slug
    let redirect_slug = if found {
        let canonical = slug_val.as_deref().unwrap_or("");
        if !canonical.is_empty() && canonical != lookup {
            Some(canonical.to_string())
        } else {
            None
        }
    } else {
        None
    };

    Json(NutritionResponse {
        query: query_str, slug: slug_val.clone(), redirect_slug, name: localized,
        product_type, image_url: image, water_type, wild_farmed, sushi_grade,
        amount_g: uc::round_to(amount_g, 1),
        unit: unit_str, unit_label: label("g", lang),
        per_100g, for_amount,
        macros_ratio:      macros_ratio(prot, fat, carbs),
        nutrition_score:   nutrition_score(cal, prot, fat, carbs, fiber, sugar, salt),
        vitamins:          vitamins_for(slug_val.as_deref().unwrap_or("")),
        typical_portion_g: typical_g,
        found_in_db: found, lang: lang_str,
    })
}

// ── 2. GET /public/tools/ingredients ─────────────────────────────────────────

#[derive(Deserialize)]
pub struct IngredientsQuery {
    pub lang:         Option<String>,
    pub product_type: Option<String>,
    pub search:       Option<String>,
    pub limit:        Option<i64>,
    pub offset:       Option<i64>,
}

#[derive(Serialize)]
pub struct IngredientDbEntry {
    pub slug:              Option<String>,
    pub name:              String,
    pub name_en:           String,
    pub product_type:      Option<String>,
    pub image_url:         Option<String>,
    pub water_type:        Option<String>,
    pub wild_farmed:       Option<String>,
    pub sushi_grade:       Option<bool>,
    pub typical_portion_g: Option<f64>,
    pub per_100g:          NutritionBreakdownNullable,
    pub macros_ratio:      MacrosRatio,
    pub nutrition_score:   u8,
    pub vitamins:          VitaminData,
}

#[derive(Serialize)]
pub struct IngredientsResponse {
    pub total:  i64,
    pub limit:  i64,
    pub offset: i64,
    pub lang:   String,
    pub items:  Vec<IngredientDbEntry>,
}

pub async fn ingredients_db(
    State(pool): State<PgPool>,
    Query(params): Query<IngredientsQuery>,
) -> Json<IngredientsResponse> {
    let lang     = parse_lang(&params.lang);
    let lang_str = params.lang.clone().unwrap_or_else(|| "en".to_string());
    let limit    = params.limit.unwrap_or(200).min(200).max(1);
    let offset   = params.offset.unwrap_or(0).max(0);

    let pt_val   = params.product_type.as_deref().map(str::to_lowercase).unwrap_or_default();
    let srch_val = params.search.as_deref()
        .map(|s| format!("%{}%", s.to_lowercase()))
        .unwrap_or_default();

    // Build WHERE clause dynamically
    let mut where_parts = vec!["is_active = true".to_string()];
    let mut bind_idx = 1usize;
    if !pt_val.is_empty() {
        where_parts.push(format!("product_type = ${bind_idx}"));
        bind_idx += 1;
    }
    if !srch_val.is_empty() {
        where_parts.push(format!(
            "(LOWER(name_en) LIKE ${0} OR LOWER(name_ru) LIKE ${0} OR \
             LOWER(name_pl) LIKE ${0} OR LOWER(name_uk) LIKE ${0} OR LOWER(slug) LIKE ${0})",
            bind_idx
        ));
        bind_idx += 1;
    }
    let where_clause = where_parts.join(" AND ");

    // Count
    let count_q = format!("SELECT COUNT(*) FROM catalog_ingredients WHERE {where_clause}");
    let mut count_builder = sqlx::query_scalar::<_, i64>(&count_q);
    if !pt_val.is_empty()   { count_builder = count_builder.bind(&pt_val); }
    if !srch_val.is_empty() { count_builder = count_builder.bind(&srch_val); }
    let total = count_builder.fetch_one(&pool).await.unwrap_or(0);

    // Fetch
    let select_q = format!(
        "SELECT {CATALOG_NUTRITION_COLS} FROM catalog_ingredients \
         WHERE {where_clause} \
         ORDER BY product_type NULLS LAST, name_en ASC \
         LIMIT ${bind_idx} OFFSET ${}",
        bind_idx + 1
    );
    let mut row_builder = sqlx::query_as::<_, CatalogNutritionRow>(&select_q);
    if !pt_val.is_empty()   { row_builder = row_builder.bind(&pt_val); }
    if !srch_val.is_empty() { row_builder = row_builder.bind(&srch_val); }
    let rows: Vec<CatalogNutritionRow> = row_builder
        .bind(limit).bind(offset)
        .fetch_all(&pool).await.unwrap_or_default();

    let items = rows.into_iter().map(|row| {
        let salt = row.salt();
        let slug_str = row.slug.clone().unwrap_or_default();
        IngredientDbEntry {
            per_100g:          breakdown_per_100g_nullable(
                row.cal_opt(), row.prot_opt(), row.fat_opt(), row.carbs_opt(),
                row.fiber_opt(), row.sugar_opt(), row.salt_opt(),
            ),
            macros_ratio:      macros_ratio(row.prot(), row.fat(), row.carbs()),
            nutrition_score:   nutrition_score(row.cal(), row.prot(), row.fat(), row.carbs(), row.fiber(), row.sugar(), salt),
            vitamins:          vitamins_for(&slug_str),
            slug:              row.slug.clone(),
            name:              row.localized_name(lang).to_string(),
            name_en:           row.name_en.clone(),
            product_type:      row.product_type.clone(),
            image_url:         row.image_url.clone(),
            water_type:        row.water_type.clone(),
            wild_farmed:       row.wild_farmed.clone(),
            sushi_grade:       row.sushi_grade,
            typical_portion_g: row.typical_g(),
        }
    }).collect();

    Json(IngredientsResponse { total, limit, offset, lang: lang_str, items })
}

// ── 3. GET /public/tools/compare ─────────────────────────────────────────────

#[derive(Deserialize)]
pub struct CompareQuery {
    pub food1: String,
    pub food2: String,
    pub lang:  Option<String>,
}

#[derive(Serialize)]
pub struct CompareSide {
    pub query:           String,
    pub slug:            Option<String>,
    pub name:            String,
    pub product_type:    Option<String>,
    pub image_url:       Option<String>,
    pub water_type:      Option<String>,
    pub wild_farmed:     Option<String>,
    pub sushi_grade:     Option<bool>,
    pub per_100g:        NutritionBreakdownNullable,
    pub macros_ratio:    MacrosRatio,
    pub nutrition_score: u8,
    pub vitamins:        VitaminData,
    pub found_in_db:     bool,
}

#[derive(Serialize)]
pub struct CompareWinner {
    pub calories_lower:  String,
    pub protein_higher:  String,
    pub fat_lower:       String,
    pub carbs_lower:     String,
    pub fiber_higher:    String,
    pub nutrition_score: String,
}

#[derive(Serialize)]
pub struct CompareResponse {
    pub food1:  CompareSide,
    pub food2:  CompareSide,
    pub winner: CompareWinner,
    pub lang:   String,
}

async fn lookup_one(pool: &PgPool, query: &str) -> Option<CatalogNutritionRow> {
    let lookup = query.to_lowercase();
    sqlx::query_as(&format!(
        r#"SELECT {CATALOG_NUTRITION_COLS}
           FROM catalog_ingredients
           LEFT JOIN slug_aliases sa ON sa.ingredient_id = catalog_ingredients.id AND sa.old_slug = $1
           WHERE is_active = true
             AND (slug = $1 OR sa.old_slug = $1 OR LOWER(name_en) = $1 OR LOWER(name_en) LIKE '%' || $1 || '%')
           ORDER BY (slug = $1)::int DESC, (sa.old_slug = $1)::int DESC, (LOWER(name_en) = $1)::int DESC, LENGTH(name_en) ASC
           LIMIT 1"#,
    ))
    .bind(&lookup)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
}

fn build_side(query: String, row: Option<CatalogNutritionRow>, lang: Language) -> CompareSide {
    match row {
        Some(r) => {
            let salt     = r.salt();
            let slug_str = r.slug.clone().unwrap_or_default();
            CompareSide {
                query, slug: r.slug.clone(), name: r.localized_name(lang).to_string(),
                product_type: r.product_type.clone(), image_url: r.image_url.clone(),
                water_type: r.water_type.clone(), wild_farmed: r.wild_farmed.clone(),
                sushi_grade: r.sushi_grade,
                per_100g:        breakdown_per_100g_nullable(
                    r.cal_opt(), r.prot_opt(), r.fat_opt(), r.carbs_opt(),
                    r.fiber_opt(), r.sugar_opt(), r.salt_opt(),
                ),
                macros_ratio:    macros_ratio(r.prot(), r.fat(), r.carbs()),
                nutrition_score: nutrition_score(r.cal(), r.prot(), r.fat(), r.carbs(), r.fiber(), r.sugar(), salt),
                vitamins:        vitamins_for(&slug_str),
                found_in_db:     true,
            }
        }
        None => CompareSide {
            query, slug: None, name: "Not found".to_string(),
            product_type: None, image_url: None,
            water_type: None, wild_farmed: None, sushi_grade: None,
            per_100g:        NutritionBreakdownNullable::empty(),
            macros_ratio:    MacrosRatio { protein_pct: 0.0, fat_pct: 0.0, carbs_pct: 0.0 },
            nutrition_score: 0,
            vitamins:        VitaminData::unknown(),
            found_in_db:     false,
        },
    }
}

fn winner(a: f64, b: f64, higher_is_better: bool) -> String {
    if (a - b).abs() < 0.05 { return "tie".to_string(); }
    if higher_is_better {
        if a > b { "food1".to_string() } else { "food2".to_string() }
    } else {
        if a < b { "food1".to_string() } else { "food2".to_string() }
    }
}

/// Compare Option<f64> values — treat None as 0 for winner logic
fn winner_opt(a: Option<f64>, b: Option<f64>, higher_is_better: bool) -> String {
    winner(a.unwrap_or(0.0), b.unwrap_or(0.0), higher_is_better)
}

pub async fn compare_foods(
    State(pool): State<PgPool>,
    Query(params): Query<CompareQuery>,
) -> Json<CompareResponse> {
    let lang     = parse_lang(&params.lang);
    let lang_str = params.lang.clone().unwrap_or_else(|| "en".to_string());

    let (r1, r2) = tokio::join!(
        lookup_one(&pool, &params.food1),
        lookup_one(&pool, &params.food2),
    );

    let s1 = build_side(params.food1, r1, lang);
    let s2 = build_side(params.food2, r2, lang);

    let w = CompareWinner {
        calories_lower:  winner_opt(s1.per_100g.calories,  s2.per_100g.calories,  false),
        protein_higher:  winner_opt(s1.per_100g.protein_g, s2.per_100g.protein_g, true),
        fat_lower:       winner_opt(s1.per_100g.fat_g,     s2.per_100g.fat_g,     false),
        carbs_lower:     winner_opt(s1.per_100g.carbs_g,   s2.per_100g.carbs_g,   false),
        fiber_higher:    winner_opt(s1.per_100g.fiber_g,   s2.per_100g.fiber_g,   true),
        nutrition_score: winner(s1.nutrition_score as f64, s2.nutrition_score as f64, true),
    };

    Json(CompareResponse { food1: s1, food2: s2, winner: w, lang: lang_str })
}

// ── 4. GET /public/tools/resolve-slug ────────────────────────────────────────

#[derive(Deserialize)]
pub struct ResolveSlugQuery {
    pub slug: String,
}

#[derive(Serialize)]
pub struct ResolveSlugResponse {
    pub slug:          String,
    pub canonical:     String,
    pub is_redirect:   bool,
    pub ingredient_id: Option<String>,
}

/// Resolve a slug: returns the canonical (current) slug.
/// If the input slug is an old alias, `is_redirect: true` + canonical slug.
/// Useful for frontend SSR/ISR to do 301 redirects.
pub async fn resolve_slug(
    State(pool): State<PgPool>,
    Query(params): Query<ResolveSlugQuery>,
) -> Json<ResolveSlugResponse> {
    let slug = params.slug.to_lowercase();

    // 1. Direct match?
    let direct: Option<(uuid::Uuid, String)> = sqlx::query_as(
        "SELECT id, slug FROM catalog_ingredients WHERE slug = $1 AND is_active = true LIMIT 1",
    )
    .bind(&slug)
    .fetch_optional(&pool)
    .await
    .ok()
    .flatten();

    if let Some((id, canonical)) = direct {
        return Json(ResolveSlugResponse {
            slug: slug.clone(),
            canonical,
            is_redirect: false,
            ingredient_id: Some(id.to_string()),
        });
    }

    // 2. Alias?
    let alias: Option<(uuid::Uuid, String)> = sqlx::query_as(
        r#"SELECT ci.id, ci.slug
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

    if let Some((id, canonical)) = alias {
        return Json(ResolveSlugResponse {
            slug,
            canonical,
            is_redirect: true,
            ingredient_id: Some(id.to_string()),
        });
    }

    // 3. Not found
    Json(ResolveSlugResponse {
        slug: slug.clone(),
        canonical: slug,
        is_redirect: false,
        ingredient_id: None,
    })
}

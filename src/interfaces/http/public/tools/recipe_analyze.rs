//! Handler for POST /public/tools/recipe-analyze
//!
//! Accepts a list of ingredients (slug + grams), fetches nutrition + culinary
//! + diet flags + pairings from the DB, then runs the domain-layer engines:
//!   - recipe_analyzer::analyze_recipe()
//!   - suggestion_engine::suggest_ingredients()
//!
//! Returns a unified JSON response with nutrition, flavor radar, and suggestions.

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::domain::tools::flavor_graph::{self, FlavorVector, FlavorIngredient};
use crate::domain::tools::nutrition::{self as nut, NutritionBreakdown};
use crate::domain::tools::recipe_analyzer::{self, DietFlags, RecipeIngredientInput};
use crate::domain::tools::suggestion_engine::{self, Candidate};
use crate::domain::tools::unit_converter as uc;
use crate::domain::tools::rule_engine;

// ── Request ──────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct RecipeAnalyzeRequest {
    pub ingredients: Vec<IngredientInput>,
    #[serde(default = "default_portions")]
    pub portions: u32,
    /// Optional language code (en, ru, pl, uk) for localized names
    pub lang: Option<String>,
}

fn default_portions() -> u32 { 1 }

#[derive(Debug, Deserialize)]
pub struct IngredientInput {
    pub slug:  String,
    pub grams: f64,
}

// ── Response ─────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct RecipeAnalyzeResponse {
    pub nutrition:   NutritionSummary,
    pub per_portion: Option<NutritionSummary>,
    pub portions:    u32,
    pub macros:      MacrosSummary,
    pub score:       u8,
    pub flavor:      FlavorSummary,
    pub diet:        Vec<String>,
    pub suggestions: Vec<SuggestionItem>,
    pub ingredients: Vec<IngredientDetail>,
    pub flavor_contributions: Vec<FlavorContribution>,
    pub diagnosis: rule_engine::RuleDiagnosis,
}

/// Per-ingredient flavor influence: weighted raw values + percentages
#[derive(Debug, Serialize)]
pub struct FlavorContribution {
    pub slug:        String,
    pub sweetness:   f64,
    pub acidity:     f64,
    pub bitterness:  f64,
    pub umami:       f64,
    pub fat:         f64,
    pub aroma:       f64,
    /// Percentage contribution to each dimension (0–100)
    pub pct_sweetness:   f64,
    pub pct_acidity:     f64,
    pub pct_bitterness:  f64,
    pub pct_umami:       f64,
    pub pct_fat:         f64,
    pub pct_aroma:       f64,
}

#[derive(Debug, Serialize)]
pub struct NutritionSummary {
    pub calories: f64,
    pub protein:  f64,
    pub fat:      f64,
    pub carbs:    f64,
    pub fiber:    f64,
    pub sugar:    f64,
}

#[derive(Debug, Serialize)]
pub struct MacrosSummary {
    pub protein_pct: f64,
    pub fat_pct:     f64,
    pub carbs_pct:   f64,
}

#[derive(Debug, Serialize)]
pub struct FlavorSummary {
    pub sweetness:  f64,
    pub acidity:    f64,
    pub bitterness: f64,
    pub umami:      f64,
    pub fat:        f64,
    pub aroma:      f64,
    pub balance_score: u8,
    pub weak:       Vec<String>,
    pub strong:     Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct SuggestionItem {
    pub slug:      String,
    pub name:      String,
    pub name_en:   String,
    pub name_ru:   Option<String>,
    pub name_pl:   Option<String>,
    pub name_uk:   Option<String>,
    pub image_url: Option<String>,
    pub score:     u8,
    pub reasons:   Vec<String>,
    pub fills:     Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct IngredientDetail {
    pub slug:         String,
    pub name:         String,
    pub name_en:      String,
    pub name_ru:      Option<String>,
    pub name_pl:      Option<String>,
    pub name_uk:      Option<String>,
    pub image_url:    Option<String>,
    pub grams:        f64,
    pub calories:     f64,
    pub protein:      f64,
    pub fat:          f64,
    pub carbs:        f64,
    pub fiber:        f64,
    pub sugar:        f64,
    pub product_type: Option<String>,
    pub found:        bool,
}

// ── DB row types ─────────────────────────────────────────────────────────────

#[derive(Debug, sqlx::FromRow)]
struct IngredientRow {
    slug:          String,
    name_en:       String,
    name_ru:       Option<String>,
    name_pl:       Option<String>,
    name_uk:       Option<String>,
    image_url:     Option<String>,
    product_type:  Option<String>,
    calories_kcal: Option<f32>,
    protein_g:     Option<f32>,
    fat_g:         Option<f32>,
    carbs_g:       Option<f32>,
    fiber_g:       Option<f32>,
    sugar_g:       Option<f32>,
    sweetness:     Option<f32>,
    acidity:       Option<f32>,
    bitterness:    Option<f32>,
    umami:         Option<f32>,
    aroma:         Option<f32>,
    vegan:         Option<bool>,
    vegetarian:    Option<bool>,
    keto:          Option<bool>,
    paleo:         Option<bool>,
    gluten_free:   Option<bool>,
    mediterranean: Option<bool>,
    low_carb:      Option<bool>,
}

#[derive(Debug, sqlx::FromRow)]
struct CandidateRow {
    slug:          String,
    name_en:       String,
    name_ru:       Option<String>,
    name_pl:       Option<String>,
    name_uk:       Option<String>,
    image_url:     Option<String>,
    calories_kcal: Option<f32>,
    protein_g:     Option<f32>,
    fat_g:         Option<f32>,
    carbs_g:       Option<f32>,
    fiber_g:       Option<f32>,
    sugar_g:       Option<f32>,
    sweetness:     Option<f32>,
    acidity:       Option<f32>,
    bitterness:    Option<f32>,
    umami:         Option<f32>,
    aroma:         Option<f32>,
    avg_pair_score: Option<f64>,
}

// ── Lang helper ──────────────────────────────────────────────────────────────

fn pick_name(
    name_en: &str,
    name_ru: &Option<String>,
    name_pl: &Option<String>,
    name_uk: &Option<String>,
    lang: &str,
) -> String {
    match lang {
        "ru" => name_ru.as_deref().unwrap_or(name_en).to_string(),
        "pl" => name_pl.as_deref().unwrap_or(name_en).to_string(),
        "uk" => name_uk.as_deref().unwrap_or(name_en).to_string(),
        _    => name_en.to_string(),
    }
}

// ── Handler ──────────────────────────────────────────────────────────────────

pub async fn recipe_analyze(
    State(pool): State<PgPool>,
    Json(body): Json<RecipeAnalyzeRequest>,
) -> Result<Json<RecipeAnalyzeResponse>, (StatusCode, Json<serde_json::Value>)> {
    if body.ingredients.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "ingredients list is empty" })),
        ));
    }
    if body.ingredients.len() > 30 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "max 30 ingredients" })),
        ));
    }

    let slugs: Vec<String> = body.ingredients.iter().map(|i| i.slug.clone()).collect();
    let lang = body.lang.as_deref().unwrap_or("en");

    // ── 1. Fetch ingredient data from DB (nutrition + culinary + diet) ──
    let rows: Vec<IngredientRow> = sqlx::query_as(
        r#"
        SELECT p.slug, p.name_en, p.name_ru, p.name_pl, p.name_uk,
               COALESCE(p.image_url, ci.image_url) AS image_url,
               p.product_type,
               nm.calories_kcal, nm.protein_g, nm.fat_g, nm.carbs_g, nm.fiber_g, nm.sugar_g,
               fc.sweetness, fc.acidity, fc.bitterness, fc.umami, fc.aroma,
               df.vegan, df.vegetarian, df.keto, df.paleo,
               df.gluten_free, df.mediterranean, df.low_carb
        FROM products p
        LEFT JOIN catalog_ingredients ci ON ci.slug = p.slug
        LEFT JOIN nutrition_macros nm  ON nm.product_id = p.id
        LEFT JOIN food_culinary_properties fc ON fc.product_id = p.id
        LEFT JOIN diet_flags df        ON df.product_id = p.id
        WHERE p.slug = ANY($1)
        "#,
    )
    .bind(&slugs)
    .fetch_all(&pool)
    .await
    .map_err(|e| {
        tracing::error!("recipe_analyze DB error: {e}");
        (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": "Database error" })))
    })?;

    // Build lookup
    let find_row = |slug: &str| -> Option<&IngredientRow> {
        rows.iter().find(|r| r.slug == slug)
    };

    // ── 2. Build domain inputs ──
    let mut domain_inputs: Vec<RecipeIngredientInput> = Vec::new();
    let mut ingredient_details: Vec<IngredientDetail> = Vec::new();

    for inp in &body.ingredients {
        let row = find_row(&inp.slug);
        let found = row.is_some();

        let cal   = row.and_then(|r| r.calories_kcal).map(|v| v as f64).unwrap_or(0.0);
        let prot  = row.and_then(|r| r.protein_g).map(|v| v as f64).unwrap_or(0.0);
        let fat   = row.and_then(|r| r.fat_g).map(|v| v as f64).unwrap_or(0.0);
        let carbs = row.and_then(|r| r.carbs_g).map(|v| v as f64).unwrap_or(0.0);
        let fiber = row.and_then(|r| r.fiber_g).map(|v| v as f64).unwrap_or(0.0);
        let sugar = row.and_then(|r| r.sugar_g).map(|v| v as f64).unwrap_or(0.0);

        let factor = inp.grams / 100.0;

        ingredient_details.push(IngredientDetail {
            slug: inp.slug.clone(),
            name: row.map(|r| pick_name(&r.name_en, &r.name_ru, &r.name_pl, &r.name_uk, lang))
                     .unwrap_or_else(|| inp.slug.clone()),
            name_en: row.map(|r| r.name_en.clone()).unwrap_or_else(|| inp.slug.clone()),
            name_ru: row.and_then(|r| r.name_ru.clone()),
            name_pl: row.and_then(|r| r.name_pl.clone()),
            name_uk: row.and_then(|r| r.name_uk.clone()),
            image_url: row.and_then(|r| r.image_url.clone()),
            grams: inp.grams,
            calories: uc::round_to(cal * factor, 1),
            protein: uc::round_to(prot * factor, 1),
            fat: uc::round_to(fat * factor, 1),
            carbs: uc::round_to(carbs * factor, 1),
            fiber: uc::round_to(fiber * factor, 1),
            sugar: uc::round_to(sugar * factor, 1),
            product_type: row.and_then(|r| r.product_type.clone()),
            found,
        });

        let nutrition_100g = NutritionBreakdown {
            calories: cal,
            protein_g: prot,
            fat_g: fat,
            carbs_g: carbs,
            fiber_g: fiber,
            sugar_g: sugar,
            salt_g: 0.0,
            sodium_mg: 0.0,
        };

        let flavor = row.map(|r| {
            flavor_graph::flavor_from_culinary(
                r.sweetness.map(|v| v as f64).unwrap_or(0.0),
                r.acidity.map(|v| v as f64).unwrap_or(0.0),
                r.bitterness.map(|v| v as f64).unwrap_or(0.0),
                r.umami.map(|v| v as f64).unwrap_or(0.0),
                r.aroma.map(|v| v as f64).unwrap_or(0.0),
                fat,
            )
        }).unwrap_or(FlavorVector::zero());

        let diet_flags = row.map(|r| DietFlags {
            vegan: r.vegan.unwrap_or(false),
            vegetarian: r.vegetarian.unwrap_or(false),
            keto: r.keto.unwrap_or(false),
            paleo: r.paleo.unwrap_or(false),
            gluten_free: r.gluten_free.unwrap_or(false),
            mediterranean: r.mediterranean.unwrap_or(false),
            low_carb: r.low_carb.unwrap_or(false),
        }).unwrap_or_default();

        domain_inputs.push(RecipeIngredientInput {
            slug: inp.slug.clone(),
            grams: inp.grams,
            nutrition_100g,
            flavor,
            cost_per_kg: None,
            diet_flags,
        });
    }

    // ── 3. Run recipe analyzer ──
    let analysis = recipe_analyzer::analyze_recipe(&domain_inputs, body.portions);

    // ── 4. Fetch suggestion candidates (top pairings for recipe ingredients) ──
    let candidates_rows: Vec<CandidateRow> = sqlx::query_as(
        r#"
        WITH recipe_ids AS (
            SELECT id FROM products WHERE slug = ANY($1)
        ),
        pair_scores AS (
            SELECT p.slug, p.name_en, p.name_ru, p.name_pl, p.name_uk,
                   COALESCE(p.image_url, ci.image_url) AS image_url,
                   nm.calories_kcal, nm.protein_g, nm.fat_g, nm.carbs_g, nm.fiber_g, nm.sugar_g,
                   fc.sweetness, fc.acidity, fc.bitterness, fc.umami, fc.aroma,
                   AVG(fp.pair_score::float8) AS avg_pair_score
            FROM food_pairing fp
            JOIN products p ON p.id = fp.ingredient_b
            LEFT JOIN catalog_ingredients ci ON ci.slug = p.slug
            LEFT JOIN nutrition_macros nm ON nm.product_id = p.id
            LEFT JOIN food_culinary_properties fc ON fc.product_id = p.id
            WHERE fp.ingredient_a IN (SELECT id FROM recipe_ids)
              AND p.slug != ALL($1)
            GROUP BY p.slug, p.name_en, p.name_ru, p.name_pl, p.name_uk,
                     COALESCE(p.image_url, ci.image_url),
                     nm.calories_kcal, nm.protein_g, nm.fat_g, nm.carbs_g, nm.fiber_g, nm.sugar_g,
                     fc.sweetness, fc.acidity, fc.bitterness, fc.umami, fc.aroma
            ORDER BY avg_pair_score DESC NULLS LAST
            LIMIT 20
        )
        SELECT * FROM pair_scores
        "#,
    )
    .bind(&slugs)
    .fetch_all(&pool)
    .await
    .unwrap_or_default();

    let candidates: Vec<Candidate> = candidates_rows.iter().map(|r| {
        let fat_val = r.fat_g.map(|v| v as f64).unwrap_or(0.0);
        Candidate {
            slug: r.slug.clone(),
            name: r.name_en.clone(),
            image_url: r.image_url.clone(),
            flavor: flavor_graph::flavor_from_culinary(
                r.sweetness.map(|v| v as f64).unwrap_or(0.0),
                r.acidity.map(|v| v as f64).unwrap_or(0.0),
                r.bitterness.map(|v| v as f64).unwrap_or(0.0),
                r.umami.map(|v| v as f64).unwrap_or(0.0),
                r.aroma.map(|v| v as f64).unwrap_or(0.0),
                fat_val,
            ),
            nutrition: NutritionBreakdown {
                calories: r.calories_kcal.map(|v| v as f64).unwrap_or(0.0),
                protein_g: r.protein_g.map(|v| v as f64).unwrap_or(0.0),
                fat_g: fat_val,
                carbs_g: r.carbs_g.map(|v| v as f64).unwrap_or(0.0),
                fiber_g: r.fiber_g.map(|v| v as f64).unwrap_or(0.0),
                sugar_g: r.sugar_g.map(|v| v as f64).unwrap_or(0.0),
                salt_g: 0.0,
                sodium_mg: 0.0,
            },
            pair_score: r.avg_pair_score.unwrap_or(0.0),
            typical_g: 30.0, // default suggestion amount
            product_type: None, // not loaded in this context; slug-based rules still apply
        }
    }).collect();

    let total_grams: f64 = domain_inputs.iter().map(|i| i.grams).sum();

    // ── 5. Run Rule Engine diagnosis (BEFORE suggestions — so we can pass issues) ──
    let rule_ctx = rule_engine::RecipeContext {
        flavor: analysis.flavor.vector.clone(),
        balance_score: analysis.flavor.balance_score,
        total_calories: analysis.total_nutrition.calories,
        protein_pct: analysis.macros.protein_pct,
        fat_pct: analysis.macros.fat_pct,
        carbs_pct: analysis.macros.carbs_pct,
        fiber_g: analysis.total_nutrition.fiber_g,
        sugar_g: analysis.total_nutrition.sugar_g,
        total_grams,
        ingredients: body.ingredients.iter().map(|inp| {
            let pt = find_row(&inp.slug).and_then(|r| r.product_type.clone());
            (inp.slug.clone(), inp.grams, pt)
        }).collect(),
        nutrition_score: analysis.nutrition_score,
    };
    let diagnosis = rule_engine::diagnose(&rule_ctx);

    // ── 6. Run suggestion engine (with rule issues for global health scoring) ──
    let suggestion_result = suggestion_engine::suggest_ingredients(
        &analysis.flavor,
        &candidates,
        &slugs,
        5,
        &diagnosis.issues,
    );

    let suggestions: Vec<SuggestionItem> = suggestion_result.suggestions.iter().map(|s| {
        // Look up localized names from the candidates DB rows
        let cand = candidates_rows.iter().find(|c| c.slug == s.slug);
        SuggestionItem {
            slug: s.slug.clone(),
            name: cand.map(|c| pick_name(&c.name_en, &c.name_ru, &c.name_pl, &c.name_uk, lang))
                      .unwrap_or_else(|| s.name.clone()),
            name_en: cand.map(|c| c.name_en.clone()).unwrap_or_else(|| s.name.clone()),
            name_ru: cand.and_then(|c| c.name_ru.clone()),
            name_pl: cand.and_then(|c| c.name_pl.clone()),
            name_uk: cand.and_then(|c| c.name_uk.clone()),
            image_url: s.image_url.clone(),
            score: s.score,
            reasons: s.reasons.clone(),
            fills: s.fills_gaps.clone(),
        }
    }).collect();

    // ── 7. Compute flavor influence map ──

    // Weighted absolute values: flavor_dimension * (grams / total_grams)
    struct WeightedFlavor { slug: String, s: f64, a: f64, b: f64, u: f64, f: f64, ar: f64 }
    let weighted: Vec<WeightedFlavor> = domain_inputs.iter().map(|i| {
        let w = if total_grams > 0.0 { i.grams / total_grams } else { 0.0 };
        WeightedFlavor {
            slug: i.slug.clone(),
            s:  i.flavor.sweetness  * w,
            a:  i.flavor.acidity    * w,
            b:  i.flavor.bitterness * w,
            u:  i.flavor.umami      * w,
            f:  i.flavor.fat        * w,
            ar: i.flavor.aroma      * w,
        }
    }).collect();

    // Totals per dimension for percentage calc
    let ts: f64 = weighted.iter().map(|w| w.s).sum();
    let ta: f64 = weighted.iter().map(|w| w.a).sum();
    let tb: f64 = weighted.iter().map(|w| w.b).sum();
    let tu: f64 = weighted.iter().map(|w| w.u).sum();
    let tf: f64 = weighted.iter().map(|w| w.f).sum();
    let tar: f64 = weighted.iter().map(|w| w.ar).sum();

    let pct = |val: f64, total: f64| -> f64 {
        if total > 0.0 { uc::round_to(val / total * 100.0, 1) } else { 0.0 }
    };

    let flavor_contributions: Vec<FlavorContribution> = weighted.iter().map(|w| {
        FlavorContribution {
            slug: w.slug.clone(),
            sweetness:   uc::round_to(w.s,  2),
            acidity:     uc::round_to(w.a,  2),
            bitterness:  uc::round_to(w.b,  2),
            umami:       uc::round_to(w.u,  2),
            fat:         uc::round_to(w.f,  2),
            aroma:       uc::round_to(w.ar, 2),
            pct_sweetness:   pct(w.s,  ts),
            pct_acidity:     pct(w.a,  ta),
            pct_bitterness:  pct(w.b,  tb),
            pct_umami:       pct(w.u,  tu),
            pct_fat:         pct(w.f,  tf),
            pct_aroma:       pct(w.ar, tar),
        }
    }).collect();

    // ── 8. Build response ──
    let fv = &analysis.flavor.vector;

    let response = RecipeAnalyzeResponse {
        nutrition: NutritionSummary {
            calories: analysis.total_nutrition.calories,
            protein: analysis.total_nutrition.protein_g,
            fat: analysis.total_nutrition.fat_g,
            carbs: analysis.total_nutrition.carbs_g,
            fiber: analysis.total_nutrition.fiber_g,
            sugar: analysis.total_nutrition.sugar_g,
        },
        per_portion: if body.portions > 1 {
            Some(NutritionSummary {
                calories: analysis.per_portion.calories,
                protein: analysis.per_portion.protein_g,
                fat: analysis.per_portion.fat_g,
                carbs: analysis.per_portion.carbs_g,
                fiber: analysis.per_portion.fiber_g,
                sugar: analysis.per_portion.sugar_g,
            })
        } else {
            None
        },
        portions: body.portions,
        macros: MacrosSummary {
            protein_pct: analysis.macros.protein_pct,
            fat_pct: analysis.macros.fat_pct,
            carbs_pct: analysis.macros.carbs_pct,
        },
        score: analysis.nutrition_score,
        flavor: FlavorSummary {
            sweetness: fv.sweetness,
            acidity: fv.acidity,
            bitterness: fv.bitterness,
            umami: fv.umami,
            fat: fv.fat,
            aroma: fv.aroma,
            balance_score: analysis.flavor.balance_score,
            weak: analysis.flavor.weak_dimensions.iter().map(|d| d.dimension.clone()).collect(),
            strong: analysis.flavor.strong_dimensions.iter().map(|d| d.dimension.clone()).collect(),
        },
        diet: analysis.diet_flags.active_labels().into_iter().map(|s| s.to_string()).collect(),
        suggestions,
        ingredients: ingredient_details,
        flavor_contributions,
        diagnosis,
    };

    Ok(Json(response))
}

//! Pipeline v2 — orchestrates DB queries + domain engines into SmartResponse.
//!
//! v2 improvements:
//! 1. State-aware: state affects nutrition, flavor, pairings, suggestions, explain
//! 2. Equivalents: unit equivalents from density
//! 3. Processing state explain: detailed before/after comparison
//!
//! Data flow:
//! 1. Lookup main ingredient in catalog_ingredients (+ culinary props, pairings, seasonality)
//! 2. Load state data from ingredient_states (if state requested)
//! 3. Build FlavorVector from culinary properties, adjusted by state
//! 4. Compute FlavorBalance (aggregate with additional ingredients)
//! 5. Run SuggestionEngine on pairing candidates
//! 6. Run RuleEngine diagnostics (if additional ingredients present)
//! 7. Compute unit equivalents from density
//! 8. Build deterministic explanation text (with state-change details)
//! 9. Compose SmartResponse

use sqlx::PgPool;

use crate::shared::{AppError, AppResult, Language};
use crate::domain::tools::catalog_row::{CatalogNutritionRow, CATALOG_NUTRITION_COLS};
use crate::domain::tools::flavor_graph::{
    self, FlavorVector, FlavorIngredient, FlavorBalance,
};
use crate::domain::tools::nutrition::{
    self, breakdown_per_100g_nullable, NutritionBreakdown,
};
use crate::domain::tools::suggestion_engine::{self, Candidate};
use crate::domain::tools::rule_engine::{self, RecipeContext};
use crate::domain::tools::unit_converter as uc;

use super::context::CulinaryContext;
use super::response::*;

// ── DB row types ─────────────────────────────────────────────────────────────

#[derive(sqlx::FromRow)]
struct CulinaryRow {
    sweetness:  Option<f32>,
    acidity:    Option<f32>,
    bitterness: Option<f32>,
    umami:      Option<f32>,
    aroma:      Option<f32>,
}

#[derive(sqlx::FromRow)]
struct PairingRow {
    slug:            Option<String>,
    name_en:         String,
    name_ru:         String,
    name_pl:         String,
    name_uk:         String,
    image_url:       Option<String>,
    pair_score:      Option<f32>,
    flavor_score:    Option<f32>,
    nutrition_score: Option<f32>,
}

#[derive(sqlx::FromRow)]
struct SeasonRow {
    month:  i32,
    status: String,
    note:   Option<String>,
}

#[derive(sqlx::FromRow)]
struct StateRow {
    state:                String,
    description:          Option<String>,
    calories_per_100g:    Option<f64>,
    protein_per_100g:     Option<f64>,
    fat_per_100g:         Option<f64>,
    carbs_per_100g:       Option<f64>,
    fiber_per_100g:       Option<f64>,
    water_percent:        Option<f64>,
    shelf_life_hours:     Option<i32>,
    storage_temp_c:       Option<i32>,
    texture:              Option<String>,
    weight_change_percent: Option<f64>,
    oil_absorption_g:     Option<f64>,
    water_loss_percent:   Option<f64>,
    glycemic_index:       Option<i16>,
}

// ── Pipeline entry point ─────────────────────────────────────────────────────

pub async fn execute(pool: &PgPool, ctx: &CulinaryContext) -> AppResult<SmartResponse> {
    let lang = Language::from_code(&ctx.lang).unwrap_or(Language::En);
    let start = std::time::Instant::now();

    // ── 1. Main ingredient from catalog ──────────────────────────────────────
    let sql = format!(
        "SELECT {} FROM catalog_ingredients WHERE slug = $1 AND COALESCE(is_active, true) = true",
        CATALOG_NUTRITION_COLS
    );
    let row: CatalogNutritionRow = sqlx::query_as(&sql)
        .bind(&ctx.ingredient)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::not_found(format!("ingredient '{}' not found", ctx.ingredient)))?;

    let ingredient_info = IngredientInfo {
        slug: row.slug.clone().unwrap_or_default(),
        name: row.localized_name(lang).to_string(),
        image_url: row.image_url.clone(),
        product_type: row.product_type.clone(),
        sushi_grade: row.sushi_grade,
    };

    // ── 2. Nutrition per 100g ────────────────────────────────────────────────
    let nutrition = breakdown_per_100g_nullable(
        row.cal_opt(), row.prot_opt(), row.fat_opt(), row.carbs_opt(),
        row.fiber_opt(), row.sugar_opt(), row.salt_opt(),
    );

    // ── 3. Vitamins (static USDA lookup) ─────────────────────────────────────
    let vitamins = nutrition::vitamins_for(&ctx.ingredient);

    // ── 4. Culinary flavor vector ────────────────────────────────────────────
    // Need product_id for JOINs. Get it from a quick lookup.
    let product_id: Option<uuid::Uuid> = sqlx::query_scalar(
        "SELECT p.id FROM products p JOIN catalog_ingredients ci ON ci.slug = p.slug WHERE ci.slug = $1 LIMIT 1"
    )
    .bind(&ctx.ingredient)
    .fetch_optional(pool)
    .await?;

    let culinary: Option<CulinaryRow> = if let Some(pid) = product_id {
        sqlx::query_as(
            "SELECT sweetness, acidity, bitterness, umami, aroma FROM food_culinary_properties WHERE product_id = $1"
        )
        .bind(pid)
        .fetch_optional(pool)
        .await?
    } else {
        None
    };

    let main_flavor = culinary
        .as_ref()
        .map(|c| {
            flavor_graph::flavor_from_culinary(
                c.sweetness.unwrap_or(0.0) as f64,
                c.acidity.unwrap_or(0.0) as f64,
                c.bitterness.unwrap_or(0.0) as f64,
                c.umami.unwrap_or(0.0) as f64,
                c.aroma.unwrap_or(0.0) as f64,
                row.fat(),
            )
        })
        .unwrap_or_else(FlavorVector::zero);

    // ── 5. Build flavor ingredients list (main + additional) ─────────────────
    let typical_g = row.typical_g().unwrap_or(100.0);
    let mut flavor_ingredients = vec![FlavorIngredient {
        slug: ctx.ingredient.clone(),
        grams: typical_g,
        flavor: main_flavor.clone(),
    }];

    // Load additional ingredients' flavor vectors
    let mut additional_rows: Vec<(CatalogNutritionRow, FlavorVector)> = Vec::new();
    for extra_slug in &ctx.additional_ingredients {
        let extra_sql = format!(
            "SELECT {} FROM catalog_ingredients WHERE slug = $1 AND COALESCE(is_active, true) = true",
            CATALOG_NUTRITION_COLS
        );
        if let Some(extra_row) = sqlx::query_as::<_, CatalogNutritionRow>(&extra_sql)
            .bind(extra_slug)
            .fetch_optional(pool)
            .await?
        {
            // Try to get culinary props for this extra ingredient
            let extra_pid: Option<uuid::Uuid> = sqlx::query_scalar(
                "SELECT p.id FROM products p WHERE p.slug = $1 LIMIT 1"
            )
            .bind(extra_slug)
            .fetch_optional(pool)
            .await?;

            let extra_culinary: Option<CulinaryRow> = if let Some(epid) = extra_pid {
                sqlx::query_as(
                    "SELECT sweetness, acidity, bitterness, umami, aroma FROM food_culinary_properties WHERE product_id = $1"
                )
                .bind(epid)
                .fetch_optional(pool)
                .await?
            } else {
                None
            };

            let extra_flavor = extra_culinary
                .as_ref()
                .map(|c| {
                    flavor_graph::flavor_from_culinary(
                        c.sweetness.unwrap_or(0.0) as f64,
                        c.acidity.unwrap_or(0.0) as f64,
                        c.bitterness.unwrap_or(0.0) as f64,
                        c.umami.unwrap_or(0.0) as f64,
                        c.aroma.unwrap_or(0.0) as f64,
                        extra_row.fat(),
                    )
                })
                .unwrap_or_else(FlavorVector::zero);

            let extra_typical = extra_row.typical_g().unwrap_or(50.0);
            flavor_ingredients.push(FlavorIngredient {
                slug: extra_slug.clone(),
                grams: extra_typical,
                flavor: extra_flavor.clone(),
            });

            additional_rows.push((extra_row, extra_flavor));
        }
    }

    // ── 6. FlavorBalance ─────────────────────────────────────────────────────
    let balance = flavor_graph::analyze_balance(&flavor_ingredients);

    let flavor_profile = FlavorProfileInfo {
        vector: main_flavor.clone(),
        balance: balance.clone(),
    };

    // ── 7. Pairings from DB ─────────────────────────────────────────────────
    let pairings_raw: Vec<PairingRow> = if let Some(pid) = product_id {
        sqlx::query_as(
            r#"SELECT b.slug, b.name_en, b.name_ru, b.name_pl, b.name_uk, b.image_url,
                      fp.pair_score, fp.flavor_score, fp.nutrition_score
               FROM food_pairing fp
               JOIN products b ON b.id = fp.ingredient_b
               WHERE fp.ingredient_a = $1
               ORDER BY fp.pair_score DESC NULLS LAST
               LIMIT 8"#,
        )
        .bind(pid)
        .fetch_all(pool)
        .await
        .unwrap_or_default()
    } else {
        vec![]
    };

    let pairings: Vec<PairingInfo> = pairings_raw
        .iter()
        .map(|p| {
            let name = match lang {
                Language::Ru => &p.name_ru,
                Language::Pl => &p.name_pl,
                Language::Uk => &p.name_uk,
                Language::En => &p.name_en,
            };
            PairingInfo {
                slug: p.slug.clone().unwrap_or_default(),
                name: name.clone(),
                image_url: p.image_url.clone(),
                pair_score: p.pair_score.unwrap_or(0.0) as f64,
                flavor_score: p.flavor_score.map(|v| v as f64),
                nutrition_score: p.nutrition_score.map(|v| v as f64),
            }
        })
        .collect();

    // ── 8. Suggestions (SuggestionEngine) ────────────────────────────────────
    // Build candidates from pairings + any additional catalog items
    let existing_slugs: Vec<String> = flavor_ingredients.iter().map(|fi| fi.slug.clone()).collect();

    let candidates: Vec<Candidate> = pairings_raw
        .iter()
        .filter(|p| {
            let s = p.slug.as_deref().unwrap_or("");
            !existing_slugs.contains(&s.to_string())
        })
        .map(|p| {
            Candidate {
                slug: p.slug.clone().unwrap_or_default(),
                name: match lang {
                    Language::Ru => p.name_ru.clone(),
                    Language::Pl => p.name_pl.clone(),
                    Language::Uk => p.name_uk.clone(),
                    Language::En => p.name_en.clone(),
                },
                image_url: p.image_url.clone(),
                flavor: FlavorVector::zero(), // pairing doesn't carry flavor — use pair_score
                nutrition: NutritionBreakdown {
                    calories: 0.0, protein_g: 0.0, fat_g: 0.0, carbs_g: 0.0,
                    fiber_g: 0.0, sugar_g: 0.0, salt_g: 0.0, sodium_mg: 0.0,
                },
                pair_score: p.pair_score.unwrap_or(0.0) as f64,
                typical_g: 50.0,
            }
        })
        .collect();

    // Suggestions will be computed after potential state adjustment (step 11c)

    // ── 9. Diagnostics (RuleEngine — only if we have 2+ ingredients) ─────────
    let diagnostics = if !ctx.additional_ingredients.is_empty() {
        // Build RecipeContext from all ingredients
        let total_grams: f64 = flavor_ingredients.iter().map(|fi| fi.grams).sum();
        let total_cal: f64 = {
            let main_cal = row.cal() * (typical_g / 100.0);
            let extra_cal: f64 = additional_rows.iter()
                .map(|(r, _)| r.cal() * (r.typical_g().unwrap_or(50.0) / 100.0))
                .sum();
            main_cal + extra_cal
        };
        let total_prot: f64 = {
            let main = row.prot() * (typical_g / 100.0);
            let extra: f64 = additional_rows.iter()
                .map(|(r, _)| r.prot() * (r.typical_g().unwrap_or(50.0) / 100.0))
                .sum();
            main + extra
        };
        let total_fat: f64 = {
            let main = row.fat() * (typical_g / 100.0);
            let extra: f64 = additional_rows.iter()
                .map(|(r, _)| r.fat() * (r.typical_g().unwrap_or(50.0) / 100.0))
                .sum();
            main + extra
        };
        let total_carbs: f64 = {
            let main = row.carbs() * (typical_g / 100.0);
            let extra: f64 = additional_rows.iter()
                .map(|(r, _)| r.carbs() * (r.typical_g().unwrap_or(50.0) / 100.0))
                .sum();
            main + extra
        };
        let total_fiber: f64 = {
            let main = row.fiber() * (typical_g / 100.0);
            let extra: f64 = additional_rows.iter()
                .map(|(r, _)| r.fiber() * (r.typical_g().unwrap_or(50.0) / 100.0))
                .sum();
            main + extra
        };
        let total_sugar: f64 = {
            let main = row.sugar() * (typical_g / 100.0);
            let extra: f64 = additional_rows.iter()
                .map(|(r, _)| r.sugar() * (r.typical_g().unwrap_or(50.0) / 100.0))
                .sum();
            main + extra
        };

        let macros = nutrition::macros_ratio(total_prot, total_fat, total_carbs);

        // Build ingredients list for RuleEngine
        let mut rule_ingredients: Vec<(String, f64, Option<String>)> = vec![(
            ctx.ingredient.clone(),
            typical_g,
            row.product_type.clone(),
        )];
        for (i, extra_slug) in ctx.additional_ingredients.iter().enumerate() {
            if let Some((r, _)) = additional_rows.get(i) {
                rule_ingredients.push((
                    extra_slug.clone(),
                    r.typical_g().unwrap_or(50.0),
                    r.product_type.clone(),
                ));
            }
        }

        let recipe_ctx = RecipeContext {
            flavor: balance.vector.clone(),
            balance_score: balance.balance_score,
            total_calories: total_cal,
            protein_pct: macros.protein_pct,
            fat_pct: macros.fat_pct,
            carbs_pct: macros.carbs_pct,
            fiber_g: total_fiber,
            sugar_g: total_sugar,
            total_grams,
            ingredients: rule_ingredients,
        };

        let diag = rule_engine::diagnose(&recipe_ctx);

        Some(DiagnosticsInfo {
            health_score: diag.health_score,
            category_scores: {
                let mut m = std::collections::HashMap::new();
                m.insert("flavor".to_string(), diag.category_scores.flavor);
                m.insert("nutrition".to_string(), diag.category_scores.nutrition);
                m.insert("dominance".to_string(), diag.category_scores.dominance);
                m.insert("structure".to_string(), diag.category_scores.structure);
                m
            },
            issues: diag.issues.iter()
                .map(|i| DiagnosticIssue {
                    severity: i.severity.clone(),
                    code: i.rule.clone(),
                    message: i.description_key.clone(),
                })
                .collect(),
        })
    } else {
        None
    };

    // ── 10. Seasonality ──────────────────────────────────────────────────────
    let seasonality_rows: Vec<SeasonRow> = if let Some(_pid) = product_id {
        sqlx::query_as(
            "SELECT month, status, note FROM catalog_product_seasonality WHERE product_id = (SELECT id FROM catalog_ingredients WHERE slug = $1 LIMIT 1) ORDER BY month"
        )
        .bind(&ctx.ingredient)
        .fetch_all(pool)
        .await
        .unwrap_or_default()
    } else {
        vec![]
    };

    let seasonality: Vec<SeasonalityInfo> = seasonality_rows
        .into_iter()
        .map(|s| SeasonalityInfo { month: s.month, status: s.status, note: s.note })
        .collect();

    // ── 11. State info (v2: full row from ingredient_states) ───────────────
    let full_state: Option<StateRow> = if let Some(ref state_name) = ctx.state {
        sqlx::query_as(
            r#"SELECT
                state::text as state,
                COALESCE(notes_en, notes) as description,
                calories_per_100g::float8 as calories_per_100g,
                protein_per_100g::float8 as protein_per_100g,
                fat_per_100g::float8 as fat_per_100g,
                carbs_per_100g::float8 as carbs_per_100g,
                fiber_per_100g::float8 as fiber_per_100g,
                water_percent::float8 as water_percent,
                shelf_life_hours, storage_temp_c, texture,
                weight_change_percent::float8 as weight_change_percent,
                oil_absorption_g::float8 as oil_absorption_g,
                water_loss_percent::float8 as water_loss_percent,
                glycemic_index
            FROM ingredient_states
            WHERE ingredient_id = (SELECT id FROM catalog_ingredients WHERE slug = $1 LIMIT 1)
              AND state = $2::processing_state"#,
        )
        .bind(&ctx.ingredient)
        .bind(state_name)
        .fetch_optional(pool)
        .await
        .unwrap_or(None)
    } else {
        None
    };

    // v2: Build StateInfo with full cooking details
    let state_info: Option<StateInfo> = full_state.as_ref().map(|r| StateInfo {
        state: r.state.clone(),
        description: r.description.clone(),
        nutrition: if r.calories_per_100g.is_some() || r.protein_per_100g.is_some() {
            Some(StateNutrition {
                calories: r.calories_per_100g,
                protein_g: r.protein_per_100g,
                fat_g: r.fat_per_100g,
                carbs_g: r.carbs_per_100g,
                fiber_g: r.fiber_per_100g,
                water_percent: r.water_percent,
            })
        } else {
            None
        },
        texture: r.texture.clone(),
        weight_change_percent: r.weight_change_percent,
        oil_absorption_g: r.oil_absorption_g,
        water_loss_percent: r.water_loss_percent,
        glycemic_index: r.glycemic_index,
        shelf_life_hours: r.shelf_life_hours,
        storage_temp_c: r.storage_temp_c,
    });

    // ── 11a. v2: Override nutrition with state nutrition if available ─────────
    let nutrition = if let Some(ref st) = full_state {
        if st.calories_per_100g.is_some() || st.protein_per_100g.is_some() {
            // Use state-specific nutrition instead of raw
            breakdown_per_100g_nullable(
                st.calories_per_100g,
                st.protein_per_100g,
                st.fat_per_100g,
                st.carbs_per_100g,
                st.fiber_per_100g,
                None, // sugar not tracked per state
                None, // salt not tracked per state
            )
        } else {
            nutrition
        }
    } else {
        nutrition
    };

    // ── 11b. v2: Adjust flavor vector based on state ─────────────────────────
    let (main_flavor, flavor_profile) = if let Some(ref st) = full_state {
        let adjusted = adjust_flavor_for_state(&main_flavor, &st.state, st.oil_absorption_g, st.water_loss_percent);
        // Rebuild flavor_ingredients with adjusted main flavor
        let mut adj_ingredients = vec![FlavorIngredient {
            slug: ctx.ingredient.clone(),
            grams: typical_g,
            flavor: adjusted.clone(),
        }];
        for (i, extra_slug) in ctx.additional_ingredients.iter().enumerate() {
            if let Some((_, extra_fv)) = additional_rows.get(i) {
                let extra_typical = additional_rows[i].0.typical_g().unwrap_or(50.0);
                adj_ingredients.push(FlavorIngredient {
                    slug: extra_slug.clone(),
                    grams: extra_typical,
                    flavor: extra_fv.clone(),
                });
            }
        }
        let adj_balance = flavor_graph::analyze_balance(&adj_ingredients);
        let fpi = FlavorProfileInfo {
            vector: adjusted.clone(),
            balance: adj_balance,
        };
        (adjusted, fpi)
    } else {
        (main_flavor, flavor_profile)
    };

    // Use the (possibly adjusted) balance for subsequent engines
    let balance = &flavor_profile.balance;

    // ── 11c. v2: Re-run suggestions with state-adjusted balance ──────────────
    let suggestion_result = suggestion_engine::suggest_ingredients(
        balance, &candidates, &existing_slugs, 5,
    );
    let suggestions: Vec<SuggestionInfo> = suggestion_result
        .suggestions
        .into_iter()
        .map(|s| SuggestionInfo {
            slug: s.slug,
            name: s.name,
            image_url: s.image_url,
            score: s.score,
            reasons: s.reasons,
            fills_gaps: s.fills_gaps,
            suggested_grams: s.suggested_grams,
        })
        .collect();

    // ── 12. Unit equivalents from density (v2) ──────────────────────────────
    let density = row.density();
    let equivalents = build_equivalents(100.0, density);

    // ── 13. Build explanation (v2: includes state-change details) ─────────────
    let explain = build_explain_v2(
        &ingredient_info, balance, &pairings, &suggestions,
        &diagnostics, &ctx.goal, &full_state, &row,
    );

    // ── 14. Compose response ─────────────────────────────────────────────────
    let timing_ms = start.elapsed().as_millis() as u64;

    Ok(SmartResponse {
        ingredient: ingredient_info,
        state: state_info,
        nutrition,
        vitamins,
        flavor_profile,
        pairings,
        suggestions,
        diagnostics,
        equivalents,
        seasonality,
        explain,
        meta: SmartMeta {
            timing_ms,
            cached: false,
            cache_key: ctx.cache_key(),
            engine_version: "2.0.0".to_string(),
        },
    })
}

// ── State → flavor adjustment (v2) ──────────────────────────────────────────

/// Deterministic flavor adjustment based on cooking method.
/// Each state shifts specific flavor dimensions.
fn adjust_flavor_for_state(
    base: &FlavorVector,
    state: &str,
    oil_absorption: Option<f64>,
    water_loss: Option<f64>,
) -> FlavorVector {
    let mut v = base.clone();

    match state {
        "grilled" => {
            v.umami      = (v.umami + 1.5).min(10.0);       // Maillard reaction
            v.aroma      = (v.aroma + 2.0).min(10.0);       // smoke + char
            v.sweetness  = (v.sweetness + 0.5).min(10.0);   // caramelization
            v.bitterness = (v.bitterness + 0.3).min(10.0);  // slight char
        }
        "fried" => {
            let oil_boost = oil_absorption.unwrap_or(5.0) / 10.0; // 0-1 scale
            v.fat        = (v.fat + 2.0 + oil_boost).min(10.0);
            v.umami      = (v.umami + 1.0).min(10.0);
            v.aroma      = (v.aroma + 1.5).min(10.0);
            v.sweetness  = (v.sweetness + 0.3).min(10.0);
        }
        "baked" => {
            v.sweetness  = (v.sweetness + 1.0).min(10.0);   // caramelization
            v.aroma      = (v.aroma + 1.5).min(10.0);       // toasting
            v.umami      = (v.umami + 0.5).min(10.0);
        }
        "boiled" | "steamed" => {
            // Cooking in water dilutes some flavors
            let loss = water_loss.unwrap_or(10.0) / 100.0;
            v.aroma      = (v.aroma * (1.0 - loss * 0.3)).max(0.0);
            v.sweetness  = (v.sweetness * (1.0 - loss * 0.2)).max(0.0);
            // But can concentrate umami
            if state == "boiled" {
                v.umami = (v.umami + 0.3).min(10.0);
            }
        }
        "smoked" => {
            v.aroma      = (v.aroma + 3.0).min(10.0);       // heavy smoke
            v.umami      = (v.umami + 1.0).min(10.0);       // concentration
            v.bitterness = (v.bitterness + 0.5).min(10.0);  // smoke phenols
        }
        "dried" => {
            // Concentration effect — everything intensifies except water-based notes
            v.sweetness  = (v.sweetness * 1.5).min(10.0);
            v.umami      = (v.umami * 1.5).min(10.0);
            v.aroma      = (v.aroma * 1.3).min(10.0);
        }
        "pickled" => {
            v.acidity    = (v.acidity + 3.0).min(10.0);     // vinegar/brine
            v.sweetness  = (v.sweetness * 0.7).max(0.0);    // suppressed
            v.aroma      = (v.aroma + 0.5).min(10.0);       // fermentation
        }
        "frozen" => {
            // Freezing dulls flavors slightly
            v.aroma      = (v.aroma * 0.85).max(0.0);
            v.sweetness  = (v.sweetness * 0.9).max(0.0);
        }
        // "raw" or unknown → no change
        _ => {}
    }

    v.round();
    v
}

// ── Unit equivalents builder (v2) ────────────────────────────────────────────

fn build_equivalents(grams: f64, density: f64) -> Vec<EquivalentInfo> {
    let targets: &[(&str, &str)] = &[
        ("kg", "kg"), ("oz", "oz"), ("lb", "lb"),
        ("ml", "ml"), ("l", "l"),
        ("tsp", "tsp"), ("tbsp", "tbsp"), ("cup", "cup"),
    ];

    targets
        .iter()
        .filter_map(|&(unit, label)| {
            uc::convert_with_density(grams, "g", unit, density).map(|v| EquivalentInfo {
                unit: unit.to_string(),
                label: label.to_string(),
                value: uc::display_round(v),
            })
        })
        .collect()
}

// ── Deterministic explanation builder v2 ─────────────────────────────────────

fn build_explain_v2(
    ingredient: &IngredientInfo,
    balance: &FlavorBalance,
    pairings: &[PairingInfo],
    suggestions: &[SuggestionInfo],
    diagnostics: &Option<DiagnosticsInfo>,
    goal: &Option<String>,
    full_state: &Option<StateRow>,
    raw_row: &CatalogNutritionRow,
) -> Vec<String> {
    let mut lines = Vec::new();

    // Ingredient overview
    lines.push(format!(
        "{} ({}) — flavor balance score: {}/100.",
        ingredient.name,
        ingredient.product_type.as_deref().unwrap_or("unknown"),
        balance.balance_score,
    ));

    // ── State-change explanation (v2 step 4) ────────────────────────────────
    if let Some(st) = full_state {
        if st.state != "raw" {
            lines.push(format!("Processing state: {}.", st.state));

            // Nutrition comparison (raw vs cooked)
            if let Some(cooked_cal) = st.calories_per_100g {
                let raw_cal = raw_row.cal();
                let diff = cooked_cal - raw_cal;
                if diff.abs() > 1.0 {
                    let direction = if diff > 0.0 { "+" } else { "" };
                    lines.push(format!(
                        "Calories: {:.0} → {:.0} kcal/100g ({}{:.0}).",
                        raw_cal, cooked_cal, direction, diff,
                    ));
                }
            }
            if let Some(cooked_fat) = st.fat_per_100g {
                let raw_fat = raw_row.fat();
                let diff = cooked_fat - raw_fat;
                if diff.abs() > 0.5 {
                    let direction = if diff > 0.0 { "+" } else { "" };
                    lines.push(format!(
                        "Fat: {:.1} → {:.1} g/100g ({}{:.1}).",
                        raw_fat, cooked_fat, direction, diff,
                    ));
                }
            }
            if let Some(cooked_prot) = st.protein_per_100g {
                let raw_prot = raw_row.prot();
                let diff = cooked_prot - raw_prot;
                if diff.abs() > 0.5 {
                    let direction = if diff > 0.0 { "+" } else { "" };
                    lines.push(format!(
                        "Protein: {:.1} → {:.1} g/100g ({}{:.1}).",
                        raw_prot, cooked_prot, direction, diff,
                    ));
                }
            }

            // Physical changes
            if let Some(wc) = st.weight_change_percent {
                if wc.abs() > 1.0 {
                    if wc < 0.0 {
                        lines.push(format!("Weight loss: {:.0}% (concentrates flavor).", wc.abs()));
                    } else {
                        lines.push(format!("Weight gain: +{:.0}% (absorbs liquid).", wc));
                    }
                }
            }
            if let Some(oil) = st.oil_absorption_g {
                if oil > 1.0 {
                    lines.push(format!("Oil absorption: {:.1}g per 100g raw product.", oil));
                }
            }
            if let Some(wl) = st.water_loss_percent {
                if wl > 2.0 {
                    lines.push(format!("Water loss: {:.0}%.", wl));
                }
            }

            // Texture
            if let Some(ref tex) = st.texture {
                lines.push(format!("Texture: {}.", tex));
            }

            // Storage
            if let Some(hours) = st.shelf_life_hours {
                let days = hours / 24;
                if days > 0 {
                    lines.push(format!("Shelf life: ~{} days.", days));
                } else {
                    lines.push(format!("Shelf life: ~{} hours.", hours));
                }
            }
        }
    }

    // Weak dimensions
    if !balance.weak_dimensions.is_empty() {
        let weak: Vec<&str> = balance.weak_dimensions.iter().map(|d| d.dimension.as_str()).collect();
        lines.push(format!("Weak flavor areas: {}.", weak.join(", ")));
    }

    // Strong dimensions
    if !balance.strong_dimensions.is_empty() {
        let strong: Vec<&str> = balance.strong_dimensions.iter().map(|d| d.dimension.as_str()).collect();
        lines.push(format!("Strong flavor areas: {}.", strong.join(", ")));
    }

    // Top pairing
    if let Some(top) = pairings.first() {
        lines.push(format!(
            "Best pairing: {} (score {:.1}).",
            top.name, top.pair_score,
        ));
    }

    // Top suggestion
    if let Some(top) = suggestions.first() {
        lines.push(format!(
            "Top suggestion: {} — {} (score {}/100).",
            top.name,
            top.reasons.first().map(|s| s.as_str()).unwrap_or("complementary"),
            top.score,
        ));
    }

    // Diagnostics summary
    if let Some(diag) = diagnostics {
        lines.push(format!("Recipe health score: {}/100.", diag.health_score));
        let critical = diag.issues.iter().filter(|i| i.severity == "critical").count();
        let warnings = diag.issues.iter().filter(|i| i.severity == "warning").count();
        if critical > 0 || warnings > 0 {
            lines.push(format!("{} critical, {} warnings found.", critical, warnings));
        }
    }

    // Goal hint
    if let Some(goal) = goal {
        match goal.as_str() {
            "high-protein" => lines.push("Goal: high-protein — prioritize protein-rich ingredients.".to_string()),
            "low-carb" => lines.push("Goal: low-carb — minimize carbohydrate sources.".to_string()),
            "flavor-boost" => lines.push("Goal: flavor-boost — focus on filling weak flavor dimensions.".to_string()),
            _ => lines.push(format!("Goal: {} — balanced approach.", goal)),
        }
    }

    lines
}

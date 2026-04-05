//! Pipeline v3 — orchestrates DB queries + domain engines into SmartResponse.
//!
//! v3 improvements:
//! 1. Goal Engine: typed Goal enum affects suggestion scoring, diagnostics priority, explain
//! 2. Feedback Loop: diagnostics issues → synthetic suggestion candidates
//! 3. Confidence System: data-completeness scores per section
//! 4. Next Actions: actionable [{type, ingredient, reason}] from issues + weak dims + goal
//! 5. Session: session_id for continuity (storage in SmartService)
//!
//! Data flow:
//! 1. Lookup main ingredient in catalog_ingredients (+ culinary props, pairings, seasonality)
//! 2. Load state data from ingredient_states (if state requested)
//! 3. Build FlavorVector from culinary properties, adjusted by state
//! 4. Compute FlavorBalance (aggregate with additional ingredients)
//! 5. Run RuleEngine diagnostics (if additional ingredients present)
//! 6. Feedback Loop: extract fix_slugs from diagnostics → inject candidates
//! 7. Run SuggestionEngine with goal-aware weight adjustments
//! 8. Compute confidence scores from data completeness
//! 9. Build next_actions from diagnostics + weak dimensions + goal
//! 10. Compute unit equivalents from density
//! 11. Build deterministic explanation text (goal-aware + feedback-loop)
//! 12. Compose SmartResponse

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

use super::context::{CulinaryContext, Goal};
use super::response::*;
use super::recipe_builder;

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
struct ExtraCulinaryRow {
    slug:       String,
    sweetness:  Option<f32>,
    acidity:    Option<f32>,
    bitterness: Option<f32>,
    umami:      Option<f32>,
    aroma:      Option<f32>,
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

    // ═══════════════════════════════════════════════════════════════════════
    //  PARALLEL PHASE 1: main_row + product_id  (2 queries in parallel)
    // ═══════════════════════════════════════════════════════════════════════
    let main_sql = format!(
        "SELECT {} FROM catalog_ingredients WHERE slug = $1 AND COALESCE(is_active, true) = true",
        CATALOG_NUTRITION_COLS
    );
    let pid_fut = sqlx::query_scalar::<_, uuid::Uuid>(
        "SELECT p.id FROM products p JOIN catalog_ingredients ci ON ci.slug = p.slug WHERE ci.slug = $1 LIMIT 1"
    )
    .bind(&ctx.ingredient)
    .fetch_optional(pool);

    let row_fut = sqlx::query_as::<_, CatalogNutritionRow>(&main_sql)
        .bind(&ctx.ingredient)
        .fetch_optional(pool);

    let (row_opt, product_id) = tokio::try_join!(row_fut, pid_fut)?;
    let row = row_opt.ok_or_else(|| AppError::not_found(format!("ingredient '{}' not found", ctx.ingredient)))?;

    let ingredient_info = IngredientInfo {
        slug: row.slug.clone().unwrap_or_default(),
        name: row.localized_name(lang).to_string(),
        image_url: row.image_url.clone(),
        product_type: row.product_type.clone(),
        sushi_grade: row.sushi_grade,
    };

    // Nutrition + vitamins (CPU-only, instant)
    let nutrition = breakdown_per_100g_nullable(
        row.cal_opt(), row.prot_opt(), row.fat_opt(), row.carbs_opt(),
        row.fiber_opt(), row.sugar_opt(), row.salt_opt(),
    );
    let vitamins = nutrition::vitamins_for(&ctx.ingredient);

    // ═══════════════════════════════════════════════════════════════════════
    //  PARALLEL PHASE 2: culinary + pairings + seasonality + state + batch_additional
    //  (up to 5 queries in parallel — single DB round-trip)
    // ═══════════════════════════════════════════════════════════════════════

    // 2a. Culinary flavor
    let culinary_fut = async {
        if let Some(pid) = product_id {
            sqlx::query_as::<_, CulinaryRow>(
                "SELECT sweetness, acidity, bitterness, umami, aroma FROM food_culinary_properties WHERE product_id = $1"
            )
            .bind(pid)
            .fetch_optional(pool)
            .await
        } else {
            Ok(None)
        }
    };

    // 2b. Pairings
    let pairings_fut = async {
        if let Some(pid) = product_id {
            sqlx::query_as::<_, PairingRow>(
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
        } else {
            Ok(vec![])
        }
    };

    // 2c. Seasonality
    let season_fut = async {
        sqlx::query_as::<_, SeasonRow>(
            "SELECT month, status, note FROM catalog_product_seasonality WHERE product_id = (SELECT id FROM catalog_ingredients WHERE slug = $1 LIMIT 1) ORDER BY month"
        )
        .bind(&ctx.ingredient)
        .fetch_all(pool)
        .await
        .unwrap_or_default()
    };

    // 2d. State info
    let state_fut = async {
        if let Some(ref state_name) = ctx.state {
            sqlx::query_as::<_, StateRow>(
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
        }
    };

    // 2e. Batch additional ingredients (single query with ANY instead of loop)
    let additional_slugs: Vec<String> = ctx.additional_ingredients.clone();
    let batch_additional_fut = async {
        if additional_slugs.is_empty() {
            return Ok::<Vec<(CatalogNutritionRow, FlavorVector)>, AppError>(vec![]);
        }
        // Batch load catalog rows
        let batch_sql = format!(
            "SELECT {} FROM catalog_ingredients WHERE slug = ANY($1) AND COALESCE(is_active, true) = true",
            CATALOG_NUTRITION_COLS
        );
        let extra_rows: Vec<CatalogNutritionRow> = sqlx::query_as(&batch_sql)
            .bind(&additional_slugs)
            .fetch_all(pool)
            .await?;

        // Batch load culinary properties (single JOIN query)
        let extra_culinary_rows: Vec<ExtraCulinaryRow> = if !extra_rows.is_empty() {
            let slugs: Vec<String> = extra_rows.iter().filter_map(|r| r.slug.clone()).collect();
            sqlx::query_as(
                r#"SELECT p.slug,
                          fcp.sweetness, fcp.acidity, fcp.bitterness, fcp.umami, fcp.aroma
                   FROM products p
                   JOIN food_culinary_properties fcp ON fcp.product_id = p.id
                   WHERE p.slug = ANY($1)"#,
            )
            .bind(&slugs)
            .fetch_all(pool)
            .await
            .unwrap_or_default()
        } else {
            vec![]
        };

        let mut result = Vec::new();
        for extra_row in extra_rows {
            let slug = extra_row.slug.clone().unwrap_or_default();
            let extra_flavor = extra_culinary_rows.iter()
                .find(|ecr| ecr.slug == slug)
                .map(|ecr| flavor_graph::flavor_from_culinary(
                    ecr.sweetness.unwrap_or(0.0) as f64,
                    ecr.acidity.unwrap_or(0.0) as f64,
                    ecr.bitterness.unwrap_or(0.0) as f64,
                    ecr.umami.unwrap_or(0.0) as f64,
                    ecr.aroma.unwrap_or(0.0) as f64,
                    extra_row.fat(),
                ))
                .unwrap_or_else(FlavorVector::zero);
            result.push((extra_row, extra_flavor));
        }
        Ok(result)
    };

    // Fire all 5 in parallel
    let (culinary, pairings_raw, seasonality_rows, full_state, additional_rows) = tokio::join!(
        culinary_fut,
        pairings_fut,
        season_fut,
        state_fut,
        batch_additional_fut,
    );
    let culinary = culinary?;
    let pairings_raw = pairings_raw?;
    let additional_rows = additional_rows?;
    // seasonality_rows and full_state are already unwrapped

    // ═══════════════════════════════════════════════════════════════════════
    //  CPU PHASE: all computation from here on — no more DB queries (except feedback)
    // ═══════════════════════════════════════════════════════════════════════

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

    // Build flavor ingredients list (main + additional)
    let typical_g = row.typical_g().unwrap_or(100.0);
    let mut flavor_ingredients = vec![FlavorIngredient {
        slug: ctx.ingredient.clone(),
        grams: typical_g,
        flavor: main_flavor.clone(),
    }];

    for (i, extra_slug) in ctx.additional_ingredients.iter().enumerate() {
        if let Some((extra_row, extra_flavor)) = additional_rows.get(i) {
            let extra_typical = extra_row.typical_g().unwrap_or(50.0);
            flavor_ingredients.push(FlavorIngredient {
                slug: extra_slug.clone(),
                grams: extra_typical,
                flavor: extra_flavor.clone(),
            });
        }
    }

    // FlavorBalance
    let balance = flavor_graph::analyze_balance(&flavor_ingredients);

    let flavor_profile = FlavorProfileInfo {
        vector: main_flavor.clone(),
        balance: balance.clone(),
    };

    // Pairings
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

    // Seasonality
    let seasonality: Vec<SeasonalityInfo> = seasonality_rows
        .into_iter()
        .map(|s| SeasonalityInfo { month: s.month, status: s.status, note: s.note })
        .collect();

    // Candidates from pairings — hydrated with real nutrition + flavor + product_type
    let existing_slugs: Vec<String> = flavor_ingredients.iter().map(|fi| fi.slug.clone()).collect();

    let pairing_slugs: Vec<String> = pairings_raw
        .iter()
        .filter_map(|p| p.slug.clone())
        .filter(|s| !existing_slugs.contains(s))
        .collect();

    // BATCH: load catalog rows for all pairing slugs (nutrition + product_type)
    let pairing_catalog_rows: Vec<CatalogNutritionRow> = if !pairing_slugs.is_empty() {
        let sql = format!(
            "SELECT {} FROM catalog_ingredients WHERE slug = ANY($1) AND COALESCE(is_active, true) = true",
            CATALOG_NUTRITION_COLS
        );
        sqlx::query_as(&sql)
            .bind(&pairing_slugs)
            .fetch_all(pool)
            .await
            .unwrap_or_default()
    } else {
        vec![]
    };

    // BATCH: load culinary properties for all pairing slugs
    let pairing_culinary_rows: Vec<ExtraCulinaryRow> = if !pairing_slugs.is_empty() {
        sqlx::query_as(
            r#"SELECT p.slug,
                      fcp.sweetness, fcp.acidity, fcp.bitterness, fcp.umami, fcp.aroma
               FROM products p
               JOIN food_culinary_properties fcp ON fcp.product_id = p.id
               WHERE p.slug = ANY($1)"#,
        )
        .bind(&pairing_slugs)
        .fetch_all(pool)
        .await
        .unwrap_or_default()
    } else {
        vec![]
    };

    let candidates: Vec<Candidate> = pairings_raw
        .iter()
        .filter(|p| {
            let s = p.slug.as_deref().unwrap_or("");
            !existing_slugs.contains(&s.to_string())
        })
        .map(|p| {
            let slug = p.slug.clone().unwrap_or_default();

            // Look up real nutrition from catalog
            let cat = pairing_catalog_rows.iter().find(|r| r.slug.as_deref() == Some(&slug));
            let (nutrition, typical_g, product_type) = if let Some(r) = cat {
                (
                    NutritionBreakdown {
                        calories: r.cal(), protein_g: r.prot(), fat_g: r.fat(),
                        carbs_g: r.carbs(), fiber_g: r.fiber(), sugar_g: r.sugar(),
                        salt_g: 0.0, sodium_mg: 0.0,
                    },
                    r.typical_g().unwrap_or(50.0),
                    r.product_type.clone(),
                )
            } else {
                (
                    NutritionBreakdown {
                        calories: 0.0, protein_g: 0.0, fat_g: 0.0, carbs_g: 0.0,
                        fiber_g: 0.0, sugar_g: 0.0, salt_g: 0.0, sodium_mg: 0.0,
                    },
                    50.0,
                    None,
                )
            };

            // Look up real flavor from culinary
            let flavor = pairing_culinary_rows.iter()
                .find(|ecr| ecr.slug == slug)
                .map(|ecr| flavor_graph::flavor_from_culinary(
                    ecr.sweetness.unwrap_or(0.0) as f64,
                    ecr.acidity.unwrap_or(0.0) as f64,
                    ecr.bitterness.unwrap_or(0.0) as f64,
                    ecr.umami.unwrap_or(0.0) as f64,
                    ecr.aroma.unwrap_or(0.0) as f64,
                    nutrition.fat_g,
                ))
                .unwrap_or_else(FlavorVector::zero);

            Candidate {
                slug,
                name: match lang {
                    Language::Ru => p.name_ru.clone(),
                    Language::Pl => p.name_pl.clone(),
                    Language::Uk => p.name_uk.clone(),
                    Language::En => p.name_en.clone(),
                },
                image_url: p.image_url.clone(),
                flavor,
                nutrition,
                pair_score: p.pair_score.unwrap_or(0.0) as f64,
                typical_g,
                product_type,
            }
        })
        .collect();

    // ── Culinary rules: filter forbidden pairs + boost contextual scores ──────
    let candidates = super::culinary_rules::apply(
        &ctx.ingredient,
        &candidates,
        &ctx.additional_ingredients,
        ctx.state.as_deref(),
        ctx.resolved_meal_type(),
        ctx.resolved_diet(),
    );

    // Suggestions will be computed after potential state adjustment (step 11c)

    // ── 9. Diagnostics (RuleEngine — only if we have 2+ ingredients) ─────────
    // v3: We keep the full RuleDiagnosis for feedback loop + next_actions
    let raw_diag = if !ctx.additional_ingredients.is_empty() {
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
            nutrition_score: nutrition::nutrition_score(
                total_cal, total_prot, total_fat, total_carbs,
                total_fiber, total_sugar, 0.0,
            ),
        };

        Some(rule_engine::diagnose(&recipe_ctx))
    } else {
        None
    };

    // Convert to response DiagnosticsInfo (v3: localized messages + fix_slugs)
    let diagnostics = raw_diag.as_ref().map(|diag| DiagnosticsInfo {
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
                message: localize_diagnostic(&i.rule, &i.severity, lang),
                fix_slugs: i.fix_slugs.clone(),
            })
            .collect(),
    });

    // Seasonality already loaded in parallel phase 2

    // ── 11. State info (already loaded in parallel phase 2) ────────────────
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
            breakdown_per_100g_nullable(
                st.calories_per_100g,
                st.protein_per_100g,
                st.fat_per_100g,
                st.carbs_per_100g,
                st.fiber_per_100g,
                None, None,
            )
        } else {
            nutrition
        }
    } else {
        nutrition
    };

    // ── 11b. v2: Adjust flavor vector based on state ─────────────────────────
    let (_main_flavor, flavor_profile) = if let Some(ref st) = full_state {
        let adjusted = adjust_flavor_for_state(&main_flavor, &st.state, st.oil_absorption_g, st.water_loss_percent);
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

    // ── v3 Step 1 — Goal Engine: resolve goal ────────────────────────────────
    let goal = ctx.resolved_goal();

    // ── v3 Step 2 — Feedback Loop: batch load fix_slug candidates ──────────
    let mut all_candidates = candidates;

    if let Some(ref diag) = raw_diag {
        // Collect fix_slugs from issues, excluding ingredients already present
        let feedback_slugs: Vec<String> = diag.issues.iter()
            .filter(|issue| issue.severity == "critical" || issue.severity == "warning")
            .flat_map(|issue| issue.fix_slugs.clone())
            .filter(|slug| !existing_slugs.contains(slug))
            .filter(|slug| !all_candidates.iter().any(|c| c.slug == *slug))
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        if !feedback_slugs.is_empty() {
            // BATCH: single query for ALL feedback slugs
            let fix_sql = format!(
                "SELECT {} FROM catalog_ingredients WHERE slug = ANY($1) AND COALESCE(is_active, true) = true",
                CATALOG_NUTRITION_COLS
            );
            let fix_rows: Vec<CatalogNutritionRow> = sqlx::query_as(&fix_sql)
                .bind(&feedback_slugs)
                .fetch_all(pool)
                .await
                .unwrap_or_default();

            // BATCH: single query for culinary props of all feedback slugs
            let fix_slugs_for_culinary: Vec<String> = fix_rows.iter().filter_map(|r| r.slug.clone()).collect();
            let fix_culinary_rows: Vec<ExtraCulinaryRow> = if !fix_slugs_for_culinary.is_empty() {
                sqlx::query_as(
                    r#"SELECT p.slug,
                              fcp.sweetness, fcp.acidity, fcp.bitterness, fcp.umami, fcp.aroma
                       FROM products p
                       JOIN food_culinary_properties fcp ON fcp.product_id = p.id
                       WHERE p.slug = ANY($1)"#,
                )
                .bind(&fix_slugs_for_culinary)
                .fetch_all(pool)
                .await
                .unwrap_or_default()
            } else {
                vec![]
            };

            for fix_row in &fix_rows {
                let slug = fix_row.slug.clone().unwrap_or_default();
                let fix_flavor = fix_culinary_rows.iter()
                    .find(|ecr| ecr.slug == slug)
                    .map(|ecr| flavor_graph::flavor_from_culinary(
                        ecr.sweetness.unwrap_or(0.0) as f64,
                        ecr.acidity.unwrap_or(0.0) as f64,
                        ecr.bitterness.unwrap_or(0.0) as f64,
                        ecr.umami.unwrap_or(0.0) as f64,
                        ecr.aroma.unwrap_or(0.0) as f64,
                        fix_row.fat(),
                    ))
                    .unwrap_or_else(FlavorVector::zero);

                let fix_nutrition = NutritionBreakdown {
                    calories: fix_row.cal(),
                    protein_g: fix_row.prot(),
                    fat_g: fix_row.fat(),
                    carbs_g: fix_row.carbs(),
                    fiber_g: fix_row.fiber(),
                    sugar_g: fix_row.sugar(),
                    salt_g: 0.0,
                    sodium_mg: 0.0,
                };

                all_candidates.push(Candidate {
                    slug: slug.clone(),
                    name: fix_row.localized_name(lang).to_string(),
                    image_url: fix_row.image_url.clone(),
                    flavor: fix_flavor,
                    nutrition: fix_nutrition,
                    pair_score: 5.0,
                    typical_g: fix_row.typical_g().unwrap_or(30.0),
                    product_type: fix_row.product_type.clone(),
                });
            }
        }
    }

    // ── v3 Step 1+11c: Goal-aware suggestions ────────────────────────────────
    // Run SuggestionEngine, then re-rank by goal
    let suggestion_result = suggestion_engine::suggest_ingredients(
        balance, &all_candidates, &existing_slugs, 10, // fetch more, then re-rank
    );
    let mut scored_suggestions: Vec<SuggestionInfo> = suggestion_result
        .suggestions
        .into_iter()
        .map(|s| {
            let base_score = s.score as f64;
            // Goal-aware bonus (up to +15 points)
            let goal_bonus = goal_bonus_for_candidate(&s.slug, &s.reasons, &s.fills_gaps, goal, &all_candidates);
            let final_score = (base_score + goal_bonus).clamp(0.0, 100.0).round() as u8;
            SuggestionInfo {
                slug: s.slug,
                name: s.name,
                image_url: s.image_url,
                score: final_score,
                reasons: s.reasons,
                fills_gaps: s.fills_gaps,
                suggested_grams: s.suggested_grams,
            }
        })
        .collect();

    // Re-sort by adjusted score and take top 5
    scored_suggestions.sort_by(|a, b| b.score.cmp(&a.score));
    scored_suggestions.truncate(5);
    let suggestions = scored_suggestions;

    // ── 12. Unit equivalents from density (v2) ──────────────────────────────
    let density = row.density();
    let equivalents = build_equivalents(100.0, density);

    // ── v3 Step 3 — Confidence System ────────────────────────────────────────
    let confidence = build_confidence(
        &row,
        culinary.is_some(),
        pairings.len(),
        &full_state,
        &nutrition,
    );

    // ── v3 Step 4 — Next Actions ─────────────────────────────────────────────
    // Build slug→name map from all known ingredients for localized display
    let mut slug_names: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for c in &all_candidates {
        slug_names.entry(c.slug.clone()).or_insert_with(|| c.name.clone());
    }
    for p in &pairings {
        slug_names.entry(p.slug.clone()).or_insert_with(|| p.name.clone());
    }
    for s in &suggestions {
        slug_names.entry(s.slug.clone()).or_insert_with(|| s.name.clone());
    }

    let next_actions = build_next_actions(
        &raw_diag,
        balance,
        goal,
        &suggestions,
        &existing_slugs,
        lang,
        &slug_names,
    );

    // ── 13. Build explanation (v3: goal-aware + feedback-loop) ───────────────
    let explain = build_explain_v3(
        &ingredient_info, balance, &pairings, &suggestions,
        &diagnostics, goal, &full_state, &row,
        &confidence, &next_actions, lang,
    );

    // ── v3 Step 5 — Session ID ───────────────────────────────────────────────
    let session_id = ctx.session_id.clone().unwrap_or_else(|| {
        use std::time::{SystemTime, UNIX_EPOCH};
        let ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis();
        format!("ss-{:x}", ts)
    });

    // ── v3 Step 6 — Recipe Builder: 3 dish variants ─────────────────────────
    // MODE: if user provided additional_ingredients → ANALYZE (use only their recipe)
    //       otherwise → BUILD (generate dish from pairings pool)
    let is_analyze_mode = !ctx.additional_ingredients.is_empty();
    let mode_label = if is_analyze_mode { "analyze" } else { "build" };

    let variants = if is_analyze_mode {
        // ANALYZE MODE: build Candidate objects from user's additional_ingredients only
        let mut recipe_candidates: Vec<Candidate> = Vec::new();
        for (i, extra_slug) in ctx.additional_ingredients.iter().enumerate() {
            if let Some((extra_row, extra_flavor)) = additional_rows.get(i) {
                recipe_candidates.push(Candidate {
                    slug: extra_row.slug.clone().unwrap_or_else(|| extra_slug.clone()),
                    name: extra_row.localized_name(lang).to_string(),
                    image_url: extra_row.image_url.clone(),
                    flavor: extra_flavor.clone(),
                    nutrition: NutritionBreakdown {
                        calories: extra_row.cal(),
                        protein_g: extra_row.prot(),
                        fat_g: extra_row.fat(),
                        carbs_g: extra_row.carbs(),
                        fiber_g: extra_row.fiber(),
                        sugar_g: extra_row.sugar(),
                        salt_g: 0.0,
                        sodium_mg: 0.0,
                    },
                    pair_score: 10.0, // user chose these — max affinity
                    typical_g: extra_row.typical_g().unwrap_or(50.0),
                    product_type: extra_row.product_type.clone(),
                });
            }
        }
        recipe_builder::build_variants(
            ingredient_info.slug.as_str(),
            ingredient_info.name.as_str(),
            ingredient_info.image_url.as_deref(),
            row.cal(),
            row.prot(),
            row.fat(),
            row.fiber(),
            typical_g,
            row.product_type.as_deref(),
            &recipe_candidates,
            balance,
            lang,
        )
    } else {
        // BUILD MODE: generate dish from pairing candidates pool
        recipe_builder::build_variants(
            ingredient_info.slug.as_str(),
            ingredient_info.name.as_str(),
            ingredient_info.image_url.as_deref(),
            row.cal(),
            row.prot(),
            row.fat(),
            row.fiber(),
            typical_g,
            row.product_type.as_deref(),
            &all_candidates,
            balance,
            lang,
        )
    };

    // ── 14. Compose response ─────────────────────────────────────────────────
    let timing_ms = start.elapsed().as_millis() as u64;

    // Resolve meal_type and diet for response (only include if user provided them)
    let meal_type_label = ctx.resolved_meal_type().map(|m| m.label().to_string());
    let diet_resolved = ctx.resolved_diet();
    let diet_label = if diet_resolved == super::context::Diet::None {
        None
    } else {
        Some(diet_resolved.label().to_string())
    };
    let cooking_time_label = ctx.resolved_cooking_time().map(|t| t.label().to_string());
    let budget_label = ctx.resolved_budget().map(|b| b.label().to_string());
    let cuisine_label = ctx.resolved_cuisine().map(|c| c.label().to_string());

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
        confidence,
        next_actions,
        explain,
        variants,
        mode: mode_label.to_string(),
        meal_type: meal_type_label,
        diet: diet_label,
        cooking_time: cooking_time_label,
        budget: budget_label,
        cuisine: cuisine_label,
        session_id,
        meta: SmartMeta {
            timing_ms,
            cached: false,
            cache_key: ctx.cache_key(),
            engine_version: "3.4.0".to_string(),
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

// ── Deterministic explanation builder v3 ─────────────────────────────────────

fn build_explain_v3(
    ingredient: &IngredientInfo,
    balance: &FlavorBalance,
    pairings: &[PairingInfo],
    suggestions: &[SuggestionInfo],
    diagnostics: &Option<DiagnosticsInfo>,
    goal: Goal,
    full_state: &Option<StateRow>,
    raw_row: &CatalogNutritionRow,
    confidence: &ConfidenceInfo,
    next_actions: &[NextAction],
    lang: Language,
) -> Vec<String> {
    let mut lines = Vec::new();

    // Ingredient overview
    let lbl_balance = match lang {
        Language::Ru => "баланс вкуса",
        Language::Uk => "баланс смаку",
        Language::Pl => "balans smaku",
        Language::En => "flavor balance score",
    };
    lines.push(format!(
        "{} ({}) — {}: {}/100.",
        ingredient.name,
        ingredient.product_type.as_deref().unwrap_or("—"),
        lbl_balance,
        balance.balance_score,
    ));

    // ── State-change explanation ─────────────────────────────────────────────
    if let Some(st) = full_state {
        if st.state != "raw" {
            let lbl_state = match lang {
                Language::Ru => "Обработка",
                Language::Uk => "Обробка",
                Language::Pl => "Obróbka",
                Language::En => "Processing state",
            };
            lines.push(format!("{}: {}.", lbl_state, st.state));

            if let Some(cooked_cal) = st.calories_per_100g {
                let raw_cal = raw_row.cal();
                let diff = cooked_cal - raw_cal;
                if diff.abs() > 1.0 {
                    let dir = if diff > 0.0 { "+" } else { "" };
                    lines.push(format!("Calories: {:.0} → {:.0} kcal/100g ({}{:.0}).", raw_cal, cooked_cal, dir, diff));
                }
            }
            if let Some(cooked_fat) = st.fat_per_100g {
                let raw_fat = raw_row.fat();
                let diff = cooked_fat - raw_fat;
                if diff.abs() > 0.5 {
                    let dir = if diff > 0.0 { "+" } else { "" };
                    lines.push(format!("Fat: {:.1} → {:.1} g/100g ({}{:.1}).", raw_fat, cooked_fat, dir, diff));
                }
            }
            if let Some(cooked_prot) = st.protein_per_100g {
                let raw_prot = raw_row.prot();
                let diff = cooked_prot - raw_prot;
                if diff.abs() > 0.5 {
                    let dir = if diff > 0.0 { "+" } else { "" };
                    lines.push(format!("Protein: {:.1} → {:.1} g/100g ({}{:.1}).", raw_prot, cooked_prot, dir, diff));
                }
            }
            if let Some(ref tex) = st.texture {
                let lbl = match lang { Language::Ru => "Текстура", Language::Uk => "Текстура", Language::Pl => "Tekstura", Language::En => "Texture" };
                lines.push(format!("{}: {}.", lbl, tex));
            }
        }
    }

    // Weak / strong dimensions (localized)
    if !balance.weak_dimensions.is_empty() {
        let weak: Vec<String> = balance.weak_dimensions.iter().map(|d| localize_dimension(&d.dimension, lang)).collect();
        let lbl = match lang { Language::Ru => "Слабые зоны вкуса", Language::Uk => "Слабкі зони смаку", Language::Pl => "Słabe strefy smaku", Language::En => "Weak flavor areas" };
        lines.push(format!("{}: {}.", lbl, weak.join(", ")));
    }
    if !balance.strong_dimensions.is_empty() {
        let strong: Vec<String> = balance.strong_dimensions.iter().map(|d| localize_dimension(&d.dimension, lang)).collect();
        let lbl = match lang { Language::Ru => "Сильные зоны вкуса", Language::Uk => "Сильні зони смаку", Language::Pl => "Silne strefy smaku", Language::En => "Strong flavor areas" };
        lines.push(format!("{}: {}.", lbl, strong.join(", ")));
    }

    // Top pairing
    if let Some(top) = pairings.first() {
        let lbl = match lang { Language::Ru => "Лучшая пара", Language::Uk => "Найкраща пара", Language::Pl => "Najlepsza para", Language::En => "Best pairing" };
        lines.push(format!("{}: {} ({:.1}).", lbl, top.name, top.pair_score));
    }

    // Top suggestion
    if let Some(top) = suggestions.first() {
        let lbl = match lang { Language::Ru => "Рекомендация", Language::Uk => "Рекомендація", Language::Pl => "Rekomendacja", Language::En => "Top suggestion" };
        lines.push(format!("{}: {} ({}/100).", lbl, top.name, top.score));
    }

    // Diagnostics summary
    if let Some(diag) = diagnostics {
        let lbl = match lang { Language::Ru => "Здоровье рецепта", Language::Uk => "Здоров'я рецепту", Language::Pl => "Zdrowie przepisu", Language::En => "Recipe health score" };
        lines.push(format!("{}: {}/100.", lbl, diag.health_score));
    }

    // Goal
    if goal != Goal::Balanced {
        lines.push(localize_goal_explain(goal, lang));
    }

    // Confidence warning
    if confidence.overall < 0.5 {
        let msg = match lang {
            Language::Ru => format!("⚠ Низкая уверенность данных ({:.0}%). Результаты приблизительные.", confidence.overall * 100.0),
            Language::Uk => format!("⚠ Низька впевненість даних ({:.0}%). Результати приблизні.", confidence.overall * 100.0),
            Language::Pl => format!("⚠ Niska pewność danych ({:.0}%). Wyniki przybliżone.", confidence.overall * 100.0),
            Language::En => format!("⚠ Low data confidence ({:.0}%). Results may be approximate.", confidence.overall * 100.0),
        };
        lines.push(msg);
    }

    // Next action hint
    if !next_actions.is_empty() {
        let a = &next_actions[0];
        let lbl = match lang { Language::Ru => "Следующий шаг", Language::Uk => "Наступний крок", Language::Pl => "Następny krok", Language::En => "Next step" };
        lines.push(format!("{}: {} {} — {}.", lbl, a.action_type, a.name, a.reason));
    }

    lines
}

// ── v3: Goal-aware suggestion bonus ──────────────────────────────────────────

/// Compute a bonus (or penalty) for a suggestion based on the active goal.
/// Looks up the candidate's nutrition to decide.
fn goal_bonus_for_candidate(
    slug: &str,
    _reasons: &[String],
    fills_gaps: &[String],
    goal: Goal,
    candidates: &[Candidate],
) -> f64 {
    let candidate = candidates.iter().find(|c| c.slug == slug);
    let n = candidate.map(|c| &c.nutrition);

    match goal {
        Goal::Balanced => 0.0, // no adjustment

        Goal::HighProtein | Goal::MuscleGain => {
            // Boost protein-rich candidates
            if let Some(nut) = n {
                let protein_ratio = if nut.calories > 0.0 {
                    (nut.protein_g * 4.0 / nut.calories).min(1.0)
                } else { 0.0 };
                protein_ratio * 15.0 // up to +15 for pure protein
            } else { 0.0 }
        }

        Goal::LowCalorie | Goal::Diet => {
            // Boost low-cal, high-fiber; penalize high-cal
            if let Some(nut) = n {
                let cal_penalty = if nut.calories > 200.0 { -8.0 } else { 0.0 };
                let fiber_bonus = (nut.fiber_g / 10.0).min(1.0) * 8.0;
                cal_penalty + fiber_bonus
            } else { 0.0 }
        }

        Goal::Keto => {
            // Boost fat, penalize carbs
            if let Some(nut) = n {
                let carb_penalty = if nut.carbs_g > 10.0 { -12.0 } else { 0.0 };
                let fat_bonus = (nut.fat_g / 30.0).min(1.0) * 10.0;
                carb_penalty + fat_bonus
            } else { 0.0 }
        }

        Goal::FlavorBoost => {
            // Bonus for candidates that fill flavor gaps
            if !fills_gaps.is_empty() {
                fills_gaps.len() as f64 * 5.0 // +5 per gap filled
            } else { 0.0 }
        }
    }
}

// ── v3: Confidence System ────────────────────────────────────────────────────

fn build_confidence(
    row: &CatalogNutritionRow,
    has_culinary: bool,
    pairing_count: usize,
    full_state: &Option<StateRow>,
    nutrition: &nutrition::NutritionBreakdownNullable,
) -> ConfidenceInfo {
    // Nutrition confidence: count non-null fields / total expected fields (7)
    let nut_fields = [
        nutrition.calories.is_some(),
        nutrition.protein_g.is_some(),
        nutrition.fat_g.is_some(),
        nutrition.carbs_g.is_some(),
        nutrition.fiber_g.is_some(),
        nutrition.sugar_g.is_some(),
        nutrition.salt_g.is_some(),
    ];
    let nut_filled = nut_fields.iter().filter(|&&b| b).count() as f64;
    let nutrition_conf = nut_filled / 7.0;

    // Flavor confidence: do we have culinary properties?
    let flavor_conf = if has_culinary { 0.95 } else { 0.3 };

    // Pairing confidence: based on how many pairings we found (0→0.2, 8→1.0)
    let pairing_conf = ((pairing_count as f64) / 8.0).min(1.0).max(0.2);

    // State bonus: if state was requested and found, boost confidence
    let state_bonus = match full_state {
        Some(_) => 0.05,
        None => 0.0,
    };

    // Density bonus: affects equivalents quality
    let density_bonus = if row.density() > 0.0 && row.density() != 1.0 { 0.05 } else { 0.0 };

    // Overall: weighted average
    let overall = (
        nutrition_conf * 0.35 +
        flavor_conf * 0.30 +
        pairing_conf * 0.25 +
        state_bonus +
        density_bonus
    ).min(1.0);

    ConfidenceInfo {
        overall:   uc::display_round(overall),
        nutrition: uc::display_round(nutrition_conf),
        pairings:  uc::display_round(pairing_conf),
        flavor:    uc::display_round(flavor_conf),
    }
}

// ── v3: Next Actions (localized) ─────────────────────────────────────────────

fn build_next_actions(
    raw_diag: &Option<rule_engine::RuleDiagnosis>,
    balance: &FlavorBalance,
    goal: Goal,
    suggestions: &[SuggestionInfo],
    existing_slugs: &[String],
    lang: Language,
    slug_names: &std::collections::HashMap<String, String>,
) -> Vec<NextAction> {
    let mut actions: Vec<NextAction> = Vec::new();
    let mut used_slugs: std::collections::HashSet<String> = std::collections::HashSet::new();

    let resolve_name = |slug: &str| -> String {
        slug_names.get(slug).cloned().unwrap_or_else(|| slug.replace('-', " "))
    };

    // 1. From diagnostics: critical/warning issues → "add" actions with fix_slugs
    if let Some(diag) = raw_diag {
        for issue in &diag.issues {
            if issue.severity != "critical" && issue.severity != "warning" {
                continue;
            }
            for slug in &issue.fix_slugs {
                if existing_slugs.contains(slug) || used_slugs.contains(slug) {
                    continue;
                }
                let reason = localize_action_reason(&issue.rule, lang);
                let priority = if issue.severity == "critical" { 1 } else { 2 };
                actions.push(NextAction {
                    action_type: "add".to_string(),
                    ingredient: slug.clone(),
                    name: resolve_name(slug),
                    reason,
                    priority,
                });
                used_slugs.insert(slug.clone());
                if actions.len() >= 5 { break; }
            }
            if actions.len() >= 5 { break; }
        }
    }

    // 2. From weak dimensions: suggest top suggestion that fills a gap
    if actions.len() < 5 {
        for gap in &balance.weak_dimensions {
            if let Some(s) = suggestions.iter().find(|s| s.fills_gaps.contains(&gap.dimension)) {
                if !used_slugs.contains(&s.slug) && !existing_slugs.contains(&s.slug) {
                    let reason = match lang {
                        Language::Ru => format!("заполняет пробел: {}", localize_dimension(&gap.dimension, lang)),
                        Language::Uk => format!("заповнює прогалину: {}", localize_dimension(&gap.dimension, lang)),
                        Language::Pl => format!("wypełnia lukę: {}", localize_dimension(&gap.dimension, lang)),
                        Language::En => format!("fills {} gap", gap.dimension),
                    };
                    actions.push(NextAction {
                        action_type: "add".to_string(),
                        ingredient: s.slug.clone(),
                        name: s.name.clone(),
                        reason,
                        priority: 3,
                    });
                    used_slugs.insert(s.slug.clone());
                }
            }
            if actions.len() >= 5 { break; }
        }
    }

    // 3. Goal-specific actions
    if actions.len() < 5 {
        match goal {
            Goal::HighProtein | Goal::MuscleGain => {
                for s in suggestions {
                    if s.reasons.iter().any(|r| r.contains("nutritional value")) && !used_slugs.contains(&s.slug) {
                        let reason = match lang {
                            Language::Ru => "повышает белок для вашей цели".to_string(),
                            Language::Uk => "підвищує білок для вашої цілі".to_string(),
                            Language::Pl => "zwiększa białko dla twojego celu".to_string(),
                            Language::En => "boosts protein for your goal".to_string(),
                        };
                        actions.push(NextAction {
                            action_type: "add".to_string(),
                            ingredient: s.slug.clone(),
                            name: s.name.clone(),
                            reason,
                            priority: 3,
                        });
                        used_slugs.insert(s.slug.clone());
                        break;
                    }
                }
            }
            Goal::FlavorBoost => {
                if let Some(s) = suggestions.first() {
                    if !used_slugs.contains(&s.slug) {
                        let reason = match lang {
                            Language::Ru => "максимизирует баланс вкуса".to_string(),
                            Language::Uk => "максимізує баланс смаку".to_string(),
                            Language::Pl => "maksymalizuje balans smaku".to_string(),
                            Language::En => "maximizes flavor balance".to_string(),
                        };
                        actions.push(NextAction {
                            action_type: "add".to_string(),
                            ingredient: s.slug.clone(),
                            name: s.name.clone(),
                            reason,
                            priority: 3,
                        });
                        used_slugs.insert(s.slug.clone());
                    }
                }
            }
            _ => {}
        }
    }

    actions.sort_by_key(|a| a.priority);
    actions.truncate(5);
    actions
}

// ═══════════════════════════════════════════════════════════════════════════
//  i18n helpers
// ═══════════════════════════════════════════════════════════════════════════

fn localize_dimension(dim: &str, lang: Language) -> String {
    match (dim, lang) {
        ("sweetness",  Language::Ru) => "сладость".into(),
        ("sweetness",  Language::Uk) => "солодкість".into(),
        ("sweetness",  Language::Pl) => "słodycz".into(),
        ("acidity",    Language::Ru) => "кислотность".into(),
        ("acidity",    Language::Uk) => "кислотність".into(),
        ("acidity",    Language::Pl) => "kwasowość".into(),
        ("bitterness", Language::Ru) => "горечь".into(),
        ("bitterness", Language::Uk) => "гіркота".into(),
        ("bitterness", Language::Pl) => "goryczka".into(),
        ("umami",      Language::Ru) => "умами".into(),
        ("umami",      Language::Uk) => "умамі".into(),
        ("umami",      Language::Pl) => "umami".into(),
        ("fat",        Language::Ru) => "жирность".into(),
        ("fat",        Language::Uk) => "жирність".into(),
        ("fat",        Language::Pl) => "tłustość".into(),
        ("aroma",      Language::Ru) => "аромат".into(),
        ("aroma",      Language::Uk) => "аромат".into(),
        ("aroma",      Language::Pl) => "aromat".into(),
        _                            => dim.to_string(),
    }
}

fn localize_diagnostic(rule: &str, severity: &str, lang: Language) -> String {
    let sev = match (severity, lang) {
        ("critical", Language::Ru) => "⛔",
        ("critical", Language::Uk) => "⛔",
        ("critical", Language::Pl) => "⛔",
        ("warning",  Language::Ru) => "⚠",
        ("warning",  Language::Uk) => "⚠",
        ("warning",  Language::Pl) => "⚠",
        ("critical", _)            => "⛔",
        ("warning",  _)            => "⚠",
        _                          => "ℹ",
    };
    let msg = match (rule, lang) {
        ("low_acidity",    Language::Ru) => "Не хватает кислотности — добавьте цитрус или уксус",
        ("low_acidity",    Language::Uk) => "Бракує кислотності — додайте цитрус або оцет",
        ("low_acidity",    Language::Pl) => "Brak kwasowości — dodaj cytrus lub ocet",
        ("low_acidity",    _)            => "Low acidity — add citrus or vinegar",

        ("low_umami",      Language::Ru) => "Не хватает умами — добавьте соевый соус, мисо или грибы",
        ("low_umami",      Language::Uk) => "Бракує умамі — додайте соєвий соус, місо або гриби",
        ("low_umami",      Language::Pl) => "Brak umami — dodaj sos sojowy, miso lub grzyby",
        ("low_umami",      _)            => "Low umami — add soy sauce, miso or mushrooms",

        ("low_fat",        Language::Ru) => "Мало жиров — добавьте масло или авокадо",
        ("low_fat",        Language::Uk) => "Мало жирів — додайте олію або авокадо",
        ("low_fat",        Language::Pl) => "Mało tłuszczu — dodaj olej lub awokado",
        ("low_fat",        _)            => "Low fat — add oil or avocado",

        ("low_aroma",      Language::Ru) => "Слабый аромат — добавьте специи или зелень",
        ("low_aroma",      Language::Uk) => "Слабкий аромат — додайте спеції або зелень",
        ("low_aroma",      Language::Pl) => "Słaby aromat — dodaj przyprawy lub zioła",
        ("low_aroma",      _)            => "Weak aroma — add spices or herbs",

        ("low_sweetness",  Language::Ru) => "Не хватает сладости — добавьте мёд или фрукты",
        ("low_sweetness",  Language::Uk) => "Бракує солодкості — додайте мед або фрукти",
        ("low_sweetness",  Language::Pl) => "Brak słodyczy — dodaj miód lub owoce",
        ("low_sweetness",  _)            => "Low sweetness — add honey or fruit",

        ("low_bitterness", Language::Ru) => "Не хватает горечи — добавьте рукколу или тёмный шоколад",
        ("low_bitterness", Language::Uk) => "Бракує гіркоти — додайте руколу або темний шоколад",
        ("low_bitterness", Language::Pl) => "Brak goryczki — dodaj rukolę lub gorzką czekoladę",
        ("low_bitterness", _)            => "Low bitterness — add arugula or dark chocolate",

        ("high_sweetness", Language::Ru) => "Избыток сладости — сбалансируйте кислым или горьким",
        ("high_sweetness", Language::Uk) => "Надлишок солодкості — збалансуйте кислим або гірким",
        ("high_sweetness", Language::Pl) => "Nadmiar słodyczy — zbalansuj kwaśnym lub gorzkim",
        ("high_sweetness", _)            => "Too sweet — balance with acidity or bitterness",

        ("high_acidity",   Language::Ru) => "Избыток кислотности — смягчите жирами или сладким",
        ("high_acidity",   Language::Uk) => "Надлишок кислотності — пом'якшіть жирами або солодким",
        ("high_acidity",   Language::Pl) => "Nadmiar kwasowości — złagodź tłuszczem lub słodyczą",
        ("high_acidity",   _)            => "Too acidic — soften with fat or sweetness",

        ("high_bitterness", Language::Ru) => "Избыток горечи — добавьте сладость или жиры",
        ("high_bitterness", Language::Uk) => "Надлишок гіркоти — додайте солодкість або жири",
        ("high_bitterness", Language::Pl) => "Nadmiar goryczki — dodaj słodycz lub tłuszcz",
        ("high_bitterness", _)            => "Too bitter — add sweetness or fat",

        ("high_umami",     Language::Ru) => "Избыток умами — разбавьте свежими овощами",
        ("high_umami",     Language::Uk) => "Надлишок умамі — розбавте свіжими овочами",
        ("high_umami",     Language::Pl) => "Nadmiar umami — rozrzedź świeżymi warzywami",
        ("high_umami",     _)            => "Too much umami — dilute with fresh vegetables",

        ("high_fat",       Language::Ru) => "Избыток жиров — добавьте кислотность",
        ("high_fat",       Language::Uk) => "Надлишок жирів — додайте кислотність",
        ("high_fat",       Language::Pl) => "Nadmiar tłuszczu — dodaj kwasowość",
        ("high_fat",       _)            => "Too much fat — add acidity",

        ("high_aroma",     Language::Ru) => "Избыток аромата — упростите набор специй",
        ("high_aroma",     Language::Uk) => "Надлишок аромату — спростіть набір спецій",
        ("high_aroma",     Language::Pl) => "Nadmiar aromatu — uprość przyprawy",
        ("high_aroma",     _)            => "Too aromatic — simplify spice blend",

        ("high_carbs",     Language::Ru) => "Много углеводов — замените часть крупой с низким ГИ",
        ("high_carbs",     Language::Uk) => "Багато вуглеводів — замініть частину крупою з низьким ГІ",
        ("high_carbs",     Language::Pl) => "Dużo węglowodanów — zamień część na nisko-GI",
        ("high_carbs",     _)            => "High carbs — swap for low-GI alternatives",

        ("low_protein",    Language::Ru) => "Мало белка — добавьте мясо, рыбу или бобовые",
        ("low_protein",    Language::Uk) => "Мало білка — додайте м'ясо, рибу або бобові",
        ("low_protein",    Language::Pl) => "Mało białka — dodaj mięso, rybę lub rośliny strączkowe",
        ("low_protein",    _)            => "Low protein — add meat, fish or legumes",

        ("high_fat_ratio", Language::Ru) => "Высокая доля жиров в калориях",
        ("high_fat_ratio", Language::Uk) => "Висока частка жирів у калоріях",
        ("high_fat_ratio", Language::Pl) => "Wysoki udział tłuszczu w kaloriach",
        ("high_fat_ratio", _)            => "High fat-to-calorie ratio",

        ("low_fiber",      Language::Ru) => "Мало клетчатки — добавьте овощи или зелень",
        ("low_fiber",      Language::Uk) => "Мало клітковини — додайте овочі або зелень",
        ("low_fiber",      Language::Pl) => "Mało błonnika — dodaj warzywa lub zioła",
        ("low_fiber",      _)            => "Low fiber — add vegetables or greens",

        ("high_sugar",     Language::Ru) => "Много сахара — уменьшите сладкие ингредиенты",
        ("high_sugar",     Language::Uk) => "Багато цукру — зменшіть солодкі інгредієнти",
        ("high_sugar",     Language::Pl) => "Dużo cukru — zmniejsz słodkie składniki",
        ("high_sugar",     _)            => "High sugar — reduce sweet ingredients",

        ("ingredient_dominance", Language::Ru) => "Один ингредиент доминирует — добавьте разнообразие",
        ("ingredient_dominance", Language::Uk) => "Один інгредієнт домінує — додайте різноманіття",
        ("ingredient_dominance", Language::Pl) => "Jeden składnik dominuje — dodaj różnorodność",
        ("ingredient_dominance", _)            => "One ingredient dominates — add variety",

        ("missing_fat_source",     Language::Ru) => "Нет источника жиров",
        ("missing_fat_source",     Language::Uk) => "Немає джерела жирів",
        ("missing_fat_source",     Language::Pl) => "Brak źródła tłuszczu",
        ("missing_fat_source",     _)            => "No fat source found",

        ("missing_aromatics",      Language::Ru) => "Нет ароматических ингредиентов",
        ("missing_aromatics",      Language::Uk) => "Немає ароматичних інгредієнтів",
        ("missing_aromatics",      Language::Pl) => "Brak składników aromatycznych",
        ("missing_aromatics",      _)            => "No aromatic ingredients",

        ("missing_vegetables",     Language::Ru) => "Нет овощей — добавьте клетчатку",
        ("missing_vegetables",     Language::Uk) => "Немає овочів — додайте клітковину",
        ("missing_vegetables",     Language::Pl) => "Brak warzyw — dodaj błonnik",
        ("missing_vegetables",     _)            => "No vegetables — add fiber",

        ("missing_protein_source", Language::Ru) => "Нет источника белка",
        ("missing_protein_source", Language::Uk) => "Немає джерела білка",
        ("missing_protein_source", Language::Pl) => "Brak źródła białka",
        ("missing_protein_source", _)            => "No protein source",

        ("missing_acid_source",    Language::Ru) => "Нет кислотного баланса",
        ("missing_acid_source",    Language::Uk) => "Немає кислотного балансу",
        ("missing_acid_source",    Language::Pl) => "Brak równowagi kwasowej",
        ("missing_acid_source",    _)            => "No acid source for balance",

        _                          => return format!("{} {}", sev, rule),
    };
    format!("{} {}", sev, msg)
}

fn localize_action_reason(rule: &str, lang: Language) -> String {
    match (rule, lang) {
        ("low_acidity",    Language::Ru) => "повысит кислотность".into(),
        ("low_acidity",    Language::Uk) => "підвищить кислотність".into(),
        ("low_acidity",    Language::Pl) => "zwiększy kwasowość".into(),
        ("low_acidity",    _)            => "boosts acidity".into(),

        ("low_umami",      Language::Ru) => "усилит умами".into(),
        ("low_umami",      Language::Uk) => "посилить умамі".into(),
        ("low_umami",      Language::Pl) => "wzmocni umami".into(),
        ("low_umami",      _)            => "boosts umami".into(),

        ("low_fat",        Language::Ru) => "добавит жирность".into(),
        ("low_fat",        Language::Uk) => "додасть жирність".into(),
        ("low_fat",        Language::Pl) => "doda tłustość".into(),
        ("low_fat",        _)            => "adds fat".into(),

        ("low_aroma",      Language::Ru) => "усилит аромат".into(),
        ("low_aroma",      Language::Uk) => "посилить аромат".into(),
        ("low_aroma",      Language::Pl) => "wzmocni aromat".into(),
        ("low_aroma",      _)            => "boosts aroma".into(),

        ("low_sweetness",  Language::Ru) => "добавит сладость".into(),
        ("low_sweetness",  Language::Uk) => "додасть солодкість".into(),
        ("low_sweetness",  Language::Pl) => "doda słodycz".into(),
        ("low_sweetness",  _)            => "adds sweetness".into(),

        ("low_bitterness", Language::Ru) => "добавит горечь".into(),
        ("low_bitterness", Language::Uk) => "додасть гіркоту".into(),
        ("low_bitterness", Language::Pl) => "doda goryczkę".into(),
        ("low_bitterness", _)            => "adds bitterness".into(),

        ("low_protein",    Language::Ru) => "повысит белок".into(),
        ("low_protein",    Language::Uk) => "підвищить білок".into(),
        ("low_protein",    Language::Pl) => "zwiększy białko".into(),
        ("low_protein",    _)            => "boosts protein".into(),

        ("low_fiber",      Language::Ru) => "добавит клетчатку".into(),
        ("low_fiber",      Language::Uk) => "додасть клітковину".into(),
        ("low_fiber",      Language::Pl) => "doda błonnik".into(),
        ("low_fiber",      _)            => "adds fiber".into(),

        ("missing_fat_source",     Language::Ru) => "источник жиров".into(),
        ("missing_fat_source",     Language::Uk) => "джерело жирів".into(),
        ("missing_fat_source",     Language::Pl) => "źródło tłuszczu".into(),
        ("missing_fat_source",     _)            => "fat source".into(),

        ("missing_aromatics",      Language::Ru) => "ароматический ингредиент".into(),
        ("missing_aromatics",      Language::Uk) => "ароматичний інгредієнт".into(),
        ("missing_aromatics",      Language::Pl) => "składnik aromatyczny".into(),
        ("missing_aromatics",      _)            => "aromatic ingredient".into(),

        ("missing_vegetables",     Language::Ru) => "источник клетчатки".into(),
        ("missing_vegetables",     Language::Uk) => "джерело клітковини".into(),
        ("missing_vegetables",     Language::Pl) => "źródło błonnika".into(),
        ("missing_vegetables",     _)            => "fiber source".into(),

        ("missing_protein_source", Language::Ru) => "источник белка".into(),
        ("missing_protein_source", Language::Uk) => "джерело білка".into(),
        ("missing_protein_source", Language::Pl) => "źródło białka".into(),
        ("missing_protein_source", _)            => "protein source".into(),

        ("missing_acid_source",    Language::Ru) => "источник кислоты".into(),
        ("missing_acid_source",    Language::Uk) => "джерело кислоти".into(),
        ("missing_acid_source",    Language::Pl) => "źródło kwasu".into(),
        ("missing_acid_source",    _)            => "acid source".into(),

        _                          => rule.replace('_', " "),
    }
}

fn localize_goal_explain(goal: Goal, lang: Language) -> String {
    match (goal, lang) {
        (Goal::HighProtein, Language::Ru) => "🎯 Цель: высокий белок — приоритет белковым ингредиентам, меньше углеводов.".into(),
        (Goal::HighProtein, Language::Uk) => "🎯 Ціль: високий білок — пріоритет білковим інгредієнтам, менше вуглеводів.".into(),
        (Goal::HighProtein, Language::Pl) => "🎯 Cel: wysokie białko — priorytet składnikom białkowym, mniej węglowodanów.".into(),
        (Goal::HighProtein, _)            => "🎯 Goal: high-protein — prioritize protein-rich, reduce carbs.".into(),

        (Goal::LowCalorie,  Language::Ru) => "🎯 Цель: низкие калории — выбирайте лёгкие ингредиенты.".into(),
        (Goal::LowCalorie,  Language::Uk) => "🎯 Ціль: низькі калорії — обирайте легкі інгредієнти.".into(),
        (Goal::LowCalorie,  Language::Pl) => "🎯 Cel: niskie kalorie — wybieraj lekkie składniki.".into(),
        (Goal::LowCalorie,  _)            => "🎯 Goal: low-calorie — prefer low-energy-density ingredients.".into(),

        (Goal::Keto,        Language::Ru) => "🎯 Цель: кето — минимум углеводов (<20г), больше жиров и белка.".into(),
        (Goal::Keto,        Language::Uk) => "🎯 Ціль: кето — мінімум вуглеводів (<20г), більше жирів та білка.".into(),
        (Goal::Keto,        Language::Pl) => "🎯 Cel: keto — minimum węglowodanów (<20g), więcej tłuszczu i białka.".into(),
        (Goal::Keto,        _)            => "🎯 Goal: keto — minimize carbs (<20g), favor fat + protein.".into(),

        (Goal::MuscleGain,  Language::Ru) => "🎯 Цель: набор массы — много белка + умеренные углеводы.".into(),
        (Goal::MuscleGain,  Language::Uk) => "🎯 Ціль: набір маси — багато білка + помірні вуглеводи.".into(),
        (Goal::MuscleGain,  Language::Pl) => "🎯 Cel: budowa mięśni — dużo białka + umiarkowane węglowodany.".into(),
        (Goal::MuscleGain,  _)            => "🎯 Goal: muscle-gain — high protein + moderate carbs.".into(),

        (Goal::Diet,        Language::Ru) => "🎯 Цель: диета — дефицит калорий, высокая сытость.".into(),
        (Goal::Diet,        Language::Uk) => "🎯 Ціль: дієта — дефіцит калорій, висока ситість.".into(),
        (Goal::Diet,        Language::Pl) => "🎯 Cel: dieta — deficyt kaloryczny, wysoka sytość.".into(),
        (Goal::Diet,        _)            => "🎯 Goal: diet — calorie deficit, high satiety.".into(),

        (Goal::FlavorBoost, Language::Ru) => "🎯 Цель: усиление вкуса — заполняем слабые зоны.".into(),
        (Goal::FlavorBoost, Language::Uk) => "🎯 Ціль: підсилення смаку — заповнюємо слабкі зони.".into(),
        (Goal::FlavorBoost, Language::Pl) => "🎯 Cel: wzmocnienie smaku — uzupełniamy słabe strefy.".into(),
        (Goal::FlavorBoost, _)            => "🎯 Goal: flavor-boost — fill weak flavor dimensions.".into(),

        (Goal::Balanced, _) => String::new(),
    }
}

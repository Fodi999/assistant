// ─── AI SEO Enrichment — Gemini-powered recipe + text rewriting ─────────────
//
// Pipeline:
//   1. Build prompt via ChefPromptBuilder (with DishProfile constraints)
//   2. Call Gemini → parse JSON response
//   3. Post-process: inject pre-calculated nutrition into title/intro
//   4. Validate recipe via RecipeValidator (using DishProfile)
//   5. If validation fails → auto-fix second pass
//   6. Update DB

use crate::infrastructure::llm_adapter::LlmAdapter;
use crate::shared::{AppError, AppResult};
use sqlx::PgPool;
use uuid::Uuid;

use super::chef_prompt;
use super::dish_classifier;
use super::nutrition::NutritionTotals;
use super::recipe_validator;
use super::templates::smart_truncate;

/// Rewrite template-based SEO text into unique, Gemini-generated copy.
/// Called asynchronously after combo creation. Updates DB in place.
pub async fn enrich_seo_with_ai(
    pool: &PgPool,
    llm: &LlmAdapter,
    combo_id: Uuid,
    ingredients: &[String],
    locale: &str,
    goal: Option<&str>,
    meal_type: Option<&str>,
    dish_name: Option<&str>,
    model: &str,
    nt: &NutritionTotals,
) -> AppResult<()> {
    // ── Step 1: Classify dish ───────────────────────────────────────────
    let profile = if let Some(dn) = dish_name {
        dish_classifier::classify_dish(dn)
    } else {
        // Fallback: classify from ingredient names
        let combined = ingredients.join(" ");
        dish_classifier::classify_dish(&combined)
    };

    tracing::info!(
        "🧠 Dish classified: type={}, forming={}, liquid={}, oven={}, techniques={:?}",
        profile.dish_type,
        profile.requires_forming,
        profile.requires_liquid,
        profile.requires_oven,
        profile.allowed_techniques,
    );

    // ── Step 2: Build prompt ────────────────────────────────────────────
    let prompt = chef_prompt::build_chef_prompt(
        ingredients, locale, goal, meal_type, dish_name, &profile, nt,
    );

    let estimated_protein = nt.protein_per_serving;
    let estimated_calories = nt.calories_per_serving;

    tracing::info!(
        "🤖 AI enrichment for combo {} using model: {} (type: {}, protein: {:.0}g, calories: {:.0} kcal)",
        combo_id, model, profile.dish_type, estimated_protein, estimated_calories
    );

    // ── Step 3: Call Gemini ─────────────────────────────────────────────
    let raw_response = llm.groq_raw_request_with_model(&prompt, 3000, model).await?;

    let cleaned = raw_response
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    let enriched: serde_json::Value = serde_json::from_str(cleaned)
        .or_else(|_| {
            if let Some(start) = raw_response.find('{') {
                if let Some(end) = raw_response.rfind('}') {
                    return serde_json::from_str(&raw_response[start..=end]);
                }
            }
            Err(serde_json::Error::io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "No JSON found in AI response",
            )))
        })
        .map_err(|e| {
            tracing::warn!(
                "Failed to parse AI enrichment response: {} — raw: {}",
                e,
                &raw_response[..raw_response.len().min(500)]
            );
            AppError::internal("AI enrichment parse error")
        })?;

    // ── Step 4: Extract + post-process fields ───────────────────────────
    let mut title = enriched
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    let mut description = enriched
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    let h1 = enriched
        .get("h1")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim();
    let mut intro = enriched
        .get("intro")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    let why_it_works = enriched
        .get("why_it_works")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim();

    // ── PROTEIN SAFETY NET ──────────────────────────────────────────────
    let est_protein = estimated_protein.round() as i64;
    let _est_calories = estimated_calories.round() as i64;

    // Fix title: ensure protein present
    if !title.contains("Protein")
        && !title.contains("protein")
        && !title.contains("белка")
        && !title.contains("białka")
        && !title.contains("білка")
    {
        let protein_hook = format!("({}g Protein, 15 Min)", est_protein);
        if let Some(paren_start) = title.rfind('(') {
            title = title[..paren_start].trim().to_string();
        }
        title = smart_truncate(&format!("{} {}", title, protein_hook), 60);
        tracing::warn!(
            "⚠️ AI title missing protein for combo {} — injected {}g",
            combo_id, est_protein
        );
    }

    // Fix title: replace 0g with real value
    if title.contains("0g Protein") || title.contains("0 g Protein") {
        title = title.replace("0g Protein", &format!("{}g Protein", est_protein));
        title = title.replace("0 g Protein", &format!("{}g Protein", est_protein));
        tracing::warn!(
            "⚠️ AI returned 0g protein in title for combo {} — fixed to {}g",
            combo_id, est_protein
        );
    }

    // Fix intro: replace 0g
    if intro.contains("~0g") || intro.contains("0g protein") || intro.contains("0 g protein") {
        intro = intro.replace("~0g", &format!("~{}g", est_protein));
        intro = intro.replace("0g protein", &format!("{}g protein", est_protein));
        intro = intro.replace("0 g protein", &format!("{}g protein", est_protein));
        tracing::warn!(
            "⚠️ AI returned 0g protein in intro for combo {} — fixed to {}g",
            combo_id, est_protein
        );
    }

    // Fix description: ensure protein mentioned
    if !description.contains("protein")
        && !description.contains("белка")
        && !description.contains("białka")
        && !description.contains("білка")
        && est_protein > 5
    {
        description = format!(
            "{}. {}g protein per serving.",
            description.trim_end_matches('.'),
            est_protein
        );
        if description.len() > 155 {
            description = description.chars().take(152).collect::<String>() + "...";
        }
    }

    tracing::info!(
        "✅ AI enrichment result — title: '{}', protein: {}g, model: {}",
        title, est_protein, model
    );

    // ── Step 5: Validate + auto-fix recipe ──────────────────────────────
    let mut ai_steps = enriched.get("how_to_cook").cloned();

    if let Some(ref steps_val) = ai_steps {
        let validation = recipe_validator::validate_recipe(
            &profile, steps_val, dish_name, ingredients,
        );

        if !validation.is_valid {
            tracing::warn!(
                "⚠️ Recipe validation FAILED for combo {} — {} problems: {:?}. Attempting auto-fix...",
                combo_id,
                validation.problems.len(),
                validation.problems,
            );

            // Collect step text for fix prompt
            let all_step_text: String = steps_val
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|s| s.get("text").and_then(|t| t.as_str()))
                        .collect::<Vec<_>>()
                        .join(" ")
                })
                .unwrap_or_default();

            let names = ingredients.join(", ");
            let lang = match locale {
                "ru" => "Russian",
                "pl" => "Polish",
                "uk" => "Ukrainian",
                _ => "English",
            };

            let fix_prompt = recipe_validator::build_fix_prompt(
                &validation.problems,
                &all_step_text,
                dish_name,
                &names,
                lang,
                &profile,
            );

            match llm.groq_raw_request_with_model(&fix_prompt, 2000, model).await {
                Ok(fix_response) => {
                    let fix_cleaned = fix_response
                        .trim()
                        .trim_start_matches("```json")
                        .trim_start_matches("```")
                        .trim_end_matches("```")
                        .trim();

                    if let Ok(fixed_steps) =
                        serde_json::from_str::<serde_json::Value>(fix_cleaned).or_else(|_| {
                            if let Some(start) = fix_response.find('[') {
                                if let Some(end) = fix_response.rfind(']') {
                                    return serde_json::from_str(&fix_response[start..=end]);
                                }
                            }
                            Err(serde_json::Error::io(std::io::Error::new(
                                std::io::ErrorKind::InvalidData,
                                "no JSON",
                            )))
                        })
                    {
                        if fixed_steps.is_array()
                            && fixed_steps.as_array().unwrap().len() >= profile.min_steps
                        {
                            ai_steps = Some(fixed_steps);
                            tracing::info!(
                                "✅ Auto-fix second pass SUCCESS for combo {} — steps fixed",
                                combo_id
                            );
                        } else {
                            tracing::warn!(
                                "⚠️ Auto-fix returned invalid steps for combo {} — keeping original",
                                combo_id
                            );
                        }
                    } else {
                        tracing::warn!(
                            "⚠️ Auto-fix parse failed for combo {} — keeping original",
                            combo_id
                        );
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        "⚠️ Auto-fix request failed for combo {}: {} — keeping original",
                        combo_id, e
                    );
                }
            }
        } else {
            tracing::info!(
                "✅ Recipe validation PASSED for combo {} — type: {}, steps: {}, forming: {}",
                combo_id,
                profile.dish_type,
                steps_val.as_array().map(|a| a.len()).unwrap_or(0),
                if profile.requires_forming { "✓" } else { "n/a" },
            );
        }
    }

    // ── Step 6: Update DB ───────────────────────────────────────────────
    if title.is_empty() && description.is_empty() {
        tracing::warn!("AI enrichment returned empty fields for combo {}", combo_id);
        return Ok(());
    }

    if let Some(steps) = ai_steps {
        if steps.is_array() && !steps.as_array().unwrap().is_empty() {
            sqlx::query(
                r#"UPDATE lab_combo_pages SET
                    title = CASE WHEN $1 != '' THEN $1 ELSE title END,
                    description = CASE WHEN $2 != '' THEN $2 ELSE description END,
                    h1 = CASE WHEN $3 != '' THEN $3 ELSE h1 END,
                    intro = CASE WHEN $4 != '' THEN $4 ELSE intro END,
                    why_it_works = CASE WHEN $5 != '' THEN $5 ELSE why_it_works END,
                    how_to_cook = $7,
                    updated_at = NOW()
                WHERE id = $6"#,
            )
            .bind(title)
            .bind(description)
            .bind(h1)
            .bind(intro)
            .bind(why_it_works)
            .bind(combo_id)
            .bind(&steps)
            .execute(pool)
            .await?;

            tracing::info!(
                "✅ AI-enriched SEO + cooking steps for combo {}",
                combo_id
            );
            return Ok(());
        }
    }

    // Fallback: update only text fields
    sqlx::query(
        r#"UPDATE lab_combo_pages SET
            title = CASE WHEN $1 != '' THEN $1 ELSE title END,
            description = CASE WHEN $2 != '' THEN $2 ELSE description END,
            h1 = CASE WHEN $3 != '' THEN $3 ELSE h1 END,
            intro = CASE WHEN $4 != '' THEN $4 ELSE intro END,
            why_it_works = CASE WHEN $5 != '' THEN $5 ELSE why_it_works END,
            updated_at = NOW()
        WHERE id = $6"#,
    )
    .bind(title)
    .bind(description)
    .bind(h1)
    .bind(intro)
    .bind(why_it_works)
    .bind(combo_id)
    .execute(pool)
    .await?;

    tracing::info!("✅ AI-enriched SEO content for combo {}", combo_id);
    Ok(())
}

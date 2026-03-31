// ─── RecipeGenerator — AI generation pipeline ───────────────────────────────
//
// Single responsibility: orchestrate AI recipe generation.
// Uses DishClassifier + ChefPrompt + Recipe domain aggregate + Metrics.
//
// Pipeline:
//   1. Classify dish → DishProfile
//   2. Build chef prompt (with constraints)
//   3. Call Gemini
//   4. Parse response → try Recipe::new() (invariants enforced!)
//   5. If invariant fails → auto-fix second pass → Recipe::new() again
//   6. Post-process SEO fields (protein safety net)
//   7. Update DB via repository

use crate::infrastructure::llm_adapter::LlmAdapter;
use crate::shared::{AppError, AppResult};
use uuid::Uuid;

use super::chef_prompt;
use super::dish_classifier;
use super::metrics::{self, GenerationTimer};
use super::nutrition::NutritionTotals;
use super::recipe::{Recipe, RecipeInvariantError};
use super::recipe_validator;
use super::repository::ComboRepository;
use super::seo::helpers::smart_truncate;

/// Rewrite template-based SEO text into unique, Gemini-generated copy.
/// Called asynchronously after combo creation. Updates DB in place.
pub async fn enrich_with_ai(
    repo: &ComboRepository,
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
        let combined = ingredients.join(" ");
        dish_classifier::classify_dish(&combined)
    };

    let timer = GenerationTimer::start(
        profile.type_label,
        locale,
        model,
    );

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
        "🤖 AI enrichment for combo {} using model: {} (type: {}, protein: {:.0}g, calories: {:.0} kcal, prompt_chars: {})",
        combo_id, model, profile.dish_type, estimated_protein, estimated_calories, prompt.len()
    );

    // ── Step 3: Call Gemini ─────────────────────────────────────────────
    // max_tokens=4096 — the prompt is large (type-specific instructions + nutrition),
    // 3000 was causing truncated JSON responses (EOF errors).
    metrics::record_ai_call(true, model);
    let raw_response = match llm.groq_raw_request_with_model(&prompt, 4096, model).await {
        Ok(r) => {
            tracing::info!(
                "📥 AI response for combo {} — {} chars (model: {})",
                combo_id, r.len(), model
            );
            r
        }
        Err(e) => {
            metrics::record_ai_call(false, model);
            timer.failure(&format!("AI call failed: {}", e));
            return Err(e);
        }
    };

    let cleaned = raw_response
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    let parse_result: Result<serde_json::Value, _> = serde_json::from_str(cleaned)
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
        });

    let enriched = match parse_result {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(
                "Failed to parse AI enrichment response: {} — raw: {}",
                e,
                &raw_response[..raw_response.len().min(500)]
            );
            timer.failure("JSON parse error");
            return Err(AppError::internal("AI enrichment parse error"));
        }
    };

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
        .trim()
        .to_string();
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
        .trim()
        .to_string();

    // ── PROTEIN SAFETY NET ──────────────────────────────────────────────
    let est_protein = estimated_protein.round() as i64;
    let _est_calories = estimated_calories.round() as i64;

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
    }

    if title.contains("0g Protein") || title.contains("0 g Protein") {
        title = title.replace("0g Protein", &format!("{}g Protein", est_protein));
        title = title.replace("0 g Protein", &format!("{}g Protein", est_protein));
    }

    if intro.contains("~0g") || intro.contains("0g protein") || intro.contains("0 g protein") {
        intro = intro.replace("~0g", &format!("~{}g", est_protein));
        intro = intro.replace("0g protein", &format!("{}g protein", est_protein));
        intro = intro.replace("0 g protein", &format!("{}g protein", est_protein));
    }

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

    // ── Step 5: Validate recipe via Domain Invariants ────────────────────
    let mut fix_count: u32 = 0;
    let mut final_recipe: Option<Recipe> = None;

    if let Some(steps_val) = enriched.get("how_to_cook") {
        match Recipe::new(steps_val, &profile, ingredients) {
            Ok(recipe) => {
                metrics::record_validation(true, profile.type_label, &[]);
                tracing::info!(
                    "✅ Recipe invariants PASSED for combo {} — score: {}, confidence: {:.2}, verdict: {}",
                    combo_id, recipe.quality.score, recipe.quality.confidence, recipe.quality.verdict
                );
                final_recipe = Some(recipe);
            }
            Err(err) => {
                metrics::record_validation(false, profile.type_label, &err.violations);
                tracing::warn!(
                    "⚠️ Recipe invariants FAILED for combo {} — {} violations: {:?}. Auto-fixing...",
                    combo_id, err.violations.len(), err.violations,
                );

                // ── Auto-fix pass ───────────────────────────────────────
                final_recipe = attempt_fix(
                    llm, &err, steps_val, dish_name, ingredients, locale, &profile, model,
                ).await;
                fix_count = 1;

                if final_recipe.is_some() {
                    metrics::record_fix_attempt(true, profile.type_label);
                } else {
                    metrics::record_fix_attempt(false, profile.type_label);
                }
            }
        }
    }

    // ── Step 6: Update DB ───────────────────────────────────────────────
    if title.is_empty() && description.is_empty() {
        timer.failure("empty AI response");
        return Ok(());
    }

    if let Some(recipe) = &final_recipe {
        let steps_json = recipe.steps_json();
        if steps_json.is_array() && !steps_json.as_array().unwrap().is_empty() {
            repo.update_seo_with_steps(
                combo_id, &title, &description, &h1, &intro, &why_it_works, &steps_json,
            ).await?;

            timer.success(recipe.quality.score, recipe.quality.confidence, fix_count);
            tracing::info!(
                "✅ AI-enriched SEO + cooking steps for combo {} (quality: {}/100)",
                combo_id, recipe.quality.score
            );
            return Ok(());
        }
    }

    // Fallback: update only text fields
    repo.update_seo_text(
        combo_id, &title, &description, &h1, &intro, &why_it_works,
    ).await?;

    if let Some(recipe) = &final_recipe {
        timer.success(recipe.quality.score, recipe.quality.confidence, fix_count);
    } else {
        timer.success(50, 0.5, fix_count); // partial success — text only
    }

    tracing::info!("✅ AI-enriched SEO content for combo {}", combo_id);
    Ok(())
}

/// Attempt to auto-fix a recipe that failed invariant checks.
async fn attempt_fix(
    llm: &LlmAdapter,
    err: &RecipeInvariantError,
    _original_steps: &serde_json::Value,
    dish_name: Option<&str>,
    ingredients: &[String],
    locale: &str,
    profile: &dish_classifier::DishProfile,
    model: &str,
) -> Option<Recipe> {
    let all_step_text: String = err.raw_steps
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
        &err.violations,
        &all_step_text,
        dish_name,
        &names,
        lang,
        profile,
    );

    metrics::record_ai_call(true, model);
    match llm.groq_raw_request_with_model(&fix_prompt, 2000, model).await {
        Ok(fix_response) => {
            let fix_cleaned = fix_response
                .trim()
                .trim_start_matches("```json")
                .trim_start_matches("```")
                .trim_end_matches("```")
                .trim();

            let fixed_value = serde_json::from_str::<serde_json::Value>(fix_cleaned)
                .or_else(|_| {
                    if let Some(start) = fix_response.find('[') {
                        if let Some(end) = fix_response.rfind(']') {
                            return serde_json::from_str(&fix_response[start..=end]);
                        }
                    }
                    Err(serde_json::Error::io(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "no JSON",
                    )))
                });

            match fixed_value {
                Ok(val) => {
                    match Recipe::new(&val, profile, ingredients) {
                        Ok(recipe) => {
                            tracing::info!(
                                "✅ Auto-fix SUCCESS — score: {}, verdict: {}",
                                recipe.quality.score, recipe.quality.verdict
                            );
                            Some(recipe)
                        }
                        Err(fix_err) => {
                            tracing::warn!(
                                "⚠️ Auto-fix still invalid: {:?}",
                                fix_err.violations
                            );
                            None
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("⚠️ Auto-fix JSON parse failed: {}", e);
                    None
                }
            }
        }
        Err(e) => {
            metrics::record_ai_call(false, model);
            tracing::warn!("⚠️ Auto-fix AI call failed: {}", e);
            None
        }
    }
}

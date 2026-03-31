// ─── RecipeGenerator — Split AI Pipeline ────────────────────────────────────
//
// KEY DESIGN: Two small focused AI calls instead of one big call.
// The old approach requested ALL fields in one JSON → Gemini ran out of
// output tokens → JSON truncated → parse error → fell back to garbage template.
//
// NEW PIPELINE:
//   1. Classify dish → DishProfile
//   2. CALL 1: Generate cooking steps ONLY (JSON array, ~1500 tokens max)
//   3. Validate steps → Recipe::new() → auto-fix if needed
//   4. CALL 2: Generate SEO texts ONLY (JSON object, ~800 tokens max)
//   5. Post-process SEO (protein safety net)
//   6. Update DB with both results

use crate::infrastructure::llm_adapter::LlmAdapter;
use crate::shared::AppResult;
use uuid::Uuid;

use super::chef_prompt;
use super::dish_classifier;
use super::metrics::{self, GenerationTimer};
use super::nutrition::NutritionTotals;
use super::recipe::{Recipe, RecipeInvariantError};
use super::recipe_validator;
use super::repository::ComboRepository;
use super::seo::helpers::smart_truncate;

/// Safe UTF-8 truncation for log output. Never panics on multi-byte chars.
fn safe_preview(s: &str, max: usize) -> &str {
    if s.len() <= max {
        return s;
    }
    let mut end = max;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

/// Enrich combo page with AI-generated cooking steps and SEO text.
///
/// Split into two focused AI calls to avoid Gemini output truncation.
pub async fn enrich_with_ai(
    repo: &ComboRepository,
    llm: &LlmAdapter,
    combo_id: Uuid,
    ingredients: &[String],
    locale: &str,
    _goal: Option<&str>,
    _meal_type: Option<&str>,
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

    let timer = GenerationTimer::start(profile.type_label, locale, model);

    tracing::info!(
        "🧠 Dish classified: type={}, forming={}, liquid={}, oven={}, techniques={:?}",
        profile.dish_type, profile.requires_forming, profile.requires_liquid,
        profile.requires_oven, profile.allowed_techniques,
    );

    let estimated_protein = nt.protein_per_serving;
    let _estimated_calories = nt.calories_per_serving;

    // ── CALL 1: Cooking steps ───────────────────────────────────────────
    let steps_prompt = chef_prompt::build_steps_prompt(
        ingredients, locale, dish_name, &profile,
    );

    tracing::info!(
        "🔪 CALL 1 (steps) for combo {} — model: {}, prompt: {} chars",
        combo_id, model, steps_prompt.len()
    );

    metrics::record_ai_call(true, model);
    let steps_raw = match llm.groq_raw_request_with_model(&steps_prompt, 2000, model).await {
        Ok(r) => {
            tracing::info!("📥 Steps response: {} chars — preview: {}", r.len(), safe_preview(&r, 200));
            r
        }
        Err(e) => {
            metrics::record_ai_call(false, model);
            timer.failure(&format!("Steps AI call failed: {}", e));
            return Err(e);
        }
    };

    let steps_value = parse_json_array(&steps_raw);

    // ── Structural guard: validate skeleton compliance BEFORE Recipe::new() ──
    // If AI returned fewer steps than skeleton requires, or skipped forming,
    // we catch it here and trigger auto-fix immediately.
    let steps_value = if let Some(val) = steps_value {
        let arr = val.as_array();
        let step_count = arr.map(|a| a.len()).unwrap_or(0);
        let has_forming = arr.map(|a| {
            a.iter().any(|s| s.get("type").and_then(|t| t.as_str()) == Some("forming"))
        }).unwrap_or(false);

        if step_count < profile.min_steps {
            tracing::warn!(
                "⚠️ Skeleton guard: got {} steps, need ≥{} for {} — triggering re-generation",
                step_count, profile.min_steps, profile.type_label
            );
            None // force auto-fix path
        } else if profile.requires_forming && !has_forming {
            tracing::warn!(
                "⚠️ Skeleton guard: forming required but missing in {} steps — triggering re-generation",
                step_count
            );
            None // force auto-fix path
        } else {
            tracing::info!(
                "✅ Skeleton guard OK: {} steps, forming_present={} for {}",
                step_count, has_forming, profile.type_label
            );
            Some(val)
        }
    } else {
        tracing::warn!(
            "⚠️ Steps JSON parse failed for combo {} — FULL raw ({} chars): {}",
            combo_id, steps_raw.len(), &steps_raw
        );
        None
    };

    let mut fix_count: u32 = 0;
    let final_recipe: Option<Recipe> = match steps_value {
        Some(val) => {
            match Recipe::new(&val, &profile, ingredients) {
                Ok(recipe) => {
                    metrics::record_validation(true, profile.type_label, &[]);
                    tracing::info!(
                        "✅ Recipe PASSED for combo {} — score: {}, confidence: {:.2}, verdict: {}",
                        combo_id, recipe.quality.score, recipe.quality.confidence, recipe.quality.verdict
                    );
                    Some(recipe)
                }
                Err(err) => {
                    metrics::record_validation(false, profile.type_label, &err.violations);
                    tracing::warn!(
                        "⚠️ Recipe FAILED for combo {} — {:?}. Auto-fixing...",
                        combo_id, err.violations,
                    );
                    fix_count = 1;
                    let fixed = attempt_fix(
                        llm, &err, &val, dish_name, ingredients, locale, &profile, model,
                    ).await;
                    if fixed.is_some() {
                        metrics::record_fix_attempt(true, profile.type_label);
                    } else {
                        metrics::record_fix_attempt(false, profile.type_label);
                    }
                    fixed
                }
            }
        }
        None => {
            // Parse failed or skeleton guard rejected — attempt fix with a fresh call
            tracing::warn!(
                "⚠️ Steps rejected for combo {} — attempting forced re-generation",
                combo_id,
            );
            fix_count = 1;
            let dummy_err = crate::application::lab_combos::recipe::RecipeInvariantError {
                violations: vec!["parse_failed_or_skeleton_guard".to_string()],
                raw_steps: serde_json::json!([]),
            };
            let fixed = attempt_fix(
                llm, &dummy_err, &serde_json::json!([]), dish_name, ingredients, locale, &profile, model,
            ).await;
            if fixed.is_some() {
                metrics::record_fix_attempt(true, profile.type_label);
            } else {
                metrics::record_fix_attempt(false, profile.type_label);
            }
            fixed
        }
    };

    // ── CALL 2: SEO texts ───────────────────────────────────────────────
    let seo_prompt = chef_prompt::build_seo_prompt(
        ingredients, locale, dish_name, nt,
    );

    tracing::info!(
        "📝 CALL 2 (SEO) for combo {} — prompt: {} chars",
        combo_id, seo_prompt.len()
    );

    metrics::record_ai_call(true, model);
    // 2500 tokens — cyrillic text requires ~2-3 tokens per word vs ~1 for English
    let seo_raw = match llm.groq_raw_request_with_model(&seo_prompt, 2500, model).await {
        Ok(r) => {
            tracing::info!("📥 SEO response: {} chars", r.len());
            r
        }
        Err(e) => {
            metrics::record_ai_call(false, model);
            tracing::warn!("⚠️ SEO AI call failed: {} — saving steps only", e);
            // Even if SEO call fails, save steps if we have them
            if let Some(recipe) = &final_recipe {
                let steps_json = recipe.steps_json();
                if steps_json.is_array() && !steps_json.as_array().unwrap().is_empty() {
                    repo.update_seo_with_steps(
                        combo_id, "", "", "", "", "", &steps_json,
                    ).await?;
                    repo.update_quality_score(combo_id, recipe.quality.score as i16).await?;
                    timer.success(recipe.quality.score, recipe.quality.confidence, fix_count);
                    return Ok(());
                }
            }
            timer.failure(&format!("SEO AI call failed: {}", e));
            return Err(e);
        }
    };

    let seo_obj = parse_json_object(&seo_raw);

    // ── Extract SEO fields ──────────────────────────────────────────────
    let (mut title, mut description, h1, mut intro, why_it_works) = if let Some(obj) = &seo_obj {
        (
            obj.get("title").and_then(|v| v.as_str()).unwrap_or("").trim().to_string(),
            obj.get("description").and_then(|v| v.as_str()).unwrap_or("").trim().to_string(),
            obj.get("h1").and_then(|v| v.as_str()).unwrap_or("").trim().to_string(),
            obj.get("intro").and_then(|v| v.as_str()).unwrap_or("").trim().to_string(),
            obj.get("why_it_works").and_then(|v| v.as_str()).unwrap_or("").trim().to_string(),
        )
    } else {
        tracing::warn!("⚠️ SEO JSON parse failed — raw: {}", safe_preview(&seo_raw, 500));
        (String::new(), String::new(), String::new(), String::new(), String::new())
    };

    // ── Protein safety net ──────────────────────────────────────────────
    let est_protein = estimated_protein.round() as i64;

    if !title.is_empty()
        && !title.contains("Protein") && !title.contains("protein")
        && !title.contains("белка") && !title.contains("białka") && !title.contains("білка")
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

    if !description.is_empty()
        && !description.contains("protein") && !description.contains("белка")
        && !description.contains("białka") && !description.contains("білка")
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

    // ── Update DB ───────────────────────────────────────────────────────
    if let Some(recipe) = &final_recipe {
        let steps_json = recipe.steps_json();
        if steps_json.is_array() && !steps_json.as_array().unwrap().is_empty() {
            repo.update_seo_with_steps(
                combo_id, &title, &description, &h1, &intro, &why_it_works, &steps_json,
            ).await?;

            // Persist the recipe quality score (0-100) to the DB column
            repo.update_quality_score(combo_id, recipe.quality.score as i16).await?;

            timer.success(recipe.quality.score, recipe.quality.confidence, fix_count);
            tracing::info!(
                "✅ AI-enriched steps + SEO for combo {} (quality: {}/100)",
                combo_id, recipe.quality.score
            );
            return Ok(());
        }
    }

    // Fallback: only SEO texts (steps generation failed)
    if !title.is_empty() || !description.is_empty() {
        repo.update_seo_text(
            combo_id, &title, &description, &h1, &intro, &why_it_works,
        ).await?;
        timer.success(50, 0.5, fix_count);
        tracing::info!("✅ AI-enriched SEO only for combo {} (steps failed)", combo_id);
    } else {
        timer.failure("both calls produced empty results");
        tracing::warn!("⚠️ AI enrichment produced nothing for combo {}", combo_id);
    }

    Ok(())
}

// ── JSON parsing helpers ────────────────────────────────────────────────────

/// Parse a raw AI response as a JSON array (for cooking steps).
fn parse_json_array(raw: &str) -> Option<serde_json::Value> {
    let cleaned = raw
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    // Try direct parse
    if let Ok(val) = serde_json::from_str::<serde_json::Value>(cleaned) {
        if val.is_array() {
            return Some(val);
        }
        // Gemini sometimes wraps the array in an object with various keys
        for key in &["how_to_cook", "steps", "cooking_steps", "recipe_steps", "instructions"] {
            if let Some(arr) = val.get(key) {
                if arr.is_array() {
                    return Some(arr.clone());
                }
            }
        }
    }

    // Try extracting [...] from the response
    if let Some(start) = raw.find('[') {
        if let Some(end) = raw.rfind(']') {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&raw[start..=end]) {
                if val.is_array() {
                    return Some(val);
                }
            }
        }
    }

    None
}

/// Parse a raw AI response as a JSON object (for SEO fields).
fn parse_json_object(raw: &str) -> Option<serde_json::Value> {
    let cleaned = raw
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    if let Ok(val) = serde_json::from_str::<serde_json::Value>(cleaned) {
        if val.is_object() {
            return Some(val);
        }
    }

    if let Some(start) = raw.find('{') {
        if let Some(end) = raw.rfind('}') {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&raw[start..=end]) {
                if val.is_object() {
                    return Some(val);
                }
            }
        }
    }

    None
}

// ── Auto-fix pipeline ───────────────────────────────────────────────────────

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

    tracing::info!("🔧 Auto-fix call — prompt: {} chars", fix_prompt.len());

    metrics::record_ai_call(true, model);
    match llm.groq_raw_request_with_model(&fix_prompt, 2000, model).await {
        Ok(fix_response) => {
            match parse_json_array(&fix_response) {
                Some(val) => {
                    match Recipe::new(&val, profile, ingredients) {
                        Ok(recipe) => {
                            tracing::info!(
                                "✅ Auto-fix SUCCESS — score: {}, verdict: {}",
                                recipe.quality.score, recipe.quality.verdict
                            );
                            Some(recipe)
                        }
                        Err(fix_err) => {
                            tracing::warn!("⚠️ Auto-fix still invalid: {:?}", fix_err.violations);
                            None
                        }
                    }
                }
                None => {
                    tracing::warn!(
                        "⚠️ Auto-fix JSON parse failed — raw: {}",
                        safe_preview(&fix_response, 300)
                    );
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

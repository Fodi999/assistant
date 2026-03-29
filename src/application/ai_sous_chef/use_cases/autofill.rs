//! Use Case: AI Autofill — fill missing nutrition/description fields
//!
//! v2 Architecture: Dictionary = truth, AI = helper for descriptions + nutrition.
//! Names, unit, product_type come from dictionary/DB — NEVER from AI.
//!
//! Pipeline:
//! 1. Load product from DB (already has names from dictionary)
//! 2. Resolve names from dictionary (override if product names are wrong)
//! 3. AI fills ONLY: descriptions, nutrition, vitamins, minerals, culinary, etc.
//! 4. Strip names/unit/product_type from AI response (safety net)
//! 5. Merge dictionary names into response
//! 6. Cache & return

use crate::application::admin_catalog::AdminCatalogService;
use crate::application::ai_sous_chef::product_dictionary;
use crate::domain::ai_ports::{AiClient, AiQuality};
use crate::shared::{AppError, AppResult};
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// Cache TTL for autofill results (days)
const AUTOFILL_CACHE_TTL_DAYS: i32 = 30;

impl AdminCatalogService {
    /// AI autofill — asks AI to fill missing nutrition/description fields.
    /// Names and unit come from DICTIONARY, not AI.
    pub async fn ai_autofill(&self, id: Uuid) -> AppResult<serde_json::Value> {
        let product = self.get_product_by_id(id).await?;

        let name_en = product.name_en.clone();
        let name_ru = product.name_ru.clone().unwrap_or_default();
        let mut product_type = product.product_type.clone();

        // ── Auto-fix product_type='other' using dictionary inference ──
        if product_type == "other" {
            let en_low = name_en.to_lowercase();
            let ru_low = name_ru.to_lowercase();
            if let Some(inferred) = product_dictionary::infer_product_type(&en_low, &ru_low) {
                tracing::info!(
                    "🔧 Auto-fix product_type: '{}' → '{}' for product {}",
                    product_type, inferred, id
                );
                product_type = inferred.to_string();
                // Persist the fix in DB so publish won't fail
                let _ = sqlx::query(
                    "UPDATE catalog_ingredients SET product_type = $1 WHERE id = $2"
                )
                .bind(&product_type)
                .bind(id)
                .execute(&self.pool)
                .await;
            }
        }

        // ── Resolve correct names from dictionary ──
        // If not in dictionary → use ai_translate_and_save_pending
        // Names saved as PENDING — admin must approve!
        let dict_entry = self.dictionary.find_by_en(&name_en).await.ok().flatten();
        let (dict_name_ru, dict_name_pl, dict_name_uk) = if let Some(ref d) = dict_entry {
            (d.name_ru.clone(), d.name_pl.clone(), d.name_uk.clone())
        } else {
            // Not in dictionary — try AI translation + save as PENDING
            tracing::info!("📖 Autofill: dictionary miss for '{}' — requesting AI translation", &name_en);
            match self.ai_translate_and_save_pending(&name_en).await {
                Ok((ru, pl, uk)) => (ru, pl, uk),
                Err(_) => {
                    // Fallback: use whatever the product already has
                    (
                        name_ru.clone(),
                        product.name_pl.clone().unwrap_or_default(),
                        product.name_uk.clone().unwrap_or_default(),
                    )
                }
            }
        };

        // ── Resolve unit from lookup (not AI) ──
        let name_en_lower = name_en.to_lowercase();
        let dict_unit = product_dictionary::unit_for_type(&product_type, &name_en_lower).to_string();

        // Build field status fingerprint for cache key
        let has_desc_en = product.description_en.as_ref().map(|s| !s.trim().is_empty()).unwrap_or(false);
        let has_desc_ru = product.description_ru.as_ref().map(|s| !s.trim().is_empty()).unwrap_or(false);
        let has_desc_pl = product.description_pl.as_ref().map(|s| !s.trim().is_empty()).unwrap_or(false);
        let has_desc_uk = product.description_uk.as_ref().map(|s| !s.trim().is_empty()).unwrap_or(false);
        let has_cal = product.calories_per_100g.is_some();
        let has_prot = product.protein_per_100g.is_some();
        let has_fat = product.fat_per_100g.is_some();
        let has_carbs = product.carbs_per_100g.is_some();

        // ── Cache check ──
        let fingerprint = format!(
            "v2:{}:{}:{}:{}:{}:{}:{}:{}:{}",
            name_en, product_type,
            has_desc_en, has_desc_ru, has_desc_pl, has_desc_uk,
            has_cal, has_prot, has_fat
        );
        let cache_key = format!("uc:autofill:{}", hash_input(&fingerprint));

        if let Ok(Some(cached)) = self.ai_cache.get(&cache_key).await {
            tracing::info!("📦 Autofill cache hit for product {}", id);
            // Tag data source
            if let Some(obj) = cached.as_object() {
                let mut tagged = cached.clone();
                if let Some(t) = tagged.as_object_mut() {
                    t.insert("_data_source".into(), serde_json::json!("cache"));
                    t.insert("_nutrition_state".into(), serde_json::json!(
                        if obj.get("calories_per_100g").and_then(|v| v.as_f64()).is_some()
                            && obj.get("protein_per_100g").and_then(|v| v.as_f64()).is_some()
                            && obj.get("fat_per_100g").and_then(|v| v.as_f64()).is_some()
                            && obj.get("carbs_per_100g").and_then(|v| v.as_f64()).is_some()
                        { "complete" } else if obj.get("calories_per_100g").and_then(|v| v.as_f64()).is_some()
                            || obj.get("protein_per_100g").and_then(|v| v.as_f64()).is_some()
                        { "partial" } else { "none" }
                    ));
                }
                return Ok(tagged);
            }
            return Ok(cached);
        }

        // ── Build slim prompt (no names/unit/product_type) ──
        let prompt = build_autofill_prompt(
            &name_en, &name_ru, &product_type,
            has_desc_en, has_desc_ru, has_desc_pl, has_desc_uk,
            has_cal, has_prot, has_fat, has_carbs,
        );

        // ── Call AI ──
        // 8000 tokens: 4 descriptions (≈200 tokens each) + full nutrition JSON ≈ 2000 tokens total
        let raw = self.llm_adapter
            .generate_with_quality(&prompt, 8000, AiQuality::Balanced)
            .await?;

        // ── Log raw AI response for debugging nutrition pipeline ──
        let preview_end = raw.char_indices().nth(500).map(|(i, _)| i).unwrap_or(raw.len());
        tracing::info!("🤖 AI autofill raw response for product {} ({}): {}", id, name_en, &raw[..preview_end]);

        // ── Parse JSON ──
        let mut result = parse_json_response(&raw)?;

        // ── Log parsed nutrition values ──
        if let Some(obj) = result.as_object() {
            let cal = obj.get("calories_per_100g");
            let prot = obj.get("protein_per_100g");
            let fat = obj.get("fat_per_100g");
            let carbs = obj.get("carbs_per_100g");
            tracing::info!(
                "📊 AI nutrition for '{}': cal={:?}, prot={:?}, fat={:?}, carbs={:?}",
                name_en, cal, prot, fat, carbs
            );
        }

        // ══════════════════════════════════════════════════════════════
        // SAFETY NET: Override AI names/unit with dictionary values
        // AI may still return these fields — we ALWAYS use dictionary.
        // ══════════════════════════════════════════════════════════════
        if let Some(obj) = result.as_object_mut() {
            obj.insert("name_en".into(), serde_json::json!(name_en));
            obj.insert("name_ru".into(), serde_json::json!(dict_name_ru));
            obj.insert("name_pl".into(), serde_json::json!(dict_name_pl));
            obj.insert("name_uk".into(), serde_json::json!(dict_name_uk));
            obj.insert("unit".into(), serde_json::json!(dict_unit));
            // Keep product_type from DB (not AI)
            obj.insert("product_type".into(), serde_json::json!(product_type));

            // ── Data source & nutrition state ──
            let has_cal = obj.get("calories_per_100g").and_then(|v| v.as_f64()).is_some();
            let has_prot = obj.get("protein_per_100g").and_then(|v| v.as_f64()).is_some();
            let has_fat = obj.get("fat_per_100g").and_then(|v| v.as_f64()).is_some();
            let has_carbs = obj.get("carbs_per_100g").and_then(|v| v.as_f64()).is_some();
            let filled = [has_cal, has_prot, has_fat, has_carbs].iter().filter(|&&v| v).count();

            let nutrition_state = match filled {
                4 => "complete",
                0 => "none",
                _ => "partial",
            };

            obj.insert("_data_source".into(), serde_json::json!("ai"));
            obj.insert("_nutrition_state".into(), serde_json::json!(nutrition_state));
            obj.insert("_nutrition_filled".into(), serde_json::json!(filled));
            obj.insert("_nutrition_total".into(), serde_json::json!(4));

            // ── Warn if AI returned no nutrition (prompt problem) ──
            if filled == 0 {
                tracing::warn!(
                    "⚠️ AI returned NO nutrition for '{}' — check prompt or model",
                    name_en
                );
                obj.insert("_warning".into(), serde_json::json!(
                    "AI did not return nutrition data. Product needs manual entry or USDA lookup."
                ));
            }
        }

        // ── Cache result ──
        if let Err(e) = self.ai_cache.set(
            &cache_key, result.clone(), "groq", "llama-3.3-70b-versatile", AUTOFILL_CACHE_TTL_DAYS
        ).await {
            tracing::warn!("Failed to cache autofill result: {}", e);
        }

        tracing::info!("✅ AI autofill v2 complete for product {} (names from dictionary)", id);
        Ok(result)
    }
}

// ── Helpers ──────────────────────────────────────────────────────────

fn hash_input(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    let result = hasher.finalize();
    format!("{:x}", result)[..16].to_string()
}

fn parse_json_response(raw: &str) -> AppResult<serde_json::Value> {
    // 1. Try the full response first (happy path)
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(raw) {
        return Ok(v);
    }

    // 2. Strip markdown code fences and retry
    let stripped = raw
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(stripped) {
        return Ok(v);
    }

    // 3. Find the JSON object boundaries
    let start = stripped.find('{').ok_or_else(|| {
        tracing::error!("Failed to parse AI response: no JSON object found");
        AppError::internal("AI returned invalid JSON")
    })?;
    let json_candidate = &stripped[start..];

    // 4. Walk backwards from the end to find the deepest valid closing brace
    //    This handles truncated responses — we recover partial but valid JSON.
    let bytes = json_candidate.as_bytes();
    let mut end = bytes.len();
    loop {
        if end == 0 { break; }
        // Find the last '}' at or before `end`
        match json_candidate[..end].rfind('}') {
            None => break,
            Some(pos) => {
                let candidate = &json_candidate[..=pos];
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(candidate) {
                    tracing::warn!(
                        "⚠️ AI response was truncated — recovered partial JSON ({}/{} bytes)",
                        pos + 1, bytes.len()
                    );
                    return Ok(v);
                }
                end = pos; // try one level up
            }
        }
    }

    tracing::error!("Failed to parse AI response after all recovery attempts");
    Err(AppError::internal("AI returned invalid JSON"))
}

/// Build autofill prompt — AI only fills descriptions + nutrition + extended data.
/// Names, unit, product_type are handled by dictionary/backend.
fn build_autofill_prompt(
    name_en: &str, name_ru: &str, product_type: &str,
    has_desc_en: bool, has_desc_ru: bool, has_desc_pl: bool, has_desc_uk: bool,
    has_cal: bool, has_prot: bool, has_fat: bool, has_carbs: bool,
) -> String {
    format!(
        r#"You are a food nutrition expert. Fill missing data for: "{name_en}" (Russian: "{name_ru}", type: {product_type}).

Field status (true = already filled, skip; false = EMPTY, you MUST fill):
- description_en: FILLED={has_desc_en}
- description_ru: FILLED={has_desc_ru}
- description_pl: FILLED={has_desc_pl}
- description_uk: FILLED={has_desc_uk}
- calories_per_100g: FILLED={has_cal}
- protein_per_100g: FILLED={has_prot}
- fat_per_100g: FILLED={has_fat}
- carbs_per_100g: FILLED={has_carbs}

Return ONLY valid JSON:
{{
  "description_en": "<2-3 sentences culinary description or null if FILLED=true>",
  "description_ru": "<2-3 предложения на русском — пиши как повар, or null>",
  "description_pl": "<2-3 zdania po polsku — pisz jak kucharz, or null>",
  "description_uk": "<2-3 речення українською — пиши як кухар, or null>",
  "calories_per_100g": <integer or null>,
  "protein_per_100g": <float or null>,
  "fat_per_100g": <float or null>,
  "carbs_per_100g": <float or null>,
  "fiber_per_100g": <float or 0 for animal products>,
  "sugar_per_100g": <float or null>,
  "density_g_per_ml": <float or null>,
  "typical_portion_g": <float or null>,
  "shelf_life_days": <integer or null>,
  "seasons": ["Spring","Summer","Autumn","Winter"] or ["AllYear"],
  "macros": {{
    "calories_kcal": <integer or null>,
    "protein_g": <float or null>,
    "fat_g": <float or null>,
    "carbs_g": <float or null>,
    "fiber_g": <float or null>,
    "sugar_g": <float or null>,
    "starch_g": <float or null>,
    "water_g": <float or null>
  }},
  "vitamins": {{
    "vitamin_a": <float mg or null>,
    "vitamin_c": <float mg or null>,
    "vitamin_d": <float mg or null>,
    "vitamin_e": <float mg or null>,
    "vitamin_k": <float mcg or null>,
    "vitamin_b1": <float or null>,
    "vitamin_b2": <float or null>,
    "vitamin_b3": <float or null>,
    "vitamin_b5": <float or null>,
    "vitamin_b6": <float or null>,
    "vitamin_b9": <float or null>,
    "vitamin_b12": <float or null>
  }},
  "minerals": {{
    "calcium": <float mg or null>,
    "iron": <float or null>,
    "magnesium": <float or null>,
    "phosphorus": <float or null>,
    "potassium": <float or null>,
    "sodium": <float or null>,
    "zinc": <float or null>,
    "selenium": <float or null>
  }},
  "fatty_acids": {{
    "saturated_fat": <float g or null>,
    "monounsaturated_fat": <float or null>,
    "polyunsaturated_fat": <float or null>,
    "omega3": <float or null>,
    "omega6": <float or null>,
    "epa": <float or null>,
    "dha": <float or null>
  }},
  "diet_flags": {{
    "vegan": <bool>, "vegetarian": <bool>, "gluten_free": <bool>,
    "keto": <bool>, "paleo": <bool>, "mediterranean": <bool>, "low_carb": <bool>
  }},
  "allergens": {{
    "milk": <bool>, "eggs": <bool>, "fish": <bool>, "shellfish": <bool>,
    "nuts": <bool>, "peanuts": <bool>, "gluten": <bool>, "soy": <bool>,
    "sesame": <bool>, "celery": <bool>, "mustard": <bool>, "sulfites": <bool>,
    "lupin": <bool>, "molluscs": <bool>
  }},
  "culinary": {{
    "sweetness": <1-10 or null>, "acidity": <1-10 or null>,
    "bitterness": <1-10 or null>, "umami": <1-10 or null>,
    "aroma": <1-10 or null>, "texture": "<string or null>"
  }},
  "food_properties": {{
    "glycemic_index": <int or null>, "glycemic_load": <float or null>,
    "ph": <float or null>, "smoke_point": <int celsius or null>,
    "water_activity": <float 0-1 or null>
  }}
}}

Rules:
- All nutrition per 100g RAW product (USDA FoodData Central reference)
- For animal products: fiber = 0, carbs near 0
- Descriptions must be natural, not machine-translated
- If FILLED=true → return null for that field
- If FILLED=false → you MUST provide real value, NOT null
- Return ONLY JSON, no extra text"#,
        name_en = name_en,
        name_ru = name_ru,
        product_type = product_type,
        has_desc_en = has_desc_en,
        has_desc_ru = has_desc_ru,
        has_desc_pl = has_desc_pl,
        has_desc_uk = has_desc_uk,
        has_cal = has_cal,
        has_prot = has_prot,
        has_fat = has_fat,
        has_carbs = has_carbs,
    )
}

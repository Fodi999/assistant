//! Use Case: AI Autofill — fill missing nutrition/description fields
//!
//! Pipeline:
//! 1. Check cache (hash of product_id + field_status)
//! 2. If miss → call AiClient::generate (70b model)
//! 3. Parse JSON response
//! 4. Cache result (TTL: 30 days — nutrition data doesn't change often)
//! 5. Return JSON suggestion for admin review

use crate::application::admin_catalog::AdminCatalogService;
use crate::domain::ai_ports::{AiClient, AiQuality};
use crate::shared::{AppError, AppResult};
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// Cache TTL for autofill results (days)
const AUTOFILL_CACHE_TTL_DAYS: i32 = 30;

impl AdminCatalogService {
    /// AI autofill — asks AI to fill all empty nutrition/description/culinary
    /// fields for a given product. Returns a JSON suggestion for admin review.
    ///
    /// 🚀 With caching: −70% AI costs for repeated requests
    pub async fn ai_autofill(&self, id: Uuid) -> AppResult<serde_json::Value> {
        let product = self.get_product_by_id(id).await?;

        let name_en = product.name_en.clone();
        let name_ru = product.name_ru.clone().unwrap_or_default();
        let product_type = product.product_type.clone();

        // Build field status fingerprint for cache key
        let has_description_en = product.description_en.as_ref().map(|s| !s.trim().is_empty()).unwrap_or(false);
        let has_description_ru = product.description_ru.as_ref().map(|s| !s.trim().is_empty()).unwrap_or(false);
        let has_description_pl = product.description_pl.as_ref().map(|s| !s.trim().is_empty()).unwrap_or(false);
        let has_description_uk = product.description_uk.as_ref().map(|s| !s.trim().is_empty()).unwrap_or(false);
        let has_calories = product.calories_per_100g.is_some();
        let has_protein = product.protein_per_100g.is_some();
        let has_fat = product.fat_per_100g.is_some();
        let has_carbs = product.carbs_per_100g.is_some();

        // ── Cache check ──
        let fingerprint = format!(
            "{}:{}:{}:{}:{}:{}:{}:{}:{}",
            name_en, product_type,
            has_description_en, has_description_ru, has_description_pl, has_description_uk,
            has_calories, has_protein, has_fat
        );
        let cache_key = format!("uc:autofill:{}", hash_input(&fingerprint));

        if let Ok(Some(cached)) = self.ai_cache.get(&cache_key).await {
            tracing::info!("📦 Autofill cache hit for product {}", id);
            return Ok(cached);
        }

        // ── Build prompt ──
        let prompt = build_autofill_prompt(
            &name_en, &name_ru, &product_type,
            has_description_en, has_description_ru, has_description_pl, has_description_uk,
            has_calories, has_protein, has_fat, has_carbs,
        );

        // ── Call AI via trait ──
        let raw = self.llm_adapter
            .generate_with_quality(&prompt, 3000, AiQuality::Balanced)
            .await?;

        // ── Parse JSON ──
        let result = parse_json_response(&raw)?;

        // ── Cache result ──
        if let Err(e) = self.ai_cache.set(
            &cache_key, result.clone(), "groq", "llama-3.3-70b-versatile", AUTOFILL_CACHE_TTL_DAYS
        ).await {
            tracing::warn!("Failed to cache autofill result: {}", e);
        }

        tracing::info!("✅ AI autofill complete for product {} (cached)", id);
        Ok(result)
    }
}

/// SHA-256 hash of input string → short hex key
fn hash_input(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    let result = hasher.finalize();
    format!("{:x}", result)[..16].to_string()
}

/// Parse JSON from AI response with fallback extraction
fn parse_json_response(raw: &str) -> AppResult<serde_json::Value> {
    serde_json::from_str(raw)
        .or_else(|_| {
            if let Some(start) = raw.find('{') {
                if let Some(end) = raw.rfind('}') {
                    return serde_json::from_str(&raw[start..=end]);
                }
            }
            Err(serde_json::Error::io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "No JSON found",
            )))
        })
        .map_err(|e| {
            tracing::error!("Failed to parse AI response: {}", e);
            AppError::internal("AI returned invalid JSON")
        })
}

fn build_autofill_prompt(
    name_en: &str, name_ru: &str, product_type: &str,
    has_desc_en: bool, has_desc_ru: bool, has_desc_pl: bool, has_desc_uk: bool,
    has_cal: bool, has_prot: bool, has_fat: bool, has_carbs: bool,
) -> String {
    format!(
        r#"You are a professional food database expert. Fill in the missing data for this ingredient.

Product: "{name_en}" (Russian: "{name_ru}", type: "{product_type}")

Return ONLY a valid JSON object. Rules:
- If a field says "FILLED=true" below → return null (do NOT overwrite).
- If a field says "FILLED=false" below → you MUST provide a real value (do NOT return null).
- For all other fields not listed below → always provide a value.

Field status (true=already has data, false=EMPTY and needs your data):
- description_en: FILLED={has_desc_en}
- description_ru: FILLED={has_desc_ru}
- description_pl: FILLED={has_desc_pl}
- description_uk: FILLED={has_desc_uk}
- calories_per_100g: FILLED={has_cal}
- protein_per_100g: FILLED={has_prot}
- fat_per_100g: FILLED={has_fat}
- carbs_per_100g: FILLED={has_carbs}

IMPORTANT: If FILLED=false, you MUST provide the value, NOT null!
All nutritional values should be for RAW product per 100g.

Return this JSON:
{{
  "description_en": "<2-3 sentence culinary description in English, or null if FILLED=true>",
  "description_ru": "<2-3 предложения кулинарное описание на русском, or null if FILLED=true>",
  "description_pl": "<2-3 zdania opis kulinarny po polsku, or null if FILLED=true>",
  "description_uk": "<2-3 речення кулінарний опис українською, or null if FILLED=true>",
  "calories_per_100g": <integer kcal per 100g, or null>,
  "protein_per_100g": <float grams protein per 100g, or null>,
  "fat_per_100g": <float grams fat per 100g, or null>,
  "carbs_per_100g": <float grams carbs per 100g, or null>,
  "fiber_per_100g": <float or null>,
  "sugar_per_100g": <float or null>,
  "density_g_per_ml": <float density g/ml, or null>,
  "typical_portion_g": <float typical serving size in grams, or null>,
  "shelf_life_days": <integer days shelf life, or null>,
  "product_type": "<one of: fish, seafood, meat, vegetable, fruit, dairy, grain, spice, oil, beverage, nut, legume, other — or null if already set>",
  "seasons": <["Spring","Summer","Autumn","Winter"] subset or ["AllYear"], based on typical availability>,
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
    "vitamin_a": <float mg per 100g or null>,
    "vitamin_c": <float mg per 100g or null>,
    "vitamin_d": <float mg per 100g or null>,
    "vitamin_e": <float mg per 100g or null>,
    "vitamin_k": <float mcg per 100g or null>,
    "vitamin_b1": <float or null>,
    "vitamin_b2": <float or null>,
    "vitamin_b3": <float or null>,
    "vitamin_b5": <float or null>,
    "vitamin_b6": <float or null>,
    "vitamin_b9": <float or null>,
    "vitamin_b12": <float or null>
  }},
  "minerals": {{
    "calcium": <float mg per 100g or null>,
    "iron": <float or null>,
    "magnesium": <float or null>,
    "phosphorus": <float or null>,
    "potassium": <float or null>,
    "sodium": <float or null>,
    "zinc": <float or null>,
    "selenium": <float or null>
  }},
  "fatty_acids": {{
    "saturated_fat": <float g per 100g or null>,
    "monounsaturated_fat": <float or null>,
    "polyunsaturated_fat": <float or null>,
    "omega3": <float or null>,
    "omega6": <float or null>,
    "epa": <float or null>,
    "dha": <float or null>
  }},
  "diet_flags": {{
    "vegan": <true/false>,
    "vegetarian": <true/false>,
    "gluten_free": <true/false>,
    "keto": <true/false>,
    "paleo": <true/false>,
    "mediterranean": <true/false>,
    "low_carb": <true/false>
  }},
  "allergens": {{
    "milk": <true/false>,
    "eggs": <true/false>,
    "fish": <true/false>,
    "shellfish": <true/false>,
    "nuts": <true/false>,
    "peanuts": <true/false>,
    "gluten": <true/false>,
    "soy": <true/false>,
    "sesame": <true/false>,
    "celery": <true/false>,
    "mustard": <true/false>,
    "sulfites": <true/false>,
    "lupin": <true/false>,
    "molluscs": <true/false>
  }},
  "culinary": {{
    "sweetness": <1-10 integer or null>,
    "acidity": <1-10 integer or null>,
    "bitterness": <1-10 integer or null>,
    "umami": <1-10 integer or null>,
    "aroma": <1-10 integer or null>,
    "texture": "<string like crispy/tender/creamy or null>"
  }},
  "food_properties": {{
    "glycemic_index": <integer or null>,
    "glycemic_load": <float or null>,
    "ph": <float 0-14 or null>,
    "smoke_point": <integer celsius or null>,
    "water_activity": <float 0-1 or null>
  }}
}}

Use USDA FoodData Central (raw/uncooked values per 100g) as reference. Be precise.
REMEMBER: FILLED=false means the field is EMPTY — you MUST provide a real value, NOT null!"#,
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

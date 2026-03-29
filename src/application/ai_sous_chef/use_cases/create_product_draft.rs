//! Use Case: AI Create Product Draft — generate a rich product draft from free-text input
//!
//! ═══════════════════════════════════════════════════════════════════
//! ARCHITECTURE v2: Dictionary = truth, AI = helper, Backend = brain
//! ═══════════════════════════════════════════════════════════════════
//!
//! Pipeline:
//! 1. dictionary.resolve(input) → names, product_type, unit (source of truth)
//! 2. AI.generate → ONLY descriptions + nutrition + SEO (what AI is good at)
//! 3. map_to_draft() → combine dictionary + AI data
//! 4. validate_draft() → sanity checks, calorie recalc, clamp values
//! 5. enrich_defaults() → density, shelf_life, portion from lookup tables
//! 6. Cache validated draft
//! 7. Return for admin review — NEVER auto-saves!
//!
//! Key principle: AI ≠ сохранение. AI ≠ brain. Dictionary + backend = brain.

use crate::application::admin_catalog::AdminCatalogService;
use crate::application::ai_sous_chef::product_dictionary;
use crate::domain::ai_ports::{AiClient, AiQuality};
use crate::shared::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Cache TTL for draft results (days)
const DRAFT_CACHE_TTL_DAYS: i32 = 7;

// ── Request / Response DTOs ──────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateDraftRequest {
    pub input: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldConfidence {
    High,
    Medium,
    Low,
    NotApplicable,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataSource {
    Ai,
    Manual,
    AiCorrected,
    Dictionary,
    Lookup,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DraftField<T: Serialize> {
    pub value: Option<T>,
    pub source: DataSource,
    pub confidence: FieldConfidence,
}

impl<T: Serialize + Clone> DraftField<T> {
    fn ai(value: T, confidence: FieldConfidence) -> Self {
        Self { value: Some(value), source: DataSource::Ai, confidence }
    }
    fn ai_opt(value: Option<T>, confidence: FieldConfidence) -> Self {
        Self { value, source: DataSource::Ai, confidence }
    }
    fn dict(value: T) -> Self {
        Self { value: Some(value), source: DataSource::Dictionary, confidence: FieldConfidence::High }
    }
    fn lookup(value: T) -> Self {
        Self { value: Some(value), source: DataSource::Lookup, confidence: FieldConfidence::High }
    }
    fn lookup_opt(value: Option<T>) -> Self {
        Self { value, source: DataSource::Lookup, confidence: FieldConfidence::High }
    }
    fn not_applicable() -> Self {
        Self { value: None, source: DataSource::Ai, confidence: FieldConfidence::NotApplicable }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DraftNames {
    pub en: DraftField<String>,
    pub ru: DraftField<String>,
    pub pl: DraftField<String>,
    pub uk: DraftField<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DraftNutrition {
    pub calories_per_100g: DraftField<f64>,
    pub protein_per_100g: DraftField<f64>,
    pub fat_per_100g: DraftField<f64>,
    pub carbs_per_100g: DraftField<f64>,
    pub fiber_per_100g: DraftField<f64>,
    pub sugar_per_100g: DraftField<f64>,
    pub density_g_per_ml: DraftField<f64>,
    pub typical_portion_g: DraftField<f64>,
    pub shelf_life_days: DraftField<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DraftSeo {
    pub seo_title: DraftField<String>,
    pub seo_description: DraftField<String>,
    pub seo_h1: DraftField<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductDraft {
    pub names: DraftNames,
    pub description_en: DraftField<String>,
    pub description_ru: DraftField<String>,
    pub description_pl: DraftField<String>,
    pub description_uk: DraftField<String>,
    pub product_type: DraftField<String>,
    pub unit: DraftField<String>,
    pub nutrition: DraftNutrition,
    pub seo: DraftSeo,
    pub seasons: DraftField<Vec<String>>,
    pub confidence: f64,
    pub needs_review: bool,
    pub quality_warnings: Vec<QualityWarning>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityWarning {
    pub field: String,
    pub label_ru: String,
    pub severity: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DraftCorrection {
    pub field: String,
    pub original_value: String,
    pub corrected_to: String,
    pub reason: String,
}

#[derive(Debug, Serialize)]
pub struct CreateDraftResponse {
    pub draft: ProductDraft,
    pub raw_input: String,
    pub model: String,
    pub cached: bool,
    pub corrections: Vec<DraftCorrection>,
}

// ══════════════════════════════════════════════════════════════════════
// MAIN USE CASE
// ══════════════════════════════════════════════════════════════════════

impl AdminCatalogService {
    /// AI Create Product Draft — pipeline v2 (Dictionary-first)
    ///
    /// 1. Dictionary resolve (names, type, unit) — source of truth
    /// 2. AI generate (descriptions + nutrition + SEO only)
    /// 3. Map to draft (combine dictionary + AI)
    /// 4. Validate (sanity checks, calorie recalc, clamp values)
    /// 5. Enrich defaults (density, shelf_life, portion from lookup)
    /// 6. Return for review — NEVER auto-saves
    pub async fn ai_create_product_draft(
        &self,
        req: CreateDraftRequest,
    ) -> AppResult<CreateDraftResponse> {
        let input = req.input.trim().to_string();
        if input.is_empty() {
            return Err(AppError::validation("Input text cannot be empty"));
        }

        // ── Cache check ──
        let cache_key = format!("uc:draft:v2:{}", hash_input(&input));
        if let Ok(Some(cached_json)) = self.ai_cache.get(&cache_key).await {
            tracing::info!("📦 Draft cache hit for: {}", &input[..input.len().min(50)]);
            if let Ok(mut draft) = serde_json::from_value::<ProductDraft>(cached_json) {
                let corrections = validate_draft(&mut draft);
                return Ok(CreateDraftResponse {
                    draft,
                    raw_input: input,
                    model: "cache".to_string(),
                    cached: true,
                    corrections,
                });
            }
        }

        // ══════════════════════════════════════════════════════════════
        // STEP 1: Dictionary resolve — names, type, unit (truth)
        // ══════════════════════════════════════════════════════════════
        let resolved = self.resolve_from_dictionary(&input).await;

        // ══════════════════════════════════════════════════════════════
        // STEP 2: AI generate — ONLY descriptions + nutrition + SEO
        // ══════════════════════════════════════════════════════════════
        let prompt = build_slim_prompt(&resolved.name_en, &resolved.product_type);
        let raw = self
            .llm_adapter
            .generate_with_quality(&prompt, 5000, AiQuality::Balanced)
            .await?;
        tracing::debug!("🤖 Raw AI draft response ({} chars): {}", raw.len(), &raw[..raw.len().min(300)]);
        let ai_json = parse_json_response(&raw)?;

        // ══════════════════════════════════════════════════════════════
        // STEP 3: Map to draft — dictionary data + AI data
        // ══════════════════════════════════════════════════════════════
        let mut draft = map_to_draft(&resolved, &ai_json);

        // ══════════════════════════════════════════════════════════════
        // STEP 4: Validate — sanity checks, calorie recalc
        // ══════════════════════════════════════════════════════════════
        let corrections = validate_draft(&mut draft);
        if !corrections.is_empty() {
            tracing::info!(
                "🔧 Validation: {} corrections for '{}'",
                corrections.len(),
                &input[..input.len().min(50)]
            );
        }

        // ══════════════════════════════════════════════════════════════
        // STEP 5: Enrich defaults (density, shelf_life, portion)
        // ══════════════════════════════════════════════════════════════
        enrich_defaults(&mut draft);

        // ── Cache validated draft ──
        if let Ok(draft_json) = serde_json::to_value(&draft) {
            let _ = self
                .ai_cache
                .set(
                    &cache_key,
                    draft_json,
                    "gemini",
                    "gemini-3.1-pro-preview",
                    DRAFT_CACHE_TTL_DAYS,
                )
                .await;
        }

        tracing::info!(
            "✅ Draft v2 created for '{}' (confidence: {:.0}%)",
            &input[..input.len().min(50)],
            draft.confidence * 100.0
        );

        Ok(CreateDraftResponse {
            draft,
            raw_input: input,
            model: "gemini-3.1-pro-preview".to_string(),
            cached: false,
            corrections,
        })
    }

    /// Resolve product info from dictionary + keyword inference.
    ///
    /// Pipeline:
    /// 1. Normalize input ("fresh salmon fillet" → "Salmon")
    /// 2. Dictionary lookup (ACTIVE entries only)
    /// 3. If miss → AI translate name → save as PENDING (admin must approve!)
    /// 4. Infer product_type + unit from keywords
    async fn resolve_from_dictionary(
        &self,
        input: &str,
    ) -> product_dictionary::ResolvedProduct {
        // ── STEP 1: Normalize input ──
        let normalized = product_dictionary::normalize_ingredient_name(input);
        tracing::info!("🔍 Normalized input: '{}' → '{}'", input.trim(), &normalized);

        // ── STEP 2: Dictionary lookup (ACTIVE only) ──
        let dict_entry = self.dictionary.find_by_en(&normalized).await.ok().flatten();

        let (name_en, name_ru, name_pl, name_uk, names_from_ai) = if let Some(ref d) = dict_entry {
            tracing::info!("📖 Dictionary hit: {} → RU:{}, PL:{}, UK:{}", d.name_en, d.name_ru, d.name_pl, d.name_uk);
            (
                d.name_en.clone(),
                d.name_ru.clone(),
                d.name_pl.clone(),
                d.name_uk.clone(),
                false,
            )
        } else {
            // ══════════════════════════════════════════════════════════
            // NOT IN DICTIONARY → AI translate → save as PENDING
            // Admin must approve before it becomes "truth"!
            // ══════════════════════════════════════════════════════════
            tracing::info!("📖 Dictionary miss for '{}' — requesting AI translation", &normalized);
            match self.ai_translate_and_save_pending(&normalized).await {
                Ok((ru, pl, uk)) => {
                    tracing::info!("🌍 AI translation success: RU:{}, PL:{}, UK:{}", ru, pl, uk);
                    // Return AI suggestions but they're NOT active in dictionary yet
                    (normalized.clone(), ru, pl, uk, true)
                }
                Err(e) => {
                    tracing::warn!("AI name translation failed: {} — names will be empty", e);
                    (normalized.clone(), String::new(), String::new(), String::new(), true)
                }
            }
        };

        let name_en_lower = name_en.to_lowercase();
        let name_ru_lower = name_ru.to_lowercase();

        // Infer product_type from keywords (NEVER from AI)
        let product_type =
            product_dictionary::infer_product_type(&name_en_lower, &name_ru_lower)
                .unwrap_or("other")
                .to_string();

        // Unit from lookup (NEVER from AI)
        let unit =
            product_dictionary::unit_for_type(&product_type, &name_en_lower).to_string();

        // Defaults from lookup tables
        let density = product_dictionary::default_density(&product_type);
        let portion = product_dictionary::default_portion(&product_type);
        let shelf_life = product_dictionary::default_shelf_life(&product_type);

        product_dictionary::ResolvedProduct {
            name_en,
            name_ru,
            name_pl,
            name_uk,
            product_type,
            unit,
            density_g_per_ml: density,
            typical_portion_g: portion,
            shelf_life_days: shelf_life,
            names_from_ai,
        }
    }

    /// AI translates the product name + validates + saves as PENDING.
    ///
    /// Pipeline:
    /// 1. AI translate (tiny prompt, ~50 tokens)
    /// 2. Validate all 3 names (reject garbage)
    /// 3. Save as PENDING with confidence score
    /// 4. Return names for use in draft (source: AI, not Dictionary)
    ///
    /// Admin must approve pending entries before they become active!
    pub(crate) async fn ai_translate_and_save_pending(
        &self,
        name_en: &str,
    ) -> AppResult<(String, String, String)> {
        let prompt = format!(
            r#"Translate the food ingredient name "{name_en}" into 3 languages.
Return ONLY valid JSON:
{{"ru": "<Russian>", "pl": "<Polish>", "uk": "<Ukrainian>"}}

Rules:
- Use standard culinary/market names, no adjectives
- Crucian carp: ru=Карась, pl=Karaś, uk=Карась
- Carp: ru=Карп, pl=Karp, uk=Короп
- Salmon: ru=Лосось, pl=Łosoś, uk=Лосось
- Cod: ru=Треска, pl=Dorsz, uk=Тріска
- Use singular form, base ingredient only
- Return ONLY JSON"#,
            name_en = name_en,
        );

        let raw = self
            .llm_adapter
            .generate_with_quality(&prompt, 200, AiQuality::Fast)
            .await?;

        let json = parse_json_response(&raw)?;
        let ru = json.get("ru").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
        let pl = json.get("pl").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
        let uk = json.get("uk").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();

        if ru.is_empty() && pl.is_empty() && uk.is_empty() {
            return Err(AppError::internal("AI returned empty translations"));
        }

        // ── Validate AI names (reject garbage) ──
        let en_valid = product_dictionary::is_valid_ingredient_name(name_en);
        let ru_valid = product_dictionary::is_valid_ingredient_name(&ru);
        let pl_valid = product_dictionary::is_valid_ingredient_name(&pl);
        let uk_valid = product_dictionary::is_valid_ingredient_name(&uk);

        let valid_count = [en_valid, ru_valid, pl_valid, uk_valid].iter().filter(|&&v| v).count();
        let confidence = valid_count as f32 / 4.0;

        tracing::info!(
            "🌍 AI translated '{}' → RU:{}, PL:{}, UK:{} (confidence: {:.2}, valid: {}/4)",
            name_en, ru, pl, uk, confidence, valid_count
        );

        // ── Save as PENDING (admin must approve!) ──
        if confidence >= 0.5 {
            match self.dictionary.insert_pending(name_en, &pl, &ru, &uk, confidence).await {
                Ok(entry) => {
                    tracing::info!(
                        "🟡 Saved PENDING: {} → RU:{}, PL:{}, UK:{} (awaiting admin review)",
                        entry.name_en, entry.name_ru, entry.name_pl, entry.name_uk
                    );
                }
                Err(e) => {
                    tracing::warn!("Failed to save pending entry: {}", e);
                }
            }
        } else {
            tracing::warn!(
                "⚠️ AI translation confidence too low ({:.2}) for '{}' — NOT saving to dictionary",
                confidence, name_en
            );
        }

        Ok((ru, pl, uk))
    }
}

// ══════════════════════════════════════════════════════════════════════
// STEP 2: SLIM PROMPT — AI only does what it is good at
// ══════════════════════════════════════════════════════════════════════

fn build_slim_prompt(name_en: &str, product_type: &str) -> String {
    format!(
        r#"You are a food nutrition expert. Provide data for: "{name_en}" (type: {product_type}).

Return ONLY valid JSON:
{{
  "description_en": "<2-3 sentences culinary description in English>",
  "description_ru": "<2-3 предложения кулинарное описание на русском — пиши как повар>",
  "description_pl": "<2-3 zdania opis kulinarny po polsku — pisz jak kucharz>",
  "description_uk": "<2-3 речення кулінарний опис українською — пиши як кухар>",
  "calories_per_100g": <integer>,
  "protein_per_100g": <float>,
  "fat_per_100g": <float>,
  "carbs_per_100g": <float>,
  "fiber_per_100g": <float, 0 for animal products>,
  "sugar_per_100g": <float or null>,
  "seo": {{
    "seo_title": "<50-60 chars SEO title>",
    "seo_description": "<120-160 chars meta description>",
    "seo_h1": "<H1 heading>"
  }}
}}

Rules:
- All nutrition per 100g RAW product (USDA FoodData Central reference)
- For fish/meat/dairy/eggs: fiber = 0, carbs near 0
- Descriptions must be natural (not machine-translated)
- Return ONLY JSON, no extra text"#,
        name_en = name_en,
        product_type = product_type,
    )
}

// ══════════════════════════════════════════════════════════════════════
// STEP 3: MAP — combine dictionary + AI into ProductDraft
// ══════════════════════════════════════════════════════════════════════

fn map_to_draft(
    resolved: &product_dictionary::ResolvedProduct,
    ai: &serde_json::Value,
) -> ProductDraft {
    let str_val = |key: &str| -> Option<String> {
        ai.get(key).and_then(|v| v.as_str()).map(|s| s.to_string())
    };
    let f64_val = |key: &str| -> Option<f64> {
        ai.get(key).and_then(|v| v.as_f64())
    };

    // Animal detection for fiber
    let animal_types = ["fish", "seafood", "meat", "poultry", "dairy"];
    let is_animal = animal_types
        .iter()
        .any(|t| resolved.product_type.eq_ignore_ascii_case(t));

    // ── Confidence calc: count how many AI fields returned ──
    let mut returned = 0u32;
    let total = 6u32;
    if f64_val("calories_per_100g").is_some() { returned += 1; }
    if f64_val("protein_per_100g").is_some() { returned += 1; }
    if f64_val("fat_per_100g").is_some() { returned += 1; }
    if f64_val("carbs_per_100g").is_some() { returned += 1; }
    if str_val("description_en").is_some() { returned += 1; }
    if str_val("description_ru").is_some() { returned += 1; }
    let confidence = returned as f64 / total as f64;
    let conf = if confidence >= 0.85 {
        FieldConfidence::High
    } else if confidence >= 0.6 {
        FieldConfidence::Medium
    } else {
        FieldConfidence::Low
    };

    let fiber_field = if is_animal {
        DraftField::not_applicable()
    } else {
        DraftField::ai_opt(f64_val("fiber_per_100g"), conf.clone())
    };

    // SEO
    let seo_obj = ai.get("seo");
    let seo_str = |key: &str| -> Option<String> {
        seo_obj
            .and_then(|s| s.get(key))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    };

    let mut quality_warnings = Vec::new();

    // Warn about name quality
    if resolved.name_ru.is_empty() {
        quality_warnings.push(QualityWarning {
            field: "name_ru".into(),
            label_ru: "Название RU".into(),
            severity: "critical".into(),
            message: "AI перевод не удался — добавьте перевод вручную".into(),
        });
    } else if resolved.names_from_ai {
        quality_warnings.push(QualityWarning {
            field: "name_ru".into(),
            label_ru: "Название RU".into(),
            severity: "warning".into(),
            message: "AI перевод (pending в словаре) — проверьте перед сохранением".into(),
        });
    }
    if resolved.name_pl.is_empty() {
        quality_warnings.push(QualityWarning {
            field: "name_pl".into(),
            label_ru: "Название PL".into(),
            severity: "critical".into(),
            message: "AI перевод не удался — добавьте перевод вручную".into(),
        });
    } else if resolved.names_from_ai {
        quality_warnings.push(QualityWarning {
            field: "name_pl".into(),
            label_ru: "Название PL".into(),
            severity: "warning".into(),
            message: "AI перевод (pending в словаре) — проверьте перед сохранением".into(),
        });
    }
    if resolved.name_uk.is_empty() {
        quality_warnings.push(QualityWarning {
            field: "name_uk".into(),
            label_ru: "Название UK".into(),
            severity: "critical".into(),
            message: "AI перевод не удался — добавьте перевод вручную".into(),
        });
    } else if resolved.names_from_ai {
        quality_warnings.push(QualityWarning {
            field: "name_uk".into(),
            label_ru: "Название UK".into(),
            severity: "warning".into(),
            message: "AI перевод (pending в словаре) — проверьте перед сохранением".into(),
        });
    }
    if resolved.product_type == "other" {
        quality_warnings.push(QualityWarning {
            field: "product_type".into(),
            label_ru: "Тип продукта".into(),
            severity: "critical".into(),
            message: "Не удалось определить тип — укажите вручную".into(),
        });
    }
    if f64_val("calories_per_100g").is_none() {
        quality_warnings.push(QualityWarning {
            field: "calories_per_100g".into(),
            label_ru: "Калории".into(),
            severity: "critical".into(),
            message: "AI не вернул калории — проверьте".into(),
        });
    }

    let needs_review = confidence < 0.85 || !quality_warnings.is_empty();

    ProductDraft {
        // ── From DICTIONARY (source of truth) ──
        names: DraftNames {
            en: DraftField::dict(resolved.name_en.clone()),
            ru: if resolved.names_from_ai {
                if resolved.name_ru.is_empty() {
                    DraftField { value: None, source: DataSource::Ai, confidence: FieldConfidence::Low }
                } else {
                    DraftField::ai(resolved.name_ru.clone(), FieldConfidence::Medium)
                }
            } else {
                DraftField::dict(resolved.name_ru.clone())
            },
            pl: if resolved.names_from_ai {
                if resolved.name_pl.is_empty() {
                    DraftField { value: None, source: DataSource::Ai, confidence: FieldConfidence::Low }
                } else {
                    DraftField::ai(resolved.name_pl.clone(), FieldConfidence::Medium)
                }
            } else {
                DraftField::dict(resolved.name_pl.clone())
            },
            uk: if resolved.names_from_ai {
                if resolved.name_uk.is_empty() {
                    DraftField { value: None, source: DataSource::Ai, confidence: FieldConfidence::Low }
                } else {
                    DraftField::ai(resolved.name_uk.clone(), FieldConfidence::Medium)
                }
            } else {
                DraftField::dict(resolved.name_uk.clone())
            },
        },
        product_type: DraftField::dict(resolved.product_type.clone()),
        unit: DraftField::dict(resolved.unit.clone()),

        // ── From AI (helper) ──
        description_en: DraftField::ai_opt(str_val("description_en"), conf.clone()),
        description_ru: DraftField::ai_opt(str_val("description_ru"), conf.clone()),
        description_pl: DraftField::ai_opt(str_val("description_pl"), conf.clone()),
        description_uk: DraftField::ai_opt(str_val("description_uk"), conf.clone()),

        nutrition: DraftNutrition {
            calories_per_100g: DraftField::ai_opt(f64_val("calories_per_100g"), conf.clone()),
            protein_per_100g: DraftField::ai_opt(f64_val("protein_per_100g"), conf.clone()),
            fat_per_100g: DraftField::ai_opt(f64_val("fat_per_100g"), conf.clone()),
            carbs_per_100g: DraftField::ai_opt(f64_val("carbs_per_100g"), conf.clone()),
            fiber_per_100g: fiber_field,
            sugar_per_100g: DraftField::ai_opt(f64_val("sugar_per_100g"), conf.clone()),
            // ── From LOOKUP (not AI) ──
            density_g_per_ml: DraftField::lookup_opt(resolved.density_g_per_ml),
            typical_portion_g: DraftField::lookup_opt(resolved.typical_portion_g),
            shelf_life_days: DraftField::lookup_opt(resolved.shelf_life_days),
        },
        seo: DraftSeo {
            seo_title: DraftField::ai_opt(seo_str("seo_title"), conf.clone()),
            seo_description: DraftField::ai_opt(seo_str("seo_description"), conf.clone()),
            seo_h1: DraftField::ai_opt(seo_str("seo_h1"), conf.clone()),
        },
        seasons: DraftField::ai_opt(
            ai.get("seasons")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect()),
            conf,
        ),
        confidence,
        needs_review,
        quality_warnings,
    }
}

// ══════════════════════════════════════════════════════════════════════
// STEP 4: VALIDATE — sanity checks, calorie recalc, clamp values
// ══════════════════════════════════════════════════════════════════════

fn validate_draft(draft: &mut ProductDraft) -> Vec<DraftCorrection> {
    let mut corrections = Vec::new();

    let product_type = draft
        .product_type
        .value
        .as_deref()
        .unwrap_or("other")
        .to_string();

    let is_animal = ["fish", "seafood", "meat", "poultry", "dairy"]
        .iter()
        .any(|t| product_type.eq_ignore_ascii_case(t));

    // ── Rule 1: Protein sanity (0–50g per 100g) ─────────────────────
    if let Some(p) = draft.nutrition.protein_per_100g.value {
        if p < 0.0 || p > 80.0 {
            let clamped = p.clamp(0.0, 50.0);
            corrections.push(DraftCorrection {
                field: "protein_per_100g".into(),
                original_value: format!("{:.1}", p),
                corrected_to: format!("{:.1}", clamped),
                reason: format!("Protein {:.1}g out of range — clamped to {:.1}g", p, clamped),
            });
            draft.nutrition.protein_per_100g.value = Some(clamped);
            draft.nutrition.protein_per_100g.source = DataSource::AiCorrected;
        }
    }

    // ── Rule 2: Fat sanity (0–100g per 100g) ────────────────────────
    if let Some(f) = draft.nutrition.fat_per_100g.value {
        if f < 0.0 || f > 100.0 {
            let clamped = f.clamp(0.0, 100.0);
            corrections.push(DraftCorrection {
                field: "fat_per_100g".into(),
                original_value: format!("{:.1}", f),
                corrected_to: format!("{:.1}", clamped),
                reason: format!("Fat {:.1}g out of range — clamped", f),
            });
            draft.nutrition.fat_per_100g.value = Some(clamped);
            draft.nutrition.fat_per_100g.source = DataSource::AiCorrected;
        }
    }

    // ── Rule 3: Carbs sanity (0–100g per 100g) ──────────────────────
    if let Some(c) = draft.nutrition.carbs_per_100g.value {
        if c < 0.0 || c > 100.0 {
            let clamped = c.clamp(0.0, 100.0);
            corrections.push(DraftCorrection {
                field: "carbs_per_100g".into(),
                original_value: format!("{:.1}", c),
                corrected_to: format!("{:.1}", clamped),
                reason: format!("Carbs {:.1}g out of range — clamped", c),
            });
            draft.nutrition.carbs_per_100g.value = Some(clamped);
            draft.nutrition.carbs_per_100g.source = DataSource::AiCorrected;
        }
    }

    // ── Rule 4: Fish/meat → carbs near 0 ────────────────────────────
    if is_animal && !product_type.eq_ignore_ascii_case("dairy") {
        if let Some(c) = draft.nutrition.carbs_per_100g.value {
            if c > 2.0 {
                corrections.push(DraftCorrection {
                    field: "carbs_per_100g".into(),
                    original_value: format!("{:.1}", c),
                    corrected_to: "0.0".into(),
                    reason: format!(
                        "{} products typically have ~0 carbs, AI returned {:.1}g",
                        product_type, c
                    ),
                });
                draft.nutrition.carbs_per_100g.value = Some(0.0);
                draft.nutrition.carbs_per_100g.source = DataSource::AiCorrected;
            }
        }
    }

    // ── Rule 5: Animal products → fiber = 0 ─────────────────────────
    if is_animal {
        if let Some(f) = draft.nutrition.fiber_per_100g.value {
            if f > 0.0 {
                draft.nutrition.fiber_per_100g = DraftField::not_applicable();
            }
        }
    }

    // ── Rule 6: Calorie recalculation (Atwater) ─────────────────────
    let protein = draft.nutrition.protein_per_100g.value.unwrap_or(0.0);
    let fat = draft.nutrition.fat_per_100g.value.unwrap_or(0.0);
    let carbs = draft.nutrition.carbs_per_100g.value.unwrap_or(0.0);
    let calculated = protein * 4.0 + fat * 9.0 + carbs * 4.0;

    if let Some(ai_cal) = draft.nutrition.calories_per_100g.value {
        if calculated > 0.0 && (calculated - ai_cal).abs() > 50.0 {
            let rounded = calculated.round();
            corrections.push(DraftCorrection {
                field: "calories_per_100g".into(),
                original_value: format!("{:.0}", ai_cal),
                corrected_to: format!("{:.0}", rounded),
                reason: format!(
                    "AI calories ({:.0}) differ >50 from Atwater ({:.0} = P{:.1}*4 + F{:.1}*9 + C{:.1}*4)",
                    ai_cal, calculated, protein, fat, carbs
                ),
            });
            draft.nutrition.calories_per_100g.value = Some(rounded);
            draft.nutrition.calories_per_100g.source = DataSource::AiCorrected;
        }
    } else if calculated > 0.0 {
        // AI didn't return calories but we have macros — calculate
        draft.nutrition.calories_per_100g.value = Some(calculated.round());
        draft.nutrition.calories_per_100g.source = DataSource::AiCorrected;
    }

    // ── Rule 7: Sugar <= Carbs ──────────────────────────────────────
    if let (Some(sugar), Some(carbs_val)) = (
        draft.nutrition.sugar_per_100g.value,
        draft.nutrition.carbs_per_100g.value,
    ) {
        if sugar > carbs_val {
            corrections.push(DraftCorrection {
                field: "sugar_per_100g".into(),
                original_value: format!("{:.1}", sugar),
                corrected_to: format!("{:.1}", carbs_val),
                reason: format!(
                    "Sugar ({:.1}) > Carbs ({:.1}) — clamped to carbs",
                    sugar, carbs_val
                ),
            });
            draft.nutrition.sugar_per_100g.value = Some(carbs_val);
            draft.nutrition.sugar_per_100g.source = DataSource::AiCorrected;
        }
    }

    // ── Rule 8: product_type plural → singular ──────────────────────
    let current = draft.product_type.value.as_deref().unwrap_or("other");
    let singular = match current {
        "vegetables" => Some("vegetable"),
        "fruits" => Some("fruit"),
        "grains" | "grains_and_pasta" => Some("grain"),
        "legumes" => Some("legume"),
        "nuts" => Some("nut"),
        "spices" => Some("spice"),
        "meats" => Some("meat"),
        "beverages" => Some("beverage"),
        "oils" => Some("oil"),
        _ => None,
    };
    if let Some(s) = singular {
        corrections.push(DraftCorrection {
            field: "product_type".into(),
            original_value: current.into(),
            corrected_to: s.into(),
            reason: format!("Normalized '{}' -> '{}'", current, s),
        });
        draft.product_type = DraftField::dict(s.to_string());
    }

    corrections
}

// ══════════════════════════════════════════════════════════════════════
// STEP 5: ENRICH DEFAULTS — fill gaps from lookup tables
// ══════════════════════════════════════════════════════════════════════

fn enrich_defaults(draft: &mut ProductDraft) {
    let pt = draft.product_type.value.as_deref().unwrap_or("other");

    if draft.nutrition.density_g_per_ml.value.is_none() {
        if let Some(d) = product_dictionary::default_density(pt) {
            draft.nutrition.density_g_per_ml = DraftField::lookup(d);
        }
    }
    if draft.nutrition.typical_portion_g.value.is_none() {
        if let Some(p) = product_dictionary::default_portion(pt) {
            draft.nutrition.typical_portion_g = DraftField::lookup(p);
        }
    }
    if draft.nutrition.shelf_life_days.value.is_none() {
        if let Some(s) = product_dictionary::default_shelf_life(pt) {
            draft.nutrition.shelf_life_days = DraftField::lookup(s);
        }
    }
}

// ── Helpers ──────────────────────────────────────────────────────────

fn hash_input(input: &str) -> String {
    let normalized = input.trim().to_lowercase();
    let mut hasher = Sha256::new();
    hasher.update(normalized.as_bytes());
    let result = hasher.finalize();
    format!("{:x}", result)[..16].to_string()
}

fn strip_markdown_fences(text: &str) -> String {
    let trimmed = text.trim();
    let without_prefix = if trimmed.starts_with("```json") {
        &trimmed[7..]
    } else if trimmed.starts_with("```") {
        &trimmed[3..]
    } else {
        return trimmed.to_string();
    };
    let without_suffix = if without_prefix.trim_end().ends_with("```") {
        let s = without_prefix.trim_end();
        &s[..s.len() - 3]
    } else {
        without_prefix
    };
    without_suffix.trim().to_string()
}

fn parse_json_response(raw: &str) -> AppResult<serde_json::Value> {
    // Strip markdown code fences that Gemini thinking models add
    let cleaned = strip_markdown_fences(raw);
    let s = cleaned.as_str();

    serde_json::from_str(s)
        .or_else(|_| {
            // Fallback: extract first {...} block
            if let Some(start) = s.find('{') {
                if let Some(end) = s.rfind('}') {
                    return serde_json::from_str(&s[start..=end]);
                }
            }
            Err(serde_json::Error::io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "No JSON found",
            )))
        })
        .map_err(|e| {
            tracing::error!("Failed to parse AI draft response: {}", e);
            tracing::debug!("Raw response (first 500 chars): {}", &raw[..raw.len().min(500)]);
            AppError::internal("AI returned invalid JSON for draft")
        })
}

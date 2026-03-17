//! Use Case: AI Create Product Draft — generate a rich product draft from free-text input
//!
//! Pipeline:
//! 1. Check cache (hash of input text)
//! 2. If miss → call AiClient::generate (70b model)
//! 3. Parse JSON response → ProductDraft
//! 4. Enrich with Field Requirement Level (from Data Quality Engine)
//! 5. Cache result (TTL: 7 days)
//! 6. Return draft for admin review — NEVER auto-saves!
//!
//! Key principle: AI ≠ сохранение. AI = только подготовка данных.

use crate::application::admin_catalog::AdminCatalogService;
use crate::domain::ai_ports::{AiClient, AiQuality};
use crate::shared::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Cache TTL for draft results (days)
const DRAFT_CACHE_TTL_DAYS: i32 = 7;

// ── Request / Response DTOs ──────────────────────────────────────────

/// Request to create a product draft from free-text input
#[derive(Debug, Deserialize)]
pub struct CreateDraftRequest {
    /// Free-text input in any language: "Свежее молоко 3.2% 1л"
    pub input: String,
}

/// Confidence level for an AI-generated field value
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldConfidence {
    High,
    Medium,
    Low,
    NotApplicable,
}

/// Source of the field value — who set it
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataSource {
    Ai,
    Manual,
    AiCorrected,
}

/// A single field in the draft with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DraftField<T: Serialize> {
    pub value: Option<T>,
    pub source: DataSource,
    pub confidence: FieldConfidence,
}

impl<T: Serialize + Clone> DraftField<T> {
    fn ai(value: T, confidence: FieldConfidence) -> Self {
        Self {
            value: Some(value),
            source: DataSource::Ai,
            confidence,
        }
    }
    fn ai_opt(value: Option<T>, confidence: FieldConfidence) -> Self {
        Self {
            value,
            source: DataSource::Ai,
            confidence,
        }
    }
    fn not_applicable() -> Self {
        Self {
            value: None,
            source: DataSource::Ai,
            confidence: FieldConfidence::NotApplicable,
        }
    }
}

/// Multilingual name set
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DraftNames {
    pub en: DraftField<String>,
    pub ru: DraftField<String>,
    pub pl: DraftField<String>,
    pub uk: DraftField<String>,
}

/// Core nutrition values per 100g
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

/// SEO fields
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DraftSeo {
    pub seo_title: DraftField<String>,
    pub seo_description: DraftField<String>,
    pub seo_h1: DraftField<String>,
}

/// The full product draft returned by AI — NOT saved to DB
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
    /// Overall AI confidence for the entire draft (0.0–1.0)
    pub confidence: f64,
    /// Whether the draft needs human review before saving
    pub needs_review: bool,
    /// Fields that need attention (from Data Quality Engine)
    pub quality_warnings: Vec<QualityWarning>,
}

/// A quality warning for a draft field
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityWarning {
    pub field: String,
    pub label_ru: String,
    pub severity: String,
    pub message: String,
}

/// A correction made by the Validation Layer (AI was wrong, backend fixed it)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DraftCorrection {
    pub field: String,
    pub original_value: String,
    pub corrected_to: String,
    pub reason: String,
}

/// Envelope response
#[derive(Debug, Serialize)]
pub struct CreateDraftResponse {
    pub draft: ProductDraft,
    pub raw_input: String,
    pub model: String,
    pub cached: bool,
    /// Corrections applied by the Validation Layer (backend = source of truth)
    pub corrections: Vec<DraftCorrection>,
}

// ── Implementation ───────────────────────────────────────────────────

impl AdminCatalogService {
    /// AI Create Product Draft — parse free-text, return rich draft for review.
    ///
    /// NEVER saves to DB. Frontend displays, user reviews, then POST /products.
    pub async fn ai_create_product_draft(
        &self,
        req: CreateDraftRequest,
    ) -> AppResult<CreateDraftResponse> {
        let input = req.input.trim().to_string();
        if input.is_empty() {
            return Err(AppError::validation("Input text cannot be empty"));
        }

        // ── Cache check ──
        let cache_key = format!("uc:draft:{}", hash_input(&input));
        let cached = if let Ok(Some(cached_json)) = self.ai_cache.get(&cache_key).await {
            tracing::info!("📦 Draft cache hit for input: {}", &input[..input.len().min(50)]);
            Some(cached_json)
        } else {
            None
        };

        if let Some(cached_json) = cached {
            let mut draft: ProductDraft = serde_json::from_value(cached_json)
                .map_err(|e| {
                    tracing::warn!("Failed to parse cached draft: {}", e);
                    AppError::internal("Cached draft is corrupt")
                })?;
            // Always re-validate even cached drafts (rules may have changed)
            let corrections = validate_draft(&mut draft);
            return Ok(CreateDraftResponse {
                draft,
                raw_input: input,
                model: "cache".to_string(),
                cached: true,
                corrections,
            });
        }

        // ── Build prompt ──
        let prompt = build_draft_prompt(&input);

        // ── Call AI (Balanced = 70b model) ──
        let raw = self
            .llm_adapter
            .generate_with_quality(&prompt, 4000, AiQuality::Balanced)
            .await?;

        // ── Parse AI JSON ──
        let ai_json = parse_json_response(&raw)?;

        // ── Convert to ProductDraft with confidence + field-level metadata ──
        let mut draft = build_product_draft(&ai_json, &input)?;

        // ── 🔥 AI Validation Layer — backend = source of truth ──
        let corrections = validate_draft(&mut draft);
        if !corrections.is_empty() {
            tracing::info!(
                "🔧 Validation Layer applied {} corrections for '{}'",
                corrections.len(),
                &input[..input.len().min(50)]
            );
        }

        // ── Cache the VALIDATED draft (post-correction) ──
        let draft_json = serde_json::to_value(&draft)
            .map_err(|e| AppError::internal(format!("Failed to serialize draft: {}", e)))?;
        if let Err(e) = self
            .ai_cache
            .set(
                &cache_key,
                draft_json,
                "groq",
                "llama-3.3-70b-versatile",
                DRAFT_CACHE_TTL_DAYS,
            )
            .await
        {
            tracing::warn!("Failed to cache draft: {}", e);
        }

        tracing::info!(
            "✅ AI draft created for '{}' (confidence: {:.0}%)",
            &input[..input.len().min(50)],
            draft.confidence * 100.0
        );

        Ok(CreateDraftResponse {
            draft,
            raw_input: input,
            model: "llama-3.3-70b-versatile".to_string(),
            cached: false,
            corrections,
        })
    }
}

// ── 🔥 AI Validation Layer ───────────────────────────────────────────
//
// AI ≠ source of truth. Backend = source of truth.
//
// This layer runs AFTER AI generation, BEFORE returning to frontend.
// It catches common AI mistakes and corrects them automatically.

/// Validate and correct an AI-generated draft.
/// Returns a list of corrections made (so UI can show "⚠️ Исправлено автоматически").
fn validate_draft(draft: &mut ProductDraft) -> Vec<DraftCorrection> {
    let mut corrections = Vec::new();

    let product_type = draft
        .product_type
        .value
        .as_deref()
        .unwrap_or("other")
        .to_string();

    // ── Rule 1: Fish/seafood category correction ──────────────────────
    // AI often classifies fish as "vegetable", "other", etc.
    // If the product is clearly aquatic, force product_type to the right value.
    let aquatic_keywords_en = [
        "salmon", "tuna", "cod", "trout", "carp", "herring", "mackerel",
        "sardine", "bass", "perch", "pike", "catfish", "tilapia", "halibut",
        "swordfish", "anchovy", "crucian", "bream", "zander", "walleye",
        "shrimp", "prawn", "lobster", "crab", "squid", "octopus", "mussel",
        "oyster", "clam", "scallop",
    ];
    let aquatic_keywords_ru = [
        "лосось", "тунец", "треска", "форель", "карп", "карась", "сельдь",
        "скумбрия", "сардина", "окунь", "щука", "сом", "тилапия", "палтус",
        "анчоус", "лещ", "судак", "сёмга", "горбуша", "кета", "минтай",
        "креветка", "лобстер", "краб", "кальмар", "осьминог", "мидия",
        "устрица", "гребешок",
    ];

    let name_en_lower = draft
        .names
        .en
        .value
        .as_deref()
        .unwrap_or("")
        .to_lowercase();
    let name_ru_lower = draft
        .names
        .ru
        .value
        .as_deref()
        .unwrap_or("")
        .to_lowercase();

    let is_aquatic_by_name = aquatic_keywords_en
        .iter()
        .any(|kw| name_en_lower.contains(kw))
        || aquatic_keywords_ru
            .iter()
            .any(|kw| name_ru_lower.contains(kw));

    // Determine correct product_type for aquatic products
    let fish_types = ["fish", "seafood", "fish_and_seafood"];
    let shellfish_keywords = [
        "shrimp", "prawn", "lobster", "crab", "squid", "octopus", "mussel",
        "oyster", "clam", "scallop", "креветка", "лобстер", "краб",
        "кальмар", "осьминог", "мидия", "устрица", "гребешок",
    ];

    if is_aquatic_by_name && !fish_types.iter().any(|t| product_type.eq_ignore_ascii_case(t)) {
        let is_shellfish = shellfish_keywords
            .iter()
            .any(|kw| name_en_lower.contains(kw) || name_ru_lower.contains(kw));
        let correct_type = if is_shellfish { "seafood" } else { "seafood" };

        corrections.push(DraftCorrection {
            field: "product_type".into(),
            original_value: product_type.clone(),
            corrected_to: correct_type.into(),
            reason: format!(
                "AI classified as '{}', but product name indicates fish/seafood",
                product_type
            ),
        });
        draft.product_type = DraftField {
            value: Some(correct_type.to_string()),
            source: DataSource::AiCorrected,
            confidence: FieldConfidence::High,
        };
    }

    // ── Rule 2: Meat category correction ──────────────────────────────
    let meat_keywords_en = [
        "beef", "pork", "lamb", "veal", "chicken", "turkey", "duck",
        "goose", "rabbit", "venison", "bacon", "ham", "sausage",
    ];
    let meat_keywords_ru = [
        "говядина", "свинина", "баранина", "телятина", "курица", "индейка",
        "утка", "гусь", "кролик", "оленина", "бекон", "ветчина", "колбаса",
        "фарш", "филе куриное", "грудка",
    ];
    let is_meat_by_name = meat_keywords_en
        .iter()
        .any(|kw| name_en_lower.contains(kw))
        || meat_keywords_ru
            .iter()
            .any(|kw| name_ru_lower.contains(kw));

    let meat_types = ["meat", "poultry", "meat_and_poultry"];
    let current_type = draft
        .product_type
        .value
        .as_deref()
        .unwrap_or("other");
    if is_meat_by_name && !meat_types.iter().any(|t| current_type.eq_ignore_ascii_case(t)) {
        corrections.push(DraftCorrection {
            field: "product_type".into(),
            original_value: current_type.to_string(),
            corrected_to: "meat".into(),
            reason: format!(
                "AI classified as '{}', but product name indicates meat/poultry",
                current_type
            ),
        });
        draft.product_type = DraftField {
            value: Some("meat".to_string()),
            source: DataSource::AiCorrected,
            confidence: FieldConfidence::High,
        };
    }

    // ── Rule 3: Dairy category correction ─────────────────────────────
    let dairy_keywords_en = [
        "milk", "cheese", "yogurt", "yoghurt", "butter", "cream", "kefir",
        "cottage cheese", "sour cream", "ricotta", "mozzarella",
    ];
    let dairy_keywords_ru = [
        "молоко", "сыр", "йогурт", "масло сливочное", "сливки", "кефир",
        "творог", "сметана", "рикотта", "моцарелла", "ряженка",
    ];
    let is_dairy_by_name = dairy_keywords_en
        .iter()
        .any(|kw| name_en_lower.contains(kw))
        || dairy_keywords_ru
            .iter()
            .any(|kw| name_ru_lower.contains(kw));

    let current_type = draft.product_type.value.as_deref().unwrap_or("other");
    if is_dairy_by_name && !current_type.eq_ignore_ascii_case("dairy") {
        corrections.push(DraftCorrection {
            field: "product_type".into(),
            original_value: current_type.to_string(),
            corrected_to: "dairy".into(),
            reason: format!(
                "AI classified as '{}', but product name indicates dairy",
                current_type
            ),
        });
        draft.product_type = DraftField {
            value: Some("dairy".to_string()),
            source: DataSource::AiCorrected,
            confidence: FieldConfidence::High,
        };
    }

    // ── Rule 4: Unit correction based on product_type ─────────────────
    let current_type = draft
        .product_type
        .value
        .as_deref()
        .unwrap_or("other");
    let current_unit = draft.unit.value.as_deref().unwrap_or("кг");

    let correct_unit = match current_type {
        // Fish and meat are always sold by weight
        t if ["fish", "seafood", "meat", "poultry", "fish_and_seafood", "meat_and_poultry"]
            .iter()
            .any(|x| t.eq_ignore_ascii_case(x)) =>
        {
            "кг"
        }
        // Liquids
        t if ["beverage", "oil"].iter().any(|x| t.eq_ignore_ascii_case(x)) => "л",
        // Dairy: depends on product
        "dairy" => {
            if name_en_lower.contains("milk")
                || name_en_lower.contains("cream")
                || name_en_lower.contains("kefir")
                || name_ru_lower.contains("молоко")
                || name_ru_lower.contains("сливки")
                || name_ru_lower.contains("кефир")
                || name_ru_lower.contains("ряженка")
            {
                "л"
            } else {
                "кг"
            }
        }
        // Eggs
        _ if name_en_lower.contains("egg") || name_ru_lower.contains("яйц") => "шт",
        // Everything else by weight
        _ => current_unit, // don't change if no strong rule
    };

    if correct_unit != current_unit {
        corrections.push(DraftCorrection {
            field: "unit".into(),
            original_value: current_unit.to_string(),
            corrected_to: correct_unit.into(),
            reason: format!(
                "Unit '{}' is incorrect for product type '{}', should be '{}'",
                current_unit, current_type, correct_unit
            ),
        });
        draft.unit = DraftField {
            value: Some(correct_unit.to_string()),
            source: DataSource::AiCorrected,
            confidence: FieldConfidence::High,
        };
    }

    // ── Rule 5: Fiber must be 0 (not null) for animal products ────────
    let current_type = draft.product_type.value.as_deref().unwrap_or("other");
    let animal_types = [
        "fish", "seafood", "meat", "poultry", "dairy",
        "fish_and_seafood", "meat_and_poultry", "dairy_and_eggs",
    ];
    let is_animal = animal_types
        .iter()
        .any(|t| current_type.eq_ignore_ascii_case(t));

    if is_animal {
        // Ensure fiber is marked NotApplicable (not just null)
        if !matches!(draft.nutrition.fiber_per_100g.confidence, FieldConfidence::NotApplicable) {
            draft.nutrition.fiber_per_100g = DraftField::not_applicable();
        }
    }

    // ── Rule 6: product_type normalization (plural → singular) ────────
    // Also REJECTS "other" — forces re-classification from name keywords
    let current_type = draft.product_type.value.as_deref().unwrap_or("other");

    // First: if product_type is "other", try to infer from name keywords
    if current_type.eq_ignore_ascii_case("other") || current_type.is_empty() {
        // Try to infer from all keyword lists we already checked
        let inferred = if is_aquatic_by_name {
            Some("seafood")
        } else if is_meat_by_name {
            Some("meat")
        } else if is_dairy_by_name {
            Some("dairy")
        } else {
            // Check additional categories
            let fruit_keywords = [
                "apple", "banana", "orange", "grape", "lemon", "lime", "mango",
                "peach", "pear", "plum", "cherry", "strawberry", "blueberry",
                "raspberry", "watermelon", "melon", "kiwi", "pineapple", "coconut",
                "яблоко", "банан", "апельсин", "виноград", "лимон", "лайм",
                "манго", "персик", "груша", "слива", "вишня", "клубника",
                "черника", "малина", "арбуз", "дыня", "киви", "ананас", "кокос",
            ];
            let vegetable_keywords = [
                "carrot", "potato", "tomato", "onion", "garlic", "pepper",
                "cucumber", "cabbage", "broccoli", "spinach", "lettuce", "celery",
                "zucchini", "eggplant", "corn", "peas", "beet", "radish",
                "морковь", "картофель", "помидор", "лук", "чеснок", "перец",
                "огурец", "капуста", "брокколи", "шпинат", "салат", "сельдерей",
                "кабачок", "баклажан", "кукуруза", "горох", "свёкла", "редис",
            ];
            let grain_keywords = [
                "rice", "wheat", "oat", "barley", "buckwheat", "corn", "quinoa",
                "pasta", "noodle", "bread", "flour",
                "рис", "пшеница", "овёс", "ячмень", "гречка", "кукуруза",
                "киноа", "паста", "макароны", "хлеб", "мука",
            ];
            let spice_keywords = [
                "cinnamon", "pepper", "turmeric", "cumin", "paprika", "basil",
                "oregano", "thyme", "rosemary", "dill", "parsley", "bay leaf",
                "корица", "перец", "куркума", "тмин", "паприка", "базилик",
                "орегано", "тимьян", "розмарин", "укроп", "петрушка", "лавр",
            ];

            let is_fruit = fruit_keywords.iter().any(|kw| name_en_lower.contains(kw) || name_ru_lower.contains(kw));
            let is_vegetable = vegetable_keywords.iter().any(|kw| name_en_lower.contains(kw) || name_ru_lower.contains(kw));
            let is_grain = grain_keywords.iter().any(|kw| name_en_lower.contains(kw) || name_ru_lower.contains(kw));
            let is_spice = spice_keywords.iter().any(|kw| name_en_lower.contains(kw) || name_ru_lower.contains(kw));

            if is_fruit {
                Some("fruit")
            } else if is_vegetable {
                Some("vegetable")
            } else if is_grain {
                Some("grain")
            } else if is_spice {
                Some("spice")
            } else {
                None // Truly unknown — leave as "other", Data Quality Engine will flag it
            }
        };

        if let Some(inferred_type) = inferred {
            corrections.push(DraftCorrection {
                field: "product_type".into(),
                original_value: current_type.to_string(),
                corrected_to: inferred_type.into(),
                reason: format!(
                    "AI returned '{}' — inferred '{}' from product name keywords",
                    current_type, inferred_type
                ),
            });
            draft.product_type = DraftField {
                value: Some(inferred_type.to_string()),
                source: DataSource::AiCorrected,
                confidence: FieldConfidence::High,
            };
        } else {
            // Add a quality warning so the admin knows to set it manually
            draft.quality_warnings.push(QualityWarning {
                field: "product_type".into(),
                label_ru: "Тип продукта".into(),
                severity: "critical".into(),
                message: "AI не смог определить тип продукта. Укажите вручную перед сохранением.".into(),
            });
            draft.needs_review = true;
        }
    }

    // Now normalize plural → singular for the (possibly corrected) product_type
    let current_type = draft.product_type.value.as_deref().unwrap_or("other");
    let normalized = match current_type {
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

    if let Some(singular) = normalized {
        corrections.push(DraftCorrection {
            field: "product_type".into(),
            original_value: current_type.to_string(),
            corrected_to: singular.into(),
            reason: format!("Normalized plural '{}' → singular '{}'", current_type, singular),
        });
        draft.product_type = DraftField {
            value: Some(singular.to_string()),
            source: DataSource::AiCorrected,
            confidence: FieldConfidence::High,
        };
    }

    corrections
}

// ── Helpers ──────────────────────────────────────────────────────────

fn hash_input(input: &str) -> String {
    let normalized = input.trim().to_lowercase();
    let mut hasher = Sha256::new();
    hasher.update(normalized.as_bytes());
    let result = hasher.finalize();
    format!("{:x}", result)[..16].to_string()
}

fn parse_json_response(raw: &str) -> AppResult<serde_json::Value> {
    serde_json::from_str(raw)
        .or_else(|_| {
            // Try extracting JSON from markdown code block or text
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
            tracing::error!("Failed to parse AI draft response: {}", e);
            AppError::internal("AI returned invalid JSON for draft")
        })
}

/// Build a ProductDraft from raw AI JSON, adding confidence levels
/// and integrating with the Field Requirement Engine
fn build_product_draft(
    ai: &serde_json::Value,
    _input: &str,
) -> AppResult<ProductDraft> {
    let str_val = |key: &str| -> Option<String> {
        ai.get(key).and_then(|v| v.as_str()).map(|s| s.to_string())
    };
    let f64_val = |key: &str| -> Option<f64> {
        ai.get(key).and_then(|v| v.as_f64())
    };
    let i32_val = |key: &str| -> Option<i32> {
        ai.get(key).and_then(|v| v.as_i64()).map(|v| v as i32)
    };

    // Detect product_type for field requirement rules
    let product_type = str_val("product_type").unwrap_or_else(|| "other".to_string());

    // ── Animal / plant detection for fiber logic ──
    let animal_types = [
        "fish", "seafood", "meat", "poultry", "eggs", "dairy",
        "fish_and_seafood", "meat_and_poultry", "dairy_and_eggs",
    ];
    let is_animal = animal_types
        .iter()
        .any(|t| product_type.eq_ignore_ascii_case(t));

    // ── Names ──
    let names_obj = ai.get("name");
    let name_en = names_obj
        .and_then(|n| n.get("en"))
        .and_then(|v| v.as_str())
        .or_else(|| ai.get("name_en").and_then(|v| v.as_str()))
        .unwrap_or("")
        .to_string();
    let name_ru = names_obj
        .and_then(|n| n.get("ru"))
        .and_then(|v| v.as_str())
        .or_else(|| ai.get("name_ru").and_then(|v| v.as_str()))
        .unwrap_or("")
        .to_string();
    let name_pl = names_obj
        .and_then(|n| n.get("pl"))
        .and_then(|v| v.as_str())
        .or_else(|| ai.get("name_pl").and_then(|v| v.as_str()))
        .unwrap_or("")
        .to_string();
    let name_uk = names_obj
        .and_then(|n| n.get("uk"))
        .and_then(|v| v.as_str())
        .or_else(|| ai.get("name_uk").and_then(|v| v.as_str()))
        .unwrap_or("")
        .to_string();

    // ── Confidence calc: count how many critical fields AI returned ──
    let mut fields_returned = 0u32;
    let mut fields_total = 12u32; // base critical fields
    if !name_en.is_empty() { fields_returned += 1; }
    if !name_ru.is_empty() { fields_returned += 1; }
    if !name_pl.is_empty() { fields_returned += 1; }
    if !name_uk.is_empty() { fields_returned += 1; }
    if f64_val("calories_per_100g").is_some() { fields_returned += 1; }
    if f64_val("protein_per_100g").is_some() { fields_returned += 1; }
    if f64_val("fat_per_100g").is_some() { fields_returned += 1; }
    if f64_val("carbs_per_100g").is_some() { fields_returned += 1; }
    if str_val("description_en").is_some() { fields_returned += 1; }
    if str_val("description_ru").is_some() { fields_returned += 1; }
    if str_val("product_type").is_some() { fields_returned += 1; }
    if str_val("unit").is_some() || ai.get("unit").is_some() { fields_returned += 1; }

    let confidence = fields_returned as f64 / fields_total as f64;
    let conf_level = if confidence >= 0.85 {
        FieldConfidence::High
    } else if confidence >= 0.6 {
        FieldConfidence::Medium
    } else {
        FieldConfidence::Low
    };

    // ── Fiber: respect Field Requirement Engine ──
    let fiber_field = if is_animal {
        DraftField::not_applicable()
    } else {
        DraftField::ai_opt(f64_val("fiber_per_100g"), conf_level.clone())
    };

    // ── Nutrition from AI JSON → get nested "nutrition" or flat ──
    let nutr = ai.get("nutrition");
    let nutr_f64 = |key: &str| -> Option<f64> {
        nutr.and_then(|n| n.get(key))
            .and_then(|v| v.as_f64())
            .or_else(|| f64_val(key))
    };
    let nutr_i32 = |key: &str| -> Option<i32> {
        nutr.and_then(|n| n.get(key))
            .and_then(|v| v.as_i64())
            .map(|v| v as i32)
            .or_else(|| i32_val(key))
    };

    // ── SEO ──
    let seo_obj = ai.get("seo");
    let seo_str = |key: &str| -> Option<String> {
        seo_obj
            .and_then(|s| s.get(key))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or_else(|| str_val(key))
    };

    // ── Seasons ──
    let seasons = ai
        .get("seasons")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect::<Vec<_>>()
        });

    // ── Quality warnings: find fields that AI couldn't fill ──
    let mut quality_warnings = Vec::new();

    if name_en.is_empty() {
        quality_warnings.push(QualityWarning {
            field: "name_en".into(),
            label_ru: "Название EN".into(),
            severity: "critical".into(),
            message: "AI не смог определить название на английском".into(),
        });
    }
    if nutr_f64("calories_per_100g").is_none() {
        quality_warnings.push(QualityWarning {
            field: "calories_per_100g".into(),
            label_ru: "Калории".into(),
            severity: "critical".into(),
            message: "AI не уверен в калорийности — проверьте вручную".into(),
        });
    }
    if nutr_f64("protein_per_100g").is_none() {
        quality_warnings.push(QualityWarning {
            field: "protein_per_100g".into(),
            label_ru: "Белки".into(),
            severity: "critical".into(),
            message: "AI не вернул данные по белку".into(),
        });
    }
    if !is_animal && nutr_f64("fiber_per_100g").is_none() {
        quality_warnings.push(QualityWarning {
            field: "fiber_per_100g".into(),
            label_ru: "Клетчатка".into(),
            severity: "recommended".into(),
            message: "Клетчатка важна для растительных продуктов — проверьте".into(),
        });
    }
    if seo_str("seo_title").is_none() {
        quality_warnings.push(QualityWarning {
            field: "seo_title".into(),
            label_ru: "SEO Title".into(),
            severity: "recommended".into(),
            message: "SEO-заголовок не сгенерирован — можно добавить позже".into(),
        });
    }

    let needs_review = confidence < 0.85 || !quality_warnings.is_empty();

    let draft = ProductDraft {
        names: DraftNames {
            en: DraftField::ai(name_en, conf_level.clone()),
            ru: DraftField::ai(name_ru, conf_level.clone()),
            pl: DraftField::ai(name_pl, conf_level.clone()),
            uk: DraftField::ai(name_uk, conf_level.clone()),
        },
        description_en: DraftField::ai_opt(str_val("description_en"), conf_level.clone()),
        description_ru: DraftField::ai_opt(str_val("description_ru"), conf_level.clone()),
        description_pl: DraftField::ai_opt(str_val("description_pl"), conf_level.clone()),
        description_uk: DraftField::ai_opt(str_val("description_uk"), conf_level.clone()),
        product_type: DraftField::ai(product_type, conf_level.clone()),
        unit: DraftField::ai(
            str_val("unit").unwrap_or_else(|| "кг".to_string()),
            conf_level.clone(),
        ),
        nutrition: DraftNutrition {
            calories_per_100g: DraftField::ai_opt(nutr_f64("calories_per_100g"), conf_level.clone()),
            protein_per_100g: DraftField::ai_opt(nutr_f64("protein_per_100g"), conf_level.clone()),
            fat_per_100g: DraftField::ai_opt(nutr_f64("fat_per_100g"), conf_level.clone()),
            carbs_per_100g: DraftField::ai_opt(nutr_f64("carbs_per_100g"), conf_level.clone()),
            fiber_per_100g: fiber_field,
            sugar_per_100g: DraftField::ai_opt(nutr_f64("sugar_per_100g"), conf_level.clone()),
            density_g_per_ml: DraftField::ai_opt(nutr_f64("density_g_per_ml"), conf_level.clone()),
            typical_portion_g: DraftField::ai_opt(nutr_f64("typical_portion_g"), conf_level.clone()),
            shelf_life_days: DraftField::ai_opt(nutr_i32("shelf_life_days"), conf_level.clone()),
        },
        seo: DraftSeo {
            seo_title: DraftField::ai_opt(seo_str("seo_title"), conf_level.clone()),
            seo_description: DraftField::ai_opt(seo_str("seo_description"), conf_level.clone()),
            seo_h1: DraftField::ai_opt(seo_str("seo_h1"), conf_level.clone()),
        },
        seasons: DraftField::ai_opt(seasons, conf_level.clone()),
        confidence,
        needs_review,
        quality_warnings,
    };

    Ok(draft)
}

// ── Prompt ───────────────────────────────────────────────────────────

fn build_draft_prompt(input: &str) -> String {
    format!(
        r#"You are a professional food database expert. The user wants to add a product to a restaurant catalog.

Input (may be in any language — Russian, Polish, Ukrainian, English):
"{input}"

Your task:
1. Identify the product
2. Translate the name into 4 languages
3. Classify: product_type, unit
4. Provide nutrition per 100g (raw, USDA reference)
5. Write short descriptions in 4 languages
6. Generate SEO fields
7. Determine seasonal availability

Return ONLY a valid JSON object with this exact structure:
{{
  "name": {{
    "en": "...",
    "ru": "...",
    "pl": "...",
    "uk": "..."
  }},
  "product_type": "<one of: fish, seafood, meat, poultry, dairy, vegetable, fruit, grain, legume, nut, spice, oil, beverage, other>",
  "unit": "<one of: кг, л, шт, уп, г>",
  "description_en": "<2-3 sentences, culinary description>",
  "description_ru": "<2-3 предложения, кулинарное описание>",
  "description_pl": "<2-3 zdania, opis kulinarny>",
  "description_uk": "<2-3 речення, кулінарний опис>",
  "calories_per_100g": <number>,
  "protein_per_100g": <number>,
  "fat_per_100g": <number>,
  "carbs_per_100g": <number>,
  "fiber_per_100g": <number or 0 for animal products>,
  "sugar_per_100g": <number or null>,
  "density_g_per_ml": <number or null>,
  "typical_portion_g": <number>,
  "shelf_life_days": <integer>,
  "seasons": ["Spring", "Summer", "Autumn", "Winter"],
  "seo": {{
    "seo_title": "<SEO title for product page, 50-60 chars>",
    "seo_description": "<Meta description, 120-160 chars>",
    "seo_h1": "<H1 heading for product page>"
  }}
}}

Rules:
- All nutrition per 100g RAW product (use USDA FoodData Central as reference)
- For animal products (meat, fish, dairy, eggs): fiber_per_100g = 0
- product_type MUST be singular: "fruit" not "fruits", "vegetable" not "vegetables"
- unit should match the product: liquids = "л", solids = "кг", eggs = "шт"
- seasons: use ["AllYear"] if available year-round
- Be precise with nutrition values — don't guess if unsure, use null
- Return ONLY valid JSON, no extra text"#,
        input = input
    )
}

//! SmartResponse — output contract for SmartService.

use serde::{Deserialize, Serialize};
use crate::domain::tools::flavor_graph::{FlavorVector, FlavorBalance};
use crate::domain::tools::nutrition::{NutritionBreakdownNullable, VitaminData};

/// The single intelligent response for `POST /api/smart/ingredient`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartResponse {
    /// Main ingredient info
    pub ingredient: IngredientInfo,

    /// Cooking state data (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<StateInfo>,

    /// Nutrition per 100g (null-safe: missing = null, not 0)
    pub nutrition: NutritionBreakdownNullable,

    /// Vitamins & minerals (static USDA lookup)
    pub vitamins: VitaminData,

    /// Flavor profile (6D vector + balance analysis)
    pub flavor_profile: FlavorProfileInfo,

    /// Top pairings from DB
    pub pairings: Vec<PairingInfo>,

    /// Suggested ingredients to complement (from SuggestionEngine)
    pub suggestions: Vec<SuggestionInfo>,

    /// Recipe diagnostics (from RuleEngine — if additional ingredients present)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diagnostics: Option<DiagnosticsInfo>,

    /// Seasonality data
    pub seasonality: Vec<SeasonalityInfo>,

    /// Human-readable deterministic explanation (no AI)
    pub explain: Vec<String>,

    /// Response metadata
    pub meta: SmartMeta,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngredientInfo {
    pub slug: String,
    pub name: String,
    pub image_url: Option<String>,
    pub product_type: Option<String>,
    pub sushi_grade: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateInfo {
    pub state: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlavorProfileInfo {
    /// Raw 6D flavor vector
    pub vector: FlavorVector,
    /// Balance analysis
    pub balance: FlavorBalance,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairingInfo {
    pub slug: String,
    pub name: String,
    pub image_url: Option<String>,
    pub pair_score: f64,
    pub flavor_score: Option<f64>,
    pub nutrition_score: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestionInfo {
    pub slug: String,
    pub name: String,
    pub image_url: Option<String>,
    /// 0–100 recommendation score
    pub score: u8,
    /// Why this ingredient is suggested
    pub reasons: Vec<String>,
    /// Which weak flavor dimensions it fills
    pub fills_gaps: Vec<String>,
    /// Suggested amount in grams
    pub suggested_grams: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticsInfo {
    /// 0–100 overall health score
    pub health_score: u8,
    /// Per-category scores
    pub category_scores: std::collections::HashMap<String, u8>,
    /// Issues found (critical / warning / info)
    pub issues: Vec<DiagnosticIssue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticIssue {
    pub severity: String,
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeasonalityInfo {
    pub month: i32,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartMeta {
    pub timing_ms: u64,
    pub cached: bool,
    pub cache_key: String,
    pub engine_version: String,
}

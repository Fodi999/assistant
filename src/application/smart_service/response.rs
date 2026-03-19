//! SmartResponse — output contract for SmartService v3.

use serde::{Deserialize, Serialize};
use crate::domain::tools::flavor_graph::{FlavorVector, FlavorBalance};
use crate::domain::tools::nutrition::{NutritionBreakdownNullable, VitaminData};

/// The single intelligent response for `POST /api/smart/ingredient`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartResponse {
    /// Main ingredient info
    pub ingredient: IngredientInfo,

    /// Cooking state data (if available) — v2: full nutrition + cooking details
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<StateInfo>,

    /// Nutrition per 100g (null-safe: missing = null, not 0)
    /// v2: adjusted by cooking state when state is provided
    pub nutrition: NutritionBreakdownNullable,

    /// Vitamins & minerals (static USDA lookup)
    pub vitamins: VitaminData,

    /// Flavor profile (6D vector + balance analysis)
    /// v2: flavor vector modified by cooking state
    pub flavor_profile: FlavorProfileInfo,

    /// Top pairings from DB
    pub pairings: Vec<PairingInfo>,

    /// Suggested ingredients to complement (from SuggestionEngine)
    /// v3: goal-aware scoring + feedback-loop from diagnostics
    pub suggestions: Vec<SuggestionInfo>,

    /// Recipe diagnostics (from RuleEngine — if additional ingredients present)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diagnostics: Option<DiagnosticsInfo>,

    /// Unit equivalents (g, kg, ml, cups, tbsp, tsp, etc.) — v2
    pub equivalents: Vec<EquivalentInfo>,

    /// Seasonality data
    pub seasonality: Vec<SeasonalityInfo>,

    /// Confidence scores for each data source (v3)
    pub confidence: ConfidenceInfo,

    /// Actionable next steps derived from diagnostics + goal (v3)
    pub next_actions: Vec<NextAction>,

    /// Human-readable deterministic explanation (no AI)
    /// v3: goal-aware + feedback-loop explanations
    pub explain: Vec<String>,

    /// Session ID for continuity (v3)
    pub session_id: String,

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
    /// Nutrition per 100g *in this state* (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nutrition: Option<StateNutrition>,
    /// Texture in this state (e.g. "crispy", "tender")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub texture: Option<String>,
    /// Weight change percent (negative = loss, positive = gain)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight_change_percent: Option<f64>,
    /// Oil absorbed per 100g (mainly fried states)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oil_absorption_g: Option<f64>,
    /// Water lost during cooking (0-100%)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub water_loss_percent: Option<f64>,
    /// Glycemic index in this state
    #[serde(skip_serializing_if = "Option::is_none")]
    pub glycemic_index: Option<i16>,
    /// Storage shelf life (hours)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shelf_life_hours: Option<i32>,
    /// Storage temperature (°C)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage_temp_c: Option<i32>,
}

/// Nutrition values for a specific cooking state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateNutrition {
    pub calories: Option<f64>,
    pub protein_g: Option<f64>,
    pub fat_g: Option<f64>,
    pub carbs_g: Option<f64>,
    pub fiber_g: Option<f64>,
    pub water_percent: Option<f64>,
}

/// Unit equivalent for an ingredient (100g expressed in other units).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquivalentInfo {
    pub unit: String,
    pub label: String,
    pub value: f64,
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

// ── v3: Confidence System ────────────────────────────────────────────────────

/// Confidence scores indicating data completeness / reliability (0.0–1.0).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceInfo {
    /// Overall confidence (weighted average of sub-scores)
    pub overall: f64,
    /// How complete nutrition data is (fields present / total fields)
    pub nutrition: f64,
    /// How confident we are in pairing recommendations
    pub pairings: f64,
    /// How reliable the flavor vector is (culinary props in DB?)
    pub flavor: f64,
}

// ── v3: Next Actions ─────────────────────────────────────────────────────────

/// A concrete actionable step the user can take next.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NextAction {
    /// Action type: "add", "remove", "swap", "adjust"
    #[serde(rename = "type")]
    pub action_type: String,
    /// Target ingredient slug (clickable)
    pub ingredient: String,
    /// Human-readable reason
    pub reason: String,
    /// Priority: 1 = highest
    pub priority: u8,
}

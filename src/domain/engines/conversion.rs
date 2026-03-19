//! ConversionEngine — trait definition.
//!
//! Handles all unit conversion, scaling, and density-aware operations.
//! Implementation lives in `application::engines::conversion_impl`.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

// ── Request / Response types ─────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ConvertRequest {
    pub value: f64,
    pub from:  String,
    pub to:    String,
    pub lang:  Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ConvertResult {
    pub value:        f64,
    pub from:         String,
    pub to:           String,
    pub result:       f64,
    pub from_label:   String,
    pub to_label:     String,
    pub supported:    bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub smart_result: Option<SmartUnitResult>,
}

#[derive(Debug, Serialize)]
pub struct SmartUnitResult {
    pub value: f64,
    pub unit:  String,
    pub label: String,
}

#[derive(Debug, Deserialize)]
pub struct IngredientConvertRequest {
    pub ingredient: String,
    pub value:      f64,
    pub from:       String,
    pub to:         String,
    pub lang:       Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ScaleRequest {
    pub ingredient:    Option<String>,
    pub value:         f64,
    pub unit:          Option<String>,
    pub from_portions: f64,
    pub to_portions:   f64,
    pub lang:          Option<String>,
}

// ── Engine trait ──────────────────────────────────────────────────────────────

/// ConversionEngine: all unit math lives here.
///
/// Pure conversions (no DB) are implemented directly.
/// Density-aware conversions need the catalog → DB access in impl.
#[async_trait]
pub trait ConversionEngine: Send + Sync {
    /// Universal convert: mass ↔ mass, volume ↔ volume.
    async fn convert(&self, req: ConvertRequest) -> ConvertResult;

    /// Density-aware cross-group: cup flour → grams (needs DB).
    async fn ingredient_convert(&self, req: IngredientConvertRequest) -> serde_json::Value;

    /// Scale ingredient by portion ratio.
    async fn ingredient_scale(&self, req: ScaleRequest) -> serde_json::Value;
}

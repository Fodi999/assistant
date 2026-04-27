//! DTOs for the Laboratory HTTP API.
//!
//! These types are the on-wire contract — they intentionally use plain
//! `String` / `f64` / `Decimal` and `serde_json::Value` so the frontend can
//! receive a single, fully-hydrated project document with one round-trip.

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::infrastructure::persistence::laboratory_repository::{
    LabProcessStepRow, LabProjectAnalysisRow, LabProjectIngredientRow, LabProjectRow,
};

// ─────────────────────────────────────────────────────────────────────────────
// Project
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct LabProjectSummary {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub target_product_type: Option<String>,
    pub status: String,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

impl From<LabProjectRow> for LabProjectSummary {
    fn from(r: LabProjectRow) -> Self {
        Self {
            id: r.id,
            name: r.name,
            description: r.description,
            target_product_type: r.target_product_type,
            status: r.status,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

/// Full project document — the response shape for both POST (create) and
/// GET /:id. Always includes `ingredients`, `process_steps`, `latest_analysis`
/// (possibly empty / null) so the frontend doesn't need a follow-up request.
#[derive(Debug, Clone, Serialize)]
pub struct LabProjectFull {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub target_product_type: Option<String>,
    pub status: String,
    pub ingredients: Vec<LabProjectIngredientDto>,
    pub process_steps: Vec<LabProcessStepDto>,
    pub latest_analysis: Option<LabProjectAnalysisDto>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateLabProjectRequest {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub target_product_type: Option<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Ingredient
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct LabProjectIngredientDto {
    pub id: Uuid,
    pub ingredient_slug: String,
    pub quantity: Decimal,
    pub unit: String,
    pub role: Option<String>,
    pub sort_order: i32,
    pub notes: Option<String>,
    pub created_at: OffsetDateTime,
    /// Set to `Some(true)` when this row was returned from an `add_ingredient`
    /// call that merged into an existing line (slug+unit already in project).
    /// Omitted from the JSON payload otherwise — list/get endpoints never
    /// emit this field.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub merged: Option<bool>,
}

impl From<LabProjectIngredientRow> for LabProjectIngredientDto {
    fn from(r: LabProjectIngredientRow) -> Self {
        Self {
            id: r.id,
            ingredient_slug: r.ingredient_slug,
            quantity: r.quantity,
            unit: r.unit,
            role: r.role,
            sort_order: r.sort_order,
            notes: r.notes,
            created_at: r.created_at,
            merged: None,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct AddLabIngredientRequest {
    pub ingredient_slug: String,
    pub quantity: Decimal,
    pub unit: String,
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub sort_order: Option<i32>,
    #[serde(default)]
    pub notes: Option<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Process step
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct LabProcessStepDto {
    pub id: Uuid,
    pub order_index: i32,
    pub technique: String,
    pub temperature_c: Option<Decimal>,
    pub duration_min: Option<i32>,
    pub target_slugs: Vec<String>,
    pub notes: Option<String>,
    pub created_at: OffsetDateTime,
}

impl From<LabProcessStepRow> for LabProcessStepDto {
    fn from(r: LabProcessStepRow) -> Self {
        Self {
            id: r.id,
            order_index: r.order_index,
            technique: r.technique,
            temperature_c: r.temperature_c,
            duration_min: r.duration_min,
            target_slugs: r.target_slugs.unwrap_or_default(),
            notes: r.notes,
            created_at: r.created_at,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct AddLabStepRequest {
    pub technique: String,
    #[serde(default)]
    pub order_index: Option<i32>,
    #[serde(default)]
    pub temperature_c: Option<Decimal>,
    #[serde(default)]
    pub duration_min: Option<i32>,
    #[serde(default)]
    pub target_slugs: Option<Vec<String>>,
    #[serde(default)]
    pub notes: Option<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Analysis (read-only on this step — write path arrives in Step 4)
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct LabProjectAnalysisDto {
    pub id: Uuid,
    pub shelf_life_days: Option<i32>,
    pub estimated_cost: Option<Decimal>,
    pub complexity_score: Option<i32>,
    pub risk_level: Option<String>,
    pub texture_result: Option<String>,
    pub flavor_result: JsonValue,
    pub nutrition_result: JsonValue,
    pub process_effects: JsonValue,
    pub storage_recommendations: JsonValue,
    pub pairing_suggestions: JsonValue,
    pub warnings: JsonValue,
    pub created_at: OffsetDateTime,
}

impl From<LabProjectAnalysisRow> for LabProjectAnalysisDto {
    fn from(r: LabProjectAnalysisRow) -> Self {
        Self {
            id: r.id,
            shelf_life_days: r.shelf_life_days,
            estimated_cost: r.estimated_cost,
            complexity_score: r.complexity_score,
            risk_level: r.risk_level,
            texture_result: r.texture_result,
            flavor_result: r.flavor_result,
            nutrition_result: r.nutrition_result,
            process_effects: r.process_effects,
            storage_recommendations: r.storage_recommendations,
            pairing_suggestions: r.pairing_suggestions,
            warnings: r.warnings,
            created_at: r.created_at,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Copilot
// ─────────────────────────────────────────────────────────────────────────────

/// `POST /api/laboratory/copilot/suggest` — request body.
#[derive(Debug, Deserialize)]
pub struct CopilotSuggestRequest {
    /// Free-form text describing the desired product.
    /// Examples: "strawberry sauce without sugar", "абрикосовый соус".
    pub prompt: String,
}

/// `POST /api/laboratory/copilot/suggest` — response body.
#[derive(Debug, Clone, Serialize)]
pub struct CopilotSuggestResponse {
    pub product_type: String,
    pub suggested_name: String,
    pub ingredients: Vec<CopilotSuggestIngredient>,
    pub steps: Vec<CopilotSuggestStep>,
    pub rationale: String,
    pub confidence: f64,
    pub unmatched_tokens: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CopilotSuggestIngredient {
    pub slug: String,
    pub quantity: f64,
    pub unit: String,
    pub role: String,
    /// `true` if the slug exists in the `products` catalog.
    /// `false` = the ingredient was matched by keyword but is not in the DB.
    pub in_catalog: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct CopilotSuggestStep {
    pub technique: String,
    pub temperature_c: Option<f64>,
    pub duration_min: Option<u32>,
    pub note: String,
}

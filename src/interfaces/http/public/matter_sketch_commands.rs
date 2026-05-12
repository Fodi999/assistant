//! Precision sketch command endpoints.
//!
//!   POST /api/matter/sketch/validate
//!   POST /api/matter/sketch/add-point
//!   POST /api/matter/sketch/add-edge
//!
//! All endpoints accept a SketchGraph + parameters and return a
//! `SketchCommandResult` (or `ValidationResult` for /validate). Pure
//! geometry — no DB / no auth required at this phase.

use axum::{Json, http::StatusCode};
use serde::Deserialize;

use crate::domain::matter::{
    apply_add_edge, apply_add_point,
    commands::{AddEdgeRequest, AddPointRequest, SketchCommandResult},
    sketch::SketchGraph,
    validation::{validate, ValidationResult},
};

#[derive(Deserialize)]
pub struct ValidateRequest {
    pub sketch: SketchGraph,
}

pub async fn validate_sketch_endpoint(
    Json(req): Json<ValidateRequest>,
) -> Result<Json<ValidationResult>, (StatusCode, Json<serde_json::Value>)> {
    Ok(Json(validate(&req.sketch)))
}

pub async fn add_point_endpoint(
    Json(req): Json<AddPointRequest>,
) -> Result<Json<SketchCommandResult>, (StatusCode, Json<serde_json::Value>)> {
    Ok(Json(apply_add_point(req)))
}

pub async fn add_edge_endpoint(
    Json(req): Json<AddEdgeRequest>,
) -> Result<Json<SketchCommandResult>, (StatusCode, Json<serde_json::Value>)> {
    Ok(Json(apply_add_edge(req)))
}

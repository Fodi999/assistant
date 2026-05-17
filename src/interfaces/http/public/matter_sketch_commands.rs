//! Precision sketch command endpoints.
//!
//!   POST /api/matter/sketch/validate
//!   POST /api/matter/sketch/add-point
//!   POST /api/matter/sketch/add-edge
//!   POST /api/matter/sketch/move-point
//!
//! All endpoints accept a SketchGraph + parameters and return a
//! `SketchCommandResult` (or `ValidationResult` for /validate). Pure
//! geometry — no DB / no auth required at this phase.

use axum::{Json, http::StatusCode};
use serde::Deserialize;

use crate::domain::matter::{
    apply_add_edge, apply_add_point, apply_move_point,
    commands::{AddEdgeRequest, AddPointRequest, MovePointRequest, SketchCommandResult},
    sketch::SketchGraph,
    validation::{validate, ValidationResult},
    analyze_profile, repair_profile,
    ProfileAnalyzeRequest, ProfileAnalyzeResponse,
    ProfileRepairRequest, ProfileRepairResponse,
    solve_constraints, apply_constraint_once,
    SolveConstraintsRequest, SolveResult,
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

pub async fn move_point_endpoint(
    Json(req): Json<MovePointRequest>,
) -> Result<Json<SketchCommandResult>, (StatusCode, Json<serde_json::Value>)> {
    Ok(Json(apply_move_point(req)))
}

pub async fn profile_analyze_endpoint(
    Json(req): Json<ProfileAnalyzeRequest>,
) -> Result<Json<ProfileAnalyzeResponse>, (StatusCode, Json<serde_json::Value>)> {
    Ok(Json(analyze_profile(req)))
}

pub async fn profile_repair_endpoint(
    Json(req): Json<ProfileRepairRequest>,
) -> Result<Json<ProfileRepairResponse>, (StatusCode, Json<serde_json::Value>)> {
    Ok(Json(repair_profile(req)))
}

/// POST /api/matter/sketch/solve-constraints
/// Applies HORIZONTAL / VERTICAL / EQUAL_LENGTH constraints to the sketch.
/// If `constraint` field is provided, applies only that one (preview mode).
/// Otherwise applies all constraints in sketch.constraints.
pub async fn solve_constraints_endpoint(
    Json(req): Json<SolveConstraintsRequest>,
) -> Result<Json<SolveResult>, (StatusCode, Json<serde_json::Value>)> {
    let result = if let Some(ref c) = req.constraint {
        apply_constraint_once(req.sketch, c)
    } else {
        solve_constraints(req.sketch)
    };
    Ok(Json(result))
}

//! Precision sketch command endpoints — stubbed (geometry_engine extracted).

use axum::{Json, http::StatusCode};

use crate::domain::matter::{
    apply_add_edge, apply_add_point, apply_move_point,
    AddEdgeRequest, AddPointRequest, MovePointRequest, SketchCommandResult,
    SketchGraph, validate, ValidationResult,
    analyze_profile, repair_profile,
    ProfileAnalyzeRequest, ProfileAnalyzeResponse,
    ProfileRepairRequest, ProfileRepairResponse,
    solve_constraints, SolveConstraintsRequest, SolveResult,
};

type Resp<T> = Result<Json<T>, (StatusCode, Json<serde_json::Value>)>;

pub async fn validate_sketch_endpoint(Json(_req): Json<SketchGraph>) -> Resp<ValidationResult> {
    Ok(Json(validate(&Default::default())))
}

pub async fn add_point_endpoint(Json(req): Json<AddPointRequest>) -> Resp<SketchCommandResult> {
    Ok(Json(apply_add_point(Default::default(), req)))
}

pub async fn add_edge_endpoint(Json(req): Json<AddEdgeRequest>) -> Resp<SketchCommandResult> {
    Ok(Json(apply_add_edge(Default::default(), req)))
}

pub async fn move_point_endpoint(Json(req): Json<MovePointRequest>) -> Resp<SketchCommandResult> {
    Ok(Json(apply_move_point(Default::default(), req)))
}

pub async fn profile_analyze_endpoint(Json(req): Json<ProfileAnalyzeRequest>) -> Resp<ProfileAnalyzeResponse> {
    Ok(Json(analyze_profile(req)))
}

pub async fn profile_repair_endpoint(Json(req): Json<ProfileRepairRequest>) -> Resp<ProfileRepairResponse> {
    Ok(Json(repair_profile(req)))
}

pub async fn solve_constraints_endpoint(Json(req): Json<SolveConstraintsRequest>) -> Resp<SolveResult> {
    Ok(Json(solve_constraints(req)))
}

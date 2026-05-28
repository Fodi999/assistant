//! Sketch extrude endpoint — stubbed (geometry_engine extracted to separate crate).

use axum::{Json, http::StatusCode};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct ExtrudeRequest {}

#[derive(Serialize)]
pub struct ExtrudeResponse { pub ok: bool }

pub async fn extrude_endpoint(
    Json(_req): Json<ExtrudeRequest>,
) -> Result<Json<ExtrudeResponse>, (StatusCode, Json<serde_json::Value>)> {
    Err((StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!({"error": "geometry_engine is a separate service"}))))
}

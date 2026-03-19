//! HTTP handlers for the Culinary Intelligence Platform.
//!
//! `POST /public/tools/run`   — unified entry point (RuleBot)
//! `GET  /public/tools/catalog` — full tool registry

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use std::sync::Arc;

use crate::application::rulebot::orchestrator::{RuleBot, RunToolRequest, RunToolResponse};
use crate::domain::engines::registry;

// ── POST /public/tools/run ───────────────────────────────────────────────────

pub async fn tools_run(
    State(bot): State<Arc<RuleBot>>,
    Json(req): Json<RunToolRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    let response = bot.run(req).await;

    match response {
        RunToolResponse::Ok(r) => {
            let status = StatusCode::OK;
            let body = serde_json::to_value(r).unwrap_or_default();
            (status, Json(body))
        }
        RunToolResponse::Err(e) => {
            let status = if e.code == "TOOL_NOT_FOUND" {
                StatusCode::NOT_FOUND
            } else if e.code == "BAD_REQUEST" {
                StatusCode::BAD_REQUEST
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            let body = serde_json::to_value(e).unwrap_or_default();
            (status, Json(body))
        }
    }
}

// ── GET /public/tools/catalog ────────────────────────────────────────────────

pub async fn tools_catalog() -> Json<serde_json::Value> {
    let catalog = registry::build_catalog();
    Json(serde_json::to_value(catalog).unwrap_or_default())
}

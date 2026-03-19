//! RuleBot Orchestrator — routes tool requests to the right engine.
//!
//! This is the brain of `POST /public/tools/run`.
//! It resolves tool ID → engine, validates params, times execution,
//! wraps result in ToolResponse envelope, and logs analytics.

use std::sync::Arc;
use std::time::Instant;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::PgPool;

use crate::domain::engines::types::ToolId;
use crate::domain::engines::response::{ToolResponse, ToolError};
use crate::domain::engines::registry;

// ── Request ──────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct RunToolRequest {
    /// Tool identifier (path-style): "convert", "fish-season-table", etc.
    pub tool:   String,
    /// Tool parameters as a JSON object.
    #[serde(default)]
    pub params: Value,
}

// ── Response ─────────────────────────────────────────────────────────────────

/// Unified response: wraps any tool result in the standard envelope.
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum RunToolResponse {
    Ok(ToolResponse<Value>),
    Err(ToolError),
}

// ── Orchestrator ─────────────────────────────────────────────────────────────

pub struct RuleBot {
    pool: PgPool,
}

impl RuleBot {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Route a tool request to the correct engine and return the envelope.
    pub async fn run(&self, req: RunToolRequest) -> RunToolResponse {
        let start = Instant::now();

        // 1. Resolve tool ID
        let tool_id = match resolve_tool_id(&req.tool) {
            Some(id) => id,
            None => return RunToolResponse::Err(ToolError::not_found(&req.tool)),
        };

        // 2. Special case: catalog
        if tool_id == ToolId::ToolsCatalog {
            let catalog = registry::build_catalog();
            let data = serde_json::to_value(catalog).unwrap_or_default();
            let timing = start.elapsed().as_millis() as u64;
            return RunToolResponse::Ok(ToolResponse::ok(tool_id, data, timing));
        }

        // 3. Delegate to the appropriate legacy handler via forward.
        //    This is the bridge phase: RuleBot re-uses existing handlers
        //    by calling them through the pool. As we refactor each handler
        //    into an engine impl, we'll replace these one by one.
        let result = self.dispatch(tool_id, &req.params).await;
        let timing = start.elapsed().as_millis() as u64;

        match result {
            Ok(data) => RunToolResponse::Ok(ToolResponse::ok(tool_id, data, timing)),
            Err(msg) => RunToolResponse::Err(ToolError::internal(tool_id, msg)),
        }
    }

    /// Dispatch to the correct engine/handler.
    /// Phase 1: delegates to existing handler functions.
    /// Phase 2+: will call engine trait methods directly.
    async fn dispatch(&self, tool: ToolId, params: &Value) -> Result<Value, String> {
        use ToolId::*;

        match tool {
            // ── Conversion Engine ──
            Convert => {
                let value = param_f64(params, "value").unwrap_or(0.0);
                let from  = param_str(params, "from").unwrap_or_default();
                let to    = param_str(params, "to").unwrap_or_default();

                use crate::domain::tools::unit_converter as uc;
                let result_raw = uc::convert_units(value, &from, &to);
                let supported  = result_raw.is_some();
                let result     = uc::display_round(result_raw.unwrap_or(0.0));

                Ok(serde_json::json!({
                    "value": value,
                    "from": from,
                    "to": to,
                    "result": result,
                    "supported": supported,
                }))
            }

            ListUnits => {
                use crate::domain::tools::unit_converter as uc;
                Ok(serde_json::json!({
                    "mass":   uc::mass_units(),
                    "volume": uc::volume_units(),
                }))
            }

            Scale => {
                let value = param_f64(params, "value").unwrap_or(0.0);
                let from  = param_f64(params, "from_portions").unwrap_or(1.0);
                let to    = param_f64(params, "to_portions").unwrap_or(1.0);
                use crate::domain::tools::unit_converter as uc;
                let scaled = uc::scale(value, from, to);
                Ok(serde_json::json!({
                    "original": value,
                    "from_portions": from,
                    "to_portions": to,
                    "scaled": uc::display_round(scaled),
                }))
            }

            Yield => {
                let raw    = param_f64(params, "raw_weight").unwrap_or(0.0);
                let usable = param_f64(params, "usable_weight").unwrap_or(0.0);
                use crate::domain::tools::unit_converter as uc;
                let pct   = uc::yield_percent(raw, usable);
                let waste = if raw > 0.0 { raw - usable } else { 0.0 };
                Ok(serde_json::json!({
                    "raw_weight": raw,
                    "usable_weight": usable,
                    "yield_percent": uc::round_to(pct, 2),
                    "waste_grams": uc::round_to(waste, 2),
                    "waste_percent": uc::round_to(100.0 - pct, 2),
                }))
            }

            FoodCost => {
                let price    = param_f64(params, "price").unwrap_or(0.0);
                let amount   = param_f64(params, "amount").unwrap_or(0.0);
                let portions = param_f64(params, "portions").unwrap_or(1.0);
                let sell     = param_f64(params, "sell_price");
                use crate::domain::tools::unit_converter as uc;
                let total = uc::food_cost(price, amount);
                let per_p = uc::cost_per_portion(total, portions);
                let margin = sell.map(|s| uc::margin_percent(s, total));
                Ok(serde_json::json!({
                    "total_cost": uc::round_to(total, 2),
                    "cost_per_portion": uc::round_to(per_p, 2),
                    "margin_percent": margin.map(|m| uc::round_to(m, 2)),
                }))
            }

            ToolsCatalog => {
                // Handled above, but just in case
                let catalog = registry::build_catalog();
                Ok(serde_json::to_value(catalog).unwrap_or_default())
            }

            // ── All other tools: return "not yet migrated" with hint ──
            other => {
                Err(format!(
                    "Tool '{}' is not yet migrated to RuleBot engine layer. \
                     Use the legacy endpoint: GET /public/tools/{}",
                    other.path(), other.path()
                ))
            }
        }
    }
}

// ── Tool ID resolution ───────────────────────────────────────────────────────

fn resolve_tool_id(name: &str) -> Option<ToolId> {
    // Match by path (the most natural identifier)
    ToolId::all().iter().find(|t| t.path() == name).copied()
}

// ── Param extraction helpers ─────────────────────────────────────────────────

fn param_str(params: &Value, key: &str) -> Option<String> {
    params.get(key).and_then(|v| v.as_str()).map(|s| s.to_string())
}

fn param_f64(params: &Value, key: &str) -> Option<f64> {
    params.get(key).and_then(|v| v.as_f64())
}

#[allow(dead_code)]
fn param_i64(params: &Value, key: &str) -> Option<i64> {
    params.get(key).and_then(|v| v.as_i64())
}

//! # WASM bindings — sketch_engine
//!
//! Three exported functions accept and return JSON strings (UTF-8) so the
//! JS bridge can pass the same shape it already sends to the HTTP API.
//!
//! Build:
//! ```bash
//! wasm-pack build crates/sketch_engine \
//!   --target web --features wasm \
//!   --out-dir ../../static/wasm/sketch_engine
//! ```

use wasm_bindgen::prelude::*;

use crate::commands::{apply_add_edge, apply_add_point, apply_move_point, AddEdgeRequest, AddPointRequest, MovePointRequest};
use crate::sketch::SketchGraph;
use crate::validation::validate;
use crate::solver::{solve_constraints, apply_constraint_once, SolveConstraintsRequest};

fn err_json(msg: impl Into<String>) -> String {
    serde_json::json!({ "ok": false, "error": msg.into() }).to_string()
}

/// `{ "sketch": <SketchGraph> }` → JSON-encoded `ValidationResult`.
#[wasm_bindgen]
pub fn wasm_validate_sketch(json: &str) -> String {
    #[derive(serde::Deserialize)]
    struct Req {
        sketch: SketchGraph,
    }
    let req: Req = match serde_json::from_str(json) {
        Ok(v) => v,
        Err(e) => return err_json(format!("bad json: {e}")),
    };
    let result = validate(&req.sketch);
    serde_json::to_string(&result).unwrap_or_else(|e| err_json(e.to_string()))
}

/// JSON-encoded `AddPointRequest` → JSON-encoded `SketchCommandResult`.
#[wasm_bindgen]
pub fn wasm_add_point(json: &str) -> String {
    let req: AddPointRequest = match serde_json::from_str(json) {
        Ok(v) => v,
        Err(e) => return err_json(format!("bad json: {e}")),
    };
    let result = apply_add_point(req);
    serde_json::to_string(&result).unwrap_or_else(|e| err_json(e.to_string()))
}

/// JSON-encoded `AddEdgeRequest` → JSON-encoded `SketchCommandResult`.
#[wasm_bindgen]
pub fn wasm_add_edge(json: &str) -> String {
    let req: AddEdgeRequest = match serde_json::from_str(json) {
        Ok(v) => v,
        Err(e) => return err_json(format!("bad json: {e}")),
    };
    let result = apply_add_edge(req);
    serde_json::to_string(&result).unwrap_or_else(|e| err_json(e.to_string()))
}

/// JSON-encoded `MovePointRequest` → JSON-encoded `SketchCommandResult`.
#[wasm_bindgen]
pub fn wasm_move_point(json: &str) -> String {
    let req: MovePointRequest = match serde_json::from_str(json) {
        Ok(v) => v,
        Err(e) => return err_json(format!("bad json: {e}")),
    };
    let result = apply_move_point(req);
    serde_json::to_string(&result).unwrap_or_else(|e| err_json(e.to_string()))
}

/// Returns engine version + build info — used for handshake checks.
#[wasm_bindgen]
pub fn wasm_engine_info() -> String {
    serde_json::json!({
        "name": "sketch_engine",
        "version": env!("CARGO_PKG_VERSION"),
        "schema": "sketch_graph",
        "schemaVersion": 1u32
    })
    .to_string()
}

/// JSON-encoded `SolveConstraintsRequest` → JSON-encoded `SolveResult`.
///
/// Applies HORIZONTAL / VERTICAL / EQUAL_LENGTH constraints.
/// If `constraint` is provided, applies only that one.
/// Otherwise applies all constraints in sketch.constraints in order.
///
/// Example (apply all):
///   wasm_solve_constraints('{"sketch": <SketchGraph>}')
///
/// Example (single, preview):
///   wasm_solve_constraints('{"sketch": ..., "constraint": {"type":"HORIZONTAL","targetType":"edge","targetId":"e1"}}')
#[wasm_bindgen]
pub fn wasm_solve_constraints(json: &str) -> String {
    let req: SolveConstraintsRequest = match serde_json::from_str(json) {
        Ok(v) => v,
        Err(e) => return err_json(format!("bad json: {e}")),
    };
    let result = if let Some(ref c) = req.constraint {
        apply_constraint_once(req.sketch, c)
    } else {
        solve_constraints(req.sketch)
    };
    serde_json::to_string(&result).unwrap_or_else(|e| err_json(e.to_string()))
}

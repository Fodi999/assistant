//! WASM bindings — thin JSON wrappers around the core engine functions.
//!
//! Build:
//! ```bash
//! wasm-pack build crates/sketch_engine \
//!   --target web --features wasm \
//!   --out-dir ../../static/wasm/sketch_engine
//! ```

use wasm_bindgen::prelude::*;

use crate::commands::{apply_add_edge, apply_add_point, apply_move_point,
                      AddEdgeRequest, AddPointRequest, MovePointRequest};
use crate::solver::{apply_constraint_once, solve_constraints, SolveConstraintsRequest};
use crate::types::SketchGraph;
use crate::validation::validate;

fn err_json(msg: impl Into<String>) -> String {
    serde_json::json!({ "ok": false, "error": msg.into() }).to_string()
}

#[wasm_bindgen]
pub fn wasm_validate_sketch(json: &str) -> String {
    #[derive(serde::Deserialize)]
    struct Req { sketch: SketchGraph }
    let req: Req = match serde_json::from_str(json) {
        Ok(v) => v, Err(e) => return err_json(format!("bad json: {e}")),
    };
    serde_json::to_string(&validate(&req.sketch)).unwrap_or_else(|e| err_json(e.to_string()))
}

#[wasm_bindgen]
pub fn wasm_add_point(json: &str) -> String {
    let req: AddPointRequest = match serde_json::from_str(json) {
        Ok(v) => v, Err(e) => return err_json(format!("bad json: {e}")),
    };
    serde_json::to_string(&apply_add_point(req)).unwrap_or_else(|e| err_json(e.to_string()))
}

#[wasm_bindgen]
pub fn wasm_add_edge(json: &str) -> String {
    let req: AddEdgeRequest = match serde_json::from_str(json) {
        Ok(v) => v, Err(e) => return err_json(format!("bad json: {e}")),
    };
    serde_json::to_string(&apply_add_edge(req)).unwrap_or_else(|e| err_json(e.to_string()))
}

#[wasm_bindgen]
pub fn wasm_move_point(json: &str) -> String {
    let req: MovePointRequest = match serde_json::from_str(json) {
        Ok(v) => v, Err(e) => return err_json(format!("bad json: {e}")),
    };
    serde_json::to_string(&apply_move_point(req)).unwrap_or_else(|e| err_json(e.to_string()))
}

#[wasm_bindgen]
pub fn wasm_solve_constraints(json: &str) -> String {
    let req: SolveConstraintsRequest = match serde_json::from_str(json) {
        Ok(v) => v, Err(e) => return err_json(format!("bad json: {e}")),
    };
    let result = match req.constraint {
        Some(ref c) => apply_constraint_once(req.sketch, c),
        None        => solve_constraints(req.sketch),
    };
    serde_json::to_string(&result).unwrap_or_else(|e| err_json(e.to_string()))
}

#[wasm_bindgen]
pub fn wasm_engine_info() -> String {
    serde_json::json!({
        "name": "sketch_engine",
        "version": env!("CARGO_PKG_VERSION"),
        "schema": "sketch_graph",
        "schemaVersion": 1u32,
        "constraints": [
            "HORIZONTAL","VERTICAL","EQUAL_LENGTH","FIX",
            "COINCIDENT","FIXED_LENGTH","PARALLEL","PERPENDICULAR","MIDPOINT"
        ]
    }).to_string()
}

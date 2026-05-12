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

use crate::commands::{apply_add_edge, apply_add_point, AddEdgeRequest, AddPointRequest};
use crate::sketch::SketchGraph;
use crate::validation::validate;

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

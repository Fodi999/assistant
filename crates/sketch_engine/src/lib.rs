//! # sketch_engine
//!
//! Pure Rust SketchGraph engine: types, validation, profile detection,
//! and the precision command processors (`add_point`, `add_edge`).
//!
//! Same code is used by:
//!   - the Axum backend (`crate::domain::matter` re-exports this lib)
//!   - the WebAssembly bridge (`sketch_wasm.js` in the frontend)
//!
//! Wire format is the SketchGraph v1 contract emitted by the frontend
//! `__sketchToJSON` / `__sketchExportPayload`.

pub mod sketch;
pub mod validation;
pub mod profiles;
pub mod commands;
pub mod constraints;

#[cfg(feature = "wasm")]
pub mod wasm;

pub use sketch::{Constraint, Edge, Point, SketchGraph, WorkingPlane};
pub use validation::{validate, ValidationIssue, ValidationResult};
pub use profiles::{detect_profiles, Profile};
pub use commands::{
    apply_add_edge, apply_add_point, apply_move_point,
    AddEdgeRequest, AddPointRequest, MovePointRequest, PointRefOrGrid,
    SketchCommandResult,
};

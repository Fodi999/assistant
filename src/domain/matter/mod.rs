// ── Matter domain — precision sketch commands ─────────────────────────────
// Wire-compatible with the front-end SketchGraph v1 contract emitted by
// `__sketchToJSON` / `__sketchExportPayload` in `sketch_state.rs`.

pub mod sketch;
pub mod validation;
pub mod profiles;
pub mod commands;

pub use sketch::{Constraint, Edge, Point, SketchGraph, WorkingPlane};
pub use validation::{ValidationIssue, ValidationResult};
pub use profiles::Profile;
pub use commands::{
    AddEdgeRequest, AddPointRequest, PointRefOrGrid, SketchCommandResult,
    apply_add_edge, apply_add_point,
};

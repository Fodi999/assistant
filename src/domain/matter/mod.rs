// ── Matter domain — precision sketch commands ─────────────────────────────
//
// The implementation lives in the standalone `sketch_engine` crate so the
// same code can be compiled to WebAssembly and run in the browser. This
// module is now a thin re-export layer that keeps the historical import
// paths (`crate::domain::matter::{commands, sketch, validation, profiles}`)
// working without churn for HTTP handlers.

pub use sketch_engine::commands;
pub use sketch_engine::profiles;
pub use sketch_engine::sketch;
pub use sketch_engine::validation;

pub use sketch_engine::{
    apply_add_edge, apply_add_point, apply_move_point,
    detect_profiles, validate,
    AddEdgeRequest, AddPointRequest, MovePointRequest,
    Constraint, Edge, Point, PointRefOrGrid, Profile, SketchCommandResult, SketchGraph,
    ValidationIssue, ValidationResult, WorkingPlane,
};

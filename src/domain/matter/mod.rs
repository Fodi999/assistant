// ── Matter domain — precision sketch commands ─────────────────────────────
//
// Re-exports geometry_engine::sketch — unified 2D+3D kernel.
// Keeps historical import paths working for HTTP handlers.

pub use geometry_engine::sketch::commands;
pub use geometry_engine::sketch::profiles;
pub use geometry_engine::sketch::types as sketch;
pub use geometry_engine::sketch::validation;
pub use geometry_engine::sketch::profiles as profile_repair;
pub use geometry_engine::sketch::solver;

pub use geometry_engine::sketch::{
    apply_add_edge, apply_add_point, apply_move_point,
    detect_profiles, validate,
    AddEdgeRequest, AddPointRequest, MovePointRequest,
    Constraint, Edge, Point, PointRefOrGrid, Profile, SketchCommandResult, SketchGraph,
    ValidationIssue, ValidationResult, WorkingPlane,
    analyze_profile, repair_profile,
    ProfileAnalyzeRequest, ProfileAnalyzeResponse,
    ProfileRepairRequest, ProfileRepairResponse,
    solve_constraints, apply_constraint_once,
    SolveResult, SolveConstraintsRequest, ConstraintApplyResult,
};

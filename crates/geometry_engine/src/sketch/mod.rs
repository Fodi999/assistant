//! # sketch — 2D parametric sketcher
//!
//! This module is the authoritative 2D CAD engine.  It was previously a
//! separate `sketch_engine` crate; it now lives here so that the full
//! 2D→3D pipeline (sketch → profile → extrude → B-Rep → tessellation) is
//! contained in a single crate.
//!
//! ## Layout
//! ```text
//! sketch/
//!   types/        — all data types (SketchGraph, Point, Edge, Constraint, …)
//!   constraints/  — per-constraint apply fns (HORIZONTAL, VERTICAL, …)
//!   solver/       — iterative constraint solver + diagnostics
//!   profiles/     — closed-loop detection & repair
//!   commands/     — add_point, add_edge, move_point
//!   validation/   — structural validation
//!   to_solid.rs   — SketchGraph + depth → ExtrudeBrepResult  ← THE KEY BRIDGE
//! ```
//!
//! ## WASM
//! All public functions are re-exported as `#[wasm_bindgen]` fns from
//! `crate::wasm::bindings` (feature = "wasm").

pub mod commands;
pub mod constraints;
pub mod profiles;
pub mod solver;
pub mod to_solid;
pub mod types;
pub mod validation;

// ── Flat re-exports ───────────────────────────────────────────────────────
pub use types::{
    Constraint, Edge, Point, Profile, SketchGraph, WorkingPlane,
};
pub use commands::{
    apply_add_point, apply_add_edge, apply_move_point,
    AddPointRequest, AddEdgeRequest, MovePointRequest, SketchCommandResult,
    PointRefOrGrid,
};
pub use solver::{
    solve_constraints, solve_constraints_with_config, apply_constraint_once,
    SolveConfig, SolveConstraintsRequest, SolveResult,
};
pub use constraints::ConstraintApplyResult;
pub use profiles::{
    detect_profiles,
    analyze_profile, repair_profile,
    ProfileAnalyzeRequest, ProfileAnalyzeResponse,
    ProfileRepairRequest, ProfileRepairResponse,
};
pub use validation::{validate, ValidationResult, ValidationIssue};
pub use to_solid::{sketch_extrude, SketchExtrudeRequest, SketchExtrudeResult};

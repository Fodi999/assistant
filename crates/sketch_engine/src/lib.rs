//! # sketch_engine
//!
//! Pure Rust parametric sketch engine.
//! Works as a native Axum backend library **and** compiles to WASM for the browser.
//!
//! ## Module layout
//!
//! ```text
//! sketch_engine/
//!   types/          — pure data types (Point, Edge, Constraint, SketchGraph, …)
//!   constraints/    — one file per constraint type (HORIZONTAL, PARALLEL, …)
//!   solver/         — applies constraints, returns SolveResult
//!   profiles/
//!     detect.rs     — closed-loop detection
//!     repair.rs     — analyze / snap / equalize profiles
//!   validation/     — structural checks (duplicates, off-plane, open ends)
//!   commands/       — add_point, add_edge, move_point
//!   wasm/           — wasm-bindgen exports (feature = "wasm")
//! ```
//!
//! ## Supported constraints
//!
//! | Type            | What it does                              |
//! |-----------------|-------------------------------------------|
//! | HORIZONTAL      | Both endpoints share the same V coord     |
//! | VERTICAL        | Both endpoints share the same U coord     |
//! | EQUAL_LENGTH    | Edge B gets the same length as edge A     |
//! | FIX             | Point is locked (enforced in validation)  |
//! | COINCIDENT      | Point B moves to point A's position       |
//! | FIXED_LENGTH    | Edge length = value mm (snapped to grid)  |
//! | PARALLEL        | Edge B parallel to edge A                 |
//! | PERPENDICULAR   | Edge B perpendicular to edge A            |
//! | MIDPOINT        | Point moves to midpoint of edge           |

pub mod types;
pub mod constraints;
pub mod solver;
pub mod profiles;
pub mod validation;
pub mod commands;

#[cfg(feature = "wasm")]
pub mod wasm;

// ── Public API (wire-compatible with previous flat layout) ────────────────
pub use types::{Constraint, Edge, Point, Profile, SketchGraph, WorkingPlane};
pub use validation::{validate, ValidationIssue, ValidationResult};
pub use profiles::{detect_profiles, analyze_profile, repair_profile,
                   ProfileAnalyzeRequest, ProfileAnalyzeResponse,
                   ProfileRepairRequest, ProfileRepairResponse};
pub use commands::{
    apply_add_edge, apply_add_point, apply_move_point,
    AddEdgeRequest, AddPointRequest, MovePointRequest, PointRefOrGrid,
    SketchCommandResult,
};
pub use solver::{solve_constraints, solve_constraints_with_config, apply_constraint_once,
                 SolveResult, SolveStatus, SolveConfig, SolveConstraintsRequest,
                 ConstraintResidual, SolveDiagnostics, compute_residuals, residual_one};
pub use constraints::ConstraintApplyResult;

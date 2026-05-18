//! # solver
//!
//! Applies constraints to a SketchGraph by iterating over `sketch.constraints`
//! and calling the appropriate handler from `constraints/`.

use serde::{Deserialize, Serialize};

use crate::constraints::{apply_one, ConstraintApplyResult};
use crate::profiles::detect_profiles;
use crate::types::{Constraint, SketchGraph};
use crate::validation::{validate, ValidationResult};

// ── Result types ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolveResult {
    pub ok: bool,
    pub sketch: SketchGraph,
    pub results: Vec<ConstraintApplyResult>,
    pub validation: ValidationResult,
}

/// HTTP / WASM request body.
#[derive(Debug, Clone, Deserialize)]
pub struct SolveConstraintsRequest {
    pub sketch: SketchGraph,
    /// When present, apply only this one constraint (preview / one-shot mode).
    /// When absent, apply all constraints in `sketch.constraints`.
    #[serde(default)]
    pub constraint: Option<Constraint>,
}

// ── Public API ────────────────────────────────────────────────────────────

/// Apply every constraint in `sketch.constraints` in order.
/// Runs up to MAX_PASSES iterations until the sketch converges (no more moves).
pub fn solve_constraints(mut sketch: SketchGraph) -> SolveResult {
    const MAX_PASSES: usize = 8;
    let constraints = sketch.constraints.clone();
    let mut last_results = Vec::with_capacity(constraints.len());

    for _pass in 0..MAX_PASSES {
        let mut moved_any = false;
        last_results = Vec::with_capacity(constraints.len());
        for c in &constraints {
            let r = apply_one(&mut sketch, c);
            if !r.moved_points.is_empty() { moved_any = true; }
            last_results.push(r);
        }
        if !moved_any { break; }
    }

    sketch.profiles = detect_profiles(&sketch);
    let validation  = validate(&sketch);
    SolveResult { ok: last_results.iter().all(|r| r.ok), sketch, results: last_results, validation }
}

/// Apply a single constraint without committing it to `sketch.constraints`.
/// Useful for live preview.
pub fn apply_constraint_once(mut sketch: SketchGraph, c: &Constraint) -> SolveResult {
    let result      = apply_one(&mut sketch, c);
    sketch.profiles = detect_profiles(&sketch);
    let validation  = validate(&sketch);
    SolveResult { ok: result.ok, sketch, results: vec![result], validation }
}

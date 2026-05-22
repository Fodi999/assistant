//! Solver diagnostics: DOF estimation, over/under-constrained detection.
//!
//! This is a **static analysis** — it does not run the solver.
//! The DOF calculation uses a simplified counting model (each constraint
//! removes a fixed number of degrees of freedom regardless of geometry).

use serde::{Deserialize, Serialize};

use crate::sketch::solver::residual::ConstraintResidual;
use crate::sketch::types::SketchGraph;

// ── Public types ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolveDiagnostics {
    /// Estimated degrees of freedom remaining.
    /// 0 = fully determined, > 0 = under-constrained, < 0 = over-constrained.
    pub dof: i32,
    /// Human-readable DOF status string.
    pub dof_status: String,
    /// Constraint ids that appear to be in conflict (residual still high after
    /// solve, or analytically detected duplicates).  Empty on fresh sketches.
    pub over_constrained: Vec<String>,
    /// Constraint ids whose residual is non-trivial (> 1e-3 mm) after solve.
    /// These are candidates for "unsatisfied" constraints.
    pub unsatisfied: Vec<String>,
    /// General warnings (e.g. zero-length edge referenced, unknown plane, …).
    pub warnings: Vec<String>,
}

// ── Entry point ───────────────────────────────────────────────────────────

/// Compute diagnostics from a sketch + its post-solve residuals.
///
/// `residuals` should be computed **after** the solver has run so that
/// `over_constrained` / `unsatisfied` reflect the true post-solve state.
pub fn compute_diagnostics(
    sketch: &SketchGraph,
    residuals: &[ConstraintResidual],
) -> SolveDiagnostics {
    let dof            = estimate_dof(sketch);
    let dof_status     = dof_status_string(dof);
    let over_constrained = if dof < 0 { detect_over_constrained(sketch) } else { vec![] };
    let unsatisfied    = residuals.iter()
        .filter(|r| r.error_mm > 1e-3)
        .map(|r| r.constraint_id.clone())
        .collect();
    let warnings       = collect_warnings(sketch);

    SolveDiagnostics { dof, dof_status, over_constrained, unsatisfied, warnings }
}

// ── DOF counting ──────────────────────────────────────────────────────────

/// Each point has 2 DOF on the working plane (U, V).
/// Each constraint removes the number of DOF listed below.
/// This is a conservative approximation; it does not account for redundancies.
fn dof_cost(ty: &str) -> i32 {
    match ty {
        "HORIZONTAL"    => 1,
        "VERTICAL"      => 1,
        "FIXED_LENGTH"  => 1,
        "EQUAL_LENGTH"  => 1,
        "PARALLEL"      => 1,
        "PERPENDICULAR" => 1,
        "COINCIDENT"    => 2,   // ties two points together — removes 2 DOF
        "FIX" | "FIXED_POINT" => 2,   // fixes both coords of one point
        "MIDPOINT"      => 2,   // determines both coords of a point
        _               => 1,   // unknown: assume 1
    }
}

fn estimate_dof(sketch: &SketchGraph) -> i32 {
    let free: i32 = sketch.points.len() as i32 * 2;
    let used: i32 = sketch.constraints.iter().map(|c| dof_cost(c.ty.as_str())).sum();
    free - used
}

fn dof_status_string(dof: i32) -> String {
    match dof.cmp(&0) {
        std::cmp::Ordering::Equal   => "fully-constrained".into(),
        std::cmp::Ordering::Greater => format!("under-constrained ({} DOF free)", dof),
        std::cmp::Ordering::Less    => format!("over-constrained ({} excess)", -dof),
    }
}

// ── Over-constrained detection ────────────────────────────────────────────

/// Heuristic: look for constraints that reference the exact same target_id
/// and have the same type → they are redundant.
fn detect_over_constrained(sketch: &SketchGraph) -> Vec<String> {
    let mut seen: std::collections::HashSet<String> = Default::default();
    let mut conflicting = Vec::new();
    for c in &sketch.constraints {
        let key = format!("{}:{}", c.ty, c.target_id);
        if !seen.insert(key) {
            conflicting.push(
                c.id.clone().unwrap_or_else(|| format!("{}:{}", c.ty, c.target_id))
            );
        }
    }
    conflicting
}

// ── Warning collection ────────────────────────────────────────────────────

fn collect_warnings(sketch: &SketchGraph) -> Vec<String> {
    let mut w = Vec::new();
    if sketch.points.is_empty() && !sketch.constraints.is_empty() {
        w.push("Constraints present but sketch has no points".into());
    }
    if sketch.constraints.len() > sketch.points.len() * 4 {
        w.push(format!(
            "{} constraints on {} points — likely over-specified",
            sketch.constraints.len(), sketch.points.len()
        ));
    }
    w
}

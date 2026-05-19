//! # solver
//!
//! Applies constraints to a SketchGraph by iterating over `sketch.constraints`
//! and calling the appropriate handler from `constraints/`.
//!
//! Public API:
//!   `solve_constraints(sketch)`                         — default config
//!   `solve_constraints_with_config(sketch, config)`     — custom config
//!   `apply_constraint_once(sketch, constraint)`         — single preview

pub mod config;
pub mod diagnostics;
pub mod residual;
pub mod result;

use serde::Deserialize;

pub use config::SolveConfig;
pub use diagnostics::SolveDiagnostics;
pub use residual::{compute_residuals, residual_one, ConstraintResidual};
pub use result::{SolveResult, SolveStatus};

use crate::constraints::{apply_one, ConstraintApplyResult};
use crate::profiles::detect_profiles;
use crate::types::{Constraint, SketchGraph};
use crate::validation::{validate, ValidationResult};

// ── HTTP / WASM request body ──────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct SolveConstraintsRequest {
    pub sketch: SketchGraph,
    /// When present, apply only this one constraint (preview / one-shot mode).
    #[serde(default)]
    pub constraint: Option<Constraint>,
    /// Optional solver configuration.  Falls back to `SolveConfig::default()`.
    #[serde(default)]
    pub config: Option<SolveConfig>,
}

// ── Public API ────────────────────────────────────────────────────────────

/// Apply every constraint in `sketch.constraints` with the default config.
pub fn solve_constraints(sketch: SketchGraph) -> SolveResult {
    solve_constraints_with_config(sketch, &SolveConfig::default())
}

/// Apply every constraint with a custom config.
pub fn solve_constraints_with_config(mut sketch: SketchGraph, cfg: &SolveConfig) -> SolveResult {
    if sketch.constraints.is_empty() {
        let residuals   = vec![];
        let diagnostics = diagnostics::compute_diagnostics(&sketch, &residuals);
        let validation  = validate(&sketch);
        sketch.profiles = detect_profiles(&sketch);
        return SolveResult {
            ok: true,
            sketch,
            results: vec![],
            validation,
            status:         SolveStatus::TriviallyOk.as_str().into(),
            iterations:     0,
            max_error_mm:   0.0,
            total_error_mm: 0.0,
            moved_points:   vec![],
            residuals,
            diagnostics,
        };
    }

    let constraints   = sketch.constraints.clone();
    let mut last_results: Vec<ConstraintApplyResult> = Vec::with_capacity(constraints.len());
    let mut iterations = 0usize;
    let mut all_moved: std::collections::HashSet<String> = Default::default();

    let mut status = SolveStatus::MaxIterationsReached;

    for pass in 0..cfg.max_passes {
        iterations = pass + 1;
        let mut moved_any = false;
        last_results = Vec::with_capacity(constraints.len());

        for c in &constraints {
            let r = apply_one(&mut sketch, c);
            for pid in &r.moved_points {
                all_moved.insert(pid.clone());
                moved_any = true;
            }
            last_results.push(r);
        }

        // Early-exit when nothing moved (already converged)
        if !moved_any {
            status = SolveStatus::Converged;
            break;
        }

        // Early-exit when max_error is within tolerance (requires residuals mid-solve)
        if cfg.tolerance_mm > 0.0 && cfg.compute_residuals {
            let mid_res = compute_residuals(&sketch);
            let max_err = mid_res.iter().map(|r| r.error_mm).fold(0.0_f64, f64::max);
            if max_err < cfg.tolerance_mm {
                status = SolveStatus::Converged;
                iterations = pass + 1;
                break;
            }
        }
    }

    if status == SolveStatus::MaxIterationsReached
        && iterations < cfg.max_passes
    {
        // Fell through without break — converged on last pass
        status = SolveStatus::Converged;
    }

    // Final residuals + diagnostics
    let residuals = if cfg.compute_residuals {
        compute_residuals(&sketch)
    } else {
        vec![]
    };

    let max_error_mm   = residuals.iter().map(|r| r.error_mm).fold(0.0_f64, f64::max);
    let total_error_mm = residuals.iter().map(|r| r.error_mm).sum::<f64>();

    let diagnostics = if cfg.compute_diagnostics {
        diagnostics::compute_diagnostics(&sketch, &residuals)
    } else {
        SolveDiagnostics {
            dof: 0, dof_status: String::new(),
            over_constrained: vec![], unsatisfied: vec![], warnings: vec![],
        }
    };

    sketch.profiles = detect_profiles(&sketch);
    let validation  = validate(&sketch);
    let ok          = last_results.iter().all(|r| r.ok);

    let mut moved_points: Vec<String> = all_moved.into_iter().collect();
    moved_points.sort();

    SolveResult {
        ok,
        sketch,
        results: last_results,
        validation,
        status:    status.as_str().into(),
        iterations,
        max_error_mm,
        total_error_mm,
        moved_points,
        residuals,
        diagnostics,
    }
}

/// Apply a single constraint without committing it to `sketch.constraints`.
/// Useful for live preview. Returns full SolveResult v2.
pub fn apply_constraint_once(mut sketch: SketchGraph, c: &Constraint) -> SolveResult {
    let result      = apply_one(&mut sketch, c);
    let moved_pts   = result.moved_points.clone();
    sketch.profiles = detect_profiles(&sketch);
    let validation  = validate(&sketch);

    let residuals   = compute_residuals(&sketch);
    let max_err     = residuals.iter().map(|r| r.error_mm).fold(0.0_f64, f64::max);
    let total_err   = residuals.iter().map(|r| r.error_mm).sum::<f64>();
    let diagnostics = diagnostics::compute_diagnostics(&sketch, &residuals);

    SolveResult {
        ok:     result.ok,
        sketch,
        results: vec![result],
        validation,
        status:         if max_err < 1e-6 {
                            SolveStatus::Converged.as_str()
                        } else {
                            SolveStatus::MaxIterationsReached.as_str()
                        }.into(),
        iterations:     1,
        max_error_mm:   max_err,
        total_error_mm: total_err,
        moved_points:   moved_pts,
        residuals,
        diagnostics,
    }
}


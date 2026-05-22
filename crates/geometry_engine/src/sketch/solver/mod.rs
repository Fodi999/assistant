pub mod config;
pub mod diagnostics;
pub mod residual;
pub mod result;

use serde::Deserialize;

pub use config::SolveConfig;
pub use diagnostics::SolveDiagnostics;
pub use residual::{compute_residuals, residual_one, ConstraintResidual};
pub use result::{SolveResult, SolveStatus};

use crate::sketch::constraints::{apply_one, ConstraintApplyResult};
use crate::sketch::profiles::detect_profiles;
use crate::sketch::types::{Constraint, SketchGraph};
use crate::sketch::validation::validate;

#[derive(Debug, Clone, Deserialize)]
pub struct SolveConstraintsRequest {
    pub sketch: SketchGraph,
    #[serde(default)]
    pub constraint: Option<Constraint>,
    #[serde(default)]
    pub config: Option<SolveConfig>,
}

pub fn solve_constraints(sketch: SketchGraph) -> SolveResult {
    solve_constraints_with_config(sketch, &SolveConfig::default())
}

pub fn solve_constraints_with_config(mut sketch: SketchGraph, cfg: &SolveConfig) -> SolveResult {
    if sketch.constraints.is_empty() {
        let residuals   = vec![];
        let diagnostics = diagnostics::compute_diagnostics(&sketch, &residuals);
        let validation  = validate(&sketch);
        sketch.profiles = detect_profiles(&sketch);
        return SolveResult {
            ok: true, sketch, results: vec![], validation,
            status: SolveStatus::TriviallyOk.as_str().into(),
            iterations: 0, max_error_mm: 0.0, total_error_mm: 0.0,
            moved_points: vec![], residuals, diagnostics,
        };
    }

    let constraints = sketch.constraints.clone();
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
            for pid in &r.moved_points { all_moved.insert(pid.clone()); moved_any = true; }
            last_results.push(r);
        }
        if !moved_any { status = SolveStatus::Converged; break; }
        if cfg.tolerance_mm > 0.0 && cfg.compute_residuals {
            let mid_res = compute_residuals(&sketch);
            let max_err = mid_res.iter().map(|r| r.error_mm).fold(0.0_f64, f64::max);
            if max_err < cfg.tolerance_mm { status = SolveStatus::Converged; iterations = pass + 1; break; }
        }
    }

    if status == SolveStatus::MaxIterationsReached && iterations < cfg.max_passes {
        status = SolveStatus::Converged;
    }

    let residuals = if cfg.compute_residuals { compute_residuals(&sketch) } else { vec![] };
    let max_error_mm   = residuals.iter().map(|r| r.error_mm).fold(0.0_f64, f64::max);
    let total_error_mm = residuals.iter().map(|r| r.error_mm).sum::<f64>();
    let diagnostics = if cfg.compute_diagnostics {
        diagnostics::compute_diagnostics(&sketch, &residuals)
    } else {
        SolveDiagnostics { dof: 0, dof_status: String::new(), over_constrained: vec![], unsatisfied: vec![], warnings: vec![] }
    };

    sketch.profiles = detect_profiles(&sketch);
    let validation  = validate(&sketch);
    let ok          = last_results.iter().all(|r| r.ok);
    let mut moved_points: Vec<String> = all_moved.into_iter().collect();
    moved_points.sort();

    SolveResult { ok, sketch, results: last_results, validation,
        status: status.as_str().into(), iterations,
        max_error_mm, total_error_mm, moved_points, residuals, diagnostics }
}

/// Apply a single constraint (preview / one-shot mode).
pub fn apply_constraint_once(mut sketch: SketchGraph, constraint: &Constraint) -> SolveResult {
    let r = apply_one(&mut sketch, constraint);
    let ok = r.ok;
    sketch.profiles = detect_profiles(&sketch);
    let validation  = validate(&sketch);
    SolveResult {
        ok, sketch, results: vec![r], validation,
        status: if ok { SolveStatus::Converged } else { SolveStatus::Error }.as_str().into(),
        iterations: 1, max_error_mm: 0.0, total_error_mm: 0.0,
        moved_points: vec![], residuals: vec![],
        diagnostics: SolveDiagnostics { dof: 0, dof_status: String::new(), over_constrained: vec![], unsatisfied: vec![], warnings: vec![] },
    }
}

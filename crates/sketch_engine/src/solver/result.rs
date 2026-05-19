//! Extended SolveResult (v2) types.
//!
//! The original `SolveResult` is kept **byte-for-byte compatible** — the
//! new fields (`status`, `iterations`, …) are additive, so existing WASM
//! consumers that only read `ok`, `sketch`, `results`, `validation` are
//! unaffected.

use serde::{Deserialize, Serialize};

use crate::constraints::ConstraintApplyResult;
use crate::solver::diagnostics::SolveDiagnostics;
use crate::solver::residual::ConstraintResidual;
use crate::types::SketchGraph;
use crate::validation::ValidationResult;

// ── Solve status ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SolveStatus {
    /// All constraints satisfied within tolerance before max passes.
    Converged,
    /// Solver ran the maximum number of passes without full convergence.
    MaxIterationsReached,
    /// No constraints to apply — result is trivially ok.
    TriviallyOk,
    /// Fatal error (bad input, etc.).
    Error,
}

impl SolveStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Converged             => "converged",
            Self::MaxIterationsReached  => "max_iterations_reached",
            Self::TriviallyOk           => "trivially_ok",
            Self::Error                 => "error",
        }
    }
}

// ── Result type ───────────────────────────────────────────────────────────

/// Extended solve result.  Fully backwards-compatible with the original
/// `SolveResult` — the first four fields (`ok`, `sketch`, `results`,
/// `validation`) are unchanged in JSON layout.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolveResult {
    // ── v1 fields (backwards-compatible) ─────────────────────────────
    pub ok:         bool,
    pub sketch:     SketchGraph,
    pub results:    Vec<ConstraintApplyResult>,
    pub validation: ValidationResult,

    // ── v2 fields (additive) ─────────────────────────────────────────
    /// Machine-readable solve status.
    pub status: String,

    /// Number of full passes the solver performed.
    pub iterations: usize,

    /// Maximum per-constraint error (mm) after the final pass.
    #[serde(rename = "maxErrorMm")]
    pub max_error_mm: f64,

    /// Sum of all constraint errors (mm) after the final pass.
    #[serde(rename = "totalErrorMm")]
    pub total_error_mm: f64,

    /// All point ids that were moved during the solve (deduplicated).
    #[serde(rename = "movedPoints")]
    pub moved_points: Vec<String>,

    /// Per-constraint residuals (computed on final post-solve geometry).
    pub residuals: Vec<ConstraintResidual>,

    /// DOF estimate and conflict diagnostics.
    pub diagnostics: SolveDiagnostics,
}

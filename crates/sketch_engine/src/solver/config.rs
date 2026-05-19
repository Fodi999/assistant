//! SolveConfig — tuning parameters for the iterative constraint solver.

use serde::{Deserialize, Serialize};

/// Configuration passed to `solve_constraints_with_config()`.
/// All fields have defaults matching the original hard-coded behaviour.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolveConfig {
    /// Maximum number of full-pass iterations before giving up.
    #[serde(default = "default_max_passes")]
    pub max_passes: usize,

    /// Stop early when `max_error_mm` drops below this threshold (mm).
    /// Set to 0.0 to always run all passes.
    #[serde(default = "default_tolerance_mm")]
    pub tolerance_mm: f64,

    /// If true, compute per-constraint residuals after the final pass.
    /// Slightly more expensive; skip when only geometry is needed.
    #[serde(default = "default_true")]
    pub compute_residuals: bool,

    /// If true, compute DOF estimate and diagnostics.
    #[serde(default = "default_true")]
    pub compute_diagnostics: bool,
}

fn default_max_passes() -> usize  { 8 }
fn default_tolerance_mm() -> f64  { 1e-6 }
fn default_true() -> bool         { true }

impl Default for SolveConfig {
    fn default() -> Self {
        Self {
            max_passes:          default_max_passes(),
            tolerance_mm:        default_tolerance_mm(),
            compute_residuals:   true,
            compute_diagnostics: true,
        }
    }
}

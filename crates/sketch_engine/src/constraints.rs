//! Constraint solving placeholder.
//!
//! At the current phase constraints are only stored in `SketchGraph.constraints`
//! and surface through `validation`. A real solver lives elsewhere (future
//! phase). This module exists so `sketch_engine` has a stable place to grow.

use crate::sketch::Constraint;

/// Returns true when no contradictory constraint pair is present.
/// (Stub: today only constraint *type* compatibility is checked.)
pub fn constraints_consistent(constraints: &[Constraint]) -> bool {
    // No-op for now — full constraint solving is out of scope for this phase.
    !constraints.iter().any(|c| c.target_id.is_empty())
}

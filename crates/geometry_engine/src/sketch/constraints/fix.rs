use crate::sketch::types::{Constraint, SketchGraph};
use super::ConstraintApplyResult;

/// FIX locks a point in place. The solver never moves fixed points.
/// This is a no-op here — enforcement is in validation.
pub fn apply(_sketch: &mut SketchGraph, _c: &Constraint, cid: String) -> ConstraintApplyResult {
    ConstraintApplyResult {
        constraint_id: cid,
        ok: true,
        message: Some("FIX is enforced during validation, not modified here".into()),
        moved_points: vec![],
    }
}

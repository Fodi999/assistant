use crate::sketch::types::{Constraint, SketchGraph};
use super::{ConstraintApplyResult, fail, find_edge_points, find_point_indices, parse_plane, set_uv, uv};

/// Both endpoints of the target edge share the same U coordinate (averaged).
pub fn apply(sketch: &mut SketchGraph, c: &Constraint, cid: String) -> ConstraintApplyResult {
    if c.target_type != "edge" {
        return fail(cid, "VERTICAL requires targetType=edge");
    }
    let plane = match parse_plane(sketch, &cid) { Ok(p) => p, Err(e) => return e };
    let grid  = sketch.grid_size;

    let (pid_a, pid_b) = match find_edge_points(sketch, &c.target_id) {
        Some(x) => x,
        None => return fail(cid, format!("Edge {} not found", c.target_id)),
    };
    let (ia, ib) = match find_point_indices(sketch, &pid_a, &pid_b) {
        Some(x) => x,
        None => return fail(cid, "Edge point not found"),
    };

    let (ua, va) = uv(plane, &sketch.points[ia]);
    let (ub, vb) = uv(plane, &sketch.points[ib]);
    let target_u = (ua + ub) / 2;

    let mut moved = vec![];
    if ua != target_u {
        set_uv(plane, &mut sketch.points[ia], target_u, va, grid);
        moved.push(pid_a);
    }
    if ub != target_u {
        set_uv(plane, &mut sketch.points[ib], target_u, vb, grid);
        moved.push(pid_b);
    }
    ConstraintApplyResult { constraint_id: cid, ok: true, message: None, moved_points: moved }
}

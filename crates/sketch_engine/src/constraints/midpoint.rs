use crate::types::{Constraint, SketchGraph};
use super::{ConstraintApplyResult, fail, find_edge_points, find_point_index, parse_plane, set_uv, uv};

/// Moves a point to the exact midpoint of an edge (snapped to nearest grid).
///
/// `target_type = "point"`, `target_id = "point_id,edge_id"`
pub fn apply(sketch: &mut SketchGraph, c: &Constraint, cid: String) -> ConstraintApplyResult {
    let plane = match parse_plane(sketch, &cid) { Ok(p) => p, Err(e) => return e };
    let grid  = sketch.grid_size;

    let ids: Vec<&str> = c.target_id.splitn(2, ',').collect();
    if ids.len() != 2 {
        return fail(cid, "MIDPOINT targetId must be 'pointId,edgeId'");
    }
    let (point_id, edge_id) = (ids[0].trim().to_string(), ids[1].trim());

    let (pid_a, pid_b) = match find_edge_points(sketch, edge_id) {
        Some(x) => x,
        None => return fail(cid, format!("Edge {} not found", edge_id)),
    };
    let ip  = match find_point_index(sketch, &point_id) {
        Some(i) => i,
        None => return fail(cid, format!("Point {} not found", point_id)),
    };
    let ia  = match find_point_index(sketch, &pid_a) {
        Some(i) => i,
        None => return fail(cid, format!("Edge endpoint {} not found", pid_a)),
    };
    let ib  = match find_point_index(sketch, &pid_b) {
        Some(i) => i,
        None => return fail(cid, format!("Edge endpoint {} not found", pid_b)),
    };

    let (ua, va) = uv(plane, &sketch.points[ia]);
    let (ub, vb) = uv(plane, &sketch.points[ib]);

    // Round to nearest grid cell
    let mid_u = ((ua + ub) as f64 / 2.0).round() as i32;
    let mid_v = ((va + vb) as f64 / 2.0).round() as i32;

    let (cur_u, cur_v) = uv(plane, &sketch.points[ip]);
    let changed = cur_u != mid_u || cur_v != mid_v;
    if changed {
        set_uv(plane, &mut sketch.points[ip], mid_u, mid_v, grid);
    }

    ConstraintApplyResult {
        constraint_id: cid,
        ok: true,
        message: None,
        moved_points: if changed { vec![point_id] } else { vec![] },
    }
}

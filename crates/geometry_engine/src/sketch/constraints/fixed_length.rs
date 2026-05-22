use crate::sketch::types::{Constraint, SketchGraph};
use super::{ConstraintApplyResult, fail, find_edge_points, find_point_indices};

/// Sets the length of the target edge to exactly `value` mm.
/// The B endpoint is moved along the current edge direction; snapped to grid.
///
/// `target_type = "edge"`, `target_id = "edge_id"`, `value = 50.0` (mm)
pub fn apply(sketch: &mut SketchGraph, c: &Constraint, cid: String) -> ConstraintApplyResult {
    let value_mm = match c.value {
        Some(v) if v > 0.0 => v,
        _ => return fail(cid, "FIXED_LENGTH requires value > 0 (in mm)"),
    };

    let (pid_a, pid_b) = match find_edge_points(sketch, &c.target_id) {
        Some(x) => x,
        None => return fail(cid, format!("Edge {} not found", c.target_id)),
    };
    let (ia, ib) = match find_point_indices(sketch, &pid_a, &pid_b) {
        Some(x) => x,
        None => return fail(cid, "Point not found"),
    };

    let grid     = sketch.grid_size;
    let value_m  = value_mm / 1000.0;

    let dx = sketch.points[ib].x - sketch.points[ia].x;
    let dy = sketch.points[ib].y - sketch.points[ia].y;
    let dz = sketch.points[ib].z - sketch.points[ia].z;
    let cur_len = (dx*dx + dy*dy + dz*dz).sqrt();
    if cur_len < 1e-12 {
        return fail(cid, "Edge has zero length — cannot determine direction");
    }

    let scale   = value_m / cur_len;
    let new_gx  = ((sketch.points[ia].x + dx * scale) / grid).round() as i32;
    let new_gy  = ((sketch.points[ia].y + dy * scale) / grid).round() as i32;
    let new_gz  = ((sketch.points[ia].z + dz * scale) / grid).round() as i32;

    let pb      = &mut sketch.points[ib];
    let changed = pb.gx != new_gx || pb.gy != new_gy || pb.gz != new_gz;
    if changed {
        pb.gx = new_gx; pb.gy = new_gy; pb.gz = new_gz;
        pb.x  = new_gx as f64 * grid;
        pb.y  = new_gy as f64 * grid;
        pb.z  = new_gz as f64 * grid;
    }

    ConstraintApplyResult {
        constraint_id: cid,
        ok: true,
        message: None,
        moved_points: if changed { vec![pid_b] } else { vec![] },
    }
}

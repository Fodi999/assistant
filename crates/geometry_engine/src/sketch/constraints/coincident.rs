use crate::sketch::types::{Constraint, SketchGraph};
use super::{ConstraintApplyResult, fail, find_point_indices};

/// Makes point B coincide with point A (B moves to A's exact grid position).
///
/// `target_type = "points"`, `target_id = "point_a,point_b"`
pub fn apply(sketch: &mut SketchGraph, c: &Constraint, cid: String) -> ConstraintApplyResult {
    let ids: Vec<&str> = c.target_id.splitn(2, ',').collect();
    if ids.len() != 2 {
        return fail(cid, "COINCIDENT targetId must be 'pointA,pointB'");
    }
    let (pid_a, pid_b) = (ids[0].trim().to_string(), ids[1].trim().to_string());

    let (ia, ib) = match find_point_indices(sketch, &pid_a, &pid_b) {
        Some(x) => x,
        None => return fail(cid, format!("Point not found (looking for '{}' and '{}')", pid_a, pid_b)),
    };

    let (gx, gy, gz) = (sketch.points[ia].gx, sketch.points[ia].gy, sketch.points[ia].gz);
    let (x, y, z)    = (sketch.points[ia].x,  sketch.points[ia].y,  sketch.points[ia].z);

    let changed = sketch.points[ib].gx != gx
        || sketch.points[ib].gy != gy
        || sketch.points[ib].gz != gz;

    if changed {
        sketch.points[ib].gx = gx;
        sketch.points[ib].gy = gy;
        sketch.points[ib].gz = gz;
        sketch.points[ib].x  = x;
        sketch.points[ib].y  = y;
        sketch.points[ib].z  = z;
    }

    ConstraintApplyResult {
        constraint_id: cid,
        ok: true,
        message: None,
        moved_points: if changed { vec![pid_b] } else { vec![] },
    }
}

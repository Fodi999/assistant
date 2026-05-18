use crate::types::{Constraint, SketchGraph};
use super::{ConstraintApplyResult, edge_length_m, fail, find_edge_points, find_point_indices, parse_plane, set_uv, uv};

/// Makes edge B the same length as edge A by moving its B endpoint.
/// `target_id = "edge_a_id,edge_b_id"`
pub fn apply(sketch: &mut SketchGraph, c: &Constraint, cid: String) -> ConstraintApplyResult {
    let plane = match parse_plane(sketch, &cid) { Ok(p) => p, Err(e) => return e };
    let grid  = sketch.grid_size;

    let ids: Vec<&str> = c.target_id.splitn(2, ',').collect();
    if ids.len() != 2 {
        return fail(cid, "EQUAL_LENGTH targetId must be 'edgeA,edgeB'");
    }
    let (pa_a, pa_b) = match find_edge_points(sketch, ids[0].trim()) {
        Some(x) => x, None => return fail(cid, format!("Edge {} not found", ids[0])),
    };
    let (pb_a, pb_b) = match find_edge_points(sketch, ids[1].trim()) {
        Some(x) => x, None => return fail(cid, format!("Edge {} not found", ids[1])),
    };
    let (ia_a, ia_b) = match find_point_indices(sketch, &pa_a, &pa_b) {
        Some(x) => x, None => return fail(cid, "Point not found for edge A"),
    };
    let (ib_a, ib_b) = match find_point_indices(sketch, &pb_a, &pb_b) {
        Some(x) => x, None => return fail(cid, "Point not found for edge B"),
    };

    let len_a = edge_length_m(&sketch.points[ia_a], &sketch.points[ia_b]);
    if len_a < 1e-9 { return fail(cid, "Reference edge A has zero length"); }

    let (ub_a, vb_a) = uv(plane, &sketch.points[ib_a]);
    let (ub_b, vb_b) = uv(plane, &sketch.points[ib_b]);
    let du = (ub_b - ub_a) as f64;
    let dv = (vb_b - vb_a) as f64;
    let len_b_grid = (du*du + dv*dv).sqrt();
    if len_b_grid < 1e-9 { return fail(cid, "Edge B has zero length"); }

    let target_grid = len_a / grid;
    let scale = target_grid / len_b_grid;
    let new_ub_b = ub_a + (du * scale).round() as i32;
    let new_vb_b = vb_a + (dv * scale).round() as i32;

    let mut moved = vec![];
    if new_ub_b != ub_b || new_vb_b != vb_b {
        set_uv(plane, &mut sketch.points[ib_b], new_ub_b, new_vb_b, grid);
        moved.push(pb_b);
    }
    ConstraintApplyResult { constraint_id: cid, ok: true, message: None, moved_points: moved }
}

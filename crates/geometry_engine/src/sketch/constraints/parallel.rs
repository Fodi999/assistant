use crate::sketch::types::{Constraint, SketchGraph};
use super::{ConstraintApplyResult, fail, find_edge_points, find_point_indices, parse_plane, set_uv, uv};

/// Makes edge B parallel to edge A (reference).
/// Edge B's B-endpoint is moved; its length is preserved; A-endpoint is anchored.
///
/// `target_id = "edge_ref,edge_adj"`
pub fn apply(sketch: &mut SketchGraph, c: &Constraint, cid: String) -> ConstraintApplyResult {
    let plane = match parse_plane(sketch, &cid) { Ok(p) => p, Err(e) => return e };
    let grid  = sketch.grid_size;

    let ids: Vec<&str> = c.target_id.splitn(2, ',').collect();
    if ids.len() != 2 {
        return fail(cid, "PARALLEL targetId must be 'edgeRef,edgeAdj'");
    }

    // Edge A (reference)
    let (pa_a, pa_b) = match find_edge_points(sketch, ids[0].trim()) {
        Some(x) => x, None => return fail(cid, format!("Reference edge {} not found", ids[0])),
    };
    let (ia_a, ia_b) = match find_point_indices(sketch, &pa_a, &pa_b) {
        Some(x) => x, None => return fail(cid, "Point not found for reference edge"),
    };

    // Edge B (adjusted)
    let (pb_a, pb_b) = match find_edge_points(sketch, ids[1].trim()) {
        Some(x) => x, None => return fail(cid, format!("Adjusted edge {} not found", ids[1])),
    };
    let (ib_a, ib_b) = match find_point_indices(sketch, &pb_a, &pb_b) {
        Some(x) => x, None => return fail(cid, "Point not found for adjusted edge"),
    };

    // Reference direction in UV space
    let (ua_a, va_a) = uv(plane, &sketch.points[ia_a]);
    let (ua_b, va_b) = uv(plane, &sketch.points[ia_b]);
    let du_ref = (ua_b - ua_a) as f64;
    let dv_ref = (va_b - va_a) as f64;
    let len_ref = (du_ref*du_ref + dv_ref*dv_ref).sqrt();
    if len_ref < 1e-9 { return fail(cid, "Reference edge has zero length"); }

    // Current length of edge B
    let (ub_a, vb_a) = uv(plane, &sketch.points[ib_a]);
    let (ub_b, vb_b) = uv(plane, &sketch.points[ib_b]);
    let du_adj = (ub_b - ub_a) as f64;
    let dv_adj = (vb_b - vb_a) as f64;
    let len_adj = (du_adj*du_adj + dv_adj*dv_adj).sqrt();
    if len_adj < 1e-9 { return fail(cid, "Adjusted edge has zero length"); }

    // New B endpoint: same direction as ref, same length as adj
    let new_ub_b = ub_a + (du_ref * len_adj / len_ref).round() as i32;
    let new_vb_b = vb_a + (dv_ref * len_adj / len_ref).round() as i32;

    let mut moved = vec![];
    if new_ub_b != ub_b || new_vb_b != vb_b {
        set_uv(plane, &mut sketch.points[ib_b], new_ub_b, new_vb_b, grid);
        moved.push(pb_b);
    }
    ConstraintApplyResult { constraint_id: cid, ok: true, message: None, moved_points: moved }
}

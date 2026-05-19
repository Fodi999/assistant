//! Per-constraint residual calculation.
//!
//! A **residual** measures how far the current sketch geometry is from
//! satisfying a given constraint.  The scalar `error_mm` is always ≥ 0;
//! after a successful solve it should be ≈ 0 (within grid rounding).
//!
//! Angular errors (PARALLEL, PERPENDICULAR) are converted to a linear
//! "tip displacement" in mm using the length of the adjusted edge, so
//! all residuals share the same unit and can be compared directly.

use serde::{Deserialize, Serialize};

use crate::constraints::{find_edge_points, find_point_indices, find_point_index, uv};
use crate::types::{Constraint, SketchGraph, WorkingPlane};

// ── Public types ──────────────────────────────────────────────────────────

/// Residual for a single constraint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintResidual {
    /// Same as `Constraint::id` (or synthesised from type+targetId).
    pub constraint_id: String,
    /// Upper-case constraint type string.
    pub constraint_type: String,
    /// Scalar error in mm (≥ 0).  For angular constraints this is the
    /// tip-displacement of the adjusted edge, not degrees.
    pub error_mm: f64,
    /// True when `error_mm` < 1e-3 mm (sub-micron).
    pub satisfied: bool,
    /// Optional human-readable detail (e.g. "actual 52.4 mm, target 50.0 mm").
    pub detail: Option<String>,
}

// ── Entry point ───────────────────────────────────────────────────────────

/// Compute residuals for every constraint in `sketch.constraints`.
/// Reads the sketch as-is (does **not** apply constraints).
pub fn compute_residuals(sketch: &SketchGraph) -> Vec<ConstraintResidual> {
    sketch.constraints.iter().map(|c| residual_one(sketch, c)).collect()
}

/// Compute residual for a single constraint.
pub fn residual_one(sketch: &SketchGraph, c: &Constraint) -> ConstraintResidual {
    let cid = c.id.clone()
        .unwrap_or_else(|| format!("{}:{}", c.ty, c.target_id));

    let (error_mm, detail) = match c.ty.as_str() {
        "HORIZONTAL"    => res_horizontal(sketch, c),
        "VERTICAL"      => res_vertical(sketch, c),
        "FIXED_LENGTH"  => res_fixed_length(sketch, c),
        "EQUAL_LENGTH"  => res_equal_length(sketch, c),
        "PARALLEL"      => res_parallel(sketch, c),
        "PERPENDICULAR" => res_perpendicular(sketch, c),
        "COINCIDENT"    => res_coincident(sketch, c),
        "FIX" | "FIXED_POINT" => (0.0, None),  // always satisfied — point is locked in place
        "MIDPOINT"      => res_midpoint(sketch, c),
        _               => (0.0, Some(format!("unknown type: {}", c.ty))),
    };

    ConstraintResidual {
        constraint_id:   cid,
        constraint_type: c.ty.clone(),
        error_mm,
        satisfied: error_mm < 1e-6,
        detail,
    }
}

// ── Per-type helpers ──────────────────────────────────────────────────────

fn res_horizontal(sketch: &SketchGraph, c: &Constraint) -> (f64, Option<String>) {
    let plane = plane_or_default(sketch);
    let (pa, pb) = match find_edge_points(sketch, &c.target_id) {
        Some(x) => x, None => return (0.0, Some(format!("edge {} not found", c.target_id))),
    };
    let (ia, ib) = match find_point_indices(sketch, &pa, &pb) {
        Some(x) => x, None => return (0.0, Some("point not found".into())),
    };
    let (_, va) = uv(plane, &sketch.points[ia]);
    let (_, vb) = uv(plane, &sketch.points[ib]);
    let err_grid = (va - vb).unsigned_abs() as f64;
    let err_mm   = err_grid * sketch.grid_size * 1000.0;
    let detail = if err_mm > 0.0 {
        Some(format!("V diff = {} grid cells ({:.4} mm)", (va - vb).abs(), err_mm))
    } else { None };
    (err_mm, detail)
}

fn res_vertical(sketch: &SketchGraph, c: &Constraint) -> (f64, Option<String>) {
    let plane = plane_or_default(sketch);
    let (pa, pb) = match find_edge_points(sketch, &c.target_id) {
        Some(x) => x, None => return (0.0, Some(format!("edge {} not found", c.target_id))),
    };
    let (ia, ib) = match find_point_indices(sketch, &pa, &pb) {
        Some(x) => x, None => return (0.0, Some("point not found".into())),
    };
    let (ua, _) = uv(plane, &sketch.points[ia]);
    let (ub, _) = uv(plane, &sketch.points[ib]);
    let err_grid = (ua - ub).unsigned_abs() as f64;
    let err_mm   = err_grid * sketch.grid_size * 1000.0;
    let detail = if err_mm > 0.0 {
        Some(format!("U diff = {} grid cells ({:.4} mm)", (ua - ub).abs(), err_mm))
    } else { None };
    (err_mm, detail)
}

fn res_fixed_length(sketch: &SketchGraph, c: &Constraint) -> (f64, Option<String>) {
    let target_mm = match c.value {
        Some(v) if v > 0.0 => v,
        _ => return (0.0, Some("no positive value".into())),
    };
    let (pa, pb) = match find_edge_points(sketch, &c.target_id) {
        Some(x) => x, None => return (0.0, Some(format!("edge {} not found", c.target_id))),
    };
    let (ia, ib) = match find_point_indices(sketch, &pa, &pb) {
        Some(x) => x, None => return (0.0, Some("point not found".into())),
    };
    let actual_m  = edge_len_m(sketch, ia, ib);
    let actual_mm = actual_m * 1000.0;
    let err_mm    = (actual_mm - target_mm).abs();
    let detail = Some(format!("actual {:.4} mm, target {:.4} mm", actual_mm, target_mm));
    (err_mm, detail)
}

fn res_equal_length(sketch: &SketchGraph, c: &Constraint) -> (f64, Option<String>) {
    let ids: Vec<&str> = c.target_id.splitn(2, ',').collect();
    if ids.len() != 2 {
        return (0.0, Some("invalid targetId format".into()));
    }
    let (ea, eb) = (ids[0].trim(), ids[1].trim());
    let len_a_mm = edge_len_by_id_mm(sketch, ea);
    let len_b_mm = edge_len_by_id_mm(sketch, eb);
    match (len_a_mm, len_b_mm) {
        (Some(a), Some(b)) => {
            let err = (a - b).abs();
            let detail = Some(format!("edge A {:.4} mm, edge B {:.4} mm", a, b));
            (err, detail)
        }
        _ => (0.0, Some("edge(s) not found".into())),
    }
}

fn res_parallel(sketch: &SketchGraph, c: &Constraint) -> (f64, Option<String>) {
    let plane = plane_or_default(sketch);
    let ids: Vec<&str> = c.target_id.splitn(2, ',').collect();
    if ids.len() != 2 { return (0.0, Some("invalid targetId".into())); }
    let (dir_ref, len_adj) = match (
        edge_dir_uv(sketch, ids[0].trim(), plane),
        edge_len_by_id_mm(sketch, ids[1].trim()),
    ) {
        (Some(r), Some(l)) => (r, l),
        _ => return (0.0, Some("edge(s) not found".into())),
    };
    let dir_adj = match edge_dir_uv(sketch, ids[1].trim(), plane) {
        Some(d) => d, None => return (0.0, None),
    };
    // sin of angle between the two directions = |cross product| / (|a| * |b|)
    // cross2d = du_ref * dv_adj - dv_ref * du_adj
    let cross = (dir_ref.0 * dir_adj.1 - dir_ref.1 * dir_adj.0).abs();
    // tip displacement error: edge_B_length * |sin θ|
    let err_mm = (len_adj / 1000.0) * cross * 1000.0;
    let detail = Some(format!("tip displacement {:.4} mm", err_mm));
    (err_mm, detail)
}

fn res_perpendicular(sketch: &SketchGraph, c: &Constraint) -> (f64, Option<String>) {
    let plane = plane_or_default(sketch);
    let ids: Vec<&str> = c.target_id.splitn(2, ',').collect();
    if ids.len() != 2 { return (0.0, Some("invalid targetId".into())); }
    let (dir_ref, len_adj) = match (
        edge_dir_uv(sketch, ids[0].trim(), plane),
        edge_len_by_id_mm(sketch, ids[1].trim()),
    ) {
        (Some(r), Some(l)) => (r, l),
        _ => return (0.0, Some("edge(s) not found".into())),
    };
    let dir_adj = match edge_dir_uv(sketch, ids[1].trim(), plane) {
        Some(d) => d, None => return (0.0, None),
    };
    // For perpendicular: error is |dot product| / (|a|*|b|) = |cos θ|
    // tip displacement = edge_B * |cos θ|
    let dot  = (dir_ref.0 * dir_adj.0 + dir_ref.1 * dir_adj.1).abs();
    let err_mm = (len_adj / 1000.0) * dot * 1000.0;
    let detail = Some(format!("tip displacement {:.4} mm", err_mm));
    (err_mm, detail)
}

fn res_coincident(sketch: &SketchGraph, c: &Constraint) -> (f64, Option<String>) {
    let ids: Vec<&str> = c.target_id.splitn(2, ',').collect();
    if ids.len() != 2 { return (0.0, Some("invalid targetId".into())); }
    let (ia, ib) = match find_point_indices(sketch, ids[0].trim(), ids[1].trim()) {
        Some(x) => x, None => return (0.0, Some("point(s) not found".into())),
    };
    let a = &sketch.points[ia];
    let b = &sketch.points[ib];
    let dist_m  = ((a.x-b.x).powi(2) + (a.y-b.y).powi(2) + (a.z-b.z).powi(2)).sqrt();
    let dist_mm = dist_m * 1000.0;
    let detail = if dist_mm > 0.0 {
        Some(format!("distance {:.4} mm", dist_mm))
    } else { None };
    (dist_mm, detail)
}

fn res_midpoint(sketch: &SketchGraph, c: &Constraint) -> (f64, Option<String>) {
    let plane = plane_or_default(sketch);
    let ids: Vec<&str> = c.target_id.splitn(2, ',').collect();
    if ids.len() != 2 { return (0.0, Some("invalid targetId".into())); }
    let (point_id, edge_id) = (ids[0].trim(), ids[1].trim());

    let (pa, pb) = match find_edge_points(sketch, edge_id) {
        Some(x) => x, None => return (0.0, Some(format!("edge {} not found", edge_id))),
    };
    let ip = match find_point_index(sketch, point_id) {
        Some(i) => i, None => return (0.0, Some(format!("point {} not found", point_id))),
    };
    let ia = match find_point_index(sketch, &pa) { Some(i) => i, None => return (0.0, None) };
    let ib = match find_point_index(sketch, &pb) { Some(i) => i, None => return (0.0, None) };

    let (ua, va) = uv(plane, &sketch.points[ia]);
    let (ub, vb) = uv(plane, &sketch.points[ib]);
    let (up, vp) = uv(plane, &sketch.points[ip]);

    let mid_u = (ua + ub) as f64 / 2.0;
    let mid_v = (va + vb) as f64 / 2.0;
    let du = (up as f64 - mid_u) * sketch.grid_size;
    let dv = (vp as f64 - mid_v) * sketch.grid_size;
    let dist_mm = (du*du + dv*dv).sqrt() * 1000.0;
    let detail = if dist_mm > 0.0 {
        Some(format!("offset from midpoint {:.4} mm", dist_mm))
    } else { None };
    (dist_mm, detail)
}

// ── Internal helpers ──────────────────────────────────────────────────────

fn plane_or_default(sketch: &SketchGraph) -> WorkingPlane {
    WorkingPlane::parse(&sketch.working_plane).unwrap_or(WorkingPlane::XZ)
}

fn edge_len_m(sketch: &SketchGraph, ia: usize, ib: usize) -> f64 {
    let a = &sketch.points[ia];
    let b = &sketch.points[ib];
    ((a.x-b.x).powi(2) + (a.y-b.y).powi(2) + (a.z-b.z).powi(2)).sqrt()
}

fn edge_len_by_id_mm(sketch: &SketchGraph, edge_id: &str) -> Option<f64> {
    let (pa, pb) = find_edge_points(sketch, edge_id)?;
    let (ia, ib) = find_point_indices(sketch, &pa, &pb)?;
    Some(edge_len_m(sketch, ia, ib) * 1000.0)
}

/// Returns the normalised direction vector (du, dv) of an edge on the given plane.
/// Returns None if the edge is zero-length.
fn edge_dir_uv(sketch: &SketchGraph, edge_id: &str, plane: WorkingPlane) -> Option<(f64, f64)> {
    let (pa, pb) = find_edge_points(sketch, edge_id)?;
    let (ia, ib) = find_point_indices(sketch, &pa, &pb)?;
    let (ua, va) = uv(plane, &sketch.points[ia]);
    let (ub, vb) = uv(plane, &sketch.points[ib]);
    let du = (ub - ua) as f64;
    let dv = (vb - va) as f64;
    let len = (du*du + dv*dv).sqrt();
    if len < 1e-9 { return None; }
    Some((du / len, dv / len))
}

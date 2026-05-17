// ── Constraint Solver ─────────────────────────────────────────────────────
//
// Applies geometric constraints to a SketchGraph by adjusting point coordinates.
//
// Supported constraints (Constraint.ty):
//   "HORIZONTAL"   — edge both endpoints share same "flat" axis value
//   "VERTICAL"     — edge both endpoints share same "deep" axis value
//   "EQUAL_LENGTH" — two edges share the same length (second matches first)
//   "FIX"          — point position is locked (no-op in solver, checked in validation)
//
// Plane convention (integer grid coords gx, gy, gz):
//   XZ plane: U = gx (horizontal), V = gz (vertical in 2D sketch)
//   XY plane: U = gx, V = gy
//   YZ plane: U = gy, V = gz
//
// After adjusting grid coords the float coords are recomputed:
//   x = gx * grid_size, y = gy * grid_size, z = gz * grid_size

use serde::{Deserialize, Serialize};

use crate::sketch::{Constraint, Point, SketchGraph, WorkingPlane};
use crate::profiles::detect_profiles;
use crate::validation::{validate, ValidationResult};

// ─────────────────────────────────────────────────────────────────────────────
// Result type
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintApplyResult {
    /// Constraint id (if set) or "<type>:<targetId>"
    pub constraint_id: String,
    pub ok: bool,
    pub message: Option<String>,
    /// Point ids whose coordinates were modified
    pub moved_points: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolveResult {
    pub ok: bool,
    pub sketch: SketchGraph,
    pub results: Vec<ConstraintApplyResult>,
    pub validation: ValidationResult,
}

// ─────────────────────────────────────────────────────────────────────────────
// Plane helpers
// ─────────────────────────────────────────────────────────────────────────────

/// (U, V) integer coords for a point on a given plane.
fn uv(plane: WorkingPlane, p: &Point) -> (i32, i32) {
    match plane {
        WorkingPlane::XZ => (p.gx, p.gz),
        WorkingPlane::XY => (p.gx, p.gy),
        WorkingPlane::YZ => (p.gy, p.gz),
    }
}

/// Apply (U, V) back to the point's grid coords.
fn set_uv(plane: WorkingPlane, p: &mut Point, u: i32, v: i32, grid: f64) {
    match plane {
        WorkingPlane::XZ => { p.gx = u; p.gz = v; }
        WorkingPlane::XY => { p.gx = u; p.gy = v; }
        WorkingPlane::YZ => { p.gy = u; p.gz = v; }
    }
    // Recompute float coords from grid
    p.x = p.gx as f64 * grid;
    p.y = p.gy as f64 * grid;
    p.z = p.gz as f64 * grid;
}

// ─────────────────────────────────────────────────────────────────────────────
// Apply one constraint
// ─────────────────────────────────────────────────────────────────────────────

fn apply_one(sketch: &mut SketchGraph, c: &Constraint) -> ConstraintApplyResult {
    let cid = c.id.clone().unwrap_or_else(|| format!("{}:{}", c.ty, c.target_id));
    let grid = sketch.grid_size;

    let plane = match WorkingPlane::parse(&sketch.working_plane) {
        Some(p) => p,
        None => return ConstraintApplyResult {
            constraint_id: cid,
            ok: false,
            message: Some(format!("Unknown working plane: {}", sketch.working_plane)),
            moved_points: vec![],
        },
    };

    match c.ty.as_str() {
        // ── HORIZONTAL ──────────────────────────────────────────────────────
        // Both endpoints of the edge share the same V coordinate.
        // The average V is used so neither point is "dominant".
        "HORIZONTAL" => {
            if c.target_type != "edge" {
                return fail(cid, "HORIZONTAL requires targetType=edge");
            }
            let (pid_a, pid_b) = match find_edge_points(sketch, &c.target_id) {
                Some(x) => x,
                None => return fail(cid, format!("Edge {} not found", c.target_id)),
            };
            let (idx_a, idx_b) = match find_point_indices(sketch, &pid_a, &pid_b) {
                Some(x) => x,
                None => return fail(cid, "Edge point not found in sketch"),
            };
            let (ua, va) = uv(plane, &sketch.points[idx_a]);
            let (ub, vb) = uv(plane, &sketch.points[idx_b]);
            // Average V (round to nearest grid)
            let target_v = (va + vb) / 2;
            let mut moved = vec![];
            if va != target_v {
                set_uv(plane, &mut sketch.points[idx_a], ua, target_v, grid);
                moved.push(pid_a.clone());
            }
            if vb != target_v {
                set_uv(plane, &mut sketch.points[idx_b], ub, target_v, grid);
                moved.push(pid_b.clone());
            }
            ConstraintApplyResult { constraint_id: cid, ok: true, message: None, moved_points: moved }
        }

        // ── VERTICAL ────────────────────────────────────────────────────────
        // Both endpoints share the same U coordinate.
        "VERTICAL" => {
            if c.target_type != "edge" {
                return fail(cid, "VERTICAL requires targetType=edge");
            }
            let (pid_a, pid_b) = match find_edge_points(sketch, &c.target_id) {
                Some(x) => x,
                None => return fail(cid, format!("Edge {} not found", c.target_id)),
            };
            let (idx_a, idx_b) = match find_point_indices(sketch, &pid_a, &pid_b) {
                Some(x) => x,
                None => return fail(cid, "Edge point not found in sketch"),
            };
            let (ua, va) = uv(plane, &sketch.points[idx_a]);
            let (ub, vb) = uv(plane, &sketch.points[idx_b]);
            let target_u = (ua + ub) / 2;
            let mut moved = vec![];
            if ua != target_u {
                set_uv(plane, &mut sketch.points[idx_a], target_u, va, grid);
                moved.push(pid_a.clone());
            }
            if ub != target_u {
                set_uv(plane, &mut sketch.points[idx_b], target_u, vb, grid);
                moved.push(pid_b.clone());
            }
            ConstraintApplyResult { constraint_id: cid, ok: true, message: None, moved_points: moved }
        }

        // ── EQUAL_LENGTH ────────────────────────────────────────────────────
        // target_id = "edge1_id,edge2_id"  (comma-separated)
        // Makes the second edge the same length as the first by moving its B point.
        // The direction of edge2 is preserved; only the magnitude changes.
        "EQUAL_LENGTH" => {
            let ids: Vec<&str> = c.target_id.split(',').collect();
            if ids.len() != 2 {
                return fail(cid, "EQUAL_LENGTH targetId must be 'edgeA,edgeB'");
            }
            let (pa_a, pa_b) = match find_edge_points(sketch, ids[0].trim()) {
                Some(x) => x,
                None => return fail(cid, format!("Edge {} not found", ids[0])),
            };
            let (pb_a, pb_b) = match find_edge_points(sketch, ids[1].trim()) {
                Some(x) => x,
                None => return fail(cid, format!("Edge {} not found", ids[1])),
            };

            // Get grid-integer lengths (in grid units)
            let (ia_a, ia_b) = match find_point_indices(sketch, &pa_a, &pa_b) {
                Some(x) => x,
                None => return fail(cid, "Point not found for edge A"),
            };
            let (ib_a, ib_b) = match find_point_indices(sketch, &pb_a, &pb_b) {
                Some(x) => x,
                None => return fail(cid, "Point not found for edge B"),
            };

            // Length of edge A (reference) in float meters
            let len_a = edge_length_m(&sketch.points[ia_a], &sketch.points[ia_b], grid);
            if len_a < 1e-9 {
                return fail(cid, "Reference edge A has zero length");
            }

            // Direction of edge B (unit vector in U,V space)
            let (ub_a, vb_a) = uv(plane, &sketch.points[ib_a]);
            let (ub_b, vb_b) = uv(plane, &sketch.points[ib_b]);
            let du = (ub_b - ub_a) as f64;
            let dv = (vb_b - vb_a) as f64;
            let len_b_grid = (du * du + dv * dv).sqrt();
            if len_b_grid < 1e-9 {
                return fail(cid, "Edge B has zero length");
            }

            // Target length in grid units (round to nearest)
            let target_grid = len_a / grid;
            let scale = target_grid / len_b_grid;
            let new_du = (du * scale).round() as i32;
            let new_dv = (dv * scale).round() as i32;
            let new_ub_b = ub_a + new_du;
            let new_vb_b = vb_a + new_dv;

            // Only move B endpoint of edge B
            let mut moved = vec![];
            if new_ub_b != ub_b || new_vb_b != vb_b {
                set_uv(plane, &mut sketch.points[ib_b], new_ub_b, new_vb_b, grid);
                moved.push(pb_b.clone());
            }

            ConstraintApplyResult { constraint_id: cid, ok: true, message: None, moved_points: moved }
        }

        // ── FIX (no-op in solver — position is locked, not moved) ───────────
        "FIX" => ConstraintApplyResult {
            constraint_id: cid,
            ok: true,
            message: Some("FIX is enforced during validation, not modified here".into()),
            moved_points: vec![],
        },

        other => fail(cid, format!("Unknown constraint type: {other}")),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Public API
// ─────────────────────────────────────────────────────────────────────────────

/// Apply all constraints in `sketch.constraints` in order.
/// Returns updated sketch + per-constraint results.
pub fn solve_constraints(mut sketch: SketchGraph) -> SolveResult {
    let constraints = sketch.constraints.clone();
    let mut results = Vec::with_capacity(constraints.len());

    for c in &constraints {
        let r = apply_one(&mut sketch, c);
        results.push(r);
    }

    // Recompute profiles + validation after all constraints applied.
    sketch.profiles = detect_profiles(&sketch);
    let validation = validate(&sketch);

    SolveResult {
        ok: results.iter().all(|r| r.ok),
        sketch,
        results,
        validation,
    }
}

/// Apply a single constraint (not necessarily in sketch.constraints) —
/// useful for "preview" or one-shot apply without committing to the list.
pub fn apply_constraint_once(mut sketch: SketchGraph, c: &Constraint) -> SolveResult {
    let result = apply_one(&mut sketch, c);
    sketch.profiles = detect_profiles(&sketch);
    let validation = validate(&sketch);
    SolveResult {
        ok: result.ok,
        sketch,
        results: vec![result],
        validation,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Request / Response types for HTTP / WASM
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct SolveConstraintsRequest {
    pub sketch: SketchGraph,
    /// If provided, apply only this single constraint (preview mode).
    /// If None, apply sketch.constraints in order.
    #[serde(default)]
    pub constraint: Option<Constraint>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

fn fail(cid: String, msg: impl Into<String>) -> ConstraintApplyResult {
    ConstraintApplyResult {
        constraint_id: cid,
        ok: false,
        message: Some(msg.into()),
        moved_points: vec![],
    }
}

fn find_edge_points(sketch: &SketchGraph, edge_id: &str) -> Option<(String, String)> {
    sketch.edges.iter()
        .find(|e| e.id == edge_id)
        .map(|e| (e.a.clone(), e.b.clone()))
}

fn find_point_indices(sketch: &SketchGraph, pid_a: &str, pid_b: &str) -> Option<(usize, usize)> {
    let ia = sketch.points.iter().position(|p| p.id == pid_a)?;
    let ib = sketch.points.iter().position(|p| p.id == pid_b)?;
    Some((ia, ib))
}

fn edge_length_m(a: &Point, b: &Point, _grid: f64) -> f64 {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let dz = b.z - a.z;
    (dx*dx + dy*dy + dz*dz).sqrt()
}

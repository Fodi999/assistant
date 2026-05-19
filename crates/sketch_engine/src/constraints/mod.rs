use serde::{Deserialize, Serialize};
use crate::types::{Constraint, Point, SketchGraph, WorkingPlane};

// ── Sub-constraint modules ────────────────────────────────────────────────
pub mod coincident;
pub mod equal_length;
pub mod fix;
pub mod fixed_length;
pub mod horizontal;
pub mod midpoint;
pub mod parallel;
pub mod perpendicular;
pub mod vertical;

// ── Result type (returned per-constraint by solver) ───────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintApplyResult {
    /// Constraint id (if set) or "<type>:<targetId>".
    pub constraint_id: String,
    pub ok: bool,
    pub message: Option<String>,
    /// Point ids whose coordinates were modified.
    pub moved_points: Vec<String>,
}

// ── Dispatch ──────────────────────────────────────────────────────────────

/// Apply a single constraint to the sketch, mutating point coordinates.
pub fn apply_one(sketch: &mut SketchGraph, c: &Constraint) -> ConstraintApplyResult {
    let cid = c.id.clone().unwrap_or_else(|| format!("{}:{}", c.ty, c.target_id));
    match c.ty.as_str() {
        "HORIZONTAL"    => horizontal::apply(sketch, c, cid),
        "VERTICAL"      => vertical::apply(sketch, c, cid),
        "EQUAL_LENGTH"  => equal_length::apply(sketch, c, cid),
        "FIX" | "FIXED_POINT" => fix::apply(sketch, c, cid),
        "COINCIDENT"    => coincident::apply(sketch, c, cid),
        "FIXED_LENGTH"  => fixed_length::apply(sketch, c, cid),
        "PARALLEL"      => parallel::apply(sketch, c, cid),
        "PERPENDICULAR" => perpendicular::apply(sketch, c, cid),
        "MIDPOINT"      => midpoint::apply(sketch, c, cid),
        other => fail(cid, format!("Unknown constraint type: {other}")),
    }
}

// ── Shared helpers (pub(crate) so each sub-module can use them) ───────────

pub(crate) fn fail(cid: String, msg: impl Into<String>) -> ConstraintApplyResult {
    ConstraintApplyResult {
        constraint_id: cid,
        ok: false,
        message: Some(msg.into()),
        moved_points: vec![],
    }
}

/// (U, V) integer coords for a point on the given plane.
pub(crate) fn uv(plane: WorkingPlane, p: &Point) -> (i32, i32) {
    match plane {
        WorkingPlane::XZ => (p.gx, p.gz),
        WorkingPlane::XY => (p.gx, p.gy),
        WorkingPlane::YZ => (p.gy, p.gz),
    }
}

/// Write (U, V) back and recompute float coords from grid.
pub(crate) fn set_uv(plane: WorkingPlane, p: &mut Point, u: i32, v: i32, grid: f64) {
    match plane {
        WorkingPlane::XZ => { p.gx = u; p.gz = v; }
        WorkingPlane::XY => { p.gx = u; p.gy = v; }
        WorkingPlane::YZ => { p.gy = u; p.gz = v; }
    }
    p.x = p.gx as f64 * grid;
    p.y = p.gy as f64 * grid;
    p.z = p.gz as f64 * grid;
}

pub(crate) fn find_edge_points(sketch: &SketchGraph, edge_id: &str) -> Option<(String, String)> {
    sketch.edges.iter()
        .find(|e| e.id == edge_id)
        .map(|e| (e.a.clone(), e.b.clone()))
}

pub(crate) fn find_point_indices(sketch: &SketchGraph, a: &str, b: &str) -> Option<(usize, usize)> {
    let ia = sketch.points.iter().position(|p| p.id == a)?;
    let ib = sketch.points.iter().position(|p| p.id == b)?;
    Some((ia, ib))
}

pub(crate) fn find_point_index(sketch: &SketchGraph, id: &str) -> Option<usize> {
    sketch.points.iter().position(|p| p.id == id)
}

pub(crate) fn edge_length_m(a: &Point, b: &Point) -> f64 {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let dz = b.z - a.z;
    (dx*dx + dy*dy + dz*dz).sqrt()
}

/// Resolve working plane, returning a fail result if unknown.
pub(crate) fn parse_plane(sketch: &SketchGraph, cid: &str) -> Result<WorkingPlane, ConstraintApplyResult> {
    WorkingPlane::parse(&sketch.working_plane).ok_or_else(|| {
        fail(cid.to_string(), format!("Unknown working plane: {}", sketch.working_plane))
    })
}

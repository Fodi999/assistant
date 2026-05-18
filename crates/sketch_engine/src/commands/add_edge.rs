use serde::Deserialize;
use crate::profiles::detect_profiles;
use crate::types::{Edge, Point, SketchGraph, WorkingPlane};
use crate::validation::validate;
use super::result::{err_result, finalize, PointRefOrGrid, SketchCommandResult};

#[derive(Debug, Clone, Deserialize)]
pub struct AddEdgeRequest {
    pub sketch: SketchGraph,
    #[serde(rename = "workingPlane")] pub working_plane: String,
    #[serde(rename = "gridSize")]     pub grid_size: f64,
    pub start: PointRefOrGrid,
    pub end:   PointRefOrGrid,
    #[serde(default, alias = "ignorePlaneConstraint", rename = "ignore_plane_constraint")]
    pub ignore_plane_constraint: Option<bool>,
}

pub fn apply_add_edge(req: AddEdgeRequest) -> SketchCommandResult {
    let mut sketch = req.sketch;
    sketch.working_plane = req.working_plane.clone();
    sketch.grid_size     = req.grid_size;

    if !(req.grid_size.is_finite() && req.grid_size > 0.0) {
        return err_result(sketch, format!("gridSize must be positive finite, got {}", req.grid_size));
    }
    let plane = match WorkingPlane::parse(&req.working_plane) {
        Some(p) => p,
        None => return err_result(sketch, format!("Invalid workingPlane: {}", req.working_plane)),
    };
    let ignore_plane = req.ignore_plane_constraint.unwrap_or(false);

    let (start_id, created_a) = match resolve_endpoint(&mut sketch, plane, req.grid_size, &req.start, "start", ignore_plane) {
        Ok(v) => v, Err(e) => return err_result(sketch, e),
    };
    let (end_id, created_b) = match resolve_endpoint(&mut sketch, plane, req.grid_size, &req.end, "end", ignore_plane) {
        Ok(v) => v, Err(e) => return err_result(sketch, e),
    };

    if start_id == end_id {
        return err_result(sketch, "Edge endpoints coincide (self-loop)");
    }
    if sketch.find_edge_between(&start_id, &end_id).is_some() {
        let validation = validate(&sketch);
        sketch.profiles = detect_profiles(&sketch);
        return SketchCommandResult {
            ok: false, sketch, created_point_id: created_a.or(created_b),
            reused_point_id: None, created_edge_id: None,
            validation, message: Some("Edge already exists".into()),
        };
    }

    let edge_id = sketch.next_edge_id();
    sketch.edges.push(Edge { id: edge_id.clone(), a: start_id, b: end_id });

    let created_point_id = created_a.clone().or(created_b.clone());
    let msg = if created_point_id.is_some() {
        format!("Backend created edge {} (+ new point)", edge_id)
    } else {
        format!("Backend created edge {}", edge_id)
    };
    let validation = finalize(&mut sketch);

    SketchCommandResult {
        ok: true, sketch, created_point_id,
        reused_point_id: None, created_edge_id: Some(edge_id),
        validation, message: Some(msg),
    }
}

// ── Internal ──────────────────────────────────────────────────────────────

fn resolve_endpoint(
    sketch: &mut SketchGraph,
    plane: WorkingPlane,
    grid_size: f64,
    r: &PointRefOrGrid,
    label: &str,
    ignore_plane: bool,
) -> Result<(String, Option<String>), String> {
    if let Some(pid) = &r.point_id {
        if sketch.find_point(pid).is_some() { return Ok((pid.clone(), None)); }
        return Err(format!("{} pointId not found: {}", label, pid));
    }
    let (gx, gy, gz) = match (r.gx, r.gy, r.gz) {
        (Some(x), Some(y), Some(z)) => (x, y, z),
        _ => return Err(format!("{} must provide pointId or full grid coords", label)),
    };
    if !ignore_plane && !plane.accepts_grid(gx, gy, gz) {
        return Err(format!("Invalid {} grid coordinate for plane {}: ({},{},{})",
            label, plane.as_str(), gx, gy, gz));
    }
    if let Some(existing) = sketch.find_point_by_grid(gx, gy, gz) {
        return Ok((existing.id.clone(), None));
    }
    let id = sketch.next_point_id();
    sketch.points.push(Point {
        id: id.clone(), gx, gy, gz,
        x: gx as f64 * grid_size, y: gy as f64 * grid_size, z: gz as f64 * grid_size,
    });
    Ok((id.clone(), Some(id)))
}

use serde::Deserialize;
use crate::sketch::profiles::detect_profiles;
use crate::sketch::types::{Point, SketchGraph, WorkingPlane};
use crate::sketch::validation::validate;
use super::result::{err_result, finalize, SketchCommandResult};

#[derive(Debug, Clone, Deserialize)]
pub struct AddPointRequest {
    pub sketch: SketchGraph,
    #[serde(rename = "workingPlane")] pub working_plane: String,
    #[serde(rename = "gridSize")]     pub grid_size: f64,
    pub gx: i32, pub gy: i32, pub gz: i32,
    #[serde(default, alias = "ignorePlaneConstraint", rename = "ignore_plane_constraint")]
    pub ignore_plane_constraint: Option<bool>,
}

pub fn apply_add_point(req: AddPointRequest) -> SketchCommandResult {
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
    if !ignore_plane && !plane.accepts_grid(req.gx, req.gy, req.gz) {
        return err_result(sketch, format!(
            "Invalid grid coordinate for plane {}: ({},{},{})",
            plane.as_str(), req.gx, req.gy, req.gz
        ));
    }

    if let Some(existing) = sketch.find_point_by_grid(req.gx, req.gy, req.gz) {
        let reused = existing.id.clone();
        sketch.profiles = detect_profiles(&sketch);
        let validation = validate(&sketch);
        return SketchCommandResult {
            ok: true, sketch, created_point_id: None,
            reused_point_id: Some(reused.clone()), created_edge_id: None,
            validation, message: Some(format!("Backend reused point {}", reused)),
        };
    }

    let id  = sketch.next_point_id();
    let g   = req.grid_size;
    sketch.points.push(Point {
        id: id.clone(), gx: req.gx, gy: req.gy, gz: req.gz,
        x: req.gx as f64 * g, y: req.gy as f64 * g, z: req.gz as f64 * g,
    });

    let validation = finalize(&mut sketch);
    SketchCommandResult {
        ok: true, sketch, created_point_id: Some(id.clone()),
        reused_point_id: None, created_edge_id: None,
        validation, message: Some(format!("Backend created point {}", id)),
    }
}

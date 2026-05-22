use serde::Deserialize;
use crate::sketch::types::{SketchGraph, WorkingPlane};
use super::result::{err_result, finalize, SketchCommandResult};

#[derive(Debug, Clone, Deserialize)]
pub struct MovePointRequest {
    pub sketch: SketchGraph,
    #[serde(rename = "workingPlane")] pub working_plane: String,
    #[serde(rename = "gridSize")]     pub grid_size: f64,
    #[serde(rename = "pointId")]      pub point_id: String,
    pub gx: i32, pub gy: i32, pub gz: i32,
    #[serde(default, alias = "ignorePlaneConstraint", rename = "ignore_plane_constraint")]
    pub ignore_plane_constraint: Option<bool>,
}

pub fn apply_move_point(req: MovePointRequest) -> SketchCommandResult {
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
    if sketch.find_point(&req.point_id).is_none() {
        return err_result(sketch, format!("pointId not found: {}", req.point_id));
    }
    if let Some(occ) = sketch.find_point_by_grid(req.gx, req.gy, req.gz).map(|p| p.id.clone()) {
        if occ != req.point_id {
            return err_result(sketch, format!(
                "Target ({},{},{}) is already occupied by point {}",
                req.gx, req.gy, req.gz, occ
            ));
        }
    }

    let g = req.grid_size;
    if let Some(pt) = sketch.points.iter_mut().find(|p| p.id == req.point_id) {
        pt.gx = req.gx; pt.gy = req.gy; pt.gz = req.gz;
        pt.x  = req.gx as f64 * g;
        pt.y  = req.gy as f64 * g;
        pt.z  = req.gz as f64 * g;
    }
    let validation = finalize(&mut sketch);
    SketchCommandResult {
        ok: true, sketch, created_point_id: None, reused_point_id: None, created_edge_id: None,
        validation,
        message: Some(format!("Moved point {} to ({},{},{})", req.point_id, req.gx, req.gy, req.gz)),
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use crate::sketch::commands::add_edge::{apply_add_edge, AddEdgeRequest};
    use crate::sketch::commands::add_point::{apply_add_point, AddPointRequest};
    use crate::sketch::commands::result::PointRefOrGrid;
    use crate::sketch::profiles::detect_profiles;
    use crate::sketch::types::SketchGraph;

    fn empty_xz() -> SketchGraph {
        SketchGraph { working_plane: "XZ".into(), grid_size: 1.0, ..Default::default() }
    }

    #[test]
    fn add_point_creates_p1() {
        let res = apply_add_point(AddPointRequest {
            sketch: empty_xz(), working_plane: "XZ".into(), grid_size: 1.0,
            gx: 2, gy: 0, gz: 3, ignore_plane_constraint: None,
        });
        assert!(res.ok);
        assert_eq!(res.created_point_id.as_deref(), Some("p_1"));
        let p = &res.sketch.points[0];
        assert_eq!((p.x, p.y, p.z), (2.0, 0.0, 3.0));
    }

    #[test]
    fn add_point_same_grid_reuses() {
        let r1 = apply_add_point(AddPointRequest {
            sketch: empty_xz(), working_plane: "XZ".into(), grid_size: 1.0,
            gx: 2, gy: 0, gz: 3, ignore_plane_constraint: None,
        });
        let r2 = apply_add_point(AddPointRequest {
            sketch: r1.sketch, working_plane: "XZ".into(), grid_size: 1.0,
            gx: 2, gy: 0, gz: 3, ignore_plane_constraint: None,
        });
        assert!(r2.ok);
        assert_eq!(r2.reused_point_id.as_deref(), Some("p_1"));
        assert_eq!(r2.sketch.points.len(), 1);
    }

    #[test]
    fn add_point_off_plane_rejected() {
        let res = apply_add_point(AddPointRequest {
            sketch: empty_xz(), working_plane: "XZ".into(), grid_size: 1.0,
            gx: 0, gy: 1, gz: 0, ignore_plane_constraint: None,
        });
        assert!(!res.ok);
    }

    #[test]
    fn add_edge_self_loop_rejected() {
        let r1 = apply_add_point(AddPointRequest {
            sketch: empty_xz(), working_plane: "XZ".into(), grid_size: 1.0,
            gx: 0, gy: 0, gz: 0, ignore_plane_constraint: None,
        });
        let r2 = apply_add_edge(AddEdgeRequest {
            sketch: r1.sketch, working_plane: "XZ".into(), grid_size: 1.0,
            start: PointRefOrGrid { point_id: Some("p_1".into()), gx: None, gy: None, gz: None },
            end:   PointRefOrGrid { point_id: Some("p_1".into()), gx: None, gy: None, gz: None },
            ignore_plane_constraint: None,
        });
        assert!(!r2.ok);
        assert!(r2.message.unwrap().contains("self-loop"));
    }

    #[test]
    fn rectangle_produces_one_closed_profile() {
        let mut s = empty_xz();
        for (gx, gz) in [(0,0),(2,0),(2,2),(0,2)] {
            s = apply_add_point(AddPointRequest {
                sketch: s, working_plane: "XZ".into(), grid_size: 1.0,
                gx, gy: 0, gz, ignore_plane_constraint: None,
            }).sketch;
        }
        for (a, b) in [("p_1","p_2"),("p_2","p_3"),("p_3","p_4"),("p_4","p_1")] {
            s = apply_add_edge(AddEdgeRequest {
                sketch: s, working_plane: "XZ".into(), grid_size: 1.0,
                start: PointRefOrGrid { point_id: Some(a.into()), gx:None,gy:None,gz:None },
                end:   PointRefOrGrid { point_id: Some(b.into()), gx:None,gy:None,gz:None },
                ignore_plane_constraint: None,
            }).sketch;
        }
        let profiles = detect_profiles(&s);
        assert_eq!(profiles.len(), 1);
        assert_eq!(profiles[0].edge_ids.len(), 4);
        assert!(profiles[0].closed);
    }
}

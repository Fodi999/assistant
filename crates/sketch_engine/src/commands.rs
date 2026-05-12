// ── Sketch commands — `apply_add_point`, `apply_add_edge` ────────────────
//
// Pure functions: every command returns a fresh `SketchCommandResult` with
// the normalized `SketchGraph`. Inputs are never mutated in place; callers
// receive ownership of the updated state.

use serde::{Deserialize, Serialize};

use super::profiles::detect_profiles;
use super::sketch::{Edge, Point, SketchGraph, WorkingPlane};
use super::validation::{validate, ValidationResult};

#[derive(Debug, Clone, Deserialize)]
pub struct AddPointRequest {
    pub sketch: SketchGraph,
    #[serde(rename = "workingPlane")]
    pub working_plane: String,
    #[serde(rename = "gridSize")]
    pub grid_size: f64,
    pub gx: i32,
    pub gy: i32,
    pub gz: i32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PointRefOrGrid {
    #[serde(rename = "pointId", default)]
    pub point_id: Option<String>,
    #[serde(default)]
    pub gx: Option<i32>,
    #[serde(default)]
    pub gy: Option<i32>,
    #[serde(default)]
    pub gz: Option<i32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AddEdgeRequest {
    pub sketch: SketchGraph,
    #[serde(rename = "workingPlane")]
    pub working_plane: String,
    #[serde(rename = "gridSize")]
    pub grid_size: f64,
    pub start: PointRefOrGrid,
    pub end: PointRefOrGrid,
}

#[derive(Debug, Clone, Serialize)]
pub struct SketchCommandResult {
    pub ok: bool,
    pub sketch: SketchGraph,
    #[serde(rename = "createdPointId")]
    pub created_point_id: Option<String>,
    #[serde(rename = "reusedPointId")]
    pub reused_point_id: Option<String>,
    #[serde(rename = "createdEdgeId")]
    pub created_edge_id: Option<String>,
    pub validation: ValidationResult,
    pub message: Option<String>,
}

fn err_result(sketch: SketchGraph, msg: impl Into<String>) -> SketchCommandResult {
    let validation = validate(&sketch);
    SketchCommandResult {
        ok: false,
        sketch,
        created_point_id: None,
        reused_point_id: None,
        created_edge_id: None,
        validation,
        message: Some(msg.into()),
    }
}

fn finalize(mut sketch: SketchGraph) -> ValidationResult {
    // Recompute profiles + validation after every successful mutation.
    sketch.profiles = detect_profiles(&sketch);
    validate(&sketch)
}

// ── Add point ─────────────────────────────────────────────────────────────

pub fn apply_add_point(req: AddPointRequest) -> SketchCommandResult {
    let mut sketch = req.sketch;
    sketch.working_plane = req.working_plane.clone();
    sketch.grid_size = req.grid_size;

    // Validate grid size.
    if !(req.grid_size.is_finite() && req.grid_size > 0.0) {
        return err_result(sketch, format!("gridSize must be positive finite, got {}", req.grid_size));
    }

    // Validate plane.
    let plane = match WorkingPlane::parse(&req.working_plane) {
        Some(p) => p,
        None => {
            let msg = format!("Invalid workingPlane: {}", req.working_plane);
            return err_result(sketch, msg);
        }
    };

    // Enforce plane constraint on grid coords.
    if !plane.accepts_grid(req.gx, req.gy, req.gz) {
        let msg = format!(
            "Invalid grid coordinate for plane {}: ({},{},{})",
            plane.as_str(),
            req.gx,
            req.gy,
            req.gz
        );
        return err_result(sketch, msg);
    }

    // Reuse existing point at same grid.
    if let Some(existing) = sketch.find_point_by_grid(req.gx, req.gy, req.gz) {
        let reused = existing.id.clone();
        let mut profiles_sketch = sketch.clone();
        let validation = finalize(profiles_sketch.clone());
        // Apply derived state.
        profiles_sketch.profiles = detect_profiles(&profiles_sketch);
        return SketchCommandResult {
            ok: true,
            sketch: profiles_sketch,
            created_point_id: None,
            reused_point_id: Some(reused.clone()),
            created_edge_id: None,
            validation,
            message: Some(format!("Backend reused point {}", reused)),
        };
    }

    // Create a fresh point id.
    let id = sketch.next_point_id();
    let g = req.grid_size;
    sketch.points.push(Point {
        id: id.clone(),
        gx: req.gx,
        gy: req.gy,
        gz: req.gz,
        x: (req.gx as f64) * g,
        y: (req.gy as f64) * g,
        z: (req.gz as f64) * g,
    });

    let validation = finalize(sketch.clone());
    sketch.profiles = detect_profiles(&sketch);

    SketchCommandResult {
        ok: true,
        sketch,
        created_point_id: Some(id.clone()),
        reused_point_id: None,
        created_edge_id: None,
        validation,
        message: Some(format!("Backend created point {}", id)),
    }
}

// ── Add edge ──────────────────────────────────────────────────────────────

/// Resolve a `PointRefOrGrid` to a point id, creating a new point if needed.
/// Returns (point_id, created_id_if_any) or an error message.
fn resolve_endpoint(
    sketch: &mut SketchGraph,
    plane: WorkingPlane,
    grid_size: f64,
    r: &PointRefOrGrid,
    label: &str,
) -> Result<(String, Option<String>), String> {
    if let Some(pid) = &r.point_id {
        if sketch.find_point(pid).is_some() {
            return Ok((pid.clone(), None));
        }
        return Err(format!("{} pointId not found: {}", label, pid));
    }

    let (gx, gy, gz) = match (r.gx, r.gy, r.gz) {
        (Some(x), Some(y), Some(z)) => (x, y, z),
        _ => {
            return Err(format!(
                "{} must provide either pointId or full grid coords (gx,gy,gz)",
                label
            ));
        }
    };

    if !plane.accepts_grid(gx, gy, gz) {
        return Err(format!(
            "Invalid {} grid coordinate for plane {}: ({},{},{})",
            label,
            plane.as_str(),
            gx,
            gy,
            gz
        ));
    }

    if let Some(existing) = sketch.find_point_by_grid(gx, gy, gz) {
        return Ok((existing.id.clone(), None));
    }

    let id = sketch.next_point_id();
    sketch.points.push(Point {
        id: id.clone(),
        gx,
        gy,
        gz,
        x: (gx as f64) * grid_size,
        y: (gy as f64) * grid_size,
        z: (gz as f64) * grid_size,
    });
    Ok((id.clone(), Some(id)))
}

pub fn apply_add_edge(req: AddEdgeRequest) -> SketchCommandResult {
    let mut sketch = req.sketch;
    sketch.working_plane = req.working_plane.clone();
    sketch.grid_size = req.grid_size;

    if !(req.grid_size.is_finite() && req.grid_size > 0.0) {
        return err_result(sketch, format!("gridSize must be positive finite, got {}", req.grid_size));
    }

    let plane = match WorkingPlane::parse(&req.working_plane) {
        Some(p) => p,
        None => {
            let msg = format!("Invalid workingPlane: {}", req.working_plane);
            return err_result(sketch, msg);
        }
    };

    // Resolve start.
    let (start_id, created_a) = match resolve_endpoint(&mut sketch, plane, req.grid_size, &req.start, "start") {
        Ok(v) => v,
        Err(e) => return err_result(sketch, e),
    };
    // Resolve end.
    let (end_id, created_b) = match resolve_endpoint(&mut sketch, plane, req.grid_size, &req.end, "end") {
        Ok(v) => v,
        Err(e) => return err_result(sketch, e),
    };

    if start_id == end_id {
        return err_result(sketch, "Edge endpoints coincide (self-loop)".to_string());
    }

    if sketch.find_edge_between(&start_id, &end_id).is_some() {
        let validation = validate(&sketch);
        sketch.profiles = detect_profiles(&sketch);
        return SketchCommandResult {
            ok: false,
            sketch,
            created_point_id: created_a.or(created_b),
            reused_point_id: None,
            created_edge_id: None,
            validation,
            message: Some("Edge already exists".to_string()),
        };
    }

    let edge_id = sketch.next_edge_id();
    sketch.edges.push(Edge {
        id: edge_id.clone(),
        a: start_id,
        b: end_id,
    });

    let validation = finalize(sketch.clone());
    sketch.profiles = detect_profiles(&sketch);

    let created_point_id = created_a.clone().or(created_b.clone());
    let message = if created_point_id.is_some() {
        format!("Backend created edge {} (+ new point)", edge_id)
    } else {
        format!("Backend created edge {}", edge_id)
    };

    SketchCommandResult {
        ok: true,
        sketch,
        created_point_id,
        reused_point_id: None,
        created_edge_id: Some(edge_id),
        validation,
        message: Some(message),
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_xz() -> SketchGraph {
        SketchGraph {
            working_plane: "XZ".into(),
            grid_size: 1.0,
            ..Default::default()
        }
    }

    #[test]
    fn add_point_creates_p1_from_grid() {
        let res = apply_add_point(AddPointRequest {
            sketch: empty_xz(),
            working_plane: "XZ".into(),
            grid_size: 1.0,
            gx: 2,
            gy: 0,
            gz: 3,
        });
        assert!(res.ok, "expected ok, got {:?}", res.message);
        assert_eq!(res.created_point_id.as_deref(), Some("p_1"));
        assert_eq!(res.sketch.points.len(), 1);
        let p = &res.sketch.points[0];
        assert_eq!((p.x, p.y, p.z), (2.0, 0.0, 3.0));
    }

    #[test]
    fn add_point_same_grid_reuses_p1() {
        let r1 = apply_add_point(AddPointRequest {
            sketch: empty_xz(),
            working_plane: "XZ".into(),
            grid_size: 1.0,
            gx: 2,
            gy: 0,
            gz: 3,
        });
        let r2 = apply_add_point(AddPointRequest {
            sketch: r1.sketch.clone(),
            working_plane: "XZ".into(),
            grid_size: 1.0,
            gx: 2,
            gy: 0,
            gz: 3,
        });
        assert!(r2.ok);
        assert_eq!(r2.created_point_id, None);
        assert_eq!(r2.reused_point_id.as_deref(), Some("p_1"));
        assert_eq!(r2.sketch.points.len(), 1);
    }

    #[test]
    fn add_point_xz_with_gy_nonzero_rejected() {
        let res = apply_add_point(AddPointRequest {
            sketch: empty_xz(),
            working_plane: "XZ".into(),
            grid_size: 1.0,
            gx: 0,
            gy: 1,
            gz: 0,
        });
        assert!(!res.ok);
        assert!(res.message.unwrap().contains("plane"));
    }

    #[test]
    fn add_edge_creates_missing_endpoint() {
        // Seed with one point.
        let r1 = apply_add_point(AddPointRequest {
            sketch: empty_xz(),
            working_plane: "XZ".into(),
            grid_size: 1.0,
            gx: 0,
            gy: 0,
            gz: 0,
        });
        // Add edge whose end-point doesn't yet exist.
        let r2 = apply_add_edge(AddEdgeRequest {
            sketch: r1.sketch,
            working_plane: "XZ".into(),
            grid_size: 1.0,
            start: PointRefOrGrid {
                point_id: Some("p_1".into()),
                gx: None,
                gy: None,
                gz: None,
            },
            end: PointRefOrGrid {
                point_id: None,
                gx: Some(3),
                gy: Some(0),
                gz: Some(0),
            },
        });
        assert!(r2.ok);
        assert_eq!(r2.created_point_id.as_deref(), Some("p_2"));
        assert_eq!(r2.created_edge_id.as_deref(), Some("e_1"));
        assert_eq!(r2.sketch.points.len(), 2);
        assert_eq!(r2.sketch.edges.len(), 1);
    }

    #[test]
    fn add_edge_rejects_self_loop() {
        let r1 = apply_add_point(AddPointRequest {
            sketch: empty_xz(),
            working_plane: "XZ".into(),
            grid_size: 1.0,
            gx: 0,
            gy: 0,
            gz: 0,
        });
        let r2 = apply_add_edge(AddEdgeRequest {
            sketch: r1.sketch,
            working_plane: "XZ".into(),
            grid_size: 1.0,
            start: PointRefOrGrid {
                point_id: Some("p_1".into()),
                gx: None,
                gy: None,
                gz: None,
            },
            end: PointRefOrGrid {
                point_id: Some("p_1".into()),
                gx: None,
                gy: None,
                gz: None,
            },
        });
        assert!(!r2.ok);
        assert!(r2.message.unwrap().contains("self-loop"));
    }

    #[test]
    fn add_edge_rejects_duplicate_reversed() {
        // Build A — B.
        let r1 = apply_add_point(AddPointRequest {
            sketch: empty_xz(),
            working_plane: "XZ".into(),
            grid_size: 1.0,
            gx: 0,
            gy: 0,
            gz: 0,
        });
        let r2 = apply_add_edge(AddEdgeRequest {
            sketch: r1.sketch,
            working_plane: "XZ".into(),
            grid_size: 1.0,
            start: PointRefOrGrid { point_id: Some("p_1".into()), gx: None, gy: None, gz: None },
            end:   PointRefOrGrid { point_id: None, gx: Some(2), gy: Some(0), gz: Some(0) },
        });
        // Now try B — A (reversed).
        let r3 = apply_add_edge(AddEdgeRequest {
            sketch: r2.sketch,
            working_plane: "XZ".into(),
            grid_size: 1.0,
            start: PointRefOrGrid { point_id: Some("p_2".into()), gx: None, gy: None, gz: None },
            end:   PointRefOrGrid { point_id: Some("p_1".into()), gx: None, gy: None, gz: None },
        });
        assert!(!r3.ok);
        assert_eq!(r3.message.as_deref(), Some("Edge already exists"));
    }

    #[test]
    fn rectangle_produces_one_closed_profile() {
        // Build a square in XZ: (0,0,0)-(2,0,0)-(2,0,2)-(0,0,2)-back.
        let mut s = empty_xz();
        for (gx, gz) in [(0, 0), (2, 0), (2, 2), (0, 2)] {
            s = apply_add_point(AddPointRequest {
                sketch: s,
                working_plane: "XZ".into(),
                grid_size: 1.0,
                gx,
                gy: 0,
                gz,
            })
            .sketch;
        }
        let edges = [("p_1", "p_2"), ("p_2", "p_3"), ("p_3", "p_4"), ("p_4", "p_1")];
        for (a, b) in edges {
            s = apply_add_edge(AddEdgeRequest {
                sketch: s,
                working_plane: "XZ".into(),
                grid_size: 1.0,
                start: PointRefOrGrid { point_id: Some(a.into()), gx: None, gy: None, gz: None },
                end:   PointRefOrGrid { point_id: Some(b.into()), gx: None, gy: None, gz: None },
            })
            .sketch;
        }
        let profiles = detect_profiles(&s);
        assert_eq!(profiles.len(), 1, "expected one closed profile, got {}", profiles.len());
        assert_eq!(profiles[0].edge_ids.len(), 4);
        assert!(profiles[0].closed);
    }

    #[test]
    fn world_coords_derived_from_grid_and_grid_size() {
        let res = apply_add_point(AddPointRequest {
            sketch: SketchGraph { grid_size: 0.5, working_plane: "XZ".into(), ..Default::default() },
            working_plane: "XZ".into(),
            grid_size: 0.5,
            gx: 4,
            gy: 0,
            gz: -2,
        });
        assert!(res.ok);
        let p = &res.sketch.points[0];
        assert!((p.x - 2.0).abs() < 1e-9);
        assert!((p.y - 0.0).abs() < 1e-9);
        assert!((p.z + 1.0).abs() < 1e-9);
    }
}

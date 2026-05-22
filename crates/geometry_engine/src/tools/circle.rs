// ── tools/circle.rs — Circle tool (native Rust) ───────────────────────────────
//
// Creates N-gon polyline approximating a circle.
// Center + radius in grid units → SketchDelta (points + edges).
//
// Planes:
//   XZ: circle in Y=0 plane (gx, gz axes)
//   XY: circle in Z=0 plane (gx, gy axes)
//   YZ: circle in X=0 plane (gy, gz axes)

use super::types::{SketchDelta, ToolEdge, ToolPoint};
use std::f64::consts::PI;

pub struct CircleInput {
    /// Grid coordinates of the centre point.
    pub center_gx: i64,
    pub center_gy: i64,
    pub center_gz: i64,
    /// Radius in grid units (must be >= 1).
    pub radius: f64,
    pub plane: String,
    /// Number of polygon segments (default 32).
    pub segments: usize,
    /// Starting ID offset for unique ID generation.
    pub id_offset: u64,
}

pub fn create_circle(input: CircleInput) -> SketchDelta {
    let CircleInput { center_gx, center_gy, center_gz, radius, plane, segments, id_offset } = input;

    if radius < 0.5 {
        return SketchDelta::err("circle: radius too small (< 0.5 grid units)");
    }

    let seg = segments.max(3).min(256);
    let base = id_offset;
    let mut delta = SketchDelta::empty();

    // Build corners
    let mut pts: Vec<(i64, i64, i64)> = Vec::with_capacity(seg);
    for i in 0..seg {
        let angle = 2.0 * PI * i as f64 / seg as f64;
        let cos_a = angle.cos();
        let sin_a = angle.sin();
        let (gx, gy, gz) = match plane.as_str() {
            "XY" => (
                (center_gx as f64 + radius * cos_a).round() as i64,
                (center_gy as f64 + radius * sin_a).round() as i64,
                0,
            ),
            "YZ" => (
                0,
                (center_gy as f64 + radius * cos_a).round() as i64,
                (center_gz as f64 + radius * sin_a).round() as i64,
            ),
            _ /* XZ */ => (
                (center_gx as f64 + radius * cos_a).round() as i64,
                0,
                (center_gz as f64 + radius * sin_a).round() as i64,
            ),
        };
        pts.push((gx, gy, gz));
    }

    // Points
    let pt_ids: Vec<String> = (0..seg).map(|i| format!("cpt_{}_{}", base, i)).collect();
    for (i, (gx, gy, gz)) in pts.iter().enumerate() {
        delta.new_points.push(ToolPoint {
            id: pt_ids[i].clone(),
            gx: *gx,
            gy: *gy,
            gz: *gz,
        });
    }

    // Edges (closed loop)
    for i in 0..seg {
        delta.new_edges.push(ToolEdge {
            id: format!("ce_{}_{}", base, i),
            a: pt_ids[i].clone(),
            b: pt_ids[(i + 1) % seg].clone(),
            kind: "normal".into(),
        });
    }

    delta
}

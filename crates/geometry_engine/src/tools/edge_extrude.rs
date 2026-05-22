// ── tools/edge_extrude.rs — Edge Extrude tool (native Rust) ──────────────────
//
// Blender-style wall surface generator.
// For each selected edge: create top points + top edge + 2 vertical edges.
// Returns SketchDelta + WallSurface records.
//
// Direction per plane:
//   XZ → Y axis (up)
//   XY → Z axis (depth)
//   YZ → X axis (width)

use super::types::{EdgeExtrudeResult, SketchDelta, ToolEdge, ToolPoint, WallSurface};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtrudeSourcePoint {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub gx: i64,
    pub gy: i64,
    pub gz: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtrudeSourceEdge {
    pub id: String,
    pub a: String,
    pub b: String,
}

pub struct EdgeExtrudeInput {
    pub edges: Vec<ExtrudeSourceEdge>,
    pub points: Vec<ExtrudeSourcePoint>,
    /// Extrusion height in metres (may be negative for downward).
    pub height_m: f64,
    pub plane: String,
    /// Grid size in metres.
    pub grid_size: f64,
    pub id_offset: u64,
}

/// Extrusion direction vector per plane.
fn extrude_dir(plane: &str) -> (f64, f64, f64) {
    match plane {
        "XY" => (0.0, 0.0, 1.0),
        "YZ" => (1.0, 0.0, 0.0),
        _    => (0.0, 1.0, 0.0), // XZ → Y
    }
}

pub fn edge_extrude(input: EdgeExtrudeInput) -> EdgeExtrudeResult {
    let EdgeExtrudeInput { edges, points, height_m, plane, grid_size, id_offset } = input;

    if edges.is_empty() {
        return EdgeExtrudeResult {
            ok: false,
            error: Some("edge_extrude: no edges".into()),
            delta: SketchDelta::empty(),
            wall_surfaces: vec![],
        };
    }
    if height_m == 0.0 {
        return EdgeExtrudeResult {
            ok: false,
            error: Some("edge_extrude: height is zero".into()),
            delta: SketchDelta::empty(),
            wall_surfaces: vec![],
        };
    }

    let gs = if grid_size > 0.0 { grid_size } else { 0.001 };
    let (dx, dy, dz) = extrude_dir(&plane);
    let (dx, dy, dz) = (dx * height_m, dy * height_m, dz * height_m);

    let base = id_offset;
    let mut delta = SketchDelta::empty();
    let mut walls: Vec<WallSurface> = Vec::new();

    let pt_map: std::collections::HashMap<&str, &ExtrudeSourcePoint> =
        points.iter().map(|p| (p.id.as_str(), p)).collect();

    // Cache: bottom point id → top point id (shared corners between edges)
    let mut top_pt_map: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    let mut top_pt_counter: u64 = 0;

    for (ei, edge) in edges.iter().enumerate() {
        let ba = match pt_map.get(edge.a.as_str()) {
            Some(p) => *p,
            None => continue,
        };
        let bb = match pt_map.get(edge.b.as_str()) {
            Some(p) => *p,
            None => continue,
        };

        // Get or create top point A
        let top_a_id = top_pt_map
            .entry(edge.a.clone())
            .or_insert_with(|| {
                let id = format!("ept_{}_{}a", base, top_pt_counter);
                top_pt_counter += 1;
                let gx = ((ba.x + dx) / gs).round() as i64;
                let gy = ((ba.y + dy) / gs).round() as i64;
                let gz = ((ba.z + dz) / gs).round() as i64;
                delta.new_points.push(ToolPoint { id: id.clone(), gx, gy, gz });
                id
            })
            .clone();

        // Get or create top point B
        let top_b_id = top_pt_map
            .entry(edge.b.clone())
            .or_insert_with(|| {
                let id = format!("ept_{}_{}b", base, top_pt_counter);
                top_pt_counter += 1;
                let gx = ((bb.x + dx) / gs).round() as i64;
                let gy = ((bb.y + dy) / gs).round() as i64;
                let gz = ((bb.z + dz) / gs).round() as i64;
                delta.new_points.push(ToolPoint { id: id.clone(), gx, gy, gz });
                id
            })
            .clone();

        // Top edge
        delta.new_edges.push(ToolEdge {
            id: format!("etop_{}_{}", base, ei),
            a: top_a_id.clone(),
            b: top_b_id.clone(),
            kind: "normal".into(),
        });

        // Vertical edge A
        delta.new_edges.push(ToolEdge {
            id: format!("eva_{}_{}", base, ei),
            a: edge.a.clone(),
            b: top_a_id.clone(),
            kind: "normal".into(),
        });

        // Vertical edge B
        delta.new_edges.push(ToolEdge {
            id: format!("evb_{}_{}", base, ei),
            a: edge.b.clone(),
            b: top_b_id.clone(),
            kind: "normal".into(),
        });

        // Wall surface record (world coords)
        walls.push(WallSurface {
            id: format!("wall_{}_{}", base, ei),
            source_edge_id: edge.id.clone(),
            bottom_a: [ba.x, ba.y, ba.z],
            bottom_b: [bb.x, bb.y, bb.z],
            top_a: [ba.x + dx, ba.y + dy, ba.z + dz],
            top_b: [bb.x + dx, bb.y + dy, bb.z + dz],
            height: height_m,
            plane: plane.clone(),
            top_a_id: top_a_id.clone(),
            top_b_id: top_b_id.clone(),
        });
    }

    EdgeExtrudeResult {
        ok: true,
        error: None,
        delta,
        wall_surfaces: walls,
    }
}

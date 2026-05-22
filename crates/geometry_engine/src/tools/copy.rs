// ── tools/copy.rs — Copy-Connect tool (native Rust) ───────────────────────────
//
// Duplicates a set of points + edges with a translation delta (dx, dy, dz).
// Also creates "connector" edges between each original and its copy.
// Returns SketchDelta with all new points + edges.

use super::types::{SketchDelta, ToolEdge, ToolPoint};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopySourcePoint {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopySourceEdge {
    pub a: String, // original point id
    pub b: String, // original point id
    pub kind: String,
}

pub struct CopyInput {
    pub points: Vec<CopySourcePoint>,
    pub edges: Vec<CopySourceEdge>,
    /// World-space delta to apply.
    pub dx: f64,
    pub dy: f64,
    pub dz: f64,
    /// Grid size in metres (to convert world → grid coords).
    pub grid_size: f64,
    pub id_offset: u64,
}

pub fn copy_selection(input: CopyInput) -> SketchDelta {
    let CopyInput { points, edges, dx, dy, dz, grid_size, id_offset } = input;

    if points.is_empty() {
        return SketchDelta::err("copy: nothing to copy");
    }
    if dx == 0.0 && dy == 0.0 && dz == 0.0 {
        return SketchDelta::err("copy: zero offset");
    }

    let gs = if grid_size > 0.0 { grid_size } else { 0.001 };
    let base = id_offset;
    let mut delta = SketchDelta::empty();

    // Map original id → new id
    let mut orig_to_new: Vec<(String, String)> = Vec::new();

    for (i, pt) in points.iter().enumerate() {
        let nx = pt.x + dx;
        let ny = pt.y + dy;
        let nz = pt.z + dz;
        let gx = (nx / gs).round() as i64;
        let gy = (ny / gs).round() as i64;
        let gz = (nz / gs).round() as i64;
        let new_id = format!("cppt_{}_{}", base, i);
        orig_to_new.push((pt.id.clone(), new_id.clone()));
        delta.new_points.push(ToolPoint { id: new_id, gx, gy, gz });
    }

    let id_map: std::collections::HashMap<String, String> = orig_to_new.iter().cloned().collect();

    // Copied edges
    for (i, e) in edges.iter().enumerate() {
        let a2 = id_map.get(&e.a);
        let b2 = id_map.get(&e.b);
        if let (Some(a2), Some(b2)) = (a2, b2) {
            if a2 != b2 {
                delta.new_edges.push(ToolEdge {
                    id: format!("cpe_{}_{}", base, i),
                    a: a2.clone(),
                    b: b2.clone(),
                    kind: e.kind.clone(),
                });
            }
        }
    }

    // Connector edges: original → copy for each point
    for (i, (orig_id, new_id)) in orig_to_new.iter().enumerate() {
        delta.new_edges.push(ToolEdge {
            id: format!("cpcon_{}_{}", base, i),
            a: orig_id.clone(),
            b: new_id.clone(),
            kind: "normal".into(),
        });
    }

    delta
}

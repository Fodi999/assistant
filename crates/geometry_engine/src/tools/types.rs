// ── tools/types.rs — shared data types for all tools ─────────────────────────

use serde::{Deserialize, Serialize};

/// A point to be created/updated in the sketch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolPoint {
    pub id: String,
    pub gx: i64,
    pub gy: i64,
    pub gz: i64,
}

/// An edge to be created in the sketch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolEdge {
    pub id: String,
    pub a: String,
    pub b: String,
    pub kind: String, // "normal" | "construction"
}

/// A constraint to be added to the sketch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolConstraint {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String, // "HORIZONTAL" | "VERTICAL" | "EQUAL_LENGTH" | "FIXED_LENGTH" | "edge_length"
    pub target_type: String, // "edge" | "point"
    pub target_id: String,
    pub value: Option<f64>,
}

/// The result of a tool operation — mutations to apply to sketchState.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SketchDelta {
    pub ok: bool,
    pub error: Option<String>,
    pub new_points: Vec<ToolPoint>,
    pub new_edges: Vec<ToolEdge>,
    pub new_constraints: Vec<ToolConstraint>,
    pub removed_point_ids: Vec<String>,
    pub removed_edge_ids: Vec<String>,
    pub removed_constraint_ids: Vec<String>,
}

impl SketchDelta {
    pub fn empty() -> Self {
        SketchDelta {
            ok: true,
            error: None,
            new_points: vec![],
            new_edges: vec![],
            new_constraints: vec![],
            removed_point_ids: vec![],
            removed_edge_ids: vec![],
            removed_constraint_ids: vec![],
        }
    }

    pub fn err(msg: impl Into<String>) -> Self {
        SketchDelta {
            ok: false,
            error: Some(msg.into()),
            new_points: vec![],
            new_edges: vec![],
            new_constraints: vec![],
            removed_point_ids: vec![],
            removed_edge_ids: vec![],
            removed_constraint_ids: vec![],
        }
    }
}

/// A wall surface produced by edge extrude.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WallSurface {
    pub id: String,
    pub source_edge_id: String,
    pub bottom_a: [f64; 3],
    pub bottom_b: [f64; 3],
    pub top_a: [f64; 3],
    pub top_b: [f64; 3],
    pub height: f64,
    pub plane: String,
    pub top_a_id: String,
    pub top_b_id: String,
}

/// Result of edge extrude: delta (new points/edges) + wall surface records.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeExtrudeResult {
    pub ok: bool,
    pub error: Option<String>,
    pub delta: SketchDelta,
    pub wall_surfaces: Vec<WallSurface>,
}

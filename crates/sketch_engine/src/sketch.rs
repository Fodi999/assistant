// ── SketchGraph v1 wire types (mirror of frontend `__sketchToJSON`) ───────
//
// Serde shape is fixed by the existing JS contract:
//   { schema:"sketch_graph", version:1, workingPlane, gridSize, points[], edges[], constraints[] }
//
// `Default` is implemented so HTTP handlers can accept partial payloads.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkingPlane {
    XZ,
    XY,
    YZ,
}

impl WorkingPlane {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "XZ" => Some(Self::XZ),
            "XY" => Some(Self::XY),
            "YZ" => Some(Self::YZ),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::XZ => "XZ",
            Self::XY => "XY",
            Self::YZ => "YZ",
        }
    }

    /// Returns true if grid coordinate (gx,gy,gz) lies on this plane.
    /// Convention:
    ///   XZ → gy = 0
    ///   XY → gz = 0
    ///   YZ → gx = 0
    pub fn accepts_grid(self, gx: i32, gy: i32, gz: i32) -> bool {
        match self {
            Self::XZ => gy == 0,
            Self::XY => gz == 0,
            Self::YZ => gx == 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Point {
    pub id: String,
    pub gx: i32,
    pub gy: i32,
    pub gz: i32,
    #[serde(default)]
    pub x: f64,
    #[serde(default)]
    pub y: f64,
    #[serde(default)]
    pub z: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub id: String,
    pub a: String,
    pub b: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Constraint {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(rename = "type")]
    pub ty: String,
    #[serde(rename = "targetType")]
    pub target_type: String,
    #[serde(rename = "targetId")]
    pub target_id: String,
    #[serde(default)]
    pub value: Option<f64>,
}

fn default_schema() -> String {
    "sketch_graph".to_string()
}
fn default_version() -> u32 {
    1
}
fn default_plane() -> String {
    "XZ".to_string()
}
fn default_grid_size() -> f64 {
    1.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SketchGraph {
    #[serde(default = "default_schema")]
    pub schema: String,
    #[serde(default = "default_version")]
    pub version: u32,
    #[serde(rename = "workingPlane", default = "default_plane")]
    pub working_plane: String,
    #[serde(rename = "gridSize", default = "default_grid_size")]
    pub grid_size: f64,
    #[serde(default)]
    pub points: Vec<Point>,
    #[serde(default)]
    pub edges: Vec<Edge>,
    #[serde(default)]
    pub constraints: Vec<Constraint>,
    /// Profiles are derived; backend always replaces them on every command.
    #[serde(default)]
    pub profiles: Vec<super::profiles::Profile>,
}

impl Default for SketchGraph {
    fn default() -> Self {
        Self {
            schema: default_schema(),
            version: default_version(),
            working_plane: default_plane(),
            grid_size: default_grid_size(),
            points: Vec::new(),
            edges: Vec::new(),
            constraints: Vec::new(),
            profiles: Vec::new(),
        }
    }
}

impl SketchGraph {
    /// Find existing point at given grid coordinates.
    pub fn find_point_by_grid(&self, gx: i32, gy: i32, gz: i32) -> Option<&Point> {
        self.points
            .iter()
            .find(|p| p.gx == gx && p.gy == gy && p.gz == gz)
    }

    /// Lookup point by id.
    pub fn find_point(&self, id: &str) -> Option<&Point> {
        self.points.iter().find(|p| p.id == id)
    }

    /// Check whether edge already exists between two points (either direction).
    pub fn find_edge_between(&self, a: &str, b: &str) -> Option<&Edge> {
        self.edges
            .iter()
            .find(|e| (e.a == a && e.b == b) || (e.a == b && e.b == a))
    }

    /// Highest numeric suffix used by point ids of form `p_N`.
    pub fn max_point_index(&self) -> u32 {
        max_id_suffix(self.points.iter().map(|p| p.id.as_str()), "p_")
    }

    /// Highest numeric suffix used by edge ids of form `e_N`.
    pub fn max_edge_index(&self) -> u32 {
        max_id_suffix(self.edges.iter().map(|e| e.id.as_str()), "e_")
    }

    /// Allocate next sequential point id, e.g. `p_4`.
    pub fn next_point_id(&self) -> String {
        format!("p_{}", self.max_point_index() + 1)
    }

    /// Allocate next sequential edge id, e.g. `e_5`.
    pub fn next_edge_id(&self) -> String {
        format!("e_{}", self.max_edge_index() + 1)
    }
}

fn max_id_suffix<'a, I: Iterator<Item = &'a str>>(ids: I, prefix: &str) -> u32 {
    let mut max = 0u32;
    for id in ids {
        if let Some(rest) = id.strip_prefix(prefix) {
            if let Ok(n) = rest.parse::<u32>() {
                if n > max {
                    max = n;
                }
            }
        }
    }
    max
}

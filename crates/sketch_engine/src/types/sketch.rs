use serde::{Deserialize, Serialize};

use super::constraint::Constraint;
use super::edge::Edge;
use super::point::Point;
use super::profile::Profile;

fn default_schema() -> String  { "sketch_graph".to_string() }
fn default_version() -> u32    { 1 }
fn default_plane() -> String   { "XZ".to_string() }
fn default_grid_size() -> f64  { 1.0 }

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
    /// Profiles are always derived. The backend regenerates them on every command.
    #[serde(default)]
    pub profiles: Vec<Profile>,
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
        self.points.iter().find(|p| p.gx == gx && p.gy == gy && p.gz == gz)
    }

    /// Lookup point by id.
    pub fn find_point(&self, id: &str) -> Option<&Point> {
        self.points.iter().find(|p| p.id == id)
    }

    /// Check whether edge already exists between two points (either direction).
    pub fn find_edge_between(&self, a: &str, b: &str) -> Option<&Edge> {
        self.edges.iter().find(|e| (e.a == a && e.b == b) || (e.a == b && e.b == a))
    }

    /// Highest numeric suffix used by point ids of form `p_N`.
    pub fn max_point_index(&self) -> u32 {
        max_id_suffix(self.points.iter().map(|p| p.id.as_str()), "p_")
    }

    /// Highest numeric suffix used by edge ids of form `e_N`.
    pub fn max_edge_index(&self) -> u32 {
        max_id_suffix(self.edges.iter().map(|e| e.id.as_str()), "e_")
    }

    pub fn next_point_id(&self) -> String { format!("p_{}", self.max_point_index() + 1) }
    pub fn next_edge_id(&self)  -> String { format!("e_{}", self.max_edge_index()  + 1) }
}

fn max_id_suffix<'a, I: Iterator<Item = &'a str>>(ids: I, prefix: &str) -> u32 {
    let mut max = 0u32;
    for id in ids {
        if let Some(rest) = id.strip_prefix(prefix) {
            if let Ok(n) = rest.parse::<u32>() {
                if n > max { max = n; }
            }
        }
    }
    max
}

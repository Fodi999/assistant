use serde::{Deserialize, Serialize};

/// A detected closed loop of edges in the sketch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub id: String,
    #[serde(rename = "pointIds")]
    pub point_ids: Vec<String>,
    #[serde(rename = "edgeIds")]
    pub edge_ids: Vec<String>,
    pub plane: String,
    pub closed: bool,
}

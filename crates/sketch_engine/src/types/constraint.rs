use serde::{Deserialize, Serialize};

/// A parametric constraint attached to sketch geometry.
///
/// | type              | target_type    | target_id                  | value      |
/// |-------------------|----------------|----------------------------|------------|
/// | HORIZONTAL        | edge           | edge_id                    | —          |
/// | VERTICAL          | edge           | edge_id                    | —          |
/// | EQUAL_LENGTH      | edge           | "edge_a,edge_b"            | —          |
/// | FIX               | point          | point_id                   | —          |
/// | COINCIDENT        | points         | "point_a,point_b"          | —          |
/// | FIXED_LENGTH      | edge           | edge_id                    | mm         |
/// | PARALLEL          | edge           | "edge_ref,edge_adj"        | —          |
/// | PERPENDICULAR     | edge           | "edge_ref,edge_adj"        | —          |
/// | MIDPOINT          | point          | "point_id,edge_id"         | —          |
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
    /// Numeric parameter (e.g. mm for FIXED_LENGTH, degrees for ANGLE).
    #[serde(default)]
    pub value: Option<f64>,
}

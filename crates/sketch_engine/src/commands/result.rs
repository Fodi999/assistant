use serde::{Deserialize, Serialize};
use crate::profiles::detect_profiles;
use crate::types::SketchGraph;
use crate::validation::{validate, ValidationResult};

#[derive(Debug, Clone, Deserialize)]
pub struct PointRefOrGrid {
    #[serde(rename = "pointId", default)]
    pub point_id: Option<String>,
    #[serde(default)] pub gx: Option<i32>,
    #[serde(default)] pub gy: Option<i32>,
    #[serde(default)] pub gz: Option<i32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SketchCommandResult {
    pub ok: bool,
    pub sketch: SketchGraph,
    #[serde(rename = "createdPointId")] pub created_point_id: Option<String>,
    #[serde(rename = "reusedPointId")]  pub reused_point_id: Option<String>,
    #[serde(rename = "createdEdgeId")]  pub created_edge_id: Option<String>,
    pub validation: ValidationResult,
    pub message: Option<String>,
}

pub(crate) fn err_result(sketch: SketchGraph, msg: impl Into<String>) -> SketchCommandResult {
    let validation = validate(&sketch);
    SketchCommandResult {
        ok: false, sketch,
        created_point_id: None, reused_point_id: None, created_edge_id: None,
        validation, message: Some(msg.into()),
    }
}

pub(crate) fn finalize(sketch: &mut SketchGraph) -> ValidationResult {
    sketch.profiles = detect_profiles(sketch);
    validate(sketch)
}

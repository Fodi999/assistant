//! Camera presets and the `SceneCamera` shape consumed by the frontend's
//! `<Canvas camera={...}>` plus `<OrbitControls target={...}>`.

use serde::{Deserialize, Serialize};

use super::transform::Vec3;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CameraPreset {
    Overview,
    Risk,
    Zone,
    Focused,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SceneCamera {
    pub preset: CameraPreset,
    pub position: Vec3,
    pub target: Vec3,
    pub fov: f32,
}

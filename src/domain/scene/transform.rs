//! Spatial primitives — position / rotation / scale in scene-space.
//!
//! Frontend uses these directly as `[x, y, z]` tuples for three.js.
//! All units are abstract scene units; the renderer decides the meter scale.

use serde::{Deserialize, Serialize};

pub type Vec3 = [f32; 3];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Transform {
    pub position: Vec3,
    pub rotation: Vec3,
    pub scale: Vec3,
}

impl Transform {
    pub fn at(position: Vec3) -> Self {
        Self {
            position,
            rotation: [0.0, 0.0, 0.0],
            scale: [1.0, 1.0, 1.0],
        }
    }

    pub fn with_rotation(mut self, rotation: Vec3) -> Self {
        self.rotation = rotation;
        self
    }

    pub fn with_scale(mut self, scale: Vec3) -> Self {
        self.scale = scale;
        self
    }
}

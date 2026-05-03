//! Scene environment — global lighting / ambience knobs.
//!
//! Currently kept minimal; the renderer reads these to set up its three.js
//! lights and (optionally) a backdrop. All fields are optional so the
//! frontend can fall back to its built-in defaults.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SceneEnvironment {
    /// Ambient light intensity (0..1+).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ambient_intensity: Option<f32>,
    /// Ambient light color (hex).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ambient_color: Option<String>,
    /// Key (directional) light intensity.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_light_intensity: Option<f32>,
    /// Background color hex (canvas clear color). `None` = transparent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<String>,
    /// Optional fog color hex.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fog_color: Option<String>,
    /// Fog density (0..1).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fog_density: Option<f32>,
}

//! Material / shading descriptor.
//!
//! Backend declares the *intent* (cold zone, expired item) and the visual
//! contract (accent color, glass opacity). Frontend prefab translates this
//! to MeshStandardMaterial / MeshPhysicalMaterial parameters.

use serde::{Deserialize, Serialize};

/// Semantic material theme. Drives default colors and is used by the
/// frontend legend / Copilot context. `accent_color` overrides the
/// theme color where set.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum MaterialTheme {
    // zones
    Cold,
    Dry,
    Freezer,
    Risk,
    // status
    Ok,
    Warning,
    Critical,
    Expired,
    // misc
    Neutral,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntityMaterial {
    pub theme: MaterialTheme,
    /// Per-entity accent color (hex, e.g. "#ef4444"). Overrides the
    /// `theme`'s default color when present. Was `color` in v1 — kept
    /// under that wire name for compatibility.
    #[serde(rename = "color", skip_serializing_if = "Option::is_none")]
    pub accent_color: Option<String>,
    /// 0..1 emissive intensity baseline (animations may pulse it).
    pub emissive: f32,
    /// 0..1 overall opacity. 1.0 = solid.
    pub opacity: f32,
    /// 0..1 glass-like transparency for `StorageRoom` walls. `None` =
    /// solid material.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub glass_opacity: Option<f32>,
    /// 0..1 PBR roughness override. `None` = prefab default.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub roughness: Option<f32>,
    /// 0..1 PBR metalness override. `None` = prefab default.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metalness: Option<f32>,
}

impl EntityMaterial {
    pub fn new(theme: MaterialTheme) -> Self {
        Self {
            theme,
            accent_color: None,
            emissive: 0.1,
            opacity: 1.0,
            glass_opacity: None,
            roughness: None,
            metalness: None,
        }
    }

    pub fn with_accent(mut self, color: impl Into<String>) -> Self {
        self.accent_color = Some(color.into());
        self
    }

    pub fn with_emissive(mut self, e: f32) -> Self {
        self.emissive = e;
        self
    }

    pub fn with_glass(mut self, opacity: f32, roughness: f32, metalness: f32) -> Self {
        self.glass_opacity = Some(opacity);
        self.roughness = Some(roughness);
        self.metalness = Some(metalness);
        self
    }
}

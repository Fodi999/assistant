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
    // visual upgrades
    /// Premium / gold tint — for featured or top-performing items.
    Premium,
    /// Active highlight — selection or focus ring.
    Highlight,
    /// Muted / disabled — item is unavailable or archived.
    Disabled,
    /// New arrival — fresh blue-green tint.
    New,
    // misc
    Neutral,
}

/// How the card outline / border is rendered.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum OutlineStyle {
    /// Thin 1 px solid ring matching `accent_color`.
    Solid,
    /// Animated dashed ring — signals "needs attention".
    Dashed,
    /// Soft blurred glow ring — used for selected / hover states.
    Glow,
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
    /// Optional second gradient stop color (hex). When set, the frontend
    /// renders a linear gradient from `accent_color` → `secondary_color`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secondary_color: Option<String>,
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
    /// Border / outline style. `None` = no outline.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outline: Option<OutlineStyle>,
    /// Outline / glow ring color (hex). Falls back to `accent_color`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outline_color: Option<String>,
    /// 0..1 strength of the theme-defined tint blended on top of the
    /// base albedo. Useful to "colorize" neutral prefabs without fully
    /// overriding the accent color.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tint_strength: Option<f32>,
    /// Whether this entity casts shadows onto other surfaces.
    #[serde(default = "default_true")]
    pub cast_shadow: bool,
    /// Whether this entity receives shadows from other entities.
    #[serde(default = "default_true")]
    pub receive_shadow: bool,
}

#[inline]
fn default_true() -> bool {
    true
}

impl EntityMaterial {
    pub fn new(theme: MaterialTheme) -> Self {
        Self {
            theme,
            accent_color: None,
            secondary_color: None,
            emissive: 0.1,
            opacity: 1.0,
            glass_opacity: None,
            roughness: None,
            metalness: None,
            outline: None,
            outline_color: None,
            tint_strength: None,
            cast_shadow: true,
            receive_shadow: true,
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

    /// Add a gradient second stop: `accent_color` → `secondary_color`.
    pub fn with_gradient(mut self, secondary: impl Into<String>) -> Self {
        self.secondary_color = Some(secondary.into());
        self
    }

    /// Add a border / outline ring.
    pub fn with_outline(mut self, style: OutlineStyle, color: impl Into<String>) -> Self {
        self.outline = Some(style);
        self.outline_color = Some(color.into());
        self
    }

    /// 0..1 tint strength blended over the base albedo.
    pub fn with_tint(mut self, strength: f32) -> Self {
        self.tint_strength = Some(strength.clamp(0.0, 1.0));
        self
    }

    /// Disable shadow casting and/or receiving.
    pub fn without_shadows(mut self) -> Self {
        self.cast_shadow = false;
        self.receive_shadow = false;
        self
    }
}

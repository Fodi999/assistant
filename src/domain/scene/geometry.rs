//! Geometry descriptor — what shape an entity has.
//!
//! Backend never sends mesh vertices. It sends a *descriptor* (kind + size +
//! corner radius, etc.) that the frontend's prefab resolves into actual
//! three.js geometry. Think of it as a blueprint, not a model.

use serde::{Deserialize, Serialize};

use super::transform::Vec3;

/// High-level geometry "shape family". Frontend prefab resolves this into
/// the actual three.js mesh tree. New variants must be mirrored on the
/// frontend's `prefabRegistry` (or fall back to a generic placeholder).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum GeometryKind {
    /// A storage room / fridge / freezer (open-top box with low walls).
    StorageRoom,
    /// A flat product card (billboard with image + text).
    ProductCard,
    /// A floating zone label (text only).
    ZoneLabel,
    /// A glowing risk marker (small ground decal).
    RiskMarker,
    /// A generic container box (shelves, crates).
    Container,
    /// A flat tray (bakery, sushi).
    Tray,
    /// A bottle (oils, drinks).
    Bottle,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntityGeometry {
    pub kind: GeometryKind,
    /// Optional bounding-box size (width, height, depth). When None, the
    /// prefab uses its built-in default. Always in scene units.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<Vec3>,
    /// Wall height for `StorageRoom`. `None` → prefab default.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wall_height: Option<f32>,
    /// Wall thickness for `StorageRoom`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wall_thickness: Option<f32>,
    /// Corner radius for rounded boxes / cards.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub corner_radius: Option<f32>,
}

impl EntityGeometry {
    pub fn new(kind: GeometryKind) -> Self {
        Self {
            kind,
            size: None,
            wall_height: None,
            wall_thickness: None,
            corner_radius: None,
        }
    }

    pub fn with_size(mut self, size: Vec3) -> Self {
        self.size = Some(size);
        self
    }

    pub fn with_walls(mut self, height: f32, thickness: f32) -> Self {
        self.wall_height = Some(height);
        self.wall_thickness = Some(thickness);
        self
    }

    pub fn with_corner_radius(mut self, r: f32) -> Self {
        self.corner_radius = Some(r);
        self
    }
}

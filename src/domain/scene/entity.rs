//! `SceneEntity` — single object in the world.
//!
//! Holds the *intent* (geometry kind, material theme, prefab) plus the
//! *content* and *gameplay* metadata. Frontend resolves `prefab` to a
//! React component that renders the actual three.js mesh tree.

use serde::{Deserialize, Serialize};

use super::content::EntityContent;
use super::gameplay::{EntityDataRef, EntityGameplay};
use super::geometry::EntityGeometry;
use super::material::EntityMaterial;
use super::mechanics::EntityMechanic;
use super::prefab::PrefabKey;
use super::transform::Transform;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EntityType {
    StorageZone,
    InventoryProduct,
    Label,
    Effect,
    Marker,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SceneEntity {
    pub id: String,
    pub entity_type: EntityType,
    /// Stable prefab key — the frontend `prefabRegistry` dispatches on it.
    /// Optional during the migration: when absent, the renderer falls back
    /// to picking a prefab from `geometry.kind`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefab: Option<PrefabKey>,
    pub transform: Transform,
    pub geometry: EntityGeometry,
    pub material: EntityMaterial,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<EntityContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gameplay: Option<EntityGameplay>,
    /// Per-entity visual mechanics (pulse, glow, …). Empty by default.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub mechanics: Vec<EntityMechanic>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<EntityDataRef>,
}

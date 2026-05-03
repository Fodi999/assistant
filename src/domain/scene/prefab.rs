//! Stable prefab keys — selectors for the frontend's `prefabRegistry`.
//!
//! Every variant here MUST have a corresponding entry in
//! `blog/components/visual/prefabs/registry.tsx`. Unknown prefabs fall
//! back to a generic placeholder (so deploys can land in either order).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PrefabKey {
    // Storage-room family
    GlassFridgeRoom,
    DryStorageRoom,
    FreezerRoom,
    RiskRoom,
    // Cards
    GlassProductCard,
    // Misc
    ZoneLabel,
    RiskMarker,
    /// Generic fallback when nothing else fits.
    Placeholder,
}

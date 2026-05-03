//! Scene domain — game-like authoritative SceneState contract.
//!
//! This module is the **single source of truth** for the scene wire shape.
//! Field names use `camelCase` on the wire (see `serde(rename_all)`) and
//! mirror `blog/components/visual/sceneTypes.ts` 1:1.
//!
//! Architecture:
//! ```text
//!   SceneState
//!     ├── camera        (SceneCamera)
//!     ├── environment   (SceneEnvironment, optional)
//!     ├── hud           (SceneHud)
//!     ├── entities      Vec<SceneEntity>
//!     │     ├── prefab      (PrefabKey, optional — frontend dispatch)
//!     │     ├── transform   (position/rotation/scale)
//!     │     ├── geometry    (kind + size + walls)
//!     │     ├── material    (theme + accent + glass + emissive)
//!     │     ├── content     (title/subtitle/image/badges)
//!     │     ├── gameplay    (selectable + actions + linkedEntityId)
//!     │     ├── mechanics   Vec<EntityMechanic>  (pulse/glow/…)
//!     │     └── data        (DomainKind + entityId)
//!     ├── mechanics     Vec<EntityMechanic>      (scene-wide effects)
//!     └── selectedEntityId
//! ```
//!
//! Modes today: `inventory`. Planned: `recipes`, `dishes`, `laboratory`,
//! `menuEngineering`. Each mode has its own builder service in
//! `application/scenes/*` but shares these types.

pub mod action;
pub mod camera;
pub mod content;
pub mod entity;
pub mod environment;
pub mod gameplay;
pub mod geometry;
pub mod material;
pub mod mechanics;
pub mod prefab;
pub mod scene_state;
pub mod transform;

// Backwards-compat: keep `types` as an alias so older imports
// (`crate::domain::scene::types::*`) keep working during the refactor.
pub mod types {
    //! Deprecated flat namespace — use the per-concept modules instead.
    pub use super::action::*;
    pub use super::camera::*;
    pub use super::content::*;
    pub use super::entity::*;
    pub use super::environment::*;
    pub use super::gameplay::*;
    pub use super::geometry::*;
    pub use super::material::*;
    pub use super::mechanics::*;
    pub use super::prefab::*;
    pub use super::scene_state::*;
    pub use super::transform::*;
}

// Flat re-exports — keep `crate::domain::scene::SceneState` working.
pub use action::*;
pub use camera::*;
pub use content::*;
pub use entity::*;
pub use environment::*;
pub use gameplay::*;
pub use geometry::*;
pub use material::*;
pub use mechanics::*;
pub use prefab::*;
pub use scene_state::*;
pub use transform::*;

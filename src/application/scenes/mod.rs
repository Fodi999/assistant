//! Scene services — game-like SceneState builders per mode.
//!
//! Each mode has its own builder that pulls domain rows from the
//! relevant repository/service and produces a `SceneState`. Frontend
//! never has to know about layout, severity mapping, or HUD formatting.
//!
//! Inventory mode is split into focused submodules (DDD style):
//!   * `inventory_layout`    — zone placement & slot grid (positions, sizes)
//!   * `inventory_prefabs`   — `PrefabKey`, asset key, emoji fallback
//!   * `inventory_materials` — theme + accent + glass material configs
//!   * `inventory_mechanics` — pulse / glow per severity
//!   * `inventory_actions`   — allowed `EntityAction`s per severity
//!   * `inventory_scene_service` — orchestrator (data → entities → SceneState)

pub mod inventory_actions;
pub mod inventory_layout;
pub mod inventory_materials;
pub mod inventory_mechanics;
pub mod inventory_prefabs;
pub mod inventory_scene_service;

pub use inventory_scene_service::InventorySceneService;

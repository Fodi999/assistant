//! Scene services — port `inventorySceneBuilder.ts` and friends to Rust.
//!
//! Each scene mode gets its own builder: it pulls domain rows from the
//! relevant repository/service and produces a `SceneState`. Frontend
//! never has to know about layout, severity mapping, or HUD formatting.

pub mod inventory_scene_service;

pub use inventory_scene_service::InventorySceneService;

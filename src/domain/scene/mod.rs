//! Scene domain — game-like authoritative SceneState contract.
//!
//! This is the **mirror** of `blog/components/visual/sceneTypes.ts`.
//! Field names use `camelCase` on the wire (see `serde(rename_all)`),
//! matching the TypeScript file 1:1. The frontend's `VisualSceneRenderer`
//! consumes `SceneState` produced by services in `application/scenes/*`.
//!
//! Modes today: `inventory`. Planned: `recipes`, `dishes`, `laboratory`,
//! `menuEngineering`. Each mode has its own builder service but shares
//! these types.

pub mod types;

pub use types::*;

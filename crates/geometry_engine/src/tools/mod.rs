// ── geometry_engine::tools ────────────────────────────────────────────────────
// Native Rust implementations of all CAD sketch tools.
// Each tool returns a SketchDelta — the set of mutations to apply to the sketch.
// JS thin-layer just calls WASM, receives the delta, and applies it to sketchState.

pub mod types;
pub mod rect;
pub mod circle;
pub mod copy;
pub mod edge_extrude;
pub mod tool_state;

pub use types::{SketchDelta, WallSurface, ToolPoint, ToolEdge, ToolConstraint};
pub use tool_state::{ToolState, ActiveTool, ToolPhase, ToolClickResult};

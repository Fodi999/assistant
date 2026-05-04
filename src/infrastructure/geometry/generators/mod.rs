//! Geometry generators root.
//!
//! Two completely separate pipelines:
//!
//! `food/`         — Procedural Food Mesh (lathe / extrude / noise / shader)
//! `hard_surface/` — B-Rep-lite Hard-Surface (extrude / bevel / GeometricShell)
//! `primitives`    — Primitive shapes for Copilot spawn (square/cube/sphere/…)

pub mod food;
pub mod hard_surface;
pub mod primitives;

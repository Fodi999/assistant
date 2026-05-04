//! Precise geometry domain — Parasolid-style B-Rep in f64.
//!
//! This module is the **domain layer** for geometric precision. It knows
//! nothing about meshes, GLB buffers, or f32. All coordinates are f64.
//! The conversion to triangles/f32 happens in
//! `infrastructure::geometry::kernel::precision::tessellate`.
//!
//! ## DDD structure
//! ```text
//! domain::geometry
//!   tolerance  — Tolerance Value Object (modeling / fitting / angular)
//!   vertex     — Vertex Value Object    (f64 precise, merge-aware)
//!   face       — TopoFace Entity        (loop of vertex indices, Newell normal)
//!   shell      — GeometricShell Aggregate Root  (watertight B-Rep)
//! ```
//!
//! ## Usage pattern
//! ```ignore
//! use crate::domain::geometry::prelude::*;
//!
//! let mut shell = GeometricShell::default_precision();
//! let v0 = shell.add_vertex_raw(Vertex::new(0.0, 0.0, 0.0));
//! // … add more vertices + faces …
//! shell.validate()?;
//! // Hand off to infrastructure:
//! let part = precision::tessellate(&shell).unwrap();
//! ```

pub mod face;
pub mod shell;
pub mod tolerance;
pub mod vertex;

/// Convenience re-exports for callers that `use crate::domain::geometry::prelude::*`.
pub mod prelude {
    pub use super::face::TopoFace;
    pub use super::shell::{GeometricShell, ShellError};
    pub use super::tolerance::Tolerance;
    pub use super::vertex::Vertex;
}

//! Tessellator — domain-to-mesh bridge.
//!
//! Converts `GeometricShell` (f64, B-Rep-lite) into `MeshPart` (f32)
//! for GLB export. This is the ONLY place where the domain geometry
//! crosses into infrastructure mesh types.

pub mod shell_to_mesh;

pub use shell_to_mesh::tessellate;

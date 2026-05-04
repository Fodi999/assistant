//! Procedural 3D geometry layer for Laboratory v2.
//!
//! Layout:
//!   mesh.rs             — `Mesh` + `Material` domain types
//!   obj_exporter.rs     — serialize `Mesh` → OBJ + MTL bytes (legacy)
//!   gltf_exporter.rs    — serialize `Mesh` → single .glb (PBR, used by service)
//!   generators/
//!     food/             — Procedural Food Mesh (lathe / extrude / noise)
//!     hard_surface/     — B-Rep-lite Hard-Surface (extrude / bevel / GeometricShell)
//!   tessellator/        — GeometricShell (f64) → MeshPart (f32) bridge
//!   dispatcher.rs       — routes `object_type` string → generator
//!   kernel/             — math, profile, lathe, extrude, mesh_builder, normals

pub mod dispatcher;
pub mod generators;
pub mod gltf_exporter;
pub mod kernel;
pub mod mesh;
pub mod obj_exporter;
pub mod tessellator;

pub use dispatcher::{dispatch, dispatch_with_quality};
pub use gltf_exporter::export_glb;
pub use mesh::{Material, Mesh};
pub use obj_exporter::export_obj;

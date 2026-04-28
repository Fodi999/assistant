//! Procedural 3D geometry layer for Laboratory v2.
//!
//! Layout:
//!   mesh.rs           — `Mesh` + `Material` domain types
//!   obj_exporter.rs   — serialize `Mesh` → OBJ + MTL bytes (legacy)
//!   gltf_exporter.rs  — serialize `Mesh` → single .glb (PBR, used by service)
//!   generators/       — one file per `Product3DObjectType`
//!   dispatcher.rs     — routes `object_type` string → generator

pub mod dispatcher;
pub mod generators;
pub mod gltf_exporter;
pub mod mesh;
pub mod obj_exporter;

pub use dispatcher::dispatch;
pub use gltf_exporter::export_glb;
pub use mesh::{Material, Mesh};
pub use obj_exporter::export_obj;

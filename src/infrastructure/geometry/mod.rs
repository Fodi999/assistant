//! Procedural 3D geometry layer for Laboratory v2 PR #4.
//!
//! Layout:
//!   mesh.rs          — `Mesh` + `Material` domain types
//!   obj_exporter.rs  — serialize `Mesh` → OBJ + MTL bytes
//!   generators/      — one file per `Product3DObjectType`
//!   dispatcher.rs    — routes `object_type` string → generator

pub mod dispatcher;
pub mod generators;
pub mod mesh;
pub mod obj_exporter;

pub use dispatcher::dispatch;
pub use mesh::{Material, Mesh};
pub use obj_exporter::export_obj;

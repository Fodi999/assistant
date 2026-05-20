//! `geometry_engine` — собственный процедурный геометрический движок.
//!
//! Структура:
//!   * [`math`]     — Vec2, Vec3, Plane, Aabb, точность
//!   * [`units`]    — grid (i32↔f64), mm ↔ metres
//!   * [`profile`]  — 2D профили для lathe, CCW-ориентация, валидация
//!   * [`mesh`]     — тип Mesh, нормали, weld, OBJ/STL/GLTF экспорт
//!   * [`ops`]      — extrude, lathe, bevel, CSG
//!   * [`wasm`]     — wasm_bindgen обёртки (feature = "wasm")
//!
//! Все единицы — метры, f32. Нет внешних зависимостей (кроме serde опционально).

#![forbid(unsafe_code)]

pub mod math;
pub mod mesh;
pub mod ops;
pub mod profile;
pub mod units;

#[cfg(feature = "wasm")]
pub mod wasm;

// Flat re-exports для удобного `use geometry_engine::*`
pub use math::{Aabb, GpuReal, Plane, Real, Tolerance, Vec2, Vec3};
pub use mesh::{GeometryError, GpuMesh, Material, MaterialGroup, Mesh, MeshPart};
pub use ops::extrude::{extrude_polygon, ExtrudeOptions, Point2};
pub use ops::lathe::lathe_profile;
pub use profile::{LatheProfile, LathePoint};

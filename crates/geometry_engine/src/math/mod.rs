//! Математические примитивы геометрического движка.
//!
//! Намеренно минимален — только то, что реально нужно kernel:
//! dot, cross, length, normalized, is_finite.
//! Никаких внешних зависимостей (glam/nalgebra).
//!
//! # Numeric types
//! All geometry uses [`Real`] = f64 internally.
//! Only GPU output uses [`GpuReal`] = f32 (see [`crate::mesh::GpuMesh`]).

pub mod aabb;
pub mod plane;
pub mod real;
pub mod tolerance;
pub mod vec2;
pub mod vec3;

pub use aabb::Aabb;
pub use plane::Plane;
pub use real::{GpuReal, Real};
pub use tolerance::Tolerance;
pub use vec2::Vec2;
pub use vec3::Vec3;

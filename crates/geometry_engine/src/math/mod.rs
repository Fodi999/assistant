//! Математические примитивы геометрического движка.
//!
//! Намеренно минимален — только то, что реально нужно kernel:
//! dot, cross, length, normalized, is_finite.
//! Никаких внешних зависимостей (glam/nalgebra).

pub mod aabb;
pub mod plane;
pub mod tolerance;
pub mod vec2;
pub mod vec3;

pub use aabb::Aabb;
pub use plane::Plane;
pub use tolerance::Tolerance;
pub use vec2::Vec2;
pub use vec3::Vec3;

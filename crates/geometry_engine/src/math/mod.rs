//! Math primitives — f64 precision, no external deps.

pub mod aabb;
pub mod bbox_tree;
pub mod line;
pub mod matrix3;
pub mod matrix4;
pub mod plane;
pub mod point2;
pub mod point3;
pub mod predicates;
pub mod quaternion;
pub mod ray;
pub mod real;
pub mod segment;
pub mod tolerance;
pub mod transform;
pub mod vec2;
pub mod vec3;

pub use aabb::Aabb;
pub use bbox_tree::{BboxTree, BvhNode};
pub use line::Line;
pub use matrix3::Matrix3;
pub use matrix4::Matrix4;
pub use plane::Plane;
pub use point2::Point2;
pub use point3::Point3;
pub use quaternion::Quaternion;
pub use ray::Ray;
pub use real::{GpuReal, Real, PI, TAU};
pub use segment::Segment;
pub use tolerance::Tolerance;
pub use transform::Transform;
pub use vec2::Vec2;
pub use vec3::Vec3;

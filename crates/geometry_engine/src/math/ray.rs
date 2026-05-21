//! Ray with origin and unit direction (for picking).
#![allow(dead_code, unused_variables)]
use crate::math::{Point3, Real, Vec3};

#[derive(Debug, Clone, Copy)]
pub struct Ray { pub origin: Point3, pub direction: Vec3 }

impl Ray {
    pub fn new(origin: Point3, direction: Vec3) -> Self { Self { origin, direction } }
    /// Point along the ray at parameter `t`: `origin + t * direction`.
    #[inline]
    pub fn at(&self, t: Real) -> Point3 {
        Point3::new(
            self.origin.x + self.direction.x * t,
            self.origin.y + self.direction.y * t,
            self.origin.z + self.direction.z * t,
        )
    }

    /// Build a ray from two NDC coordinates `(nx, ny) ∈ [-1, 1]` and a
    /// camera matrix defined by `eye`, `target`, `up`, and `fov_y_rad`.
    /// Useful for converting a mouse click into a world-space pick ray.
    pub fn from_ndc(
        nx: Real, ny: Real,
        eye: Point3, target: Point3, up: Vec3,
        fov_y_rad: Real,
        aspect: Real,
    ) -> Self {
        let fwd = Vec3::new(
            target.x - eye.x,
            target.y - eye.y,
            target.z - eye.z,
        ).normalized();
        let right = fwd.cross(up).normalized();
        let up_ortho = right.cross(fwd);
        let h = (fov_y_rad * 0.5).tan();
        let direction = Vec3::new(
            fwd.x + right.x * nx * aspect * h + up_ortho.x * ny * h,
            fwd.y + right.y * nx * aspect * h + up_ortho.y * ny * h,
            fwd.z + right.z * nx * aspect * h + up_ortho.z * ny * h,
        ).normalized();
        Self { origin: eye, direction }
    }
}

//! Plane in 3D: normal + offset. Uses Real (f64).

use crate::math::{Real, Vec2, Vec3};

/// Infinite plane defined by `normal · point = offset`.
#[derive(Debug, Clone, Copy)]
pub struct Plane {
    pub normal: Vec3,
    pub offset: Real,
}

impl Plane {
    pub fn new(normal: Vec3, offset: Real) -> Self {
        Self { normal: normal.normalized(), offset }
    }

    /// Standard axis planes.
    pub const XY: Plane = Plane { normal: Vec3 { x: 0.0, y: 0.0, z: 1.0 }, offset: 0.0 };
    pub const XZ: Plane = Plane { normal: Vec3 { x: 0.0, y: 1.0, z: 0.0 }, offset: 0.0 };
    pub const YZ: Plane = Plane { normal: Vec3 { x: 1.0, y: 0.0, z: 0.0 }, offset: 0.0 };

    /// Signed distance from point to plane.
    #[inline]
    pub fn distance(&self, p: Vec3) -> Real {
        self.normal.dot(p) - self.offset
    }

    /// Project a 3D point onto the plane's (u, v) tangent coordinate system.
    pub fn project_2d(&self, p: Vec3) -> Vec2 {
        let n = self.normal;
        if n.y.abs() > 0.9 {
            Vec2::new(p.x, p.z) // XZ plane
        } else if n.z.abs() > 0.9 {
            Vec2::new(p.x, p.y) // XY plane
        } else {
            Vec2::new(p.y, p.z) // YZ plane
        }
    }
}

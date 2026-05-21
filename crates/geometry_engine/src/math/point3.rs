//! 3D point in metres (f64).
#![allow(dead_code, unused_variables)]
use crate::math::Real;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point3 { pub x: Real, pub y: Real, pub z: Real }

impl Point3 {
    pub const ORIGIN: Self = Self { x: 0.0, y: 0.0, z: 0.0 };
    pub const fn new(x: Real, y: Real, z: Real) -> Self { Self { x, y, z } }
    pub fn distance(self, other: Self) -> Real {
        let dx = self.x - other.x; let dy = self.y - other.y; let dz = self.z - other.z;
        (dx*dx + dy*dy + dz*dz).sqrt()
    }
}

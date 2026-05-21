//! Unit quaternion for 3D rotations (f64).
#![allow(dead_code, unused_variables)]
use crate::math::{Real, Vec3};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Quaternion { pub x: Real, pub y: Real, pub z: Real, pub w: Real }

impl Quaternion {
    pub const IDENTITY: Self = Self { x: 0.0, y: 0.0, z: 0.0, w: 1.0 };
    pub fn from_axis_angle(axis: Vec3, angle: Real) -> Self { todo!() }
    pub fn normalize(&self) -> Self { todo!() }
    pub fn conjugate(&self) -> Self { Self { x: -self.x, y: -self.y, z: -self.z, w: self.w } }
    pub fn rotate(&self, v: Vec3) -> Vec3 { todo!() }
}

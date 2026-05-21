//! Rigid + uniform-scale transform.
#![allow(dead_code, unused_variables)]
use crate::math::{Matrix4, Quaternion, Real, Vec3};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform {
    pub translation: Vec3,
    pub rotation: Quaternion,
    pub scale: Real,
}

impl Transform {
    pub const IDENTITY: Self = Self {
        translation: Vec3 { x: 0.0, y: 0.0, z: 0.0 },
        rotation: Quaternion::IDENTITY,
        scale: 1.0,
    };
    pub fn to_matrix(&self) -> Matrix4 { todo!() }
    pub fn inverse(&self) -> Self { todo!() }
    pub fn compose(&self, rhs: &Self) -> Self { todo!() }
}

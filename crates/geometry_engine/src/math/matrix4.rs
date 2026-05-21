//! 4×4 affine matrix (column-major, f64).
#![allow(dead_code, unused_variables)]
use crate::math::Real;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Matrix4 { pub cols: [[Real; 4]; 4] }

impl Matrix4 {
    pub const IDENTITY: Self = Self {
        cols: [[1.0,0.0,0.0,0.0],[0.0,1.0,0.0,0.0],[0.0,0.0,1.0,0.0],[0.0,0.0,0.0,1.0]]
    };
    pub fn translation(x: Real, y: Real, z: Real) -> Self { todo!() }
    pub fn rotation_x(theta: Real) -> Self { todo!() }
    pub fn rotation_y(theta: Real) -> Self { todo!() }
    pub fn rotation_z(theta: Real) -> Self { todo!() }
    pub fn scale(s: Real) -> Self { todo!() }
    pub fn inverse(&self) -> Option<Self> { todo!() }
    pub fn compose(&self, rhs: &Self) -> Self { todo!() }
}

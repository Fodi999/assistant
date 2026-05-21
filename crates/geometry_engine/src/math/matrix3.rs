//! 3×3 matrix (column-major, f64).
#![allow(dead_code, unused_variables)]
use crate::math::Real;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Matrix3 { pub cols: [[Real; 3]; 3] }

impl Matrix3 {
    pub const IDENTITY: Self = Self { cols: [[1.0,0.0,0.0],[0.0,1.0,0.0],[0.0,0.0,1.0]] };
    pub fn determinant(&self) -> Real { todo!() }
    pub fn inverse(&self) -> Option<Self> { todo!() }
    pub fn transpose(&self) -> Self { todo!() }
}

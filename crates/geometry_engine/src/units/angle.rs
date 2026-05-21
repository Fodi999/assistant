//! Angle wrapper with degrees/radians safety.
#![allow(dead_code, unused_variables, unused_imports)]
use crate::math::Real;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Angle(pub Real);

impl Angle {
    pub const fn from_radians(r: Real) -> Self { Self(r) }
    pub fn from_degrees(d: Real) -> Self { Self(d.to_radians()) }
    pub fn radians(self) -> Real { self.0 }
    pub fn degrees(self) -> Real { self.0.to_degrees() }
}


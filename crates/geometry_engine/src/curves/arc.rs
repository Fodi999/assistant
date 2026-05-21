//! Planar circular arc (sub-range of a circle).
#![allow(dead_code, unused_variables, unused_imports)]
use crate::math::{Plane, Point3, Real, Vec3};
use super::curve::Curve;

#[derive(Debug, Clone)]
pub struct ArcCurve {
    pub plane: Plane,
    pub radius: Real,
    pub start_angle: Real,
    pub end_angle: Real,
}

impl Curve for ArcCurve {
    fn domain(&self) -> (Real, Real) { (self.start_angle, self.end_angle) }
    fn evaluate(&self, t: Real) -> Point3 { todo!() }
    fn derivative(&self, t: Real) -> Vec3 { todo!() }
}


//! Bounded line segment curve.
#![allow(dead_code, unused_variables, unused_imports)]
use crate::math::{Point3, Real, Vec3};
use super::curve::Curve;

#[derive(Debug, Clone)]
pub struct LineCurve { pub start: Point3, pub end: Point3 }

impl Curve for LineCurve {
    fn domain(&self) -> (Real, Real) { (0.0, 1.0) }
    fn evaluate(&self, t: Real) -> Point3 { todo!() }
    fn derivative(&self, t: Real) -> Vec3 { todo!() }
}


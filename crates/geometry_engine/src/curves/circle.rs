//! Planar circle (full, parameter ∈ [0, 2π)).
#![allow(dead_code, unused_variables, unused_imports)]
use crate::math::{Plane, Point3, Real, Vec3};
use super::curve::Curve;

#[derive(Debug, Clone)]
pub struct CircleCurve { pub plane: Plane, pub radius: Real }

impl Curve for CircleCurve {
    fn domain(&self) -> (Real, Real) { (0.0, std::f64::consts::TAU) }
    fn evaluate(&self, t: Real) -> Point3 { todo!() }
    fn derivative(&self, t: Real) -> Vec3 { todo!() }
}


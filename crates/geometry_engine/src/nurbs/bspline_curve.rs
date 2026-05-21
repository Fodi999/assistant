//! B-Spline curve (non-rational).
#![allow(dead_code, unused_variables, unused_imports)]
use crate::math::Point3;
use super::knot_vector::KnotVector;

#[derive(Debug, Clone)]
pub struct BSplineCurve {
    pub control_points: Vec<Point3>,
    pub knots: KnotVector,
}


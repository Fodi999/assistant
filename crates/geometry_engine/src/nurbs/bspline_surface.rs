//! B-Spline surface (non-rational).
#![allow(dead_code, unused_variables, unused_imports)]
use crate::math::Point3;
use super::knot_vector::KnotVector;

#[derive(Debug, Clone)]
pub struct BSplineSurface {
    pub control_grid: Vec<Vec<Point3>>,
    pub knots_u: KnotVector,
    pub knots_v: KnotVector,
}


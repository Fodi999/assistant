//! Rational B-Spline surface (NURBS).
#![allow(dead_code, unused_variables, unused_imports)]
use super::{control_point::ControlPoint, knot_vector::KnotVector};

#[derive(Debug, Clone)]
pub struct NurbsSurface {
    pub control_grid: Vec<Vec<ControlPoint>>,
    pub knots_u: KnotVector,
    pub knots_v: KnotVector,
}


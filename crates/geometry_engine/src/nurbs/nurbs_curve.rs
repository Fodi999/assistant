//! Rational B-Spline curve (NURBS).
#![allow(dead_code, unused_variables, unused_imports)]
use super::{control_point::ControlPoint, knot_vector::KnotVector};

#[derive(Debug, Clone)]
pub struct NurbsCurve { pub control_points: Vec<ControlPoint>, pub knots: KnotVector }


//! Bezier curve of arbitrary degree.
#![allow(dead_code, unused_variables, unused_imports)]
use crate::math::Point3;

#[derive(Debug, Clone)]
pub struct BezierCurve { pub control_points: Vec<Point3> }


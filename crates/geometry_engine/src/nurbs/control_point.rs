//! Weighted control point (NURBS).
#![allow(dead_code, unused_variables, unused_imports)]
use crate::math::{Point3, Real};

#[derive(Debug, Clone, Copy)]
pub struct ControlPoint { pub point: Point3, pub weight: Real }


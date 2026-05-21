//! 2D profile (closed planar polygon in XY).
#![allow(dead_code, unused_variables, unused_imports)]
use crate::math::Point2;

#[derive(Debug, Clone)]
pub struct Profile2 { pub points: Vec<Point2>, pub closed: bool }


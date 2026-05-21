//! 3D profile (closed planar polygon embedded in 3D).
#![allow(dead_code, unused_variables, unused_imports)]
use crate::math::{Plane, Point3};

#[derive(Debug, Clone)]
pub struct Profile3 { pub plane: Plane, pub points: Vec<Point3>, pub closed: bool }


//! Sphere surface (centre + radius).
#![allow(dead_code, unused_variables, unused_imports)]
use crate::math::{Point3, Real};

#[derive(Debug, Clone)]
pub struct SphereSurface { pub centre: Point3, pub radius: Real }


//! Torus surface (centre + axis + major/minor radii).
#![allow(dead_code, unused_variables, unused_imports)]
use crate::math::{Point3, Real, Vec3};

#[derive(Debug, Clone)]
pub struct TorusSurface { pub centre: Point3, pub axis: Vec3, pub r_major: Real, pub r_minor: Real }


//! Cone surface (apex + axis + half-angle).
#![allow(dead_code, unused_variables, unused_imports)]
use crate::math::{Point3, Real, Vec3};

#[derive(Debug, Clone)]
pub struct ConeSurface { pub apex: Point3, pub axis: Vec3, pub half_angle: Real }


//! Infinite 3D line: point + unit direction.
#![allow(dead_code, unused_variables)]
use crate::math::{Point3, Real, Vec3};

#[derive(Debug, Clone, Copy)]
pub struct Line { pub origin: Point3, pub direction: Vec3 }

impl Line {
    pub fn new(origin: Point3, direction: Vec3) -> Self { Self { origin, direction } }
    pub fn point_at(&self, t: Real) -> Point3 { todo!() }
    pub fn distance_to_point(&self, p: Point3) -> Real { todo!() }
}

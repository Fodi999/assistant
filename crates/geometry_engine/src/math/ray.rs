//! Ray with origin and unit direction (for picking).
#![allow(dead_code, unused_variables)]
use crate::math::{Point3, Real, Vec3};

#[derive(Debug, Clone, Copy)]
pub struct Ray { pub origin: Point3, pub direction: Vec3 }

impl Ray {
    pub fn new(origin: Point3, direction: Vec3) -> Self { Self { origin, direction } }
    pub fn at(&self, t: Real) -> Point3 { todo!() }
}

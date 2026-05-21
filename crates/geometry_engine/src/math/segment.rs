//! Finite 3D segment between two points.
#![allow(dead_code, unused_variables)]
use crate::math::{Point3, Real};

#[derive(Debug, Clone, Copy)]
pub struct Segment { pub start: Point3, pub end: Point3 }

impl Segment {
    pub fn new(start: Point3, end: Point3) -> Self { Self { start, end } }
    pub fn length(&self) -> Real { self.start.distance(self.end) }
    pub fn midpoint(&self) -> Point3 { todo!() }
}

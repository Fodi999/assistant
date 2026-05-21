//! Robust geometric predicates.
#![allow(dead_code, unused_variables)]
use crate::math::{Point2, Real};

/// Sign of triangle area (CCW > 0, CW < 0, collinear = 0).
pub fn orient2d(a: Point2, b: Point2, c: Point2) -> Real {
    (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
}

pub fn in_circle(a: Point2, b: Point2, c: Point2, d: Point2) -> bool { todo!() }

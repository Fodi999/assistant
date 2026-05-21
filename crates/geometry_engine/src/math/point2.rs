//! 2D point in metres (f64).
#![allow(dead_code, unused_variables)]
use crate::math::Real;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point2 { pub x: Real, pub y: Real }

impl Point2 {
    pub const ORIGIN: Self = Self { x: 0.0, y: 0.0 };
    pub const fn new(x: Real, y: Real) -> Self { Self { x, y } }
    pub fn distance(self, other: Self) -> Real {
        let dx = self.x - other.x; let dy = self.y - other.y;
        (dx*dx + dy*dy).sqrt()
    }
}

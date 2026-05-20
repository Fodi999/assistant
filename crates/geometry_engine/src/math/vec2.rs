//! 2D vector — uses Real (f64) for CAD precision.

use crate::math::Real;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec2 {
    pub x: Real,
    pub y: Real,
}

impl Vec2 {
    pub const ZERO: Vec2 = Vec2 { x: 0.0, y: 0.0 };

    #[inline]
    pub const fn new(x: Real, y: Real) -> Self { Self { x, y } }

    #[inline]
    pub fn length(self) -> Real { (self.x * self.x + self.y * self.y).sqrt() }

    #[inline]
    pub fn dot(self, o: Vec2) -> Real { self.x * o.x + self.y * o.y }

    /// Z-component of the 3D cross product (scalar in 2D).
    #[inline]
    pub fn cross_z(self, o: Vec2) -> Real { self.x * o.y - self.y * o.x }

    #[inline]
    pub fn normalized(self) -> Vec2 {
        let l = self.length();
        if l > 1e-15 { Vec2::new(self.x / l, self.y / l) } else { Vec2::ZERO }
    }

    #[inline]
    pub fn is_finite(self) -> bool { self.x.is_finite() && self.y.is_finite() }
}

impl std::ops::Add for Vec2 {
    type Output = Vec2;
    fn add(self, o: Vec2) -> Vec2 { Vec2::new(self.x + o.x, self.y + o.y) }
}
impl std::ops::Sub for Vec2 {
    type Output = Vec2;
    fn sub(self, o: Vec2) -> Vec2 { Vec2::new(self.x - o.x, self.y - o.y) }
}
impl std::ops::Mul<Real> for Vec2 {
    type Output = Vec2;
    fn mul(self, s: Real) -> Vec2 { Vec2::new(self.x * s, self.y * s) }
}

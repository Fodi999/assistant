//! 2D vector (f32).

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub const ZERO: Vec2 = Vec2 { x: 0.0, y: 0.0 };

    #[inline]
    pub const fn new(x: f32, y: f32) -> Self { Self { x, y } }

    #[inline]
    pub fn length(self) -> f32 { (self.x * self.x + self.y * self.y).sqrt() }

    #[inline]
    pub fn dot(self, o: Vec2) -> f32 { self.x * o.x + self.y * o.y }

    #[inline]
    pub fn cross_z(self, o: Vec2) -> f32 { self.x * o.y - self.y * o.x }

    #[inline]
    pub fn normalized(self) -> Vec2 {
        let l = self.length();
        if l > 1e-8 { Vec2::new(self.x / l, self.y / l) } else { Vec2::ZERO }
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
impl std::ops::Mul<f32> for Vec2 {
    type Output = Vec2;
    fn mul(self, s: f32) -> Vec2 { Vec2::new(self.x * s, self.y * s) }
}

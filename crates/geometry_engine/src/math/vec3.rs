//! 3D vector (f32).

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub const ZERO: Vec3 = Vec3 { x: 0.0, y: 0.0, z: 0.0 };
    pub const UP:   Vec3 = Vec3 { x: 0.0, y: 1.0, z: 0.0 };
    pub const X:    Vec3 = Vec3 { x: 1.0, y: 0.0, z: 0.0 };
    pub const Z:    Vec3 = Vec3 { x: 0.0, y: 0.0, z: 1.0 };

    #[inline]
    pub const fn new(x: f32, y: f32, z: f32) -> Self { Self { x, y, z } }

    #[inline]
    pub fn from_array(a: [f32; 3]) -> Self { Self { x: a[0], y: a[1], z: a[2] } }

    #[inline]
    pub fn to_array(self) -> [f32; 3] { [self.x, self.y, self.z] }

    #[inline]
    pub fn dot(self, o: Vec3) -> f32 { self.x*o.x + self.y*o.y + self.z*o.z }

    #[inline]
    pub fn cross(self, o: Vec3) -> Vec3 {
        Vec3::new(
            self.y*o.z - self.z*o.y,
            self.z*o.x - self.x*o.z,
            self.x*o.y - self.y*o.x,
        )
    }

    #[inline]
    pub fn length_sq(self) -> f32 { self.dot(self) }

    #[inline]
    pub fn length(self) -> f32 { self.length_sq().sqrt() }

    #[inline]
    pub fn normalized(self) -> Vec3 {
        let l = self.length();
        if l > 1e-8 {
            Vec3::new(self.x/l, self.y/l, self.z/l)
        } else {
            Vec3::UP
        }
    }

    #[inline]
    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite() && self.z.is_finite()
    }
}

impl std::ops::Add for Vec3 {
    type Output = Vec3;
    fn add(self, o: Vec3) -> Vec3 { Vec3::new(self.x+o.x, self.y+o.y, self.z+o.z) }
}
impl std::ops::Sub for Vec3 {
    type Output = Vec3;
    fn sub(self, o: Vec3) -> Vec3 { Vec3::new(self.x-o.x, self.y-o.y, self.z-o.z) }
}
impl std::ops::Mul<f32> for Vec3 {
    type Output = Vec3;
    fn mul(self, s: f32) -> Vec3 { Vec3::new(self.x*s, self.y*s, self.z*s) }
}
impl std::ops::Neg for Vec3 {
    type Output = Vec3;
    fn neg(self) -> Vec3 { Vec3::new(-self.x, -self.y, -self.z) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cross_x_y_is_z() {
        let c = Vec3::X.cross(Vec3::new(0.0, 1.0, 0.0));
        assert!((c.z - 1.0).abs() < 1e-6);
    }

    #[test]
    fn normalized_zero_falls_back_to_up() {
        assert_eq!(Vec3::ZERO.normalized(), Vec3::UP);
    }
}

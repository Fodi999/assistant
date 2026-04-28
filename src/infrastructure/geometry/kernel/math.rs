//! Minimal vector math for the geometry kernel.
//!
//! Intentionally tiny — only the ops that the kernel itself needs
//! (`dot`, `cross`, `length`, `normalized`, `is_finite`). We deliberately
//! avoid pulling `glam` or `nalgebra` so the kernel stays a thin, audit-able
//! piece of code with no external surface.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub const ZERO: Vec2 = Vec2 { x: 0.0, y: 0.0 };

    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn length(self) -> f32 {
        (self.x * self.x + self.y * self.y).sqrt()
    }

    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub const ZERO: Vec3 = Vec3 { x: 0.0, y: 0.0, z: 0.0 };
    pub const UP: Vec3 = Vec3 { x: 0.0, y: 1.0, z: 0.0 };

    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub fn from_array(a: [f32; 3]) -> Self {
        Self { x: a[0], y: a[1], z: a[2] }
    }

    pub fn to_array(self) -> [f32; 3] {
        [self.x, self.y, self.z]
    }

    pub fn dot(self, other: Vec3) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    pub fn cross(self, other: Vec3) -> Vec3 {
        Vec3 {
            x: self.y * other.z - self.z * other.y,
            y: self.z * other.x - self.x * other.z,
            z: self.x * other.y - self.y * other.x,
        }
    }

    pub fn length_squared(self) -> f32 {
        self.dot(self)
    }

    pub fn length(self) -> f32 {
        self.length_squared().sqrt()
    }

    /// Returns a unit-length copy of `self`. If `self` is the zero vector
    /// (or near-zero), returns `Vec3::UP` to keep downstream shading sane.
    pub fn normalized(self) -> Vec3 {
        let len = self.length();
        if len > 1e-8 {
            Vec3 {
                x: self.x / len,
                y: self.y / len,
                z: self.z / len,
            }
        } else {
            Vec3::UP
        }
    }

    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite() && self.z.is_finite()
    }
}

impl std::ops::Add for Vec3 {
    type Output = Vec3;
    fn add(self, other: Vec3) -> Vec3 {
        Vec3::new(self.x + other.x, self.y + other.y, self.z + other.z)
    }
}

impl std::ops::Sub for Vec3 {
    type Output = Vec3;
    fn sub(self, other: Vec3) -> Vec3 {
        Vec3::new(self.x - other.x, self.y - other.y, self.z - other.z)
    }
}

impl std::ops::Mul<f32> for Vec3 {
    type Output = Vec3;
    fn mul(self, s: f32) -> Vec3 {
        Vec3::new(self.x * s, self.y * s, self.z * s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dot_orthogonal_is_zero() {
        let a = Vec3::new(1.0, 0.0, 0.0);
        let b = Vec3::new(0.0, 1.0, 0.0);
        assert!(a.dot(b).abs() < 1e-6);
    }

    #[test]
    fn cross_x_y_is_z() {
        let a = Vec3::new(1.0, 0.0, 0.0);
        let b = Vec3::new(0.0, 1.0, 0.0);
        let c = a.cross(b);
        assert!((c.x).abs() < 1e-6);
        assert!((c.y).abs() < 1e-6);
        assert!((c.z - 1.0).abs() < 1e-6);
    }

    #[test]
    fn normalized_unit_length() {
        let v = Vec3::new(3.0, 0.0, 4.0).normalized();
        assert!((v.length() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn normalized_zero_vector_falls_back_to_up() {
        let v = Vec3::ZERO.normalized();
        assert_eq!(v, Vec3::UP);
    }

    #[test]
    fn is_finite_rejects_nan() {
        let v = Vec3::new(1.0, f32::NAN, 0.0);
        assert!(!v.is_finite());
    }

    #[test]
    fn add_sub_mul_compose() {
        let a = Vec3::new(1.0, 2.0, 3.0);
        let b = Vec3::new(4.0, 5.0, 6.0);
        let s = (a + b) - Vec3::new(2.0, 2.0, 2.0);
        assert_eq!(s, Vec3::new(3.0, 5.0, 7.0));
        assert_eq!(a * 2.0, Vec3::new(2.0, 4.0, 6.0));
    }
}

//! Axis-aligned bounding box — Real (f64), 3D.

use crate::math::{Real, Vec3};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}

impl Aabb {
    /// Construct from raw arrays.
    pub fn from_arrays(min: [Real; 3], max: [Real; 3]) -> Self {
        Self {
            min: Vec3::from_array(min),
            max: Vec3::from_array(max),
        }
    }

    /// Does point `p` lie strictly inside this AABB?
    #[inline]
    pub fn contains(&self, p: Vec3) -> bool {
        p.x > self.min.x && p.x < self.max.x &&
        p.y > self.min.y && p.y < self.max.y &&
        p.z > self.min.z && p.z < self.max.z
    }

    /// Does point (array form) lie inside this AABB?
    #[inline]
    pub fn contains_arr(&self, p: [Real; 3]) -> bool {
        self.contains(Vec3::from_array(p))
    }

    /// Center of the AABB.
    pub fn center(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    /// Half-extents.
    pub fn half_extents(&self) -> Vec3 {
        (self.max - self.min) * 0.5
    }
}

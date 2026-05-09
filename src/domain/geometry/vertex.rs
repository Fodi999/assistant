//! Precise 3D vertex — domain Value Object.
//!
//! Coordinates are stored in **f64** throughout the domain layer. The
//! conversion to `f32` happens only at the infrastructure boundary
//! (tessellator → GLB buffer).
//!
//! ## Equality
//! Structural (bitwise) equality is intentionally **not** derived: two
//! vertices at different coordinates are never equal even if they happen to
//! produce the same f32 after truncation. Use
//! [`Vertex::coincident_with`] + [`Tolerance`] for merge decisions.
//!
//! ## DDD role
//! `Vertex` is a **Value Object**: no identity key, immutable, compared by
//! value. Topological entities reference vertices by index in a `Shell`.

use serde::{Deserialize, Serialize};

use super::tolerance::Tolerance;

/// An immutable precise 3-D point (f64).
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Vertex {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Vertex {
    pub const ORIGIN: Self = Self {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };

    #[inline]
    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    /// Euclidean distance to `other`.
    #[inline]
    pub fn distance_to(self, other: Vertex) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }

    /// Returns `true` if `self` and `other` are within the modeling
    /// tolerance (should be merged / treated as the same point).
    #[inline]
    pub fn coincident_with(self, other: Vertex, tol: Tolerance) -> bool {
        tol.vertices_coincident(self.distance_to(other))
    }

    /// Unit-length direction vector from `self` to `other`.
    /// Returns `None` if the distance is below the angular tolerance
    /// (degenerate / zero-length edge).
    pub fn direction_to(self, other: Vertex, tol: Tolerance) -> Option<[f64; 3]> {
        let dx = other.x - self.x;
        let dy = other.y - self.y;
        let dz = other.z - self.z;
        let len = (dx * dx + dy * dy + dz * dz).sqrt();
        if tol.is_degenerate_length(len) {
            None
        } else {
            Some([dx / len, dy / len, dz / len])
        }
    }

    /// Linear interpolation: `self + t * (other - self)`, `t ∈ [0, 1]`.
    #[inline]
    pub fn lerp(self, other: Vertex, t: f64) -> Vertex {
        Vertex {
            x: self.x + t * (other.x - self.x),
            y: self.y + t * (other.y - self.y),
            z: self.z + t * (other.z - self.z),
        }
    }

    /// Downcast to `[f32; 3]` for the infrastructure (GLB buffer).
    ///
    /// Precision is intentionally lost here — this is the only place
    /// where f64 → f32 truncation occurs in the geometry pipeline.
    #[inline]
    pub fn to_f32(self) -> [f32; 3] {
        [self.x as f32, self.y as f32, self.z as f32]
    }

    /// Upgrade from `[f32; 3]` (e.g. reading legacy mesh data).
    #[inline]
    pub fn from_f32(v: [f32; 3]) -> Self {
        Self {
            x: v[0] as f64,
            y: v[1] as f64,
            z: v[2] as f64,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Arithmetic helpers (no trait implementations to keep the VO lightweight)
// ─────────────────────────────────────────────────────────────────────────────

impl Vertex {
    /// Cross product of vectors (self→a) × (self→b). Used for face normals.
    pub fn cross_to(self, a: Vertex, b: Vertex) -> [f64; 3] {
        let u = [a.x - self.x, a.y - self.y, a.z - self.z];
        let v = [b.x - self.x, b.y - self.y, b.z - self.z];
        [
            u[1] * v[2] - u[2] * v[1],
            u[2] * v[0] - u[0] * v[2],
            u[0] * v[1] - u[1] * v[0],
        ]
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coincident_within_modeling_tolerance() {
        let a = Vertex::new(0.0, 0.0, 0.0);
        let b = Vertex::new(5e-8, 0.0, 0.0);
        assert!(a.coincident_with(b, Tolerance::DEFAULT));
        let c = Vertex::new(2e-7, 0.0, 0.0);
        assert!(!a.coincident_with(c, Tolerance::DEFAULT));
    }

    #[test]
    fn direction_to_returns_none_for_degenerate_edge() {
        let a = Vertex::new(0.0, 0.0, 0.0);
        let b = Vertex::new(1e-10, 0.0, 0.0); // below angular tol
        assert!(a.direction_to(b, Tolerance::DEFAULT).is_none());
    }

    #[test]
    fn direction_to_is_unit_length() {
        let a = Vertex::new(0.0, 0.0, 0.0);
        let b = Vertex::new(3.0, 4.0, 0.0); // distance = 5
        let d = a.direction_to(b, Tolerance::DEFAULT).unwrap();
        let len = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
        assert!((len - 1.0).abs() < 1e-12);
    }

    #[test]
    fn lerp_midpoint() {
        let a = Vertex::new(0.0, 0.0, 0.0);
        let b = Vertex::new(2.0, 4.0, 6.0);
        let mid = a.lerp(b, 0.5);
        assert!((mid.x - 1.0).abs() < 1e-12);
        assert!((mid.y - 2.0).abs() < 1e-12);
        assert!((mid.z - 3.0).abs() < 1e-12);
    }

    #[test]
    fn f64_to_f32_roundtrip_preserves_sign() {
        let v = Vertex::new(-0.012345, 0.067890, 0.100001);
        let f = v.to_f32();
        assert!(f[0] < 0.0);
        assert!(f[1] > 0.0);
        assert!(f[2] > 0.0);
    }
}

//! Geometric surfaces — parametric surface trait + primitive implementations.
//!
//! ## Parasolid correspondence
//!
//! | This crate       | Parasolid          | Status      |
//! |------------------|--------------------|-------------|
//! | `Plane`          | plane              | ✅ done     |
//! | `CylindricalSurface` | cylinder       | ✅ done     |
//! | `SphericalSurface`   | sphere         | ✅ done     |
//! | `NurbsSurface`   | bs3_surface        | 🔜 planned  |
//!
//! ## Design
//! All surfaces implement `ParametricSurface` with `(u, v) ∈ [0,1]²`.
//! This is the minimum needed for:
//!   - Face geometry in B-Rep
//!   - Normal evaluation for shading / offsetting
//!   - Future UV unwrapping for textures

use crate::domain::geometry::vertex::Vertex;

// ─────────────────────────────────────────────────────────────────────────────
// Trait
// ─────────────────────────────────────────────────────────────────────────────

/// Minimal parametric surface contract.
///
/// Both `u` and `v` are normalised to `[0, 1]`.
pub trait ParametricSurface: Send + Sync {
    /// Point on the surface at `(u, v)`.
    fn point_at(&self, u: f64, v: f64) -> Vertex;

    /// Outward-pointing surface normal at `(u, v)` (not necessarily unit length).
    fn normal_at(&self, u: f64, v: f64) -> [f64; 3];

    /// Tessellate into a `(2*n_u * 2*n_v)` quad mesh (returned as flat vertex list
    /// with row-major ordering). Each quad is two triangles.
    fn tessellate_grid(&self, n_u: usize, n_v: usize) -> Vec<Vertex> {
        let nu = n_u.max(1);
        let nv = n_v.max(1);
        let mut pts = Vec::with_capacity(nu * nv * 6);
        for i in 0..nu {
            for j in 0..nv {
                let u0 = i as f64 / nu as f64;
                let u1 = (i + 1) as f64 / nu as f64;
                let v0 = j as f64 / nv as f64;
                let v1 = (j + 1) as f64 / nv as f64;
                let p00 = self.point_at(u0, v0);
                let p10 = self.point_at(u1, v0);
                let p01 = self.point_at(u0, v1);
                let p11 = self.point_at(u1, v1);
                // Triangle 1: p00, p10, p11
                pts.push(p00);
                pts.push(p10);
                pts.push(p11);
                // Triangle 2: p00, p11, p01
                pts.push(p00);
                pts.push(p11);
                pts.push(p01);
            }
        }
        pts
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Plane
// ─────────────────────────────────────────────────────────────────────────────

/// Finite planar patch defined by an `origin` corner, two edge vectors `u_dir`
/// and `v_dir`. The normal is `u_dir × v_dir`.
pub struct Plane {
    pub origin: Vertex,
    /// Direction and magnitude of the U edge.
    pub u_dir: [f64; 3],
    /// Direction and magnitude of the V edge.
    pub v_dir: [f64; 3],
}

impl Plane {
    /// Axis-aligned XZ plane (Y=0) from `(-half, 0, -half)` to `(half, 0, half)`.
    pub fn horizontal(half: f64) -> Self {
        Self {
            origin: Vertex::new(-half, 0.0, -half),
            u_dir: [2.0 * half, 0.0, 0.0],
            v_dir: [0.0, 0.0, 2.0 * half],
        }
    }
}

impl ParametricSurface for Plane {
    fn point_at(&self, u: f64, v: f64) -> Vertex {
        Vertex::new(
            self.origin.x + u * self.u_dir[0] + v * self.v_dir[0],
            self.origin.y + u * self.u_dir[1] + v * self.v_dir[1],
            self.origin.z + u * self.u_dir[2] + v * self.v_dir[2],
        )
    }

    fn normal_at(&self, _u: f64, _v: f64) -> [f64; 3] {
        // n = u_dir × v_dir
        let [ux, uy, uz] = self.u_dir;
        let [vx, vy, vz] = self.v_dir;
        [uy * vz - uz * vy, uz * vx - ux * vz, ux * vy - uy * vx]
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Cylindrical surface
// ─────────────────────────────────────────────────────────────────────────────

/// Open cylindrical surface: axis along Y, centred at `centre`.
///
/// - `u ∈ [0,1]` sweeps the angle from `angle_start` to `angle_end` (radians).
/// - `v ∈ [0,1]` sweeps from `y_bottom` to `y_top`.
pub struct CylindricalSurface {
    pub centre: Vertex,
    pub radius: f64,
    pub y_bottom: f64,
    pub y_top: f64,
    pub angle_start: f64,
    pub angle_end: f64,
}

impl CylindricalSurface {
    /// Full cylinder (360°).
    pub fn full(centre: Vertex, radius: f64, y_bottom: f64, y_top: f64) -> Self {
        Self {
            centre,
            radius,
            y_bottom,
            y_top,
            angle_start: 0.0,
            angle_end: std::f64::consts::TAU,
        }
    }
}

impl ParametricSurface for CylindricalSurface {
    fn point_at(&self, u: f64, v: f64) -> Vertex {
        let a = self.angle_start + u * (self.angle_end - self.angle_start);
        let y = self.y_bottom + v * (self.y_top - self.y_bottom);
        Vertex::new(
            self.centre.x + self.radius * a.cos(),
            self.centre.y + y,
            self.centre.z + self.radius * a.sin(),
        )
    }

    fn normal_at(&self, u: f64, _v: f64) -> [f64; 3] {
        let a = self.angle_start + u * (self.angle_end - self.angle_start);
        [a.cos(), 0.0, a.sin()]
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Spherical surface
// ─────────────────────────────────────────────────────────────────────────────

/// Full unit sphere scaled to `radius`, centred at `centre`.
///
/// - `u ∈ [0,1]` → longitude 0..2π
/// - `v ∈ [0,1]` → latitude −π/2..+π/2 (south pole → north pole)
pub struct SphericalSurface {
    pub centre: Vertex,
    pub radius: f64,
}

impl SphericalSurface {
    pub fn new(centre: Vertex, radius: f64) -> Self {
        Self { centre, radius }
    }
}

impl ParametricSurface for SphericalSurface {
    fn point_at(&self, u: f64, v: f64) -> Vertex {
        let lon = u * std::f64::consts::TAU;
        let lat = (v - 0.5) * std::f64::consts::PI;
        Vertex::new(
            self.centre.x + self.radius * lat.cos() * lon.cos(),
            self.centre.y + self.radius * lat.sin(),
            self.centre.z + self.radius * lat.cos() * lon.sin(),
        )
    }

    fn normal_at(&self, u: f64, v: f64) -> [f64; 3] {
        let lon = u * std::f64::consts::TAU;
        let lat = (v - 0.5) * std::f64::consts::PI;
        [lat.cos() * lon.cos(), lat.sin(), lat.cos() * lon.sin()]
    }
}

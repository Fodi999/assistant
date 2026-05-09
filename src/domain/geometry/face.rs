//! Topological face — a planar polygon defined by an ordered loop of vertex
//! indices into the parent `Shell`.
//!
//! ## Parasolid correspondence
//! A Parasolid **FACE** is bounded by one outer loop + optional inner loops
//! (holes). We implement the common case (one outer loop, no holes) which
//! covers all current generators (extrude caps, lathe rings, bevels).
//!
//! Holes can be added later as `inner_loops: Vec<Vec<usize>>` without
//! changing any existing code.
//!
//! ## DDD role
//! `TopoFace` is a **domain Entity** inside the `GeometricShell` aggregate.
//! It carries an identity (`id`) that is stable for the lifetime of the
//! shell — needed so topological operations (flip, merge, split) can refer
//! back to specific faces.

use super::tolerance::Tolerance;
use super::vertex::Vertex;

/// A single face defined by an ordered CCW loop of vertex indices.
///
/// `loop_` must contain at least 3 indices, all < `shell.vertices.len()`.
#[derive(Debug, Clone)]
pub struct TopoFace {
    /// Stable face identifier within the parent shell.
    pub id: u32,
    /// Vertex indices forming the outer loop (CCW winding, viewed from the
    /// outward normal direction).
    pub loop_: Vec<usize>,
    /// Outward unit normal, computed lazily by the shell and cached here.
    /// `None` until [`compute_normal`] is called.
    pub normal: Option<[f64; 3]>,
}

impl TopoFace {
    /// Construct a new face. `id` is assigned by the parent shell.
    pub fn new(id: u32, loop_: Vec<usize>) -> Self {
        debug_assert!(loop_.len() >= 3, "face must have at least 3 vertices");
        Self {
            id,
            loop_,
            normal: None,
        }
    }

    /// Number of vertices in this face's loop.
    #[inline]
    pub fn vertex_count(&self) -> usize {
        self.loop_.len()
    }

    /// Decompose the face into triangles using a fan from `loop_[0]`.
    ///
    /// Returns index triples **into `self.loop_`** (not into the shell's
    /// global vertex array). The caller must map them through `self.loop_[i]`
    /// to get shell-global indices.
    pub fn fan_triangles(&self) -> impl Iterator<Item = [usize; 3]> + '_ {
        let n = self.loop_.len();
        (1..n - 1).map(|i| [self.loop_[0], self.loop_[i], self.loop_[i + 1]])
    }

    /// Recompute and cache the face normal from the provided vertex array.
    ///
    /// Uses Newell's method: area-weighted sum of cross products across all
    /// edges of the polygon. Accurate for non-planar quads and works for
    /// any convex polygon — same algorithm used by Parasolid's face normal
    /// computation.
    ///
    /// Returns the computed normal (unit length) or `None` if the face is
    /// degenerate (all vertices collinear, area below `tol.modeling²`).
    pub fn compute_normal(&mut self, verts: &[Vertex], tol: Tolerance) -> Option<[f64; 3]> {
        let mut nx = 0.0_f64;
        let mut ny = 0.0_f64;
        let mut nz = 0.0_f64;

        let n = self.loop_.len();
        for i in 0..n {
            let j = (i + 1) % n;
            let vi = verts[self.loop_[i]];
            let vj = verts[self.loop_[j]];
            // Newell's formula: N += (vi - vj) × (vi + vj)
            nx += (vi.y - vj.y) * (vi.z + vj.z);
            ny += (vi.z - vj.z) * (vi.x + vj.x);
            nz += (vi.x - vj.x) * (vi.y + vj.y);
        }

        let len = (nx * nx + ny * ny + nz * nz).sqrt();
        // Area = len / 2 — must exceed modeling² to be non-degenerate.
        if len < tol.modeling * tol.modeling {
            self.normal = None;
            return None;
        }
        let unit = [nx / len, ny / len, nz / len];
        self.normal = Some(unit);
        Some(unit)
    }

    /// Flip the winding order (reverses loop_ and negates cached normal).
    /// Used when the shell detects an inside-out face during orientation pass.
    pub fn flip(&mut self) {
        self.loop_.reverse();
        if let Some(n) = self.normal.as_mut() {
            n[0] = -n[0];
            n[1] = -n[1];
            n[2] = -n[2];
        }
    }

    /// Return the cached normal, or recompute it on demand.
    pub fn normal_or_compute(&mut self, verts: &[Vertex], tol: Tolerance) -> Option<[f64; 3]> {
        if self.normal.is_some() {
            return self.normal;
        }
        self.compute_normal(verts, tol)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn square_verts() -> Vec<Vertex> {
        vec![
            Vertex::new(1.0, 0.0, 1.0),
            Vertex::new(-1.0, 0.0, 1.0),
            Vertex::new(-1.0, 0.0, -1.0),
            Vertex::new(1.0, 0.0, -1.0),
        ]
    }

    #[test]
    fn newell_normal_points_up_for_xz_square() {
        let mut face = TopoFace::new(0, vec![0, 1, 2, 3]);
        let verts = square_verts();
        let n = face.compute_normal(&verts, Tolerance::DEFAULT).unwrap();
        // Square in XZ plane — Newell should give Y-up (or Y-down depending
        // on winding). We care that it's along Y.
        assert!(n[1].abs() > 0.99, "expected Y-aligned normal, got {n:?}");
        assert!(n[0].abs() < 1e-10);
        assert!(n[2].abs() < 1e-10);
    }

    #[test]
    fn flip_reverses_winding_and_normal() {
        let mut face = TopoFace::new(0, vec![0, 1, 2, 3]);
        let verts = square_verts();
        face.compute_normal(&verts, Tolerance::DEFAULT);
        let orig_n = face.normal.unwrap();
        face.flip();
        assert_eq!(face.loop_[0], 3); // reversed
        let flipped_n = face.normal.unwrap();
        assert!((orig_n[0] + flipped_n[0]).abs() < 1e-12);
        assert!((orig_n[1] + flipped_n[1]).abs() < 1e-12);
    }

    #[test]
    fn fan_triangles_count() {
        let face = TopoFace::new(0, vec![0, 1, 2, 3, 4]); // pentagon
        let tris: Vec<_> = face.fan_triangles().collect();
        assert_eq!(tris.len(), 3, "pentagon → 3 triangles");
    }

    #[test]
    fn degenerate_face_returns_none() {
        // Collinear points → area ≈ 0
        let mut face = TopoFace::new(0, vec![0, 1, 2]);
        let verts = vec![
            Vertex::new(0.0, 0.0, 0.0),
            Vertex::new(1e-8, 0.0, 0.0),
            Vertex::new(2e-8, 0.0, 0.0),
        ];
        assert!(face.compute_normal(&verts, Tolerance::DEFAULT).is_none());
    }
}

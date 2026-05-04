//! `GeometricShell` — the root aggregate for precise B-Rep geometry.
//!
//! ## Parasolid correspondence
//! A Parasolid **SHELL** is a connected set of faces that together form a
//! closed (watertight) or open surface. This is the aggregate root for
//! our domain geometry:
//!
//! ```text
//! GeometricShell          ← aggregate root
//!   vertices: Vec<Vertex> ← Value Objects (f64)
//!   faces:    Vec<TopoFace> ← Entities (id-stable)
//!   tolerance: Tolerance  ← precision contract
//! ```
//!
//! ## Invariants enforced
//! 1. Every face index is in-bounds.
//! 2. No degenerate faces (area < modeling²).
//! 3. All vertex coordinates are finite.
//! 4. Optional watertight check: every edge appears exactly twice
//!    (once per adjacent face) across the whole shell.
//!
//! ## DDD role
//! `GeometricShell` is the **Aggregate Root** for the geometry subdomain.
//! No external code mutates faces/vertices directly; all changes go through
//! shell methods that keep invariants consistent.

use super::face::TopoFace;
use super::tolerance::Tolerance;
use super::vertex::Vertex;

// ─────────────────────────────────────────────────────────────────────────────
// Error type
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShellError {
    /// A face index references a vertex that doesn't exist.
    OutOfBoundsIndex { face_id: u32, index: usize, vertex_count: usize },
    /// A vertex has a non-finite coordinate.
    NonFiniteVertex(usize),
    /// A face's geometry is degenerate (zero area).
    DegenerateFace(u32),
    /// Watertight check failed: edge appears an unexpected number of times.
    OpenEdge { v0: usize, v1: usize, count: usize },
    /// Shell has no faces.
    Empty,
}

impl std::fmt::Display for ShellError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShellError::OutOfBoundsIndex { face_id, index, vertex_count } =>
                write!(f, "face {face_id}: vertex index {index} >= vertex count {vertex_count}"),
            ShellError::NonFiniteVertex(i) =>
                write!(f, "vertex {i} has non-finite coordinate"),
            ShellError::DegenerateFace(id) =>
                write!(f, "face {id} is degenerate (zero area)"),
            ShellError::OpenEdge { v0, v1, count } =>
                write!(f, "open edge ({v0}, {v1}) appears {count} times (expected 2)"),
            ShellError::Empty =>
                write!(f, "shell has no faces"),
        }
    }
}

impl std::error::Error for ShellError {}

// ─────────────────────────────────────────────────────────────────────────────
// Aggregate Root
// ─────────────────────────────────────────────────────────────────────────────

/// A connected set of faces in precise f64 space, governed by a `Tolerance`.
pub struct GeometricShell {
    vertices:  Vec<Vertex>,
    faces:     Vec<TopoFace>,
    tolerance: Tolerance,
    next_face_id: u32,
}

impl GeometricShell {
    // ── Construction ────────────────────────────────────────────────────────

    /// Create an empty shell with the given precision contract.
    pub fn new(tolerance: Tolerance) -> Self {
        Self {
            vertices: Vec::new(),
            faces: Vec::new(),
            tolerance,
            next_face_id: 0,
        }
    }

    /// Create a shell with Parasolid default tolerances.
    pub fn default_precision() -> Self {
        Self::new(Tolerance::DEFAULT)
    }

    // ── Vertex management ───────────────────────────────────────────────────

    /// Push a vertex and return its index.
    ///
    /// If a coincident vertex (within `modeling` tolerance) already exists,
    /// returns its index without pushing a duplicate — Parasolid-style
    /// vertex merge.
    pub fn add_vertex_merged(&mut self, v: Vertex) -> usize {
        for (i, existing) in self.vertices.iter().enumerate() {
            if existing.coincident_with(v, self.tolerance) {
                return i;
            }
        }
        self.vertices.push(v);
        self.vertices.len() - 1
    }

    /// Push a vertex unconditionally (no merge). Use when you know there
    /// is no coincident vertex (e.g. building a revolved ring).
    pub fn add_vertex_raw(&mut self, v: Vertex) -> usize {
        self.vertices.push(v);
        self.vertices.len() - 1
    }

    /// Bulk-push a slice of vertices without merging. Returns the starting
    /// index of the first pushed vertex.
    pub fn add_vertices_raw(&mut self, vs: &[Vertex]) -> usize {
        let start = self.vertices.len();
        self.vertices.extend_from_slice(vs);
        start
    }

    // ── Face management ─────────────────────────────────────────────────────

    /// Add a face defined by a loop of vertex indices.
    /// Returns the face's stable `id`.
    ///
    /// The normal is computed immediately. Returns `ShellError::DegenerateFace`
    /// if the face has zero area.
    pub fn add_face(&mut self, loop_: Vec<usize>) -> Result<u32, ShellError> {
        let id = self.next_face_id;
        self.next_face_id += 1;

        let mut face = TopoFace::new(id, loop_);

        // Validate index bounds.
        for &idx in &face.loop_ {
            if idx >= self.vertices.len() {
                return Err(ShellError::OutOfBoundsIndex {
                    face_id: id,
                    index: idx,
                    vertex_count: self.vertices.len(),
                });
            }
        }

        // Compute normal — reject degenerate faces.
        if face.compute_normal(&self.vertices, self.tolerance).is_none() {
            return Err(ShellError::DegenerateFace(id));
        }

        self.faces.push(face);
        Ok(id)
    }

    /// Flip all faces so their normals point outward (consistent orientation).
    ///
    /// Uses the sign of the Y-component of each normal as a heuristic:
    /// if the average Y is negative the shell is likely inside-out (common
    /// after building a bowl or concave surface). For production use, replace
    /// with a proper ray-casting orientation test.
    pub fn orient_outward(&mut self) {
        let avg_ny: f64 = self
            .faces
            .iter()
            .filter_map(|f| f.normal.map(|n| n[1]))
            .sum::<f64>()
            / self.faces.len().max(1) as f64;

        if avg_ny < 0.0 {
            for face in &mut self.faces {
                face.flip();
            }
        }
    }

    // ── Validation ──────────────────────────────────────────────────────────

    /// Validate all structural invariants. Cheap — O(V + F).
    pub fn validate(&self) -> Result<(), ShellError> {
        if self.faces.is_empty() {
            return Err(ShellError::Empty);
        }

        // Finite vertices.
        for (i, v) in self.vertices.iter().enumerate() {
            if !v.x.is_finite() || !v.y.is_finite() || !v.z.is_finite() {
                return Err(ShellError::NonFiniteVertex(i));
            }
        }

        // Index bounds + degenerate faces.
        let vc = self.vertices.len();
        for face in &self.faces {
            for &idx in &face.loop_ {
                if idx >= vc {
                    return Err(ShellError::OutOfBoundsIndex {
                        face_id: face.id,
                        index: idx,
                        vertex_count: vc,
                    });
                }
            }
            if face.normal.is_none() {
                return Err(ShellError::DegenerateFace(face.id));
            }
        }

        Ok(())
    }

    /// Watertight check: every directed edge (v_i → v_j) must appear
    /// exactly once, and its reverse (v_j → v_i) must also appear once.
    ///
    /// Call *after* `validate()`. Returns the first offending edge, or `Ok`
    /// if the shell is manifold and watertight.
    pub fn check_watertight(&self) -> Result<(), ShellError> {
        use std::collections::HashMap;

        // Count directed edges.
        let mut edge_count: HashMap<(usize, usize), usize> = HashMap::new();
        for face in &self.faces {
            let n = face.loop_.len();
            for i in 0..n {
                let a = face.loop_[i];
                let b = face.loop_[(i + 1) % n];
                *edge_count.entry((a, b)).or_insert(0) += 1;
            }
        }

        // Each directed edge should appear exactly once; its reverse once.
        for (&(v0, v1), &count) in &edge_count {
            if count != 1 {
                return Err(ShellError::OpenEdge { v0, v1, count });
            }
            // Reverse must exist exactly once.
            let rev_count = edge_count.get(&(v1, v0)).copied().unwrap_or(0);
            if rev_count != 1 {
                return Err(ShellError::OpenEdge { v0: v1, v1: v0, count: rev_count });
            }
        }

        Ok(())
    }

    // ── Read access ─────────────────────────────────────────────────────────

    pub fn vertices(&self) -> &[Vertex]    { &self.vertices }
    pub fn faces(&self)    -> &[TopoFace]  { &self.faces    }
    pub fn tolerance(&self) -> Tolerance   { self.tolerance }
    pub fn vertex_count(&self) -> usize    { self.vertices.len() }
    pub fn face_count(&self) -> usize      { self.faces.len() }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn unit_cube_shell() -> GeometricShell {
        let mut s = GeometricShell::default_precision();

        // 8 cube vertices.
        let indices: Vec<usize> = [
            [-0.5_f64, -0.5, -0.5], [ 0.5, -0.5, -0.5],
            [ 0.5,  0.5, -0.5], [-0.5,  0.5, -0.5],
            [-0.5, -0.5,  0.5], [ 0.5, -0.5,  0.5],
            [ 0.5,  0.5,  0.5], [-0.5,  0.5,  0.5],
        ]
        .iter()
        .map(|&[x, y, z]| s.add_vertex_raw(Vertex::new(x, y, z)))
        .collect();

        // 6 faces (CCW from outside).
        let faces = [
            [0, 3, 2, 1], // back   -Z
            [4, 5, 6, 7], // front  +Z
            [0, 1, 5, 4], // bottom -Y
            [3, 7, 6, 2], // top    +Y
            [0, 4, 7, 3], // left   -X
            [1, 2, 6, 5], // right  +X
        ];
        for f in faces {
            s.add_face(f.to_vec()).unwrap();
        }
        s
    }

    #[test]
    fn cube_validates_ok() {
        let s = unit_cube_shell();
        s.validate().unwrap();
    }

    #[test]
    fn cube_is_watertight() {
        let s = unit_cube_shell();
        s.check_watertight().unwrap();
    }

    #[test]
    fn vertex_merge_deduplicates() {
        let mut s = GeometricShell::default_precision();
        let i0 = s.add_vertex_merged(Vertex::new(0.0, 0.0, 0.0));
        // Coincident within modeling tolerance → same index.
        let i1 = s.add_vertex_merged(Vertex::new(5e-8, 0.0, 0.0));
        assert_eq!(i0, i1);
        // Outside tolerance → new vertex.
        let i2 = s.add_vertex_merged(Vertex::new(1e-6, 0.0, 0.0));
        assert_ne!(i0, i2);
    }

    #[test]
    fn degenerate_face_rejected() {
        let mut s = GeometricShell::default_precision();
        // Collinear points.
        s.add_vertex_raw(Vertex::new(0.0, 0.0, 0.0));
        s.add_vertex_raw(Vertex::new(1e-8, 0.0, 0.0));
        s.add_vertex_raw(Vertex::new(2e-8, 0.0, 0.0));
        let result = s.add_face(vec![0, 1, 2]);
        assert!(matches!(result, Err(ShellError::DegenerateFace(_))));
    }

    #[test]
    fn empty_shell_fails_validate() {
        let s = GeometricShell::default_precision();
        assert_eq!(s.validate(), Err(ShellError::Empty));
    }
}

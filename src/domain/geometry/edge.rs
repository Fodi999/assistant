//! Topological edge — half-edge structure for B-Rep.
//!
//! ## Parasolid correspondence
//! A Parasolid **EDGE** is bounded by two vertices and bordered by two
//! **COEDGEs** (one per adjacent face). We implement a minimal half-edge
//! that stores both coedge references, enabling:
//!
//! - Watertight validation (each edge shared by exactly 2 faces)
//! - Boolean readiness (edge classification: on boundary / inside / outside)
//! - Future: smooth normals at sharp creases, crease angle detection
//!
//! ## Layout
//! ```text
//! Edge { v0, v1, face_a, face_b }
//!           │
//!           ├── HalfEdge { face: face_a, prev, next }   (CCW in face_a)
//!           └── HalfEdge { face: face_b, prev, next }   (CW  in face_b)
//! ```
//!
//! ## DDD role
//! `Edge` is a **domain Entity** inside the `GeometricShell` aggregate.
//! `HalfEdge` is a value object carried inside `Edge`.

// ─────────────────────────────────────────────────────────────────────────────
// Half-edge (directed)
// ─────────────────────────────────────────────────────────────────────────────

/// One directed half of an undirected edge.
///
/// `face` is the face on whose boundary this half-edge travels.
/// `prev` / `next` are indices into the parent shell's half-edge table
/// (not yet materialised — kept as `Option<usize>` for future use).
#[derive(Debug, Clone, Copy)]
pub struct HalfEdge {
    /// Index of the face this half-edge belongs to.
    pub face: u32,
    /// Previous half-edge in the face loop (index in shell half-edge table).
    pub prev: Option<usize>,
    /// Next half-edge in the face loop.
    pub next: Option<usize>,
}

impl HalfEdge {
    pub fn new(face: u32) -> Self {
        Self { face, prev: None, next: None }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Edge (undirected)
// ─────────────────────────────────────────────────────────────────────────────

/// An undirected topological edge between two vertices, shared by up to two
/// faces (manifold / watertight condition).
///
/// `v0 < v1` is always enforced at construction so edge equality is canonical.
#[derive(Debug, Clone)]
pub struct Edge {
    /// Stable edge identifier within the parent shell.
    pub id: u32,
    /// Smaller vertex index.
    pub v0: usize,
    /// Larger vertex index.
    pub v1: usize,
    /// The two half-edges — one per bordering face.
    /// A boundary (open) edge has `he1 == None`.
    pub he0: HalfEdge,
    pub he1: Option<HalfEdge>,
}

impl Edge {
    /// Construct an edge with its first half-edge. Call `attach_second_face`
    /// to complete it for a watertight shell.
    pub fn new(id: u32, a: usize, b: usize, face: u32) -> Self {
        let (v0, v1) = if a <= b { (a, b) } else { (b, a) };
        Self { id, v0, v1, he0: HalfEdge::new(face), he1: None }
    }

    /// Attach the second bordering face. Returns `Err` if already complete.
    pub fn attach_second_face(&mut self, face: u32) -> Result<(), EdgeError> {
        if self.he1.is_some() {
            return Err(EdgeError::NonManifold { edge_id: self.id });
        }
        self.he1 = Some(HalfEdge::new(face));
        Ok(())
    }

    /// `true` if both half-edges are present (closed / manifold edge).
    #[inline]
    pub fn is_manifold(&self) -> bool {
        self.he1.is_some()
    }

    /// The two face indices bordering this edge.
    /// Returns `(face_a, Some(face_b))` or `(face_a, None)` for open edges.
    pub fn bordering_faces(&self) -> (u32, Option<u32>) {
        (self.he0.face, self.he1.map(|h| h.face))
    }

    /// Canonical (sorted) vertex-pair key for hash-map lookups.
    #[inline]
    pub fn key(&self) -> (usize, usize) {
        (self.v0, self.v1)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Error
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EdgeError {
    /// A third face tried to claim the same edge (non-manifold geometry).
    NonManifold { edge_id: u32 },
}

impl std::fmt::Display for EdgeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EdgeError::NonManifold { edge_id } =>
                write!(f, "edge {edge_id} already has two bordering faces (non-manifold)"),
        }
    }
}
impl std::error::Error for EdgeError {}

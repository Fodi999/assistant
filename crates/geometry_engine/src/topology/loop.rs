//! Closed sequence of co-edges forming a face boundary.
//! An outer loop is CCW when viewed from the face normal side.
//! Inner (hole) loops are CW from the same viewpoint.
#![allow(dead_code, unused_variables, unused_imports)]
use super::ids::{CoEdgeId, FaceId};

#[derive(Debug, Clone)]
pub struct Loop {
    /// The face this loop bounds.
    pub face: FaceId,
    /// Ordered co-edges forming the closed boundary.
    pub coedges: Vec<CoEdgeId>,
    /// True = outer boundary, False = inner hole.
    pub is_outer: bool,
}

impl Loop {
    pub fn new(face: FaceId, is_outer: bool) -> Self {
        Self {
            face,
            coedges: Vec::new(),
            is_outer,
        }
    }

    pub fn outer(face: FaceId) -> Self {
        Self::new(face, true)
    }

    pub fn hole(face: FaceId) -> Self {
        Self::new(face, false)
    }

    pub fn add_coedge(&mut self, ce: CoEdgeId) {
        self.coedges.push(ce);
    }

    /// Number of co-edges (= number of edges bounding this loop).
    pub fn coedge_count(&self) -> usize {
        self.coedges.len()
    }

    /// A loop is valid if it has at least 3 co-edges (minimum face boundary).
    pub fn is_valid(&self) -> bool {
        self.coedges.len() >= 3
    }
}


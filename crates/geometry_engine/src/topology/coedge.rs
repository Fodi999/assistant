//! Co-edge: directed use of an edge by a loop.
//! Each undirected Edge has exactly two co-edges (one per adjacent face),
//! or one co-edge for a wire/boundary edge.
#![allow(dead_code, unused_variables, unused_imports)]
use super::ids::{CoEdgeId, EdgeId, LoopId};

#[derive(Debug, Clone)]
pub struct CoEdge {
    /// The underlying undirected edge.
    pub edge: EdgeId,
    /// The loop this co-edge belongs to.
    pub loop_id: LoopId,
    /// If true, traverse edge in reverse direction (end→start).
    pub reversed: bool,
    /// Next co-edge in the loop (CCW order).
    pub next: Option<CoEdgeId>,
    /// Previous co-edge in the loop.
    pub prev: Option<CoEdgeId>,
    /// Partner co-edge on the adjacent face (None for boundary edges).
    pub partner: Option<CoEdgeId>,
}

impl CoEdge {
    pub fn new(edge: EdgeId, loop_id: LoopId, reversed: bool) -> Self {
        Self {
            edge,
            loop_id,
            reversed,
            next: None,
            prev: None,
            partner: None,
        }
    }

    /// Link next and prev pointers for consecutive co-edges.
    pub fn link(a: &mut CoEdge, a_id: CoEdgeId, b: &mut CoEdge, b_id: CoEdgeId) {
        a.next = Some(b_id);
        b.prev = Some(a_id);
    }

    /// Attach the partner co-edge from the adjacent face.
    pub fn with_partner(mut self, partner: CoEdgeId) -> Self {
        self.partner = Some(partner);
        self
    }

    /// True if this is a boundary (open) edge with no adjacent face.
    pub fn is_boundary(&self) -> bool {
        self.partner.is_none()
    }
}


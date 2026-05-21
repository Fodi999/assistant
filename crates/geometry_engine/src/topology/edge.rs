//! Edge: topological 1D entity connecting two vertices via a 3D curve.
//!
//! An Edge is shared by exactly two CoEdges (one per adjacent face) in a
//! manifold shell. The `curve_id` is an index into the B-Rep curve table.
#![allow(dead_code, unused_variables, unused_imports)]
use crate::math::Real;
use super::ids::{EdgeId, VertexId};

#[derive(Debug, Clone)]
pub struct Edge {
    /// Start vertex (parameter = t_min on underlying curve).
    pub start: VertexId,
    /// End vertex (parameter = t_max on underlying curve).
    pub end: VertexId,
    /// Index into the B-Rep edge-curve table (None = degenerate / straight).
    pub curve_id: Option<u64>,
    /// Cached length in metres (set by B-Rep builder, else NaN).
    pub length: Real,
    /// True if this edge is a seam (shared by the same face twice).
    pub is_seam: bool,
}

impl Edge {
    pub fn new(start: VertexId, end: VertexId) -> Self {
        Self { start, end, curve_id: None, length: Real::NAN, is_seam: false }
    }

    pub fn with_curve(mut self, curve_id: u64) -> Self {
        self.curve_id = Some(curve_id);
        self
    }

    pub fn is_degenerate(&self) -> bool {
        self.length.is_nan() || self.length < 1e-12
    }
}


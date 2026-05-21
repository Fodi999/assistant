//! Face: a bounded region of a surface, bounded by one outer loop + optional
//! inner (hole) loops. Orientation flag indicates whether the face normal
//! agrees with the underlying surface normal.
#![allow(dead_code, unused_variables, unused_imports)]
use super::ids::{LoopId, ShellId};

#[derive(Debug, Clone)]
pub struct Face {
    /// Shell that owns this face.
    pub shell: ShellId,
    /// Outer boundary loop (CCW when viewed from outside).
    pub outer_loop: LoopId,
    /// Inner hole loops (CW when viewed from outside).
    pub inner_loops: Vec<LoopId>,
    /// Index into the B-Rep surface table.
    pub surface_id: Option<u64>,
    /// True = face normal agrees with surface normal.
    pub orientation: bool,
    /// Optional material / visual group index.
    pub group: Option<u32>,
}

impl Face {
    pub fn new(shell: ShellId, outer_loop: LoopId) -> Self {
        Self {
            shell,
            outer_loop,
            inner_loops: Vec::new(),
            surface_id: None,
            orientation: true,
            group: None,
        }
    }

    pub fn add_hole(&mut self, loop_id: LoopId) {
        self.inner_loops.push(loop_id);
    }

    pub fn with_surface(mut self, id: u64) -> Self {
        self.surface_id = Some(id);
        self
    }

    pub fn flipped(mut self) -> Self {
        self.orientation = !self.orientation;
        self
    }
}


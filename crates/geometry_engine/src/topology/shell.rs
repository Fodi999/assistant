//! Shell: connected set of faces forming a watertight (or open) surface.
//!
//! A Shell holds references to its Faces. The `is_closed` flag indicates
//! whether the shell has no boundary edges (i.e. it encloses a volume).
#![allow(dead_code, unused_variables, unused_imports)]
use super::ids::{EdgeId, FaceId, SolidId};

#[derive(Debug, Clone)]
pub struct Shell {
    /// The solid that owns this shell (None for free shells / sheets).
    pub solid:     Option<SolidId>,
    /// Ordered face handles belonging to this shell.
    pub faces:     Vec<FaceId>,
    /// True when the shell is closed (no boundary edges).
    pub is_closed: bool,
}

impl Shell {
    pub fn new() -> Self {
        Self { solid: None, faces: Vec::new(), is_closed: false }
    }

    pub fn with_solid(mut self, id: SolidId) -> Self {
        self.solid = Some(id);
        self
    }

    pub fn add_face(&mut self, id: FaceId) {
        self.faces.push(id);
    }

    pub fn face_count(&self) -> usize {
        self.faces.len()
    }

    /// Mark the shell as closed after construction (call after all faces added).
    pub fn mark_closed(&mut self) {
        self.is_closed = true;
    }
}

impl Default for Shell {
    fn default() -> Self { Self::new() }
}


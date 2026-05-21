//! Solid: one outer shell + optional inner (cavity) shells.
//! In the B-Rep sense a solid is a closed, orientable manifold.
#![allow(dead_code, unused_variables, unused_imports)]
use super::ids::{BodyId, ShellId};

#[derive(Debug, Clone)]
pub struct Solid {
    /// Parent body.
    pub body: BodyId,
    /// The single outer (bounding) shell. Must be closed.
    pub outer: ShellId,
    /// Inner void shells representing hollowed-out regions.
    pub cavities: Vec<ShellId>,
    /// Optional user-facing name / label.
    pub name: Option<String>,
}

impl Solid {
    pub fn new(body: BodyId, outer: ShellId) -> Self {
        Self {
            body,
            outer,
            cavities: Vec::new(),
            name: None,
        }
    }

    pub fn add_cavity(&mut self, shell: ShellId) {
        self.cavities.push(shell);
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn is_hollow(&self) -> bool {
        !self.cavities.is_empty()
    }

    pub fn cavity_count(&self) -> usize {
        self.cavities.len()
    }
}


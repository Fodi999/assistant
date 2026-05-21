//! Top-level CAD body: a named collection of solids.
//! A body may contain disconnected lumps (multi-body parts).
#![allow(dead_code, unused_variables, unused_imports)]
use super::ids::SolidId;

#[derive(Debug, Clone, Default)]
pub struct Body {
    /// All solids belonging to this body (usually 1, >1 for multi-lump parts).
    pub solids: Vec<SolidId>,
    /// Optional user-facing name / label.
    pub name: Option<String>,
    /// Opaque metadata slot (e.g. linked CAD file path).
    pub metadata: Option<String>,
}

impl Body {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn add_solid(&mut self, solid: SolidId) {
        self.solids.push(solid);
    }

    pub fn solid_count(&self) -> usize {
        self.solids.len()
    }

    /// True if the body contains more than one disconnected solid (multi-lump).
    pub fn is_multi_lump(&self) -> bool {
        self.solids.len() > 1
    }
}


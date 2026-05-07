// ── Scene: SelectionState — which object(s) are highlighted ──────────────────────
// Domain: UI selection model, independent of rendering.
// The shader reads selection state to apply highlight color / outline.

/// What the user has selected in the scene.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectionState {
    /// Nothing selected.
    None,
    /// A single particle by instance index.
    Single(u32),
    /// A rectangular range of particles (e.g. a face of the cube formation).
    Range { start: u32, end: u32 },
}

impl SelectionState {
    pub fn is_selected(&self, instance: u32) -> bool {
        match self {
            Self::None              => false,
            Self::Single(i)         => *i == instance,
            Self::Range { start, end } => instance >= *start && instance <= *end,
        }
    }

    pub fn clear(&mut self) {
        *self = Self::None;
    }
}

impl Default for SelectionState {
    fn default() -> Self {
        Self::None
    }
}

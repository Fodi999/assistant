// ── Scene: SceneObject — the atomic renderable unit ──────────────────────────────
// Domain: A single particle / tile in the scene.
// Owns its transform, color, and the metadata the shader needs (cellMask).

use super::transform::Transform;

/// RGBA color (linear, 0..1).
#[derive(Debug, Clone, Copy)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

impl Color {
    pub const WHITE: Self = Self {
        r: 1.0,
        g: 1.0,
        b: 1.0,
    };

    pub fn new(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b }
    }
}

/// Which of the 6 axis-aligned faces of this cell are exposed to the outside.
/// Bit layout mirrors `kernel::particle_shape`:
///   bit 0 = +X   bit 1 = −X
///   bit 2 = +Y   bit 3 = −Y
///   bit 4 = +Z   bit 5 = −Z
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellMask(pub u32);

impl CellMask {
    /// All six faces exposed (cloud / isolated particle).
    pub const ALL: Self = Self(63);
    /// No faces exposed (fully interior — should be culled).
    pub const NONE: Self = Self(0);

    pub fn exposed_count(self) -> u32 {
        self.0.count_ones()
    }

    pub fn is_interior(self) -> bool {
        self.0 == 0
    }
}

/// The atomic unit rendered by the WebGPU particle pipeline.
#[derive(Debug, Clone)]
pub struct SceneObject {
    pub transform: Transform,
    pub color: Color,
    /// World-space radius of this particle's billboard / cell half-extent.
    pub radius: f32,
    /// Exposed-face bitmask (used for seam culling and mesh-mode face dispatch).
    pub cell_mask: CellMask,
}

impl SceneObject {
    pub fn new(transform: Transform, color: Color, radius: f32) -> Self {
        Self {
            transform,
            color,
            radius,
            cell_mask: CellMask::ALL,
        }
    }
}

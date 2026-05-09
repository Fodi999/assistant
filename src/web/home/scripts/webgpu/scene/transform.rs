// ── Scene: 3D transform — position, rotation (phase), scale ──────────────────────
// Domain: Spatial representation of any object in the scene.
// Kept as plain data — no GPU types, no JS, no rendering logic.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub const ZERO: Self = Self {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };

    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub fn length(self) -> f32 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }
}

/// World-space transform of a scene object.
#[derive(Debug, Clone, Copy)]
pub struct Transform {
    /// World-space centre position.
    pub position: Vec3,
    /// Per-object rotation phase (0..2π) — maps to `rotMat(phase)` in WGSL.
    pub phase: f32,
    /// Uniform scale multiplier.
    pub scale: f32,
}

impl Transform {
    pub fn identity() -> Self {
        Self {
            position: Vec3::ZERO,
            phase: 0.0,
            scale: 1.0,
        }
    }

    pub fn at(position: Vec3) -> Self {
        Self {
            position,
            ..Self::identity()
        }
    }
}

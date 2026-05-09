// ── Scene: SceneAction — user-intent events that mutate scene state ──────────────
// Domain: Command pattern — separates intent from execution.
// The render loop dispatches these; the scene applies them to its state.

/// A discrete action the user (or animation system) can request.
#[derive(Debug, Clone)]
pub enum SceneAction {
    /// Switch particle count.
    SetParticleCount(u32),
    /// Change formation mode.
    SetFormation(FormationMode),
    /// Toggle cell-SDF render path.
    ToggleCellSdf,
    /// Change shape roundness slider (0 = super-cube, 1 = sphere).
    SetRoundness(f32),
    /// Change cell-SDF corner radius (0..0.5).
    SetCellRadius(f32),
    /// Cycle debug color mode (normal / normals-RGB / mask-color).
    CycleColorMode,
    /// Toggle hide-low (show only edges + corners).
    ToggleHideLow,
    /// Toggle camera auto-rotate.
    ToggleAutoRotate,
    /// Start full benchmark sweep.
    RunBenchmark,
}

/// The three formation modes available.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormationMode {
    Cloud,
    Cube,
    Wall,
}

impl FormationMode {
    /// Maps to the float sent in UBO u6.y.
    pub fn to_shader_id(self) -> f32 {
        match self {
            Self::Cloud => 0.0,
            Self::Cube => 1.0,
            Self::Wall => 2.0,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Cloud => "cloud",
            Self::Cube => "cube",
            Self::Wall => "wall",
        }
    }
}

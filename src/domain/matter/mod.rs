// ── Matter domain — precision sketch commands ─────────────────────────────
// geometry_engine has been extracted to a separate crate; these re-exports
// are stubbed until the dependency is wired back in.

pub mod commands {
    use serde::{Deserialize, Serialize};
    #[derive(Debug, Serialize, Deserialize, Clone, Default)]
    pub struct AddPointRequest {}
    #[derive(Debug, Serialize, Deserialize, Clone, Default)]
    pub struct AddEdgeRequest {}
    #[derive(Debug, Serialize, Deserialize, Clone, Default)]
    pub struct MovePointRequest {}
    #[derive(Debug, Serialize, Deserialize, Clone, Default)]
    pub struct SketchCommandResult {
        pub ok: bool,
    }
}
pub mod sketch {
    use serde::{Deserialize, Serialize};
    #[derive(Debug, Serialize, Deserialize, Clone, Default)]
    pub struct SketchGraph {}
}
pub mod validation {
    use serde::{Deserialize, Serialize};
    #[derive(Debug, Serialize, Deserialize, Clone, Default)]
    pub struct ValidationResult {
        pub valid: bool,
    }
    #[derive(Debug, Serialize, Deserialize, Clone, Default)]
    pub struct ValidationIssue {}
}
pub mod profiles {}
pub mod solver {}

use serde::{Deserialize, Serialize};

pub use commands::{AddEdgeRequest, AddPointRequest, MovePointRequest, SketchCommandResult};
pub use sketch::SketchGraph;
pub use validation::{ValidationIssue, ValidationResult};

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Point {}
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Edge {}
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Profile {}
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Constraint {}
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct WorkingPlane {}
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct PointRefOrGrid {}
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct ProfileAnalyzeRequest {}
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct ProfileAnalyzeResponse {}
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct ProfileRepairRequest {}
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct ProfileRepairResponse {}
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct SolveConstraintsRequest {}
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct SolveResult {
    pub ok: bool,
}
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct ConstraintApplyResult {
    pub ok: bool,
}

pub fn apply_add_point(_g: SketchGraph, _r: AddPointRequest) -> SketchCommandResult {
    Default::default()
}
pub fn apply_add_edge(_g: SketchGraph, _r: AddEdgeRequest) -> SketchCommandResult {
    Default::default()
}
pub fn apply_move_point(_g: SketchGraph, _r: MovePointRequest) -> SketchCommandResult {
    Default::default()
}
pub fn validate(_g: &SketchGraph) -> ValidationResult {
    Default::default()
}
pub fn detect_profiles(_g: &SketchGraph) -> Vec<Profile> {
    vec![]
}
pub fn analyze_profile(_r: ProfileAnalyzeRequest) -> ProfileAnalyzeResponse {
    Default::default()
}
pub fn repair_profile(_r: ProfileRepairRequest) -> ProfileRepairResponse {
    Default::default()
}
pub fn solve_constraints(_r: SolveConstraintsRequest) -> SolveResult {
    Default::default()
}
pub fn apply_constraint_once(_g: SketchGraph, _c: Constraint) -> ConstraintApplyResult {
    Default::default()
}

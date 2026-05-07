// ── Scene: domain root ────────────────────────────────────────────────────────────
// Re-exports all scene types for consumers of this layer.
//
// Rule: nothing in scene/ imports from shader/ or js/.
// Direction of dependencies:
//   js/ → scene/   (JS reads scene state to build UBO)
//   shader/ has no Rust imports (it's a string of WGSL)

pub mod actions;
pub mod ingredient;
pub mod object;
pub mod recipe;
pub mod selection;
pub mod transform;

pub use actions::{FormationMode, SceneAction};
pub use ingredient::Ingredient;
pub use object::{CellMask, Color, SceneObject};
pub use recipe::Recipe;
pub use selection::SelectionState;
pub use transform::{Transform, Vec3};

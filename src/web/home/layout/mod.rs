// ── layout/ — HTML-шаблоны и CSS (Presentation layer) ───────────────────────
// Всё что видит пользователь как оболочку редактора.

pub mod template;
pub mod styles;
pub mod matter_lab;
pub mod matter_lab_styles;
// `panels` and `outliner` removed in UI cleanup (Patch #2):
//   - `panels.rs`   — 9 legacy `.matter-panel-right` aside helpers, never called.
//   - `outliner.rs` — HTML never injected; JS recursed forever via setTimeout.
// Replaced by UI Shell v1 (`ui/scene_tree.rs` + `ui/right_inspector.rs`).
pub mod cad_side_panel;
pub mod cad_side_panel_styles;

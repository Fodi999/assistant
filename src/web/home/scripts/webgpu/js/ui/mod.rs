// ── ui/ — Floating panels, HUD, overlays, N-panel matter UI ─────────────────

pub mod hud;
pub mod gizmo;
pub mod controls;
pub mod dimension_editor;
pub mod profile_popup;
pub mod profile_backend;
pub mod matter_ui;
pub mod matter_state;
pub mod cad_side_panel;
pub mod constraint_solver;
pub mod sketch_extrude_bridge;
pub mod view_cube;
pub mod selection_mode_hud;

// ── UI Shell v1 — professional shell (top bar / bottom toolbar / inspector /
//    scene tree / dev mode) — additive, defensive, depends on ui_shell first.
pub mod ui_shell;
pub mod document;
pub mod dev_mode;
pub mod top_bar;
pub mod bottom_toolbar;
pub mod right_inspector;
pub mod scene_tree;

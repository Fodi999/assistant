// ── JS assembly: ordered composition of all JS domains ───────────────────────
//
// Architecture (DDD-inspired, 2026):
//
//   design/   — единый стиль: tokens.rs  (меняй здесь → меняется всё)
//   core/     — WebGPU runtime: init, state, buffers, pipeline, render_loop
//   input/    — keyboard, mouse, touchpad
//   sketch/   — CAD domain: state, pick, backend, wasm, engine, io, constraints
//   tools/    — редакторные инструменты: select, grab, line, extrude, hotkeys
//   ui/       — floating panels, HUD, gizmo, controls, matter UI
//   scene/    — scene domain objects (geometry, selection)
//   shader/   — WGSL шейдеры

pub mod design;
pub mod core;
pub mod input;
pub mod sketch;
pub mod tools;
pub mod ui;

/// Assembles the complete JS source, embedding both WGSL shaders.
pub fn assemble(shader: &str, cad_shader: &str) -> String {
    let mut out = String::with_capacity(
        design::tokens::JS.len()
        + core::init::JS.len()
        + core::state::JS.len()
        + core::buffers::JS.len()
        + core::pipeline::JS.len()
        + core::render_loop::JS.len()
        + core::perf_hud::JS.len()
        + input::keyboard::JS.len()
        + input::mouse::JS.len()
        + input::touchpad::JS.len()
        + sketch::state::JS.len()
        + sketch::pick::JS.len()
        + sketch::rect::JS.len()
        + sketch::circle::JS.len()
        + sketch::dim::JS.len()
        + sketch::io::JS.len()
        + sketch::backend::JS.len()
        + sketch::wasm::JS.len()
        + sketch::cad_engine::JS.len()
        + sketch::constraints::JS.len()
        + tools::select_tool::JS.len()
        + tools::grab_tool::JS.len()
        + tools::grab_gizmo::JS.len()
        + tools::copy_tool::JS.len()
        + tools::line_tool::JS.len()
        + tools::hotkeys::JS.len()
        + tools::extrude::JS.len()
        + tools::extrude_gizmo::JS.len()
        + ui::matter_state::JS.len()
        + ui::hud::JS.len()
        + ui::controls::JS.len()
        + ui::gizmo::JS.len()
        + ui::matter_ui::JS.len()
        + ui::dimension_editor::JS.len()
        + ui::profile_popup::JS.len()
        + ui::profile_backend::JS.len()
        + ui::cad_side_panel::JS.len()
        + ui::view_cube::JS.len()
        + ui::selection_mode_hud::JS.len()
        + ui::constraint_solver::JS.len()
        + shader.len()
        + cad_shader.len()
        + 512,
    );

    // ── 1. Design tokens (first — everything reads __modalTheme) ────────────
    out.push_str(design::tokens::JS);

    // ── 2. Core runtime ──────────────────────────────────────────────────────
    out.push_str(core::init::JS);
    out.push_str(core::state::JS);

    // ── 3. Input ─────────────────────────────────────────────────────────────
    out.push_str(input::keyboard::JS);
    out.push_str(input::mouse::JS);
    out.push_str(input::touchpad::JS);

    // ── 4. Sketch domain ─────────────────────────────────────────────────────
    out.push_str(sketch::state::JS);
    out.push_str(sketch::pick::JS);
    out.push_str(sketch::rect::JS);
    out.push_str(sketch::circle::JS);
    out.push_str(sketch::dim::JS);
    out.push_str(sketch::io::JS);
    out.push_str(sketch::backend::JS);
    out.push_str(sketch::wasm::JS);
    out.push_str(sketch::cad_engine::JS);
    out.push_str(sketch::constraints::JS);

    // ── 5. Tools ─────────────────────────────────────────────────────────────
    out.push_str(tools::select_tool::JS);
    out.push_str(tools::grab_tool::JS);
    out.push_str(tools::grab_gizmo::JS);  // overrides drawGrabGizmo + __startGrabFromGizmo
    out.push_str(tools::gizmo_controller::JS);  // single source of truth for gizmo pointer events
    out.push_str(tools::copy_tool::JS);
    out.push_str(tools::line_tool::JS);
    out.push_str(tools::hotkeys::JS);
    out.push_str(tools::extrude::JS);
    out.push_str(tools::extrude_gizmo::JS);

    // ── 6. GPU buffers + WGSL shaders + pipeline ─────────────────────────────
    out.push_str(core::buffers::JS);
    out.push_str("\n// ── WGSL ────────────────────────────────────────────────\n");
    out.push_str("const shaderSrc = `\n");
    out.push_str(shader);
    out.push_str("\n`;\n");
    out.push_str("const cadShaderSrc = `\n");
    out.push_str(cad_shader);
    out.push_str("\n`;\n");
    out.push_str(core::pipeline::JS);

    // ── 7. UI panels + HUD ───────────────────────────────────────────────────
    out.push_str(ui::hud::JS);
    out.push_str(ui::controls::JS);
    out.push_str(ui::gizmo::JS);
    out.push_str(ui::matter_state::JS);
    out.push_str(ui::matter_ui::JS);
    out.push_str(ui::dimension_editor::JS);
    out.push_str(ui::profile_popup::JS);
    out.push_str(ui::profile_backend::JS);
    out.push_str(ui::cad_side_panel::JS);
    out.push_str(ui::view_cube::JS);
    out.push_str(ui::selection_mode_hud::JS);
    out.push_str(ui::constraint_solver::JS);

    // ── 8. Render loop (last — depends on everything above) ──────────────────
    out.push_str(core::render_loop::JS);
    out.push_str(core::perf_hud::JS);

    out
}

// ── JS assembly: injects WGSL shader source and concatenates all JS fragments ─────
// Domain: Application — ordered composition of all JS domains.

mod buffers;
mod controls;
pub mod components;
mod gizmo;
mod hud;
mod init;
pub mod input;
pub mod tools;
mod matter_state;
mod matter_ui;
mod pipeline;
mod render_loop;
mod state;
mod sketch_state;
mod sketch_pick;
mod sketch_rect;
mod sketch_circle;
mod sketch_dim;
mod sketch_tools;
mod sketch_io;
mod sketch_backend;
mod sketch_wasm;
mod sketch_cad_engine;
mod dimension_editor;
mod sketch_constraints;
mod profile_popup;
mod perf_hud;
mod extrude;

/// Assembles the complete JS source, embedding both WGSL shaders.
pub fn assemble(shader: &str, cad_shader: &str) -> String {
    let mut out = String::with_capacity(
        init::JS.len()
            + state::JS.len()
            + input::keyboard::JS.len()
            + input::mouse::JS.len()
            + input::touchpad::JS.len()
            + sketch_state::JS.len()
            + sketch_pick::JS.len()
            + sketch_rect::JS.len()
            + sketch_circle::JS.len()
            + sketch_dim::JS.len()
            + tools::select_tool::JS.len()
            + tools::grab_tool::JS.len()
            + tools::copy_tool::JS.len()
            + tools::line_tool::JS.len()
            + tools::hotkeys::JS.len()
            + sketch_io::JS.len()
            + matter_state::JS.len()
            + buffers::JS.len()
            + shader.len()
            + cad_shader.len()
            + pipeline::JS.len()
            + hud::JS.len()
            + controls::JS.len()
            + gizmo::JS.len()
            + matter_ui::JS.len()
            + render_loop::JS.len()
            + extrude::JS.len()
            + perf_hud::JS.len()
            + sketch_wasm::JS.len()
            + sketch_cad_engine::JS.len()
            + components::modal_theme::JS.len()
            + dimension_editor::JS.len()
            + sketch_constraints::JS.len()
            + profile_popup::JS.len()
            + 256,
    );
    out.push_str(init::JS);
    // 1. Core runtime state (particles, camera, scene)
    out.push_str(state::JS);
    // 1a. Input — keyboard shortcuts
    out.push_str(input::keyboard::JS);
    // 1b. Input — mouse (pointer events, orbit, pan, click, hover, snap)
    out.push_str(input::mouse::JS);
    // 1c. Input — touchpad / wheel zoom
    out.push_str(input::touchpad::JS);
    // 2. Sketch data model (sketchState, snapToGrid, extrudePreview, UI sync)
    out.push_str(sketch_state::JS);
    // 3. Sketch raycasting (raycastSketchPlane, buildPickRay, raycastCadSolids)
    out.push_str(sketch_pick::JS);
    // 4. Rectangle tool
    out.push_str(sketch_rect::JS);
    // 5. Circle tool
    out.push_str(sketch_circle::JS);
    // 6. Dimension tool
    out.push_str(sketch_dim::JS);
    // 7a. Select tool (click / double-click dispatch) — tools/select_tool.rs
    out.push_str(tools::select_tool::JS);
    // 7b. Grab tool — G-key + toolbar grab
    out.push_str(tools::grab_tool::JS);
    // 7c. Copy-connect tool — Shift+G
    out.push_str(tools::copy_tool::JS);
    // 7d. Line tool — L key, click1=start, click2=edge
    out.push_str(tools::line_tool::JS);
    // 7e. Hotkeys + constraints + line preview
    out.push_str(tools::hotkeys::JS);
    // 8. Sketch I/O — JSON export / import / backend payload preview
    out.push_str(sketch_io::JS);
    // 9. Backend precision commands (Phase 7) — POST add-point / add-edge
    out.push_str(sketch_backend::JS);
    // 10. WASM bridge (Phase 10) — shared sketch_engine in the browser
    out.push_str(sketch_wasm::JS);
    // 10b. CAD Engine adapter — WASM-first + backend-sync, single entry point
    //      for all tools. Overrides __createPointViaEngine / __createEdgeViaEngine.
    out.push_str(sketch_cad_engine::JS);
    // 10c. Modal theme — единый стиль для всех CAD попапов (меняй здесь).
    out.push_str(components::modal_theme::JS);
    // 10d. Dimension editor popup — click drafting labels to edit geometry.
    out.push_str(dimension_editor::JS);
    // 10d. Profile constraints — Analyze / Make Rectangle / Make Square / Equalize.
    out.push_str(sketch_constraints::JS);
    // 10e. Profile Check popup — floating panel opened by double-clicking a profile.
    out.push_str(profile_popup::JS);
    // 11. Performance HUD (Phase 9) — perfState + frame/render/overlay/pick/backend ms
    out.push_str(perf_hud::JS);
    out.push_str(matter_state::JS);
    out.push_str(buffers::JS);
    // ── WGSL shaders ────────────────────────────────────────────
    out.push_str("\n      // ── WGSL ────────────────────────────────────────────────\n");
    out.push_str("      const shaderSrc = `\n");
    out.push_str(shader);
    out.push_str("\n`;\n");
    out.push_str("      const cadShaderSrc = `\n");
    out.push_str(cad_shader);
    out.push_str("\n`;\n");
    out.push_str(pipeline::JS);
    out.push_str(hud::JS);
    out.push_str(controls::JS);
    out.push_str(gizmo::JS);
    out.push_str(extrude::JS);
    out.push_str(matter_ui::JS);
    out.push_str(render_loop::JS);
    out
}

// ── JS assembly: injects WGSL shader source and concatenates all JS fragments ─────
// Domain: Application — ordered composition of all JS domains.

mod buffers;
mod controls;
mod gizmo;
mod hud;
mod init;
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
mod extrude;

/// Assembles the complete JS source, embedding both WGSL shaders.
pub fn assemble(shader: &str, cad_shader: &str) -> String {
    let mut out = String::with_capacity(
        init::JS.len()
            + state::JS.len()
            + sketch_state::JS.len()
            + sketch_pick::JS.len()
            + sketch_rect::JS.len()
            + sketch_circle::JS.len()
            + sketch_dim::JS.len()
            + sketch_tools::JS.len()
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
            + 256,
    );
    out.push_str(init::JS);
    // 1. Core runtime state (particles, camera, scene, event dispatcher)
    out.push_str(state::JS);
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
    // 7. Sketch tools dispatcher
    out.push_str(sketch_tools::JS);
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

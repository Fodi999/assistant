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

/// Assembles the complete JS source, embedding both WGSL shaders.
pub fn assemble(shader: &str, cad_shader: &str) -> String {
    let mut out = String::with_capacity(
        init::JS.len()
            + state::JS.len()
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
            + 128,
    );
    out.push_str(init::JS);
    out.push_str(state::JS);
    out.push_str(matter_state::JS);
    out.push_str(buffers::JS);
    // ── 6. WGSL (Particle / Morph pipeline) ─────────────────────
    out.push_str("\n      // ── 6. WGSL ─────────────────────────────────────────────────\n");
    out.push_str("      const shaderSrc = `\n");
    out.push_str(shader);
    out.push_str("\n`;\n");
    // ── 6b. WGSL (CAD / Solid pipeline) ────────────────────────
    out.push_str("      const cadShaderSrc = `\n");
    out.push_str(cad_shader);
    out.push_str("\n`;\n");
    out.push_str(pipeline::JS);
    out.push_str(hud::JS);
    out.push_str(controls::JS);
    out.push_str(gizmo::JS);
    out.push_str(matter_ui::JS);
    out.push_str(render_loop::JS);
    out
}

// ── WebGPU scene — top-level assembly ────────────────────────────────────────────
// Composes shader WGSL + JS infrastructure into the single script string
// served by the home page.
//
// Layers:
//   shader/   — WGSL GPU program (uniforms, background, geometry, SDF, vertex, fragment)
//   js/       — JS application domains: core, input, sketch, tools, ui, design

mod js;
mod shader;

use std::sync::OnceLock;

static WEBGPU_JS: OnceLock<String> = OnceLock::new();

/// Returns the complete WebGPU JS source, assembled once and cached for the
/// lifetime of the process.
pub fn webgpu_js() -> &'static str {
    WEBGPU_JS.get_or_init(|| js::assemble(shader::wgsl(), &shader::cad_wgsl()))
}

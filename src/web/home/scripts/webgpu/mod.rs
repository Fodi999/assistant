// ── WebGPU scene — top-level assembly ────────────────────────────────────────────
// Composes shader WGSL + JS infrastructure into the single script string
// served by the home page.
//
// DDD layers:
//   scene/    — domain model  (what exists in the world: objects, transforms,
//                              ingredients, recipes, selections, actions)
//   shader/   — WGSL GPU program (uniforms, background, geometry, SDF, vertex, fragment)
//   js/       — JS application  (init, state, buffers, pipeline, hud, controls, benchmark,
//                                render_loop)

mod js;
pub mod scene;
mod shader;

use std::sync::OnceLock;

static WEBGPU_JS: OnceLock<String> = OnceLock::new();

/// Returns the complete WebGPU JS source, assembled once and cached for the
/// lifetime of the process.
pub fn webgpu_js() -> &'static str {
    WEBGPU_JS.get_or_init(|| js::assemble(shader::wgsl()))
}

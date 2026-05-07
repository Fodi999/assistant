// ── Shader assembly: concatenates WGSL fragments in dependency order ─────────────
// Domain: GPU shader program — single WGSL source passed to createShaderModule.

mod uniforms;
mod background;
mod geometry;
mod sdf;
mod particles_vert;
mod particles_frag;

use std::sync::OnceLock;

static SHADER: OnceLock<String> = OnceLock::new();

/// Returns the complete WGSL source, assembled once and cached.
pub fn wgsl() -> &'static str {
    SHADER.get_or_init(|| {
        [
            uniforms::WGSL,
            background::WGSL,
            geometry::WGSL,
            sdf::WGSL,
            particles_vert::WGSL,
            particles_frag::WGSL,
        ]
        .join("\n")
    })
}

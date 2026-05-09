// ── Shader assembly: concatenates WGSL fragments in dependency order ─────────────
// Domain: GPU shader program — single WGSL source passed to createShaderModule.

mod background;
mod geometry;
mod particles_frag;
mod particles_vert;
mod sdf;
mod uniforms;

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

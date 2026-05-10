// ── Shader assembly: concatenates WGSL fragments in dependency order ─────────────
// Domain: GPU shader program — single WGSL source passed to createShaderModule.

mod background;
mod geometry;
mod particles_frag;
mod particles_vert;
pub mod cad_frag;
pub mod cad_vert;
mod sdf;
mod uniforms;

use std::sync::OnceLock;

static SHADER: OnceLock<String> = OnceLock::new();

/// Returns the complete WGSL source, assembled once and cached.
/// Note: cad_vert / cad_frag are NOT included in the monolithic shader module —
/// they will be compiled into a separate GPUShaderModule once the CAD pipeline is wired.
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

/// Returns the WGSL source for the separate CAD solid pipeline.
pub fn cad_wgsl() -> String {
    [
        uniforms::WGSL,
        geometry::WGSL,
        cad_vert::WGSL,
        cad_frag::WGSL,
    ]
    .join("\n")
}

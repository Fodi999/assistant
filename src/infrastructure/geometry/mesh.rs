//! Core mesh types for the procedural geometry generators.
//!
//! PR #28: `Material` now carries full PBR properties (roughness, metalness,
//! opacity) so the glTF exporter can emit physically-correct materials instead
//! of the old Phong→roughness approximation.  Generators must set these fields
//! via the new builder helpers; the `shininess` field is kept for backwards
//! compatibility but is no longer used by the exporter when `roughness` > 0.
//! No GPU types, no normal compression, no skinning.

/// A triangle mesh ready for OBJ serialisation.
#[derive(Debug, Clone)]
pub struct Mesh {
    /// XYZ positions, one per vertex.
    pub vertices: Vec<[f32; 3]>,
    /// Per-vertex normals (same length as `vertices`).
    pub normals: Vec<[f32; 3]>,
    /// Per-vertex UV coordinates (same length as `vertices`).
    pub uvs: Vec<[f32; 2]>,
    /// Triangles — each entry is three indices into `vertices`.
    /// Used only when `groups` is empty (legacy single-material path).
    pub faces: Vec<[usize; 3]>,
    /// Single material for the whole mesh.
    /// Used only when `groups` is empty.
    pub material: Material,
    /// Optional multi-material groups (PR #6).
    /// When non-empty, the OBJ exporter writes one `usemtl` block per group
    /// and the MTL file contains one `newmtl` entry per group.
    /// `faces` / `material` are then ignored.
    pub groups: Vec<MaterialGroup>,
}

/// One material slot inside a mesh — its own faces, its own material.
#[derive(Debug, Clone)]
pub struct MaterialGroup {
    pub material: Material,
    pub faces: Vec<[usize; 3]>,
}

impl Mesh {
    /// Convenience constructor — validates that normals/uvs have the same
    /// length as vertices (panics in debug builds only).
    pub fn new(
        vertices: Vec<[f32; 3]>,
        normals: Vec<[f32; 3]>,
        uvs: Vec<[f32; 2]>,
        faces: Vec<[usize; 3]>,
        material: Material,
    ) -> Self {
        debug_assert_eq!(vertices.len(), normals.len(), "normals length mismatch");
        debug_assert_eq!(vertices.len(), uvs.len(), "uvs length mismatch");
        Self {
            vertices,
            normals,
            uvs,
            faces,
            material,
            groups: Vec::new(),
        }
    }

    /// Multi-material constructor (PR #6). Each entry in `groups` defines its
    /// own material and triangle list — the OBJ exporter will emit them as
    /// separate `usemtl` blocks.
    pub fn new_multi(
        vertices: Vec<[f32; 3]>,
        normals: Vec<[f32; 3]>,
        uvs: Vec<[f32; 2]>,
        groups: Vec<MaterialGroup>,
    ) -> Self {
        debug_assert_eq!(vertices.len(), normals.len(), "normals length mismatch");
        debug_assert_eq!(vertices.len(), uvs.len(), "uvs length mismatch");
        debug_assert!(!groups.is_empty(), "new_multi requires at least one group");
        // Mirror the first group into the legacy fields so older code paths
        // still see a sensible material/face list.
        let first = groups[0].clone();
        Self {
            vertices,
            normals,
            uvs,
            faces: first.faces,
            material: first.material,
            groups,
        }
    }
}

/// Simple PBR material, maps 1-to-1 to glTF `pbrMetallicRoughness`.
#[derive(Debug, Clone)]
pub struct Material {
    /// Used as the `newmtl` name in the MTL file and `usemtl` in OBJ.
    pub name: String,
    /// Diffuse colour as sRGB floats 0..1. Derived from `Product3DSpec.product.color_hex`.
    pub diffuse_color: [f32; 3],
    /// Optional texture filename (relative, e.g. `"source.png"`).
    /// If present, the MTL file will include `map_Kd <texture_file>`.
    pub texture_file: Option<String>,
    /// Specular intensity 0..1 (mapped to MTL `Ks`). Default 0.15.
    /// Legacy — used only by the OBJ exporter; glTF ignores this.
    pub specular: f32,
    /// Shininess exponent (mapped to MTL `Ns`). Legacy — kept for OBJ.
    /// The glTF exporter uses `roughness` directly when it is > 0.
    pub shininess: f32,
    /// Optional URL of a remote label / decal texture (PR #15).
    pub texture_url: Option<String>,

    // ── PR #28: PBR fields ────────────────────────────────────────────────
    /// PBR roughness 0.0 (mirror) … 1.0 (fully matte). 0.0 = use legacy
    /// `shininess` formula in the exporter.
    pub roughness: f32,
    /// PBR metalness 0.0 (dielectric) … 1.0 (metal). Food/ceramics = 0.
    pub metalness: f32,
    /// Alpha / opacity 0.0 (invisible) … 1.0 (solid). < 1.0 triggers
    /// `alphaMode: BLEND` and `KHR_materials_transmission` in the GLB.
    pub opacity: f32,
    /// `material_class` hint for the frontend shader picker.
    /// One of: `"opaque"` | `"glass"` | `"liquid"` | `"metal"` | `"ceramic"` | `"food"`.
    pub material_class: String,
}

impl Material {
    pub fn solid(name: impl Into<String>, diffuse_color: [f32; 3]) -> Self {
        Self {
            name: name.into(),
            diffuse_color,
            texture_file: None,
            specular: 0.15,
            shininess: 32.0,
            texture_url: None,
            roughness: 0.0,
            metalness: 0.0,
            opacity: 1.0,
            material_class: "opaque".to_string(),
        }
    }

    /// Builder helper — make this material glossy (used for sauces / liquids).
    pub fn with_gloss(mut self, specular: f32, shininess: f32) -> Self {
        self.specular = specular.clamp(0.0, 1.0);
        self.shininess = shininess.max(1.0);
        self
    }

    /// Set PBR roughness + metalness explicitly (PR #28).
    pub fn with_pbr(mut self, roughness: f32, metalness: f32) -> Self {
        self.roughness = roughness.clamp(0.0, 1.0);
        self.metalness = metalness.clamp(0.0, 1.0);
        self
    }

    /// Set opacity < 1.0 for glass / translucent materials (PR #28).
    pub fn with_opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity.clamp(0.0, 1.0);
        self
    }

    /// Tag this material with a class hint for the frontend (PR #28).
    pub fn with_class(mut self, class: impl Into<String>) -> Self {
        self.material_class = class.into();
        self
    }

    /// Attach a remote label / decal texture URL (PR #15). The frontend
    /// reads this via glTF `extras` and applies it as `map` on the material.
    pub fn with_texture_url(mut self, url: impl Into<String>) -> Self {
        self.texture_url = Some(url.into());
        self
    }
}

impl Default for Material {
    fn default() -> Self {
        Self {
            name: String::new(),
            diffuse_color: [0.8, 0.8, 0.8],
            texture_file: None,
            specular: 0.15,
            shininess: 32.0,
            texture_url: None,
            roughness: 0.0,
            metalness: 0.0,
            opacity: 1.0,
            material_class: "opaque".to_string(),
        }
    }
}

/// Parse a `"#RRGGBB"` hex colour string into `[f32; 3]` (0.0..1.0).
/// Falls back to `[0.8, 0.8, 0.8]` on any parse error.
pub fn hex_to_rgb(hex: &str) -> [f32; 3] {
    let hex = hex.trim().trim_start_matches('#');
    if hex.len() != 6 {
        return [0.8, 0.8, 0.8];
    }
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(200) as f32 / 255.0;
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(200) as f32 / 255.0;
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(200) as f32 / 255.0;
    [r, g, b]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_to_rgb_parses_red() {
        let [r, g, b] = hex_to_rgb("#FF0000");
        assert!((r - 1.0).abs() < 1e-4);
        assert!(g.abs() < 1e-4);
        assert!(b.abs() < 1e-4);
    }

    #[test]
    fn hex_to_rgb_fallback_on_invalid() {
        // Wrong length → fallback to [0.8, 0.8, 0.8]
        let color = hex_to_rgb("short");
        assert_eq!(color, [0.8, 0.8, 0.8]);
        // Empty → fallback
        let color2 = hex_to_rgb("");
        assert_eq!(color2, [0.8, 0.8, 0.8]);
    }
}

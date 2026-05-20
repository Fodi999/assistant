//! Типы меша: Mesh, Material, MaterialGroup, MeshPart.

/// Triangulated mesh for OBJ/GLTF export.
#[derive(Debug, Clone)]
pub struct Mesh {
    pub vertices: Vec<[f32; 3]>,
    pub normals:  Vec<[f32; 3]>,
    pub uvs:      Vec<[f32; 2]>,
    /// Legacy single-material faces (used when `groups` is empty).
    pub faces:    Vec<[usize; 3]>,
    pub material: Material,
    /// Multi-material groups; takes priority over `faces`/`material`.
    pub groups:   Vec<MaterialGroup>,
}

impl Mesh {
    pub fn new(
        vertices: Vec<[f32; 3]>,
        normals:  Vec<[f32; 3]>,
        uvs:      Vec<[f32; 2]>,
        faces:    Vec<[usize; 3]>,
        material: Material,
    ) -> Self {
        debug_assert_eq!(vertices.len(), normals.len());
        debug_assert_eq!(vertices.len(), uvs.len());
        Self { vertices, normals, uvs, faces, material, groups: Vec::new() }
    }

    pub fn new_multi(
        vertices: Vec<[f32; 3]>,
        normals:  Vec<[f32; 3]>,
        uvs:      Vec<[f32; 2]>,
        groups:   Vec<MaterialGroup>,
    ) -> Self {
        debug_assert_eq!(vertices.len(), normals.len());
        debug_assert_eq!(vertices.len(), uvs.len());
        debug_assert!(!groups.is_empty());
        let first = groups[0].clone();
        Self { vertices, normals, uvs, faces: first.faces, material: first.material, groups }
    }
}

/// One material slot (multi-material mesh).
#[derive(Debug, Clone)]
pub struct MaterialGroup {
    pub material: Material,
    pub faces:    Vec<[usize; 3]>,
}

/// Self-contained vertex/index block returned by kernel operations.
/// Can be appended into a Mesh via MeshBuilder.
#[derive(Debug, Clone)]
pub struct MeshPart {
    pub vertices: Vec<[f32; 3]>,
    pub normals:  Vec<[f32; 3]>,
    pub uvs:      Vec<[f32; 2]>,
    pub faces:    Vec<[usize; 3]>,
}

impl MeshPart {
    pub fn vertex_count(&self) -> usize { self.vertices.len() }
    pub fn face_count(&self)   -> usize { self.faces.len() }

    /// Return copy with reversed winding and negated normals.
    pub fn flipped(&self) -> Self {
        let normals = self.normals.iter().map(|n| [-n[0], -n[1], -n[2]]).collect();
        let faces   = self.faces.iter().map(|f| [f[0], f[2], f[1]]).collect();
        Self { vertices: self.vertices.clone(), normals, uvs: self.uvs.clone(), faces }
    }
}

// ── Material ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Material {
    pub name:           String,
    pub diffuse_color:  [f32; 3],
    pub texture_file:   Option<String>,
    pub specular:       f32,
    pub shininess:      f32,
    pub texture_url:    Option<String>,
    pub roughness:      f32,
    pub metalness:      f32,
    pub opacity:        f32,
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

    pub fn with_gloss(mut self, specular: f32, shininess: f32) -> Self {
        self.specular  = specular.clamp(0.0, 1.0);
        self.shininess = shininess.max(1.0);
        self
    }

    pub fn with_pbr(mut self, roughness: f32, metalness: f32) -> Self {
        self.roughness = roughness.clamp(0.0, 1.0);
        self.metalness = metalness.clamp(0.0, 1.0);
        self
    }

    pub fn with_opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity.clamp(0.0, 1.0);
        self
    }

    pub fn with_class(mut self, class: impl Into<String>) -> Self {
        self.material_class = class.into();
        self
    }

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

/// Parse `"#RRGGBB"` → `[f32; 3]` sRGB 0..1. Falls back to `[0.8,0.8,0.8]`.
pub fn hex_to_rgb(hex: &str) -> [f32; 3] {
    let hex = hex.trim().trim_start_matches('#');
    if hex.len() != 6 { return [0.8, 0.8, 0.8]; }
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(200) as f32 / 255.0;
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(200) as f32 / 255.0;
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(200) as f32 / 255.0;
    [r, g, b]
}

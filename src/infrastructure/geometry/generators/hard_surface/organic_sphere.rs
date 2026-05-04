//! Organic Sphere — ZBrush-style subdivision surface base mesh.
//!
//! A UV sphere with smooth per-vertex normals and 3 material groups:
//!   - `sphere_body`    — mid-latitudes (clay grey)
//!   - `sphere_poles`   — top/bottom caps (darker)
//!   - `sphere_equator` — equatorial band (lighter)
//!
//! Use-case: "organic" sculpt preview, contrast to the hard-surface `sci_fi_card`.
//!
//! URL: GET /api/laboratory/debug-glb/organic_sphere

use std::f32::consts::{PI, TAU};
use crate::infrastructure::geometry::mesh::{Material, MaterialGroup, Mesh};
use crate::infrastructure::geometry::kernel::GeometryQuality;

#[derive(Debug, Clone)]
pub struct OrganicSphereSpec {
    /// Sphere radius in metres. Default 0.12.
    pub radius: f32,
    /// Longitude segments. Scaled by quality.
    pub seg_lon: u32,
    /// Latitude segments. Scaled by quality.
    pub seg_lat: u32,
    /// Base body colour hex. Default "#B8B8C8" (clay grey).
    pub color_hex: String,
    pub quality: GeometryQuality,
}

impl Default for OrganicSphereSpec {
    fn default() -> Self {
        Self {
            radius: 0.12,
            seg_lon: 32,
            seg_lat: 24,
            color_hex: "#B8B8C8".to_string(),
            quality: GeometryQuality::default(),
        }
    }
}

impl OrganicSphereSpec {
    pub fn with_quality(quality: GeometryQuality) -> Self {
        let (lon, lat) = match quality {
            GeometryQuality::Draft    => (16, 12),
            GeometryQuality::Standard => (32, 24),
            GeometryQuality::High     => (48, 36),
            GeometryQuality::Ultra    => (64, 48),
        };
        Self { seg_lon: lon, seg_lat: lat, quality, ..Default::default() }
    }
}

fn hex_to_rgb(hex: &str) -> [f32; 3] {
    let h = hex.trim_start_matches('#');
    if h.len() < 6 { return [0.72, 0.72, 0.78]; }
    let r = u8::from_str_radix(&h[0..2], 16).unwrap_or(184) as f32 / 255.0;
    let g = u8::from_str_radix(&h[2..4], 16).unwrap_or(184) as f32 / 255.0;
    let b = u8::from_str_radix(&h[4..6], 16).unwrap_or(200) as f32 / 255.0;
    [r, g, b]
}

fn make_material(name: &str, color: [f32; 3], roughness: f32, metalness: f32) -> Material {
    Material {
        name: name.to_string(),
        diffuse_color: color,
        roughness,
        metalness,
        opacity: 1.0,
        material_class: "opaque".to_string(),
        texture_file: None,
        texture_url: None,
        specular: 0.08,
        shininess: 12.0,
    }
}

pub fn generate_organic_sphere(spec: &OrganicSphereSpec) -> Mesh {
    let r     = spec.radius;
    let n_lon = spec.seg_lon.max(6) as usize;
    let n_lat = spec.seg_lat.max(4) as usize;

    let col_body    = hex_to_rgb(&spec.color_hex);
    // Darker poles: multiply by 0.65
    let col_poles   = [col_body[0] * 0.65, col_body[1] * 0.65, col_body[2] * 0.80];
    // Lighter equator: add 0.10 luminance
    let col_equator = [
        (col_body[0] + 0.10).min(1.0),
        (col_body[1] + 0.08).min(1.0),
        (col_body[2] + 0.12).min(1.0),
    ];

    // ── Vertices ──────────────────────────────────────────────────────────
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut normals:   Vec<[f32; 3]> = Vec::new();
    let mut uvs:       Vec<[f32; 2]> = Vec::new();

    for lat in 0..=n_lat {
        let phi     = PI * lat as f32 / n_lat as f32;
        let sin_phi = phi.sin();
        let cos_phi = phi.cos();
        for lon in 0..=n_lon {
            let theta   = TAU * lon as f32 / n_lon as f32;
            let nx = sin_phi * theta.cos();
            let ny = cos_phi;
            let nz = sin_phi * theta.sin();
            positions.push([r * nx, r * ny, r * nz]);
            normals.push([nx, ny, nz]);
            uvs.push([lon as f32 / n_lon as f32, lat as f32 / n_lat as f32]);
        }
    }

    // ── Faces per group ───────────────────────────────────────────────────
    let stride = n_lon + 1;
    let mut body_faces:    Vec<[usize; 3]> = Vec::new();
    let mut poles_faces:   Vec<[usize; 3]> = Vec::new();
    let mut equator_faces: Vec<[usize; 3]> = Vec::new();

    for lat in 0..n_lat {
        let lat_t = lat as f32 / n_lat as f32;
        let is_pole    = lat_t < 0.15 || lat_t > 0.82;
        let is_equator = (lat_t - 0.48).abs() < 0.07;

        for lon in 0..n_lon {
            let a = lat * stride + lon;
            let b = lat * stride + lon + 1;
            let c = (lat + 1) * stride + lon;
            let d = (lat + 1) * stride + lon + 1;

            let bucket = if is_pole { &mut poles_faces }
                         else if is_equator { &mut equator_faces }
                         else { &mut body_faces };

            if lat == 0 {
                bucket.push([a, d, c]);
            } else if lat == n_lat - 1 {
                bucket.push([a, b, c]);
            } else {
                bucket.push([a, b, c]);
                bucket.push([b, d, c]);
            }
        }
    }

    // ── Assemble ──────────────────────────────────────────────────────────
    let groups = vec![
        MaterialGroup {
            material: make_material("sphere_body", col_body, 0.55, 0.04),
            faces: body_faces,
        },
        MaterialGroup {
            material: make_material("sphere_poles", col_poles, 0.70, 0.02),
            faces: poles_faces,
        },
        MaterialGroup {
            material: make_material("sphere_equator", col_equator, 0.40, 0.06),
            faces: equator_faces,
        },
    ];

    Mesh::new_multi(positions, normals, uvs, groups)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn organic_sphere_generates_mesh() {
        let spec = OrganicSphereSpec::default();
        let mesh = generate_organic_sphere(&spec);
        assert!(!mesh.vertices.is_empty(), "no vertices");
        assert!(!mesh.normals.is_empty(),  "no normals");
        assert_eq!(mesh.groups.len(), 3,   "expected 3 material groups");
        // Every group must have at least one face
        for g in &mesh.groups {
            assert!(!g.faces.is_empty(), "empty group: {}", g.material.name);
        }
    }

    #[test]
    fn organic_sphere_high_quality() {
        let spec = OrganicSphereSpec::with_quality(GeometryQuality::High);
        let mesh = generate_organic_sphere(&spec);
        assert!(mesh.vertices.len() > 1000, "high quality should have many vertices");
    }
}

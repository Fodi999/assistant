//! Flat-card generator — fallback for any product photo.
//!
//! Produces a thin rectangular box (a "card") standing upright:
//!   • width  = 0.10 m  (10 cm)
//!   • height = 0.14 m  (14 cm — ~A6 proportion)
//!   • depth  = 0.004 m (4 mm)
//!
//! The front face maps UV [0,1]² so that a `map_Kd source.png` in the MTL
//! shows the product photo on the front.
//!
//! The generator accepts an optional `color_hex` for the solid-color fallback
//! and an optional `texture_file` name (e.g. `"source.png"`).

use crate::infrastructure::geometry::mesh::{hex_to_rgb, Material, Mesh};

/// Card dimensions (metres).
const W: f32 = 0.10;
const H: f32 = 0.14;
const D: f32 = 0.004;

/// Generate a flat-card box mesh.
///
/// - `color_hex` — hex colour for the material diffuse (`"#RRGGBB"`).
/// - `texture_file` — optional texture filename for `map_Kd`.
pub fn generate(color_hex: &str, texture_file: Option<String>) -> Mesh {
    let hw = W / 2.0;
    let hh = H / 2.0;
    let hd = D / 2.0;

    // 24 unique vertices (4 per face × 6 faces) so each face has its own UVs
    // and normals without sharing corners.
    #[rustfmt::skip]
    let (vertices, normals, uvs) = box_vertices(hw, hh, hd);

    // 6 faces × 2 triangles × 3 indices = 36
    let faces = box_faces();

    let mut mat = Material::solid("product_card", hex_to_rgb(color_hex));
    mat.texture_file = texture_file;

    Mesh::new(vertices, normals, uvs, faces, mat)
}

/// Build 24 vertices for a box (4 per face, 6 faces).
/// Order: +Z (front), -Z (back), +Y (top), -Y (bottom), +X (right), -X (left).
fn box_vertices(hw: f32, hh: f32, hd: f32) -> (Vec<[f32; 3]>, Vec<[f32; 3]>, Vec<[f32; 2]>) {
    let mut verts: Vec<[f32; 3]> = Vec::with_capacity(24);
    let mut norms: Vec<[f32; 3]> = Vec::with_capacity(24);
    let mut uvs: Vec<[f32; 2]> = Vec::with_capacity(24);

    // Helper: push 4 verts for one face
    let mut face = |corners: [[f32; 3]; 4], n: [f32; 3], face_uvs: [[f32; 2]; 4]| {
        for (v, uv) in corners.iter().zip(face_uvs.iter()) {
            verts.push(*v);
            norms.push(n);
            uvs.push(*uv);
        }
    };

    // Front (+Z) — full UV for texture
    face(
        [[-hw, -hh, hd], [hw, -hh, hd], [hw, hh, hd], [-hw, hh, hd]],
        [0.0, 0.0, 1.0],
        [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
    );
    // Back (-Z)
    face(
        [[hw, -hh, -hd], [-hw, -hh, -hd], [-hw, hh, -hd], [hw, hh, -hd]],
        [0.0, 0.0, -1.0],
        [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
    );
    // Top (+Y)
    face(
        [[-hw, hh, hd], [hw, hh, hd], [hw, hh, -hd], [-hw, hh, -hd]],
        [0.0, 1.0, 0.0],
        [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]],
    );
    // Bottom (-Y)
    face(
        [[-hw, -hh, -hd], [hw, -hh, -hd], [hw, -hh, hd], [-hw, -hh, hd]],
        [0.0, -1.0, 0.0],
        [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
    );
    // Right (+X)
    face(
        [[hw, -hh, hd], [hw, -hh, -hd], [hw, hh, -hd], [hw, hh, hd]],
        [1.0, 0.0, 0.0],
        [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
    );
    // Left (-X)
    face(
        [[-hw, -hh, -hd], [-hw, -hh, hd], [-hw, hh, hd], [-hw, hh, -hd]],
        [-1.0, 0.0, 0.0],
        [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
    );

    (verts, norms, uvs)
}

/// 6 faces × 2 triangles, vertices packed in groups of 4 (quad-split).
fn box_faces() -> Vec<[usize; 3]> {
    let mut faces = Vec::with_capacity(12);
    for face_idx in 0..6_usize {
        let base = face_idx * 4;
        faces.push([base, base + 1, base + 2]);
        faces.push([base, base + 2, base + 3]);
    }
    faces
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flat_card_mesh_has_expected_counts() {
        let mesh = generate("#FF0000", None);
        assert_eq!(mesh.vertices.len(), 24);
        assert_eq!(mesh.normals.len(), 24);
        assert_eq!(mesh.uvs.len(), 24);
        assert_eq!(mesh.faces.len(), 12);
        assert_eq!(mesh.material.name, "product_card");
    }

    #[test]
    fn flat_card_with_texture_sets_texture_file() {
        let mesh = generate("#AABBCC", Some("source.png".into()));
        assert_eq!(mesh.material.texture_file.as_deref(), Some("source.png"));
    }
}

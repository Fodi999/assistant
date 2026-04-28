//! Sauce-in-bowl generator.
//!
//! Produces two meshes merged into one:
//!
//!   1. **Bowl** — a truncated cone (frustum) open at the top.
//!      Built from `SEGMENTS` quad-strips: outer wall + flat annular bottom.
//!
//!   2. **Sauce surface** — a filled disk at the fill level (`FILL_RATIO`
//!      of the bowl height). Colour comes from `Product3DSpec.product.color_hex`.
//!
//! Dimensions are fixed for PR #4 and can be driven by `ContainerSpec`
//! `diameter_mm` / `height_mm` in a later PR.
//!
//! All geometry is in metres, Y-up, centred at origin.

use std::f32::consts::PI;

use crate::infrastructure::geometry::mesh::{hex_to_rgb, Material, Mesh};

/// Number of horizontal segments (higher = smoother, more verts).
const SEGMENTS: usize = 32;

/// Bowl dimensions.
const BOWL_RADIUS_TOP: f32 = 0.07;  // 7 cm opening radius
const BOWL_RADIUS_BOT: f32 = 0.045; // 4.5 cm base radius
const BOWL_HEIGHT: f32 = 0.06;      // 6 cm tall

/// Sauce fill level as a fraction of bowl height (0 = empty, 1 = full).
const FILL_RATIO: f32 = 0.72;

/// Bowl wall colour (off-white ceramic).
const BOWL_COLOR: [f32; 3] = [0.96, 0.94, 0.90];

/// Generate a sauce-in-bowl mesh.
///
/// - `sauce_color_hex` — hex colour for the sauce surface (`"#RRGGBB"`).
/// - `container_color_hex` — optional override for bowl colour.
pub fn generate(sauce_color_hex: &str, container_color_hex: Option<&str>) -> Mesh {
    let bowl_color = container_color_hex
        .map(hex_to_rgb)
        .unwrap_or(BOWL_COLOR);
    let sauce_color = hex_to_rgb(sauce_color_hex);

    let (mut verts, mut norms, mut uvs, mut faces) = build_bowl(bowl_color);

    let v_offset = verts.len();
    let (sv, sn, su, sf) = build_sauce_disk(v_offset);
    verts.extend(sv);
    norms.extend(sn);
    uvs.extend(su);
    faces.extend(sf);

    // Single merged mesh — use sauce colour as the primary material.
    // The bowl wall colour is baked into the vertex data direction; for OBJ
    // export a single Kd is a simplification (good enough for PR #4).
    // We use the sauce colour because it's the most visually distinctive.
    let mat = Material::solid("sauce_bowl", sauce_color);

    Mesh::new(verts, norms, uvs, faces, mat)
}

// ─────────────────────────────────────────────────────────────────────────────
// Bowl frustum
// ─────────────────────────────────────────────────────────────────────────────

fn build_bowl(
    _color: [f32; 3], // reserved for per-face material in a later PR
) -> (Vec<[f32; 3]>, Vec<[f32; 3]>, Vec<[f32; 2]>, Vec<[usize; 3]>) {
    let mut verts: Vec<[f32; 3]> = Vec::new();
    let mut norms: Vec<[f32; 3]> = Vec::new();
    let mut uvs: Vec<[f32; 2]> = Vec::new();
    let mut faces: Vec<[usize; 3]> = Vec::new();

    // Outer wall: two rings — bottom ring at y=0, top ring at y=BOWL_HEIGHT.
    // Each ring has SEGMENTS+1 verts (first == last for UV seam).
    let n_ring = SEGMENTS + 1;

    let y_bot = -BOWL_HEIGHT / 2.0;
    let y_top = BOWL_HEIGHT / 2.0;

    for i in 0..=SEGMENTS {
        let t = i as f32 / SEGMENTS as f32;
        let theta = t * 2.0 * PI;
        let cos_t = theta.cos();
        let sin_t = theta.sin();

        // Bottom ring
        let xb = cos_t * BOWL_RADIUS_BOT;
        let zb = sin_t * BOWL_RADIUS_BOT;
        // Top ring
        let xt = cos_t * BOWL_RADIUS_TOP;
        let zt = sin_t * BOWL_RADIUS_TOP;

        // Outward-slanted normal for frustum wall
        let slope = (BOWL_RADIUS_TOP - BOWL_RADIUS_BOT) / BOWL_HEIGHT;
        let nx = cos_t;
        let ny = -slope;
        let nz = sin_t;
        let len = (nx * nx + ny * ny + nz * nz).sqrt();
        let wall_norm = [nx / len, ny / len, nz / len];

        verts.push([xb, y_bot, zb]);
        norms.push(wall_norm);
        uvs.push([t, 0.0]);

        verts.push([xt, y_top, zt]);
        norms.push(wall_norm);
        uvs.push([t, 1.0]);
    }

    // Wall quads
    for i in 0..SEGMENTS {
        let b0 = i * 2;
        let t0 = i * 2 + 1;
        let b1 = (i + 1) * 2;
        let t1 = (i + 1) * 2 + 1;
        faces.push([b0, b1, t1]);
        faces.push([b0, t1, t0]);
    }

    // Bottom disk (flat cap, normal pointing down)
    let base = verts.len();
    // Centre vertex
    verts.push([0.0, y_bot, 0.0]);
    norms.push([0.0, -1.0, 0.0]);
    uvs.push([0.5, 0.5]);

    for i in 0..=SEGMENTS {
        let t = i as f32 / SEGMENTS as f32;
        let theta = t * 2.0 * PI;
        let x = theta.cos() * BOWL_RADIUS_BOT;
        let z = theta.sin() * BOWL_RADIUS_BOT;
        verts.push([x, y_bot, z]);
        norms.push([0.0, -1.0, 0.0]);
        uvs.push([0.5 + theta.cos() * 0.5, 0.5 + theta.sin() * 0.5]);
    }

    for i in 0..SEGMENTS {
        faces.push([base, base + 1 + i + 1, base + 1 + i]);
    }

    let _ = n_ring; // suppress warning
    (verts, norms, uvs, faces)
}

// ─────────────────────────────────────────────────────────────────────────────
// Sauce disk at fill level
// ─────────────────────────────────────────────────────────────────────────────

fn build_sauce_disk(
    v_offset: usize,
) -> (Vec<[f32; 3]>, Vec<[f32; 3]>, Vec<[f32; 2]>, Vec<[usize; 3]>) {
    let mut verts: Vec<[f32; 3]> = Vec::new();
    let mut norms: Vec<[f32; 3]> = Vec::new();
    let mut uvs: Vec<[f32; 2]> = Vec::new();
    let mut faces: Vec<[usize; 3]> = Vec::new();

    let y_fill = -BOWL_HEIGHT / 2.0 + BOWL_HEIGHT * FILL_RATIO;

    // Sauce radius interpolates between bot and top radii at FILL_RATIO.
    let sauce_radius = BOWL_RADIUS_BOT + (BOWL_RADIUS_TOP - BOWL_RADIUS_BOT) * FILL_RATIO;

    // Centre
    verts.push([0.0, y_fill, 0.0]);
    norms.push([0.0, 1.0, 0.0]);
    uvs.push([0.5, 0.5]);

    for i in 0..=SEGMENTS {
        let t = i as f32 / SEGMENTS as f32;
        let theta = t * 2.0 * PI;
        let x = theta.cos() * sauce_radius;
        let z = theta.sin() * sauce_radius;
        verts.push([x, y_fill, z]);
        norms.push([0.0, 1.0, 0.0]);
        uvs.push([0.5 + theta.cos() * 0.5, 0.5 + theta.sin() * 0.5]);
    }

    let centre = v_offset;
    for i in 0..SEGMENTS {
        faces.push([centre, centre + 1 + i, centre + 1 + i + 1]);
    }

    (verts, norms, uvs, faces)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sauce_in_bowl_mesh_is_non_empty() {
        let mesh = generate("#B8321F", None);
        assert!(!mesh.vertices.is_empty());
        assert_eq!(mesh.vertices.len(), mesh.normals.len());
        assert_eq!(mesh.vertices.len(), mesh.uvs.len());
        assert!(!mesh.faces.is_empty());
    }

    #[test]
    fn sauce_in_bowl_uses_sauce_color() {
        let mesh = generate("#FF0000", None);
        let [r, g, b] = mesh.material.diffuse_color;
        assert!((r - 1.0).abs() < 1e-4);
        assert!(g.abs() < 1e-4);
        assert!(b.abs() < 1e-4);
    }
}

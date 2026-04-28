//! Sauce-in-bowl generator.
//!
//! Produces a single mesh with **two material groups** (PR #6):
//!
//!   1. **Bowl** — a truncated cone (frustum) open at the top, off-white
//!      ceramic. Built from `SEGMENTS` quad-strips: outer wall + flat
//!      annular bottom.
//!
//!   2. **Sauce surface** — a filled disk at the fill level (`FILL_RATIO`
//!      of the bowl height) with a **swirl relief** (sin-based displacement
//!      following a logarithmic spiral). Colour comes from
//!      `Product3DSpec.product.color_hex`. Glossy material.
//!
//! Dimensions are fixed for now and can be driven by `ContainerSpec`
//! `diameter_mm` / `height_mm` in a later PR.
//!
//! All geometry is in metres, Y-up, centred at origin.

use std::f32::consts::PI;

use crate::infrastructure::geometry::mesh::{hex_to_rgb, Material, MaterialGroup, Mesh};

/// Number of horizontal segments (higher = smoother, more verts).
const SEGMENTS: usize = 32;
/// Number of radial rings on the sauce surface (for swirl relief).
const SAUCE_RINGS: usize = 12;

/// Bowl dimensions.
const BOWL_RADIUS_TOP: f32 = 0.07;  // 7 cm opening radius
const BOWL_RADIUS_BOT: f32 = 0.045; // 4.5 cm base radius
const BOWL_HEIGHT: f32 = 0.06;      // 6 cm tall

/// Sauce fill level as a fraction of bowl height (0 = empty, 1 = full).
const FILL_RATIO: f32 = 0.72;

/// Swirl displacement amplitude (metres). Small relative to bowl height.
const SWIRL_AMPLITUDE: f32 = 0.0025;
/// Number of full swirl arms.
const SWIRL_ARMS: f32 = 3.0;

/// Bowl wall colour (off-white ceramic).
const BOWL_COLOR: [f32; 3] = [0.96, 0.94, 0.90];

/// Generate a sauce-in-bowl mesh with two material groups (bowl + sauce).
///
/// - `sauce_color_hex` — hex colour for the sauce surface (`"#RRGGBB"`).
/// - `container_color_hex` — optional override for bowl colour.
pub fn generate(sauce_color_hex: &str, container_color_hex: Option<&str>) -> Mesh {
    let bowl_color = container_color_hex
        .map(hex_to_rgb)
        .unwrap_or(BOWL_COLOR);
    let sauce_color = hex_to_rgb(sauce_color_hex);

    // Build bowl geometry → its faces become group #0
    let (mut verts, mut norms, mut uvs, bowl_faces) = build_bowl();

    // Build sauce surface — vertices appended into the same vertex array,
    // its faces (with offset already applied) become group #1.
    let v_offset = verts.len();
    let (sv, sn, su, sauce_faces) = build_sauce_surface(v_offset);
    verts.extend(sv);
    norms.extend(sn);
    uvs.extend(su);

    let bowl_material = Material::solid("bowl_material", bowl_color)
        // ceramic — slightly soft highlight
        .with_gloss(0.10, 24.0);
    let sauce_material = Material::solid("sauce_material", sauce_color)
        // glossy sauce — strong highlight
        .with_gloss(0.55, 96.0);

    Mesh::new_multi(
        verts,
        norms,
        uvs,
        vec![
            MaterialGroup { material: bowl_material, faces: bowl_faces },
            MaterialGroup { material: sauce_material, faces: sauce_faces },
        ],
    )
}

// ─────────────────────────────────────────────────────────────────────────────
// Bowl frustum
// ─────────────────────────────────────────────────────────────────────────────

fn build_bowl() -> (
    Vec<[f32; 3]>,
    Vec<[f32; 3]>,
    Vec<[f32; 2]>,
    Vec<[usize; 3]>,
) {
    let mut verts: Vec<[f32; 3]> = Vec::new();
    let mut norms: Vec<[f32; 3]> = Vec::new();
    let mut uvs: Vec<[f32; 2]> = Vec::new();
    let mut faces: Vec<[usize; 3]> = Vec::new();

    let y_bot = -BOWL_HEIGHT / 2.0;
    let y_top = BOWL_HEIGHT / 2.0;

    for i in 0..=SEGMENTS {
        let t = i as f32 / SEGMENTS as f32;
        let theta = t * 2.0 * PI;
        let cos_t = theta.cos();
        let sin_t = theta.sin();

        let xb = cos_t * BOWL_RADIUS_BOT;
        let zb = sin_t * BOWL_RADIUS_BOT;
        let xt = cos_t * BOWL_RADIUS_TOP;
        let zt = sin_t * BOWL_RADIUS_TOP;

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

    for i in 0..SEGMENTS {
        let b0 = i * 2;
        let t0 = i * 2 + 1;
        let b1 = (i + 1) * 2;
        let t1 = (i + 1) * 2 + 1;
        faces.push([b0, b1, t1]);
        faces.push([b0, t1, t0]);
    }

    // Bottom disk
    let base = verts.len();
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

    (verts, norms, uvs, faces)
}

// ─────────────────────────────────────────────────────────────────────────────
// Sauce surface with swirl relief
// ─────────────────────────────────────────────────────────────────────────────
//
// We tessellate the disk into concentric rings (`SAUCE_RINGS`) and angular
// segments (`SEGMENTS`). Each ring vertex is displaced in Y by:
//
//   dy = SWIRL_AMPLITUDE * sin(SWIRL_ARMS * theta + 2π * radius_ratio)
//
// This creates a logarithmic-spiral-style ridge pattern — a clear visual
// signature of "sauce", not just a flat disk.

fn build_sauce_surface(
    v_offset: usize,
) -> (
    Vec<[f32; 3]>,
    Vec<[f32; 3]>,
    Vec<[f32; 2]>,
    Vec<[usize; 3]>,
) {
    let mut verts: Vec<[f32; 3]> = Vec::new();
    let mut norms: Vec<[f32; 3]> = Vec::new();
    let mut uvs: Vec<[f32; 2]> = Vec::new();
    let mut faces: Vec<[usize; 3]> = Vec::new();

    let y_fill = -BOWL_HEIGHT / 2.0 + BOWL_HEIGHT * FILL_RATIO + 0.002;
    // Shrink the sauce disk inward so its rim never z-fights with the bowl
    // wall (PR #9 fix). 0.92 leaves a clean ~5 mm visible ring of ceramic.
    let sauce_radius =
        (BOWL_RADIUS_BOT + (BOWL_RADIUS_TOP - BOWL_RADIUS_BOT) * FILL_RATIO) * 0.92;

    // Centre vertex (radius 0) — apex of the swirl, undisplaced.
    verts.push([0.0, y_fill, 0.0]);
    norms.push([0.0, 1.0, 0.0]);
    uvs.push([0.5, 0.5]);

    // Rings 1..=SAUCE_RINGS
    for ring in 1..=SAUCE_RINGS {
        let r_ratio = ring as f32 / SAUCE_RINGS as f32;
        let r = sauce_radius * r_ratio;

        for seg in 0..=SEGMENTS {
            let t = seg as f32 / SEGMENTS as f32;
            let theta = t * 2.0 * PI;
            let cos_t = theta.cos();
            let sin_t = theta.sin();

            let x = cos_t * r;
            let z = sin_t * r;

            // Swirl: displacement decays slightly toward the rim so the
            // edge stays in contact with the bowl wall.
            let edge_falloff = 1.0 - (r_ratio - 0.85).max(0.0) / 0.15;
            let dy = SWIRL_AMPLITUDE
                * (SWIRL_ARMS * theta + r_ratio * 2.0 * PI).sin()
                * edge_falloff.clamp(0.0, 1.0);

            let y = y_fill + dy;
            verts.push([x, y, z]);

            // Approximate normal — we tilt slightly with the gradient of dy.
            // Cheap analytic gradient (good enough for shading).
            let dtheta = SWIRL_AMPLITUDE
                * SWIRL_ARMS
                * (SWIRL_ARMS * theta + r_ratio * 2.0 * PI).cos()
                * edge_falloff.clamp(0.0, 1.0);
            // Tangent in the angular direction has length r (per radian); we
            // treat dy/dtheta divided by r as the tangential slope.
            let slope_t = if r > 1e-5 { dtheta / r } else { 0.0 };
            // Build a quick tilted normal: start from up and lean opposite
            // to the tangential gradient.
            let nx = -slope_t * (-sin_t);
            let nz = -slope_t * cos_t;
            let ny = 1.0;
            let len = (nx * nx + ny * ny + nz * nz).sqrt();
            norms.push([nx / len, ny / len, nz / len]);

            uvs.push([0.5 + cos_t * 0.5 * r_ratio, 0.5 + sin_t * 0.5 * r_ratio]);
        }
    }

    // Triangulate.
    // Indices: centre = 0 (local). First ring starts at local idx 1.
    // Each ring has SEGMENTS+1 verts.
    let centre_local = 0usize;
    let ring_size = SEGMENTS + 1;

    // Inner fan: centre → ring 1
    for seg in 0..SEGMENTS {
        let a = 1 + seg;
        let b = 1 + seg + 1;
        faces.push([
            v_offset + centre_local,
            v_offset + a,
            v_offset + b,
        ]);
    }

    // Quads between consecutive rings
    for ring in 1..SAUCE_RINGS {
        let inner_start = 1 + (ring - 1) * ring_size;
        let outer_start = 1 + ring * ring_size;
        for seg in 0..SEGMENTS {
            let i0 = inner_start + seg;
            let i1 = inner_start + seg + 1;
            let o0 = outer_start + seg;
            let o1 = outer_start + seg + 1;
            faces.push([v_offset + i0, v_offset + o0, v_offset + o1]);
            faces.push([v_offset + i0, v_offset + o1, v_offset + i1]);
        }
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
        assert_eq!(mesh.groups.len(), 2, "expect bowl + sauce groups");
        assert!(!mesh.groups[0].faces.is_empty());
        assert!(!mesh.groups[1].faces.is_empty());
    }

    #[test]
    fn sauce_in_bowl_uses_sauce_color_in_sauce_group() {
        let mesh = generate("#FF0000", None);
        let sauce_group = mesh
            .groups
            .iter()
            .find(|g| g.material.name == "sauce_material")
            .expect("sauce_material group should exist");
        let [r, g, b] = sauce_group.material.diffuse_color;
        assert!((r - 1.0).abs() < 1e-4);
        assert!(g.abs() < 1e-4);
        assert!(b.abs() < 1e-4);
    }

    #[test]
    fn bowl_group_uses_default_ceramic_color_when_no_override() {
        let mesh = generate("#FF0000", None);
        let bowl = mesh
            .groups
            .iter()
            .find(|g| g.material.name == "bowl_material")
            .expect("bowl_material group should exist");
        // Off-white ceramic, all channels > 0.85
        let [r, g, b] = bowl.material.diffuse_color;
        assert!(r > 0.85 && g > 0.85 && b > 0.85);
    }

    #[test]
    fn sauce_surface_has_swirl_displacement() {
        // At least one sauce vertex must be off the flat fill plane.
        let mesh = generate("#FF0000", None);
        let y_fill = -BOWL_HEIGHT / 2.0 + BOWL_HEIGHT * FILL_RATIO;

        // Collect indices used by the sauce group.
        let sauce = mesh
            .groups
            .iter()
            .find(|g| g.material.name == "sauce_material")
            .unwrap();
        let mut sauce_indices: std::collections::HashSet<usize> =
            std::collections::HashSet::new();
        for [a, b, c] in &sauce.faces {
            sauce_indices.insert(*a);
            sauce_indices.insert(*b);
            sauce_indices.insert(*c);
        }
        let any_displaced = sauce_indices
            .iter()
            .any(|&i| (mesh.vertices[i][1] - y_fill).abs() > 1e-5);
        assert!(any_displaced, "sauce surface should have swirl relief");
    }
}

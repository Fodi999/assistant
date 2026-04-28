//! Plate-food generator (PR #14).
//!
//! A shallow ceramic plate with an irregular food mound on top. Two
//! material groups (PR #14 scope — garnish kept for a future PR):
//!
//!   1. **`plate_material`** — lathed plate built from foot ring → flat
//!      base → shallow rise → rolled rim. Same ceramic upgrade rules as the
//!      bowl (`*plate*|*ceramic*` triggers ceramic shading on the frontend
//!      via the existing `bowl|ceramic` rule, see note below).
//!   2. **`product_material`** — radial heightfield mound on top of the
//!      plate. Height is `BASE_DOME * (1 - r²) + small low-frequency noise`
//!      so the centre is the highest point and the edge tapers to the
//!      plate surface; tiny angular ripples make it look like food, not a
//!      perfect dome.
//!
//! Frontend material rules (`ModelViewer.tsx`):
//!   * `*plate*` does **not** match the ceramic rule yet — the `classify()`
//!     function looks for `bowl` / `ceramic`. We name the plate material
//!     `plate_ceramic` so the existing `ceramic` keyword catches it without
//!     a frontend change.
//!   * `*product*` → glossy diffuse (already handled).
//!
//! Y-up, centred at origin, all units in metres.

use std::f32::consts::PI;

use crate::infrastructure::geometry::kernel::{
    disk_fan_down, lathe_profile, MeshBuilder, Profile, ProfilePoint,
};
use crate::infrastructure::geometry::mesh::{hex_to_rgb, Material, Mesh};

const SEGMENTS: usize = 64;          // plate is wide → more slices help
const MOUND_RINGS: usize = 14;       // radial resolution of the food mound
const MOUND_SEGMENTS: usize = 48;    // angular resolution of the food mound

// ── Plate dimensions (metres) ───────────────────────────────────────────────
const PLATE_FOOT_INNER: f32 = 0.030;
const PLATE_FOOT_OUTER: f32 = 0.045;
const PLATE_BASE_RADIUS: f32 = 0.090;
const PLATE_RISE_RADIUS: f32 = 0.105;
const PLATE_RIM_OUTER: f32 = 0.112;
const PLATE_RIM_INNER: f32 = 0.104;

const Y_FOOT_BOTTOM: f32 = -0.010;
const Y_FOOT_TOP: f32 = -0.008;
const Y_BASE: f32 = -0.004;
const Y_RISE: f32 = 0.002;
const Y_RIM_TOP: f32 = 0.006;
const Y_RIM_INNER: f32 = 0.008;

/// Top of the plate where the food sits — slightly above the base disk.
const PLATE_TOP_Y: f32 = Y_BASE + 0.0008;
/// Maximum food radius — tucked just inside the rim's inner edge.
const FOOD_MAX_RADIUS: f32 = PLATE_RIM_INNER - 0.006;
/// Apex height of the food dome (above `PLATE_TOP_Y`).
const FOOD_DOME: f32 = 0.012;
/// Amplitude of the angular noise wobble.
const FOOD_NOISE: f32 = 0.0020;

// ── Default colours ─────────────────────────────────────────────────────────
const PLATE_DEFAULT_COLOR: [f32; 3] = [0.96, 0.94, 0.90];

/// Generate a plate-food mesh.
///
/// - `product_color_hex` — hex colour of the food (`product.color_hex`).
/// - `plate_color_hex` — optional override for the plate.
pub fn generate(product_color_hex: &str, plate_color_hex: Option<&str>) -> Mesh {
    let product_color = hex_to_rgb(product_color_hex);
    let plate_color = plate_color_hex.map(hex_to_rgb).unwrap_or(PLATE_DEFAULT_COLOR);

    let mut b = MeshBuilder::new();

    // Group 1 — plate (ceramic). The name contains `ceramic` so the existing
    // frontend `bowl|ceramic` upgrade rule picks it up as opaque ceramic
    // instead of glass / liquid.
    let plate_g = b.add_group(
        Material::solid("plate_ceramic", plate_color).with_gloss(0.10, 24.0),
    );
    // Group 2 — food mound.
    let food_g = b.add_group(
        Material::solid("product_material", product_color).with_gloss(0.45, 64.0),
    );

    // ── Plate via lathe ─────────────────────────────────────────────────────
    let plate_profile = Profile::new(vec![
        ProfilePoint::new(PLATE_FOOT_INNER, Y_FOOT_BOTTOM),
        ProfilePoint::new(PLATE_FOOT_OUTER, Y_FOOT_TOP),
        ProfilePoint::new(PLATE_BASE_RADIUS, Y_BASE),
        ProfilePoint::new(PLATE_RISE_RADIUS, Y_RISE),
        ProfilePoint::new(PLATE_RIM_OUTER, Y_RIM_TOP),
        ProfilePoint::new(PLATE_RIM_INNER, Y_RIM_INNER),
    ])
    .expect("hard-coded plate profile is valid");
    let plate_wall = lathe_profile(&plate_profile, SEGMENTS).expect("lathe plate");
    b.add_part(plate_g, &plate_wall);

    // Underside of the foot (small disk facing down so the plate is closed).
    let foot_disk =
        disk_fan_down(PLATE_FOOT_INNER, Y_FOOT_BOTTOM, SEGMENTS).expect("foot disk");
    b.add_part(plate_g, &foot_disk);

    // ── Food mound via radial heightfield ───────────────────────────────────
    add_food_mound(&mut b, food_g);

    b.build()
}

// ─────────────────────────────────────────────────────────────────────────────
// Food mound — concentric rings + angular displacement.
//
// Height field:
//   r_ratio = r / FOOD_MAX_RADIUS
//   dome    = FOOD_DOME * (1 - r_ratio²)         // smooth bell, 0 at edge
//   noise   = FOOD_NOISE * sin(3θ + 8 r_ratio) * sin(7θ)
//   y       = PLATE_TOP_Y + dome + noise * smoothstep(r_ratio, 0.1, 0.9)
//
// The smoothstep zeros the noise both at the apex (so the centre stays the
// highest point) and at the rim (so the mound smoothly meets the plate).
// ─────────────────────────────────────────────────────────────────────────────
fn add_food_mound(b: &mut MeshBuilder, group: usize) {
    // Centre vertex — the highest point.
    let centre = b.add_vertex(
        [0.0, PLATE_TOP_Y + FOOD_DOME, 0.0],
        [0.0, 1.0, 0.0],
        [0.5, 0.5],
    );

    let ring_size = MOUND_SEGMENTS + 1;
    let first_ring_v = centre + 1;

    // Build all ring vertices.
    for ring in 1..=MOUND_RINGS {
        let r_ratio = ring as f32 / MOUND_RINGS as f32;
        let r = FOOD_MAX_RADIUS * r_ratio;

        // Smoothstep window for the noise: full strength in the middle, zero
        // at apex and rim. Hand-rolled smoothstep clamp.
        let edge_lo = smoothstep(0.10, 0.30, r_ratio);
        let edge_hi = 1.0 - smoothstep(0.85, 1.00, r_ratio);
        let noise_w = (edge_lo * edge_hi).clamp(0.0, 1.0);

        for seg in 0..=MOUND_SEGMENTS {
            let t = seg as f32 / MOUND_SEGMENTS as f32;
            let theta = t * 2.0 * PI;
            let cos_t = theta.cos();
            let sin_t = theta.sin();

            let dome = FOOD_DOME * (1.0 - r_ratio * r_ratio);
            let noise = FOOD_NOISE
                * (3.0 * theta + 8.0 * r_ratio).sin()
                * (7.0 * theta).sin()
                * noise_w;
            let y = PLATE_TOP_Y + dome + noise;

            // Approximate normal: gradient of (dome + noise) in (r, θ) space.
            // dr/dr   = -2 * FOOD_DOME * r_ratio / FOOD_MAX_RADIUS
            // dnoise/dθ component handled approximately.
            let d_y_d_r = -2.0 * FOOD_DOME * r_ratio / FOOD_MAX_RADIUS.max(1e-6);
            let d_noise_d_theta = FOOD_NOISE
                * (3.0 * (3.0 * theta + 8.0 * r_ratio).cos() * (7.0 * theta).sin()
                    + 7.0 * (3.0 * theta + 8.0 * r_ratio).sin() * (7.0 * theta).cos())
                * noise_w;
            let slope_radial = d_y_d_r;
            let slope_tangential = if r > 1e-5 {
                d_noise_d_theta / r
            } else {
                0.0
            };

            // Surface normal: start from up (0,1,0), tilt against gradients.
            // Tangent in radial direction is (cos_t, slope_radial, sin_t);
            // tangent in angular direction is (-sin_t, slope_tangential, cos_t).
            // Cross product (radial × tangential) gives outward normal.
            let tr = [cos_t, slope_radial, sin_t];
            let tt = [-sin_t, slope_tangential, cos_t];
            let nx = tr[1] * tt[2] - tr[2] * tt[1];
            let ny = tr[2] * tt[0] - tr[0] * tt[2];
            let nz = tr[0] * tt[1] - tr[1] * tt[0];
            let len = (nx * nx + ny * ny + nz * nz).sqrt().max(1e-6);

            b.add_vertex(
                [cos_t * r, y, sin_t * r],
                [nx / len, ny / len, nz / len],
                [0.5 + cos_t * 0.5 * r_ratio, 0.5 + sin_t * 0.5 * r_ratio],
            );
        }
    }

    // Inner fan: centre → ring 1.
    for seg in 0..MOUND_SEGMENTS {
        let a = first_ring_v + seg;
        let bb = first_ring_v + seg + 1;
        b.add_triangle(group, centre, a, bb);
    }

    // Quads between consecutive rings.
    for ring in 1..MOUND_RINGS {
        let inner_start = first_ring_v + (ring - 1) * ring_size;
        let outer_start = first_ring_v + ring * ring_size;
        for seg in 0..MOUND_SEGMENTS {
            let i0 = inner_start + seg;
            let i1 = inner_start + seg + 1;
            let o0 = outer_start + seg;
            let o1 = outer_start + seg + 1;
            b.add_triangle(group, i0, o0, o1);
            b.add_triangle(group, i0, o1, i1);
        }
    }
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::geometry::kernel::validate_mesh;

    #[test]
    fn plate_food_mesh_is_non_empty() {
        let mesh = generate("#A85B12", None);
        assert!(!mesh.vertices.is_empty());
        assert_eq!(mesh.vertices.len(), mesh.normals.len());
        assert_eq!(mesh.vertices.len(), mesh.uvs.len());
        for g in &mesh.groups {
            assert!(!g.faces.is_empty(), "group {} has no faces", g.material.name);
        }
    }

    #[test]
    fn plate_food_has_plate_and_product_groups() {
        let mesh = generate("#A85B12", None);
        assert_eq!(mesh.groups.len(), 2, "plate + product");
        assert!(mesh
            .groups
            .iter()
            .any(|g| g.material.name == "plate_ceramic"));
        assert!(mesh
            .groups
            .iter()
            .any(|g| g.material.name == "product_material"));
    }

    #[test]
    fn plate_food_uses_product_color() {
        let mesh = generate("#FF0000", None);
        let product = mesh
            .groups
            .iter()
            .find(|g| g.material.name == "product_material")
            .expect("product_material group missing");
        let [r, g, b] = product.material.diffuse_color;
        assert!((r - 1.0).abs() < 1e-4);
        assert!(g.abs() < 1e-4);
        assert!(b.abs() < 1e-4);
    }

    #[test]
    fn plate_food_plate_color_override_applies() {
        let mesh = generate("#A85B12", Some("#00FF00"));
        let plate = mesh
            .groups
            .iter()
            .find(|g| g.material.name == "plate_ceramic")
            .unwrap();
        let [r, g, b] = plate.material.diffuse_color;
        assert!(r.abs() < 1e-4);
        assert!((g - 1.0).abs() < 1e-4);
        assert!(b.abs() < 1e-4);
    }

    #[test]
    fn plate_food_passes_kernel_validation() {
        let mesh = generate("#A85B12", Some("#EEEEEE"));
        validate_mesh(&mesh).expect("kernel validation should pass on plate mesh");
    }

    #[test]
    fn plate_food_mound_has_height_variation() {
        // Collect just the food vertices (those touched by product faces).
        let mesh = generate("#A85B12", None);
        let food = mesh
            .groups
            .iter()
            .find(|g| g.material.name == "product_material")
            .unwrap();
        let mut indices = std::collections::HashSet::new();
        for [a, b, c] in &food.faces {
            indices.insert(*a);
            indices.insert(*b);
            indices.insert(*c);
        }
        let (mut min_y, mut max_y) = (f32::INFINITY, f32::NEG_INFINITY);
        for i in indices {
            let y = mesh.vertices[i][1];
            if y < min_y {
                min_y = y;
            }
            if y > max_y {
                max_y = y;
            }
        }
        // Apex at centre is FOOD_DOME above PLATE_TOP_Y; rim sits at PLATE_TOP_Y.
        // Total variation must clearly exceed 2 mm.
        assert!(
            max_y - min_y > 0.002,
            "food mound should have >2mm height variation (got {})",
            max_y - min_y
        );
    }

    #[test]
    fn plate_food_widest_radius_is_plate_rim() {
        let mesh = generate("#A85B12", None);
        let mut widest_r: f32 = 0.0;
        let mut widest_y: f32 = 0.0;
        for v in &mesh.vertices {
            let r = (v[0] * v[0] + v[2] * v[2]).sqrt();
            if r > widest_r {
                widest_r = r;
                widest_y = v[1];
            }
        }
        assert!((widest_r - PLATE_RIM_OUTER).abs() < 1e-4);
        assert!((widest_y - Y_RIM_TOP).abs() < 1e-4);
    }

    #[test]
    fn plate_food_food_apex_above_plate_surface() {
        // Highest point in the mesh must clearly rise above the plate
        // surface (food dome present). Noise can bump an off-centre vertex
        // slightly above the analytical apex, so we allow a small tolerance.
        let mesh = generate("#A85B12", None);
        let max_y = mesh
            .vertices
            .iter()
            .map(|v| v[1])
            .fold(f32::NEG_INFINITY, f32::max);
        let expected_apex = PLATE_TOP_Y + FOOD_DOME;
        assert!(
            max_y > expected_apex - 0.001,
            "apex should be at least PLATE_TOP_Y + FOOD_DOME - 1mm, got {max_y}"
        );
        assert!(
            max_y < expected_apex + FOOD_NOISE * 1.5,
            "apex should not exceed dome + noise headroom, got {max_y}"
        );
        // And it must be above the plate rim.
        assert!(max_y > Y_RIM_TOP, "food apex must rise above plate rim");
    }
}

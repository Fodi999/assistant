//! Plate-food generator (PR #14 — base, PR #30 — Vision Surface Spec + PBR + food patterns).
//!
//! Material groups:
//!
//!   1. **`plate_ceramic`** — lathed plate: foot ring → flat base → shallow rise → rolled rim.
//!   2. **`food_material`** — radial heightfield mound driven by [`ProductSurfaceSpec`].
//!      Supports four patterns:
//!        * `"smooth_mound"` / `"mound"` / `"flat"` — bell dome (purée, hummus, risotto)
//!        * `"chunky"` / `"chunky_mound"` / `"waves"` — angular high-frequency noise (salad, potatoes)
//!        * `"swirl"` / `"spiral_swirl"` — low ridge-based swirl borrowed from sauce_in_bowl
//!        * default / unknown — smooth mound with moderate noise
//!
//! Y-up, centred at origin, all units in metres.

use std::f32::consts::PI;

use crate::application::laboratory_v2::ProductSurfaceSpec;
use crate::infrastructure::geometry::kernel::{
    disk_fan_down, lathe_profile, GeometryQuality, MeshBuilder, Profile, ProfilePoint,
};
use crate::infrastructure::geometry::mesh::{hex_to_rgb, Material, Mesh};

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
/// Default apex height of the food dome (above `PLATE_TOP_Y`).
const FOOD_DOME: f32 = 0.012;
/// Amplitude of the angular noise wobble.
const FOOD_NOISE: f32 = 0.0020;

// ── Default colours ─────────────────────────────────────────────────────────
const PLATE_DEFAULT_COLOR: [f32; 3] = [0.96, 0.94, 0.90];

// ─────────────────────────────────────────────────────────────────────────────
// PR #30 — Food Mound Parameters
// ─────────────────────────────────────────────────────────────────────────────

/// Controls the shape of the food mound.
#[derive(Debug, Clone, Copy)]
pub struct FoodMoundParams {
    /// Which algorithm to use.
    pub pattern: FoodPattern,
    /// Apex height in metres above the plate surface.
    pub apex_m: f32,
    /// Fraction of FOOD_MAX_RADIUS the mound actually fills (0.6–1.0).
    pub spread_ratio: f32,
    /// Organic noise amplitude multiplier (0.0–1.0).
    pub irregularity: f32,
    /// Number of angular high-frequency noise harmonics (chunky pattern).
    pub chunk_freq: f32,
    /// PBR roughness for the food material.
    pub roughness: f32,
    /// PBR gloss/specular (for things like sauce on top or rice grains).
    pub gloss: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FoodPattern {
    SmoothMound,  // purée, hummus, cream, risotto
    ChunkyMound,  // salad, roasted veg, potatoes, grains
    SwirlMound,   // sauce drizzled on plate, mashed-with-swirl
    FlatSpread,   // flatbread, pizza, thin galette
}

impl Default for FoodMoundParams {
    fn default() -> Self {
        Self {
            pattern: FoodPattern::SmoothMound,
            apex_m: FOOD_DOME,
            spread_ratio: 0.88,
            irregularity: 0.25,
            chunk_freq: 5.0,
            roughness: 0.55,
            gloss: 0.30,
        }
    }
}

/// Map an optional [`ProductSurfaceSpec`] (from Gemini Vision) to concrete
/// food mound parameters. Falls back to sensible defaults when `None`.
pub fn mound_params_from_spec(surface: Option<&ProductSurfaceSpec>) -> FoodMoundParams {
    let Some(s) = surface else {
        return FoodMoundParams::default();
    };

    let pattern = match s.pattern.as_deref().unwrap_or("unknown") {
        "chunky" | "chunky_mound" | "waves" => FoodPattern::ChunkyMound,
        "swirl" | "spiral_swirl"            => FoodPattern::SwirlMound,
        "flat"                              => FoodPattern::FlatSpread,
        _                                   => FoodPattern::SmoothMound,
    };

    let apex_m = lerp_f32(
        0.004,
        0.030,
        s.ridge_height.or(s.center_peak).unwrap_or(0.45),
    );
    let spread_ratio = s.fill_radius_ratio.unwrap_or(0.88).clamp(0.55, 1.0);
    let irregularity = s.surface_irregularity.unwrap_or(0.25).clamp(0.0, 1.0);
    let chunk_freq   = lerp_f32(3.0, 12.0, irregularity);
    let roughness    = lerp_f32(0.20, 0.80, irregularity);
    let gloss        = lerp_f32(0.55, 0.10, irregularity);

    FoodMoundParams { pattern, apex_m, spread_ratio, irregularity, chunk_freq, roughness, gloss }
}

#[inline]
fn lerp_f32(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t.clamp(0.0, 1.0)
}

// ─────────────────────────────────────────────────────────────────────────────
// Public generator API
// ─────────────────────────────────────────────────────────────────────────────

/// Generate a plate-food mesh with default quality and no Vision surface spec.
pub fn generate(product_color_hex: &str, plate_color_hex: Option<&str>) -> Mesh {
    generate_with_surface_and_quality(
        product_color_hex,
        plate_color_hex,
        None,
        GeometryQuality::default(),
    )
}

/// Same as [`generate`] but with an explicit [`GeometryQuality`] preset.
pub fn generate_with_quality(
    product_color_hex: &str,
    plate_color_hex: Option<&str>,
    quality: GeometryQuality,
) -> Mesh {
    generate_with_surface_and_quality(product_color_hex, plate_color_hex, None, quality)
}

/// Full generator — Vision [`ProductSurfaceSpec`] drives food mound shape + PBR.
pub fn generate_with_surface_and_quality(
    product_color_hex: &str,
    plate_color_hex: Option<&str>,
    surface: Option<&ProductSurfaceSpec>,
    quality: GeometryQuality,
) -> Mesh {
    let product_color = hex_to_rgb(product_color_hex);
    let plate_color   = plate_color_hex.map(hex_to_rgb).unwrap_or(PLATE_DEFAULT_COLOR);
    let params        = mound_params_from_spec(surface);

    let segments       = quality.radial_segments();
    let mound_segments = quality.radial_segments();
    let mound_rings    = quality.surface_rings();

    let mut b = MeshBuilder::new();

    // Group 1 — plate ceramic (PBR: low roughness for porcelain feel).
    let plate_g = b.add_group(
        Material::solid("plate_ceramic", plate_color)
            .with_gloss(0.10, 24.0)
            .with_pbr(0.52, 0.0)
            .with_class("ceramic"),
    );
    // Group 2 — food mound.
    let food_g = b.add_group(
        Material::solid("food_material", product_color)
            .with_gloss(params.gloss, lerp_f32(24.0, 128.0, params.gloss))
            .with_pbr(params.roughness, 0.0)
            .with_class("food"),
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
    let plate_wall = lathe_profile(&plate_profile, segments).expect("lathe plate");
    b.add_part(plate_g, &plate_wall);

    let foot_disk = disk_fan_down(PLATE_FOOT_INNER, Y_FOOT_BOTTOM, segments).expect("foot disk");
    b.add_part(plate_g, &foot_disk);

    // ── Food mound ──────────────────────────────────────────────────────────
    let food_radius = FOOD_MAX_RADIUS * params.spread_ratio;
    match params.pattern {
        FoodPattern::SmoothMound => add_smooth_mound(&mut b, food_g, mound_segments, mound_rings, &params, food_radius),
        FoodPattern::ChunkyMound => add_chunky_mound(&mut b, food_g, mound_segments, mound_rings, &params, food_radius),
        FoodPattern::SwirlMound  => add_swirl_mound (&mut b, food_g, mound_segments, mound_rings, &params, food_radius),
        FoodPattern::FlatSpread  => add_flat_spread  (&mut b, food_g, mound_segments, mound_rings, &params, food_radius),
    }

    b.build()
}

// ─────────────────────────────────────────────────────────────────────────────
// Pattern implementations
// ─────────────────────────────────────────────────────────────────────────────

/// Smooth bell dome — purée, hummus, cream, risotto.
fn add_smooth_mound(
    b: &mut MeshBuilder,
    group: usize,
    segments: usize,
    rings: usize,
    params: &FoodMoundParams,
    food_radius: f32,
) {
    let centre = b.add_vertex(
        [0.0, PLATE_TOP_Y + params.apex_m, 0.0],
        [0.0, 1.0, 0.0],
        [0.5, 0.5],
    );
    let ring_size = segments + 1;
    let first_ring_v = centre + 1;

    for ring in 1..=rings {
        let r_ratio = ring as f32 / rings as f32;
        let r = food_radius * r_ratio;
        let edge_lo = smoothstep(0.10, 0.30, r_ratio);
        let edge_hi = 1.0 - smoothstep(0.85, 1.00, r_ratio);
        let noise_w = (edge_lo * edge_hi).clamp(0.0, 1.0);

        for seg in 0..=segments {
            let t     = seg as f32 / segments as f32;
            let theta = t * 2.0 * PI;
            let cos_t = theta.cos();
            let sin_t = theta.sin();

            let dome  = params.apex_m * (1.0 - r_ratio * r_ratio);
            let noise = FOOD_NOISE * params.irregularity
                * (3.0 * theta + 8.0 * r_ratio).sin()
                * (7.0 * theta).sin()
                * noise_w;
            let y = PLATE_TOP_Y + dome + noise;

            let d_y_d_r = -2.0 * params.apex_m * r_ratio / food_radius.max(1e-6);
            let (nx, ny, nz) = approximate_normal(cos_t, sin_t, r, d_y_d_r, 0.0);

            b.add_vertex(
                [cos_t * r, y, sin_t * r],
                [nx, ny, nz],
                [0.5 + cos_t * 0.5 * r_ratio, 0.5 + sin_t * 0.5 * r_ratio],
            );
        }
    }
    stitch_rings(b, group, centre, first_ring_v, ring_size, rings, segments);
}

/// High-frequency angular noise — salad, roasted veg, rice, potatoes.
fn add_chunky_mound(
    b: &mut MeshBuilder,
    group: usize,
    segments: usize,
    rings: usize,
    params: &FoodMoundParams,
    food_radius: f32,
) {
    let centre = b.add_vertex(
        [0.0, PLATE_TOP_Y + params.apex_m * 0.85, 0.0],
        [0.0, 1.0, 0.0],
        [0.5, 0.5],
    );
    let ring_size = segments + 1;
    let first_ring_v = centre + 1;

    for ring in 1..=rings {
        let r_ratio = ring as f32 / rings as f32;
        let r = food_radius * r_ratio;
        let edge_w = (1.0 - smoothstep(0.80, 1.00, r_ratio)).clamp(0.0, 1.0);

        for seg in 0..=segments {
            let t     = seg as f32 / segments as f32;
            let theta = t * 2.0 * PI;
            let cos_t = theta.cos();
            let sin_t = theta.sin();

            // Bell envelope + chunky high-frequency noise.
            let dome   = params.apex_m * (1.0 - r_ratio * r_ratio);
            let freq   = params.chunk_freq;
            let chunk  = (freq * theta).sin().abs() * 0.4
                + (freq * 1.3 * theta + r_ratio * 5.0).cos().abs() * 0.35
                + (freq * 0.7 * theta - r_ratio * 3.0).sin() * 0.25;
            let noise = FOOD_NOISE * 2.5 * params.irregularity * chunk * edge_w;
            let y = PLATE_TOP_Y + dome + noise;

            let d_y_d_r = -2.0 * params.apex_m * r_ratio / food_radius.max(1e-6);
            let (nx, ny, nz) = approximate_normal(cos_t, sin_t, r, d_y_d_r, 0.0);

            b.add_vertex(
                [cos_t * r, y, sin_t * r],
                [nx, ny, nz],
                [0.5 + cos_t * 0.5 * r_ratio, 0.5 + sin_t * 0.5 * r_ratio],
            );
        }
    }
    stitch_rings(b, group, centre, first_ring_v, ring_size, rings, segments);
}

/// Ridge-based swirl — mashed potato with fork swirl, sauce-topped mound.
fn add_swirl_mound(
    b: &mut MeshBuilder,
    group: usize,
    segments: usize,
    rings: usize,
    params: &FoodMoundParams,
    food_radius: f32,
) {
    let centre = b.add_vertex(
        [0.0, PLATE_TOP_Y + params.apex_m, 0.0],
        [0.0, 1.0, 0.0],
        [0.5, 0.5],
    );
    let ring_size = segments + 1;
    let first_ring_v = centre + 1;

    let swirl_arms = 3.0_f32;
    let ridge_m = params.apex_m * 0.35;

    for ring in 1..=rings {
        let r_ratio = ring as f32 / rings as f32;
        let r = food_radius * r_ratio;
        let edge_falloff = (1.0 - (r_ratio - 0.80).max(0.0) / 0.20).clamp(0.0, 1.0);

        for seg in 0..=segments {
            let t     = seg as f32 / segments as f32;
            let theta = t * 2.0 * PI;
            let cos_t = theta.cos();
            let sin_t = theta.sin();

            let dome  = params.apex_m * (1.0 - r_ratio * r_ratio);
            let phase = swirl_arms * theta + r_ratio * 2.0 * PI * 1.2;
            let wave  = phase.sin();
            let ridge = wave.max(0.0).powf(2.2) * ridge_m * edge_falloff;
            let y = PLATE_TOP_Y + dome + ridge;

            let d_y_d_r = -2.0 * params.apex_m * r_ratio / food_radius.max(1e-6);
            let (nx, ny, nz) = approximate_normal(cos_t, sin_t, r, d_y_d_r, 0.0);

            b.add_vertex(
                [cos_t * r, y, sin_t * r],
                [nx, ny, nz],
                [0.5 + cos_t * 0.5 * r_ratio, 0.5 + sin_t * 0.5 * r_ratio],
            );
        }
    }
    stitch_rings(b, group, centre, first_ring_v, ring_size, rings, segments);
}

/// Thin flat spread — flatbread, pizza, galette, thin sauce pool.
fn add_flat_spread(
    b: &mut MeshBuilder,
    group: usize,
    segments: usize,
    rings: usize,
    params: &FoodMoundParams,
    food_radius: f32,
) {
    let flat_apex = params.apex_m * 0.18; // very low — nearly flat
    let centre = b.add_vertex(
        [0.0, PLATE_TOP_Y + flat_apex, 0.0],
        [0.0, 1.0, 0.0],
        [0.5, 0.5],
    );
    let ring_size = segments + 1;
    let first_ring_v = centre + 1;

    for ring in 1..=rings {
        let r_ratio = ring as f32 / rings as f32;
        let r = food_radius * r_ratio;
        let edge_drop = smoothstep(0.88, 1.00, r_ratio);

        for seg in 0..=segments {
            let t     = seg as f32 / segments as f32;
            let theta = t * 2.0 * PI;
            let cos_t = theta.cos();
            let sin_t = theta.sin();

            // Mostly flat with a slight edge crisp.
            let y = PLATE_TOP_Y + flat_apex * (1.0 - edge_drop * 0.9)
                + FOOD_NOISE * 0.4 * params.irregularity
                    * (5.0 * theta + r_ratio * 4.0).sin();

            let (nx, ny, nz) = approximate_normal(cos_t, sin_t, r, -flat_apex * 2.0 * r_ratio / food_radius.max(1e-6), 0.0);

            b.add_vertex(
                [cos_t * r, y, sin_t * r],
                [nx, ny, nz],
                [0.5 + cos_t * 0.5 * r_ratio, 0.5 + sin_t * 0.5 * r_ratio],
            );
        }
    }
    stitch_rings(b, group, centre, first_ring_v, ring_size, rings, segments);
}

// ─────────────────────────────────────────────────────────────────────────────
// Shared helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Stitch a centre vertex + N concentric rings into triangles.
fn stitch_rings(
    b: &mut MeshBuilder,
    group: usize,
    centre: usize,
    first_ring_v: usize,
    ring_size: usize,
    rings: usize,
    segments: usize,
) {
    // Centre fan → ring 1.
    for seg in 0..segments {
        let a = first_ring_v + seg;
        let bb = first_ring_v + seg + 1;
        b.add_triangle(group, centre, a, bb);
    }
    // Quads between consecutive rings.
    for ring in 1..rings {
        let inner_start = first_ring_v + (ring - 1) * ring_size;
        let outer_start = first_ring_v + ring * ring_size;
        for seg in 0..segments {
            let i0 = inner_start + seg;
            let i1 = inner_start + seg + 1;
            let o0 = outer_start + seg;
            let o1 = outer_start + seg + 1;
            b.add_triangle(group, i0, o0, o1);
            b.add_triangle(group, i0, o1, i1);
        }
    }
}

/// Approximate surface normal from the radial slope and tangential slope.
fn approximate_normal(cos_t: f32, sin_t: f32, r: f32, d_y_d_r: f32, slope_tang: f32) -> (f32, f32, f32) {
    let tr = [cos_t, d_y_d_r, sin_t];
    let tt = [-sin_t, if r > 1e-5 { slope_tang / r } else { 0.0 }, cos_t];
    let nx = tr[1] * tt[2] - tr[2] * tt[1];
    let ny = tr[2] * tt[0] - tr[0] * tt[2];
    let nz = tr[0] * tt[1] - tr[1] * tt[0];
    let len = (nx * nx + ny * ny + nz * nz).sqrt().max(1e-6);
    (nx / len, ny / len, nz / len)
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::laboratory_v2::ProductSurfaceSpec;
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
            .any(|g| g.material.name == "food_material"));
    }

    #[test]
    fn plate_food_uses_product_color() {
        let mesh = generate("#FF0000", None);
        let product = mesh
            .groups
            .iter()
            .find(|g| g.material.name == "food_material")
            .expect("food_material group missing");
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
            .find(|g| g.material.name == "food_material")
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

    // ── PR #30 tests ─────────────────────────────────────────────────────────

    #[test]
    fn plate_food_pbr_material_class_is_set() {
        let mesh = generate("#A85B12", None);
        let ceramic = mesh.groups.iter().find(|g| g.material.name == "plate_ceramic").unwrap();
        let food    = mesh.groups.iter().find(|g| g.material.name == "food_material").unwrap();
        assert_eq!(ceramic.material.material_class, "ceramic");
        assert_eq!(food.material.material_class, "food");
    }

    #[test]
    fn plate_food_pbr_roughness_fields_are_set() {
        let mesh = generate("#A85B12", None);
        let ceramic = mesh.groups.iter().find(|g| g.material.name == "plate_ceramic").unwrap();
        let food    = mesh.groups.iter().find(|g| g.material.name == "food_material").unwrap();
        let cr = ceramic.material.roughness;
        assert!((cr - 0.52).abs() < 0.01, "plate_ceramic roughness should be ~0.52, got {cr}");
        let fr = food.material.roughness;
        assert!(fr > 0.0 && fr <= 1.0, "food roughness out of range: {fr}");
    }

    #[test]
    fn plate_food_chunky_pattern_from_spec() {
        let surface = ProductSurfaceSpec {
            pattern: Some("chunky".to_string()),
            surface_irregularity: Some(0.80),
            ridge_height: Some(0.60),
            fill_radius_ratio: Some(0.88),
            ..Default::default()
        };
        let mesh = generate_with_surface_and_quality(
            "#E2A060",
            None,
            Some(&surface),
            crate::infrastructure::geometry::kernel::GeometryQuality::default(),
        );
        validate_mesh(&mesh).expect("chunky mound passes validation");
        let food = mesh.groups.iter().find(|g| g.material.name == "food_material").unwrap();
        assert!(!food.faces.is_empty());
    }

    #[test]
    fn plate_food_swirl_pattern_from_spec() {
        let surface = ProductSurfaceSpec {
            pattern: Some("swirl".to_string()),
            surface_irregularity: Some(0.30),
            ridge_height: Some(0.40),
            ..Default::default()
        };
        let mesh = generate_with_surface_and_quality(
            "#F4C261",
            None,
            Some(&surface),
            crate::infrastructure::geometry::kernel::GeometryQuality::default(),
        );
        validate_mesh(&mesh).expect("swirl mound passes validation");
    }

    #[test]
    fn plate_food_flat_spread_pattern_from_spec() {
        let surface = ProductSurfaceSpec {
            pattern: Some("flat".to_string()),
            surface_irregularity: Some(0.10),
            ..Default::default()
        };
        let mesh = generate_with_surface_and_quality(
            "#C8B08A",
            None,
            Some(&surface),
            crate::infrastructure::geometry::kernel::GeometryQuality::default(),
        );
        validate_mesh(&mesh).expect("flat spread passes validation");
        // Flat spread must have a much smaller height range than the default dome.
        let food = mesh.groups.iter().find(|g| g.material.name == "food_material").unwrap();
        let mut indices = std::collections::HashSet::new();
        for [a, b, c] in &food.faces { indices.insert(*a); indices.insert(*b); indices.insert(*c); }
        let (min_y, max_y) = indices.iter().fold(
            (f32::INFINITY, f32::NEG_INFINITY),
            |(mn, mx), &i| (mn.min(mesh.vertices[i][1]), mx.max(mesh.vertices[i][1])),
        );
        // flat spread apex ~ apex_m * 0.18 * default_apex → much less than 12 mm
        assert!(max_y - min_y < 0.010, "flat spread height range should be < 10 mm (got {})", max_y - min_y);
    }

    #[test]
    fn mound_params_from_spec_high_irregularity_gives_chunky() {
        let surface = ProductSurfaceSpec {
            pattern: Some("chunky_mound".to_string()),
            surface_irregularity: Some(0.90),
            ..Default::default()
        };
        let p = mound_params_from_spec(Some(&surface));
        assert_eq!(p.pattern, FoodPattern::ChunkyMound);
        assert!(p.roughness > 0.60, "high irregularity → rough food (got {})", p.roughness);
    }
}

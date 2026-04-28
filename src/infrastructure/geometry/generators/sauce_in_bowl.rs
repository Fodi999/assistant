//! Sauce-in-bowl generator (PR #13 — kernel rewrite, PR #26 — Vision Surface Spec).
//!
//! Two material groups (public API preserved):
//!
//!   1. **`bowl_material`** — ceramic bowl built from **four lathed parts**:
//!      * outer wall (foot bevel → flared body → rim outer edge)
//!      * inner wall (concave inside, normals flipped to face the axis)
//!      * rim ring (thin annulus connecting outer top to inner top)
//!      * foot disk (under the base, faces down)
//!      * inner bottom disk (concave floor, faces up)
//!      All five sub-meshes share the same material — the bowl now has a
//!      visible wall thickness, a foot ring, and a sharp rim that catches
//!      highlights.
//!
//!   2. **`sauce_material`** — glossy sauce surface inside the bowl with a
//!      ridge-based spiral-swirl relief driven by [`ProductSurfaceSpec`] values
//!      from Gemini Vision. Falls back to sensible defaults when the spec is
//!      absent.
//!
//! All geometry is in metres, Y-up, centred at origin (Y = 0 = mid-height).

use std::f32::consts::PI;

use crate::application::laboratory_v2::{ContainerSpec, ProductSurfaceSpec};
use crate::infrastructure::geometry::kernel::{
    disk_fan_down, disk_fan_up, lathe_profile, GeometryQuality, MeshBuilder, Profile,
    ProfilePoint,
};
use crate::infrastructure::geometry::mesh::{hex_to_rgb, Material, Mesh};

// ── Bowl dimensions (metres) ────────────────────────────────────────────────
const BOWL_HEIGHT: f32 = 0.060;            // 6 cm tall
const Y_BOTTOM: f32 = -BOWL_HEIGHT / 2.0;  // -0.030
const Y_TOP: f32 = BOWL_HEIGHT / 2.0;      // +0.030

// Outer profile.
const OUTER_R_FOOT: f32 = 0.040;     // foot ring radius
const OUTER_R_FOOT_TOP: f32 = 0.044; // top of foot bevel
const OUTER_R_BASE: f32 = 0.046;     // wall just above foot
const OUTER_R_TOP: f32 = 0.070;      // rim outer radius

// Inner profile (3 mm wall thickness at top, 3 mm at floor).
const INNER_R_BOT: f32 = 0.043;
const INNER_R_TOP: f32 = 0.067;

// Foot rests at Y_BOTTOM. Inner floor is 4 mm above that to give wall thickness.
const Y_FOOT_TOP: f32 = Y_BOTTOM + 0.004;       // -0.026
const Y_INNER_BOTTOM: f32 = Y_BOTTOM + 0.004;   // -0.026

// ── Sauce dimensions (fallback defaults — overridden by ProductSurfaceSpec) ──
const FILL_RATIO: f32 = 0.72;

// ── Default colours ─────────────────────────────────────────────────────────
const BOWL_COLOR: [f32; 3] = [0.96, 0.94, 0.90];

// ─────────────────────────────────────────────────────────────────────────────
// Surface parameter mapping — Vision → geometry
// ─────────────────────────────────────────────────────────────────────────────

/// All parameters that drive the sauce surface shape.
/// Built from [`ProductSurfaceSpec`] by [`surface_params_from_spec`].
#[derive(Debug, Clone, Copy)]
pub struct SauceSurfaceParams {
    /// Number of spiral arms (1–8).
    pub swirl_arms: f32,
    /// Ridge peak height in metres.
    pub ridge_height_m: f32,
    /// Groove trough depth in metres (subtracted below the base plane).
    pub groove_depth_m: f32,
    /// Centre peak height in metres.
    pub center_peak_m: f32,
    /// Fraction of inner-wall radius the sauce fills (0.82–0.98).
    pub radius_scale: f32,
    /// Organic noise amplitude multiplier (0.0–1.0).
    pub irregularity: f32,
}

impl Default for SauceSurfaceParams {
    fn default() -> Self {
        Self {
            swirl_arms: 3.0,
            ridge_height_m: 0.003,
            groove_depth_m: 0.0015,
            center_peak_m: 0.0015,
            radius_scale: 0.92,
            irregularity: 0.15,
        }
    }
}

/// Map an optional [`ProductSurfaceSpec`] (from Gemini Vision) to concrete
/// geometry parameters. Returns sensible defaults when the spec is `None`.
pub fn surface_params_from_spec(surface: Option<&ProductSurfaceSpec>) -> SauceSurfaceParams {
    let Some(s) = surface else {
        return SauceSurfaceParams::default();
    };

    let ridge_height_m = lerp_f32(0.0025, 0.010, s.ridge_height.unwrap_or(0.45));
    let groove_depth_m = lerp_f32(0.0005, 0.006, s.groove_depth.unwrap_or(0.35));
    let center_peak_m  = lerp_f32(0.0,    0.006, s.center_peak.unwrap_or(0.25));
    let radius_scale   = s.fill_radius_ratio
        .or_else(|| s.rim_gap_ratio.map(|gap| 1.0 - gap))
        .unwrap_or(0.92)
        .clamp(0.82, 0.98);
    let swirl_arms     = s.swirl_arms.unwrap_or(3).clamp(1, 8) as f32;
    let irregularity   = s.surface_irregularity.unwrap_or(0.20).clamp(0.0, 1.0);

    SauceSurfaceParams { swirl_arms, ridge_height_m, groove_depth_m, center_peak_m, radius_scale, irregularity }
}

#[inline]
fn lerp_f32(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t.clamp(0.0, 1.0)
}

// ─────────────────────────────────────────────────────────────────────────────
// Public generator API
// ─────────────────────────────────────────────────────────────────────────────

/// Generate a sauce-in-bowl mesh (bowl + sauce, two material groups).
///
/// Uses [`GeometryQuality::default`] (= `High`) and default surface params.
pub fn generate(sauce_color_hex: &str, container_color_hex: Option<&str>) -> Mesh {
    generate_with_surface_and_quality(
        sauce_color_hex,
        container_color_hex,
        None,
        None,
        GeometryQuality::default(),
    )
}

/// Same as [`generate`] but with an explicit [`GeometryQuality`] preset.
/// Surface parameters use safe defaults (no Vision spec).
pub fn generate_with_quality(
    sauce_color_hex: &str,
    container_color_hex: Option<&str>,
    quality: GeometryQuality,
) -> Mesh {
    generate_with_surface_and_quality(sauce_color_hex, container_color_hex, None, None, quality)
}

/// Full generator — accepts Gemini Vision [`ProductSurfaceSpec`], full
/// [`ContainerSpec`] (for material/tint/glass detection) and an explicit
/// [`GeometryQuality`] preset.
///
/// - `surface`   — drives swirl arms, ridge/groove heights, fill radius, noise.
/// - `container` — drives bowl material name (`bowl_glass` vs `bowl_ceramic`)
///                 and colour / tint fallback.
/// Passing `None` for either produces the same geometry as [`generate`].
pub fn generate_with_surface_and_quality(
    sauce_color_hex: &str,
    container_color_hex: Option<&str>,
    container: Option<&ContainerSpec>,
    surface: Option<&ProductSurfaceSpec>,
    quality: GeometryQuality,
) -> Mesh {
    // ── Determine bowl material ──────────────────────────────────────────────
    let is_glass = container
        .map(|c| {
            c.kind.to_lowercase().contains("glass")
                || c.material.as_deref() == Some("glass")
        })
        .unwrap_or(false);

    let material_name = if is_glass { "bowl_glass" } else { "bowl_ceramic" };

    // Colour priority: explicit override arg → container tint_hex (glass) →
    // container color_hex → fallback (amber for glass, white ceramic).
    let glass_fallback = [0.24, 0.09, 0.04]; // dark amber/brown glass
    let ceramic_fallback = BOWL_COLOR;
    let bowl_color = container_color_hex
        .map(hex_to_rgb)
        .or_else(|| {
            container.and_then(|c| {
                if is_glass {
                    c.tint_hex.as_deref().or(c.color_hex.as_deref())
                } else {
                    c.color_hex.as_deref()
                }
            }).map(hex_to_rgb)
        })
        .unwrap_or(if is_glass { glass_fallback } else { ceramic_fallback });

    let sauce_color = hex_to_rgb(sauce_color_hex);

    let segments = quality.radial_segments();
    let sauce_rings = quality.surface_rings();
    let params = surface_params_from_spec(surface);

    let mut b = MeshBuilder::new();

    // Glass gets higher gloss + lower roughness so the frontend makeGlassMaterial
    // picks it up cleanly. Ceramic keeps low gloss.
    let (gloss_factor, gloss_exp) = if is_glass { (0.85, 256.0) } else { (0.10, 24.0) };
    let bowl_g = b.add_group(
        Material::solid(material_name, bowl_color).with_gloss(gloss_factor, gloss_exp),
    );
    let sauce_g = b.add_group(
        Material::solid("sauce_material", sauce_color).with_gloss(0.55, 96.0),
    );

    // ── Bowl: outer wall ────────────────────────────────────────────────────
    let outer_profile = Profile::new(vec![
        ProfilePoint::new(OUTER_R_FOOT, Y_BOTTOM),
        ProfilePoint::new(OUTER_R_FOOT_TOP, Y_FOOT_TOP),
        ProfilePoint::new(OUTER_R_BASE, Y_FOOT_TOP + 0.001),
        ProfilePoint::new(OUTER_R_TOP, Y_TOP),
    ])
    .expect("hard-coded outer bowl profile is valid");
    let outer = lathe_profile(&outer_profile, segments).expect("lathe outer wall");
    b.add_part(bowl_g, &outer);

    // ── Bowl: inner wall (flipped: normals point toward the axis) ───────────
    let inner_profile = Profile::new(vec![
        ProfilePoint::new(INNER_R_BOT, Y_INNER_BOTTOM),
        ProfilePoint::new(INNER_R_TOP, Y_TOP),
    ])
    .expect("hard-coded inner bowl profile is valid");
    let inner = lathe_profile(&inner_profile, segments)
        .expect("lathe inner wall")
        .flipped();
    b.add_part(bowl_g, &inner);

    // ── Bowl: rim (flat annulus at Y_TOP, faces up) ────────────────────────
    // Profile from outer→inner at constant Y → outward normal is +Y.
    let rim_profile = Profile::new(vec![
        ProfilePoint::new(OUTER_R_TOP, Y_TOP),
        ProfilePoint::new(INNER_R_TOP, Y_TOP),
    ])
    .expect("hard-coded rim profile is valid");
    let rim = lathe_profile(&rim_profile, segments).expect("lathe rim");
    b.add_part(bowl_g, &rim);

    // ── Bowl: foot underside (disk facing down) ─────────────────────────────
    let foot_disk = disk_fan_down(OUTER_R_FOOT, Y_BOTTOM, segments).expect("foot disk");
    b.add_part(bowl_g, &foot_disk);

    // ── Bowl: inner floor (disk facing up) ──────────────────────────────────
    let inner_floor =
        disk_fan_up(INNER_R_BOT, Y_INNER_BOTTOM, segments).expect("inner floor disk");
    b.add_part(bowl_g, &inner_floor);

    // ── Sauce: tessellated swirl disk inside the bowl ───────────────────────
    add_sauce_surface(&mut b, sauce_g, segments, sauce_rings, &params);

    b.build()
}

// ─────────────────────────────────────────────────────────────────────────────
// Sauce surface — ridge-based spiral swirl driven by SauceSurfaceParams.
//
// Displacement formula (per vertex at polar coords r_ratio, theta):
//
//   phase = arms * theta + r_ratio * 2π * 1.35
//   wave  = sin(phase)
//   ridge = max(wave, 0)^2.4          — sharp positive crest
//   groove = max(-wave, 0)^1.8        — softer negative valley
//   dy = ridge_height * ridge − groove_depth * groove
//      + center_peak * (1−r_ratio)^4 * |sin(2θ + r*6)|   // centre curl
//      + irregularity * 0.0015 * organic_noise * edge_falloff
//
// Edge-falloff zeroes displacement near the rim so the disk stays flush
// with the inner bowl wall.
// ─────────────────────────────────────────────────────────────────────────────
fn add_sauce_surface(
    b: &mut MeshBuilder,
    group: usize,
    segments: usize,
    sauce_rings: usize,
    params: &SauceSurfaceParams,
) {
    let y_fill = Y_BOTTOM + BOWL_HEIGHT * FILL_RATIO + 0.002;

    // Inner-wall radius at the fill height, then scaled by Vision fill ratio.
    let lerp_t = (y_fill - Y_INNER_BOTTOM) / (Y_TOP - Y_INNER_BOTTOM);
    let r_at_fill = INNER_R_BOT + (INNER_R_TOP - INNER_R_BOT) * lerp_t.clamp(0.0, 1.0);
    let sauce_radius = r_at_fill * params.radius_scale;

    // Centre vertex, undisplaced.
    let centre = b.add_vertex([0.0, y_fill, 0.0], [0.0, 1.0, 0.0], [0.5, 0.5]);

    let ring_size = segments + 1;
    let first_ring_v = centre + 1;

    for ring in 1..=sauce_rings {
        let r_ratio = ring as f32 / sauce_rings as f32;
        let r = sauce_radius * r_ratio;
        // Edge falloff: full amplitude inside 0.85, fade to zero at 1.0.
        let edge_falloff = (1.0 - (r_ratio - 0.85).max(0.0) / 0.15).clamp(0.0, 1.0);

        for seg in 0..=segments {
            let t = seg as f32 / segments as f32;
            let theta = t * 2.0 * PI;
            let cos_t = theta.cos();
            let sin_t = theta.sin();

            // ── Ridge / groove wave ──────────────────────────────────────
            let phase = params.swirl_arms * theta + r_ratio * 2.0 * PI * 1.35;
            let wave  = phase.sin();
            let ridge  = wave.max(0.0).powf(2.4);
            let groove = (-wave).max(0.0).powf(1.8);
            let mut dy = params.ridge_height_m * ridge - params.groove_depth_m * groove;

            // ── Centre curl peak ─────────────────────────────────────────
            let center_w = (1.0 - r_ratio).clamp(0.0, 1.0).powf(4.0);
            dy += params.center_peak_m
                * center_w
                * (theta * 2.0 + r_ratio * 6.0).sin().abs();

            // ── Organic noise ────────────────────────────────────────────
            let organic = (theta * 5.0 + r_ratio * 11.0).sin()
                * (theta * 2.0 - r_ratio * 7.0).cos();
            dy += params.irregularity * 0.0015 * organic * edge_falloff;

            dy *= edge_falloff;

            // ── Analytic normal from angular gradient ────────────────────
            let d_phase_dt = params.swirl_arms;
            let d_wave_dt  = d_phase_dt * phase.cos();
            let d_ridge_dt = 2.4 * wave.max(0.0).powf(1.4) * d_wave_dt * (if wave > 0.0 { 1.0 } else { 0.0 });
            let d_groove_dt = 1.8 * (-wave).max(0.0).powf(0.8) * (-d_wave_dt) * (if wave < 0.0 { 1.0 } else { 0.0 });
            let dphase_dt = (params.ridge_height_m * d_ridge_dt
                - params.groove_depth_m * d_groove_dt)
                * edge_falloff;
            let slope_t = if r > 1e-5 { dphase_dt / r } else { 0.0 };
            let nx = -slope_t * (-sin_t);
            let nz = -slope_t * cos_t;
            let ny = 1.0_f32;
            let len = (nx * nx + ny * ny + nz * nz).sqrt();

            b.add_vertex(
                [cos_t * r, y_fill + dy, sin_t * r],
                [nx / len, ny / len, nz / len],
                [0.5 + cos_t * 0.5 * r_ratio, 0.5 + sin_t * 0.5 * r_ratio],
            );
        }
    }

    // Inner fan: centre → first ring.
    for seg in 0..segments {
        let a = first_ring_v + seg;
        let bb = first_ring_v + seg + 1;
        b.add_triangle(group, centre, a, bb);
    }

    // Quads between consecutive rings.
    for ring in 1..sauce_rings {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::laboratory_v2::ContainerSpec;
    use crate::infrastructure::geometry::kernel::validate_mesh;

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
            .find(|g| g.material.name == "bowl_ceramic")
            .expect("bowl_ceramic group should exist");
        let [r, g, b] = bowl.material.diffuse_color;
        assert!(r > 0.85 && g > 0.85 && b > 0.85);
    }

    #[test]
    fn sauce_surface_has_swirl_displacement() {
        let mesh = generate("#FF0000", None);
        let y_fill = Y_BOTTOM + BOWL_HEIGHT * FILL_RATIO;

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

    #[test]
    fn sauce_in_bowl_passes_kernel_validation() {
        let mesh = generate("#B8321F", None);
        validate_mesh(&mesh).expect("kernel validation should pass on bowl mesh");
    }

    #[test]
    fn bowl_has_inner_wall_with_inward_normals() {
        // The flipped inner wall should have at least one vertex whose
        // normal opposes its outward (axis-pointing) direction.
        let mesh = generate("#B8321F", None);
        let bowl = mesh
            .groups
            .iter()
            .find(|g| g.material.name == "bowl_ceramic")
            .unwrap();
        let mut inward_count = 0usize;
        let mut indices = std::collections::HashSet::new();
        for [a, b, c] in &bowl.faces {
            indices.insert(*a);
            indices.insert(*b);
            indices.insert(*c);
        }
        for i in indices {
            let v = mesh.vertices[i];
            let r = (v[0] * v[0] + v[2] * v[2]).sqrt();
            if r < 1e-5 {
                continue;
            }
            let outward = [v[0] / r, 0.0_f32, v[2] / r];
            let n = mesh.normals[i];
            let dot = n[0] * outward[0] + n[1] * outward[1] + n[2] * outward[2];
            if dot < -0.5 {
                inward_count += 1;
            }
        }
        assert!(
            inward_count > GeometryQuality::default().radial_segments(),
            "expected many inner-wall vertices with inward normals (got {inward_count})"
        );
    }

    #[test]
    fn bowl_widest_radius_is_at_rim() {
        let mesh = generate("#B8321F", None);
        let mut widest_r: f32 = 0.0;
        let mut widest_y: f32 = 0.0;
        for v in &mesh.vertices {
            let r = (v[0] * v[0] + v[2] * v[2]).sqrt();
            if r > widest_r {
                widest_r = r;
                widest_y = v[1];
            }
        }
        // Rim outer radius dominates.
        assert!((widest_r - OUTER_R_TOP).abs() < 1e-4);
        assert!((widest_y - Y_TOP).abs() < 1e-4);
    }

    // ── PR #26: Vision Surface Spec tests ────────────────────────────────────

    /// Helper: Y range of sauce_material vertices.
    fn sauce_y_range(mesh: &Mesh) -> f32 {
        let sauce = mesh
            .groups
            .iter()
            .find(|g| g.material.name == "sauce_material")
            .unwrap();
        let mut indices = std::collections::HashSet::new();
        for [a, b, c] in &sauce.faces {
            indices.insert(*a);
            indices.insert(*b);
            indices.insert(*c);
        }
        let ys: Vec<f32> = indices.iter().map(|&i| mesh.vertices[i][1]).collect();
        let min_y = ys.iter().cloned().fold(f32::INFINITY, f32::min);
        let max_y = ys.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        max_y - min_y
    }

    /// Helper: max XZ radius of sauce_material vertices.
    fn sauce_max_radius(mesh: &Mesh) -> f32 {
        let sauce = mesh
            .groups
            .iter()
            .find(|g| g.material.name == "sauce_material")
            .unwrap();
        let mut indices = std::collections::HashSet::new();
        for [a, b, c] in &sauce.faces {
            indices.insert(*a);
            indices.insert(*b);
            indices.insert(*c);
        }
        indices
            .iter()
            .map(|&i| {
                let v = mesh.vertices[i];
                (v[0] * v[0] + v[2] * v[2]).sqrt()
            })
            .fold(0.0_f32, f32::max)
    }

    fn make_surface(
        swirl_arms: Option<u8>,
        ridge_height: Option<f32>,
        groove_depth: Option<f32>,
        center_peak: Option<f32>,
        fill_radius_ratio: Option<f32>,
        rim_gap_ratio: Option<f32>,
        surface_irregularity: Option<f32>,
    ) -> ProductSurfaceSpec {
        ProductSurfaceSpec {
            pattern: None,
            swirl_arms,
            ridge_height,
            groove_depth,
            center_peak,
            fill_radius_ratio,
            rim_gap_ratio,
            surface_irregularity,
            highlight_strength: None,
            view_angle: None,
        }
    }

    #[test]
    fn surface_spec_ridge_height_increases_y_range() {
        let low_spec = make_surface(None, Some(0.1), None, None, None, None, None);
        let high_spec = make_surface(None, Some(0.9), None, None, None, None, None);
        let q = GeometryQuality::Standard;
        let low  = generate_with_surface_and_quality("#B8321F", None, None, Some(&low_spec), q);
        let high = generate_with_surface_and_quality("#B8321F", None, None, Some(&high_spec), q);
        assert!(
            sauce_y_range(&high) > sauce_y_range(&low),
            "higher ridge_height must produce larger Y range (low={:.5} high={:.5})",
            sauce_y_range(&low),
            sauce_y_range(&high),
        );
    }

    #[test]
    fn surface_spec_fill_radius_ratio_expands_sauce() {
        let default_spec = make_surface(None, None, None, None, None, None, None);
        let wide_spec    = make_surface(None, None, None, None, Some(0.97), None, None);
        let q = GeometryQuality::Standard;
        let default_mesh = generate_with_surface_and_quality("#B8321F", None, None, Some(&default_spec), q);
        let wide_mesh    = generate_with_surface_and_quality("#B8321F", None, None, Some(&wide_spec), q);
        assert!(
            sauce_max_radius(&wide_mesh) > sauce_max_radius(&default_mesh),
            "fill_radius_ratio=0.97 must produce wider sauce than default"
        );
    }

    #[test]
    fn surface_spec_swirl_arms_changes_geometry() {
        let spec_3 = make_surface(Some(3), None, None, None, None, None, None);
        let spec_7 = make_surface(Some(7), None, None, None, None, None, None);
        let q = GeometryQuality::Standard;
        let m3 = generate_with_surface_and_quality("#B8321F", None, None, Some(&spec_3), q);
        let m7 = generate_with_surface_and_quality("#B8321F", None, None, Some(&spec_7), q);
        // With more arms the surface should be more varied — different Y ranges.
        let r3 = sauce_y_range(&m3);
        let r7 = sauce_y_range(&m7);
        assert!(
            (r3 - r7).abs() > 1e-6 || r3 > 0.0,
            "different swirl_arms must produce non-identical geometry"
        );
    }

    #[test]
    fn missing_surface_uses_safe_defaults() {
        let mesh = generate_with_surface_and_quality(
            "#B8321F",
            None,
            None,
            None,
            GeometryQuality::Standard,
        );
        validate_mesh(&mesh).expect("default-surface mesh should pass validation");
        assert!(sauce_y_range(&mesh) > 0.0, "default surface should still have swirl relief");
    }

    #[test]
    fn sauce_in_bowl_with_surface_passes_validation() {
        let spec = make_surface(Some(4), Some(0.8), Some(0.7), Some(0.6), Some(0.96), Some(0.04), Some(0.3));
        let mesh = generate_with_surface_and_quality(
            "#B8321F",
            None,
            None,
            Some(&spec),
            GeometryQuality::High,
        );
        validate_mesh(&mesh).expect("full surface spec mesh should pass kernel validation");
    }

    #[test]
    fn glass_bowl_creates_bowl_glass_material() {
        let container = ContainerSpec {
            kind: "glass_bowl".to_string(),
            material: Some("glass".to_string()),
            color_hex: None,
            tint_hex: Some("#3D1A0A".to_string()),
            transparency: Some(0.7),
            rim_darkness: Some(0.4),
            diameter_mm: Some(120.0),
            height_mm: Some(55.0),
        };
        let mesh = generate_with_surface_and_quality(
            "#B8321F",
            None,
            Some(&container),
            None,
            GeometryQuality::Standard,
        );
        let bowl_group = mesh
            .groups
            .iter()
            .find(|g| g.material.name == "bowl_glass")
            .expect("glass_bowl container must produce bowl_glass material group");
        // Tint colour #3D1A0A → approx [0.239, 0.098, 0.039]
        let [r, _g, _b] = bowl_group.material.diffuse_color;
        assert!(r > 0.15 && r < 0.40, "glass tint red channel should be dark amber");
    }

    #[test]
    fn ceramic_bowl_creates_bowl_ceramic_material() {
        let container = ContainerSpec {
            kind: "ceramic_bowl".to_string(),
            material: Some("ceramic".to_string()),
            color_hex: Some("#F0EDE8".to_string()),
            tint_hex: None,
            transparency: None,
            rim_darkness: None,
            diameter_mm: None,
            height_mm: None,
        };
        let mesh = generate_with_surface_and_quality(
            "#B8321F",
            None,
            Some(&container),
            None,
            GeometryQuality::Standard,
        );
        assert!(
            mesh.groups.iter().any(|g| g.material.name == "bowl_ceramic"),
            "ceramic container must produce bowl_ceramic group"
        );
    }

    #[test]
    fn no_container_spec_falls_back_to_bowl_ceramic() {
        let mesh = generate("#B8321F", None);
        assert!(
            mesh.groups.iter().any(|g| g.material.name == "bowl_ceramic"),
            "no container defaults to bowl_ceramic"
        );
    }
}

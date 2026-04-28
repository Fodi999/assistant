//! Sauce-in-bowl generator (PR #13 — kernel rewrite).
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
//!      logarithmic-spiral swirl relief. Built procedurally on top of a
//!      tessellated disk (not a simple lathe — the swirl needs angular
//!      displacement, which the kernel doesn't model).
//!
//! All geometry is in metres, Y-up, centred at origin (Y = 0 = mid-height).

use std::f32::consts::PI;

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

// ── Sauce dimensions ────────────────────────────────────────────────────────
const FILL_RATIO: f32 = 0.72;
const SWIRL_AMPLITUDE: f32 = 0.0025;
const SWIRL_ARMS: f32 = 3.0;

// ── Default colours ─────────────────────────────────────────────────────────
const BOWL_COLOR: [f32; 3] = [0.96, 0.94, 0.90];

/// Generate a sauce-in-bowl mesh (bowl + sauce, two material groups).
///
/// Uses [`GeometryQuality::default`] (= `High`).
///
/// - `sauce_color_hex` — hex colour for the sauce surface.
/// - `container_color_hex` — optional override for bowl colour.
pub fn generate(sauce_color_hex: &str, container_color_hex: Option<&str>) -> Mesh {
    generate_with_quality(sauce_color_hex, container_color_hex, GeometryQuality::default())
}

/// Same as [`generate`] but with an explicit [`GeometryQuality`] preset
/// driving the radial segment count and the number of swirl rings.
pub fn generate_with_quality(
    sauce_color_hex: &str,
    container_color_hex: Option<&str>,
    quality: GeometryQuality,
) -> Mesh {
    let bowl_color = container_color_hex.map(hex_to_rgb).unwrap_or(BOWL_COLOR);
    let sauce_color = hex_to_rgb(sauce_color_hex);

    let segments = quality.radial_segments();
    let sauce_rings = quality.surface_rings();

    let mut b = MeshBuilder::new();

    // The frontend matches `*bowl*|*ceramic*` first (PR #9 polish) and applies
    // a non-transmissive ceramic upgrade. Soft highlight → low gloss.
    let bowl_g =
        b.add_group(Material::solid("bowl_material", bowl_color).with_gloss(0.10, 24.0));
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
    add_sauce_surface(&mut b, sauce_g, segments, sauce_rings);

    b.build()
}

// ─────────────────────────────────────────────────────────────────────────────
// Sauce surface — concentric rings + angular swirl displacement.
//
//   dy = SWIRL_AMPLITUDE * sin(SWIRL_ARMS * theta + 2π * radius_ratio)
//
// We push vertices straight into the `MeshBuilder` and attach faces to the
// sauce group. Edge-falloff zeros the displacement at the rim so the disk
// stays in contact with the inner bowl wall.
// ─────────────────────────────────────────────────────────────────────────────
fn add_sauce_surface(b: &mut MeshBuilder, group: usize, segments: usize, sauce_rings: usize) {
    let y_fill = Y_BOTTOM + BOWL_HEIGHT * FILL_RATIO + 0.002;

    // Inner-wall radius at the fill height (linear interp on inner profile),
    // then shrunk so the sauce rim sits ~1 mm clear of the wall.
    let lerp = (y_fill - Y_INNER_BOTTOM) / (Y_TOP - Y_INNER_BOTTOM);
    let r_at_fill = INNER_R_BOT + (INNER_R_TOP - INNER_R_BOT) * lerp.clamp(0.0, 1.0);
    let sauce_radius = r_at_fill * 0.92;

    // Centre vertex, undisplaced.
    let centre = b.add_vertex([0.0, y_fill, 0.0], [0.0, 1.0, 0.0], [0.5, 0.5]);

    // Build all ring vertices and remember their starting index.
    let ring_size = segments + 1;
    let first_ring_v = centre + 1; // next index pushed will be this

    for ring in 1..=sauce_rings {
        let r_ratio = ring as f32 / sauce_rings as f32;
        let r = sauce_radius * r_ratio;
        // Falloff window: full amplitude up to 0.85, linearly to zero by 1.0.
        let edge_falloff = (1.0 - (r_ratio - 0.85).max(0.0) / 0.15).clamp(0.0, 1.0);

        for seg in 0..=segments {
            let t = seg as f32 / segments as f32;
            let theta = t * 2.0 * PI;
            let cos_t = theta.cos();
            let sin_t = theta.sin();

            let phase = SWIRL_ARMS * theta + r_ratio * 2.0 * PI;
            let dy = SWIRL_AMPLITUDE * phase.sin() * edge_falloff;
            let dphase = SWIRL_AMPLITUDE * SWIRL_ARMS * phase.cos() * edge_falloff;

            // Cheap analytic normal: tilt up-vector against angular gradient.
            let slope_t = if r > 1e-5 { dphase / r } else { 0.0 };
            let nx = -slope_t * (-sin_t);
            let nz = -slope_t * cos_t;
            let ny = 1.0;
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
            .find(|g| g.material.name == "bowl_material")
            .expect("bowl_material group should exist");
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
            .find(|g| g.material.name == "bowl_material")
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
}

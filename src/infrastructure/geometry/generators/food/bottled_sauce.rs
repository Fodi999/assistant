//! Bottled-sauce generator (PR #12 — kernel rewrite).
//!
//! Realistic sauce / oil / dressing bottle. Built entirely from the geometry
//! kernel (profile → lathe → mesh_builder + disk fans), so shading and UVs
//! are consistent with `jar_product` and the upcoming bowl/plate generators.
//!
//! Output mesh has **four material groups**:
//!   1. `bottle_glass` / `bottle_plastic` — full lathed exterior
//!      (body → shoulder → neck) as one continuous profile so the normals
//!      flow smoothly across the shoulder transition.
//!   2. `bottle_glass` / `bottle_plastic` — bottom cap disk (same material,
//!      separate group; keeps the disk's hard edge from pulling neighbouring
//!      wall normals downward).
//!   3. `cap_metal` — lathed cap with chamfered top & bottom + sealed top
//!      and underside disks.
//!   4. `liquid_material` — inner lathed liquid surface (smaller radius
//!      than the bottle wall to avoid z-fight) + meniscus disk so the
//!      colour reads through transmissive glass.
//!
//! Frontend material upgrade rules (in `ModelViewer.tsx`):
//!   * `*glass*` → transmissive `MeshPhysicalMaterial`
//!   * `*plastic*` → falls through to default standard material
//!   * `*metal*|*lid*|*cap*` → metallic
//!   * `*liquid*|*sauce*|*product*` → glossy diffuse
//!
//! Y-up, centred at origin, all units in metres.

use crate::infrastructure::geometry::kernel::{
    cylindrical_band, disk_fan_down, disk_fan_up, lathe_profile, GeometryQuality, MeshBuilder,
    Profile, ProfilePoint,
};
use crate::infrastructure::geometry::mesh::{hex_to_rgb, Material, Mesh};

// Label band placement (PR #15) — sits on the straight body section,
// slightly outset to avoid z-fight with the bottle wall.
const LABEL_Y_MIN: f32 = -0.030;
const LABEL_Y_MAX: f32 = 0.015;
const LABEL_RADIUS_OFFSET: f32 = 0.0004; // 0.4 mm outset (slightly proud)
const LABEL_PAPER_COLOR: [f32; 3] = [0.97, 0.96, 0.93]; // off-white paper

// ── Default colours ─────────────────────────────────────────────────────────
const BOTTLE_GLASS_COLOR: [f32; 3] = [0.90, 0.93, 0.92];
const BOTTLE_PLASTIC_COLOR: [f32; 3] = [0.96, 0.96, 0.96];
const CAP_DEFAULT_COLOR: [f32; 3] = [0.55, 0.55, 0.58]; // brushed metal grey

// ── Geometry constants (metres) ─────────────────────────────────────────────
// Bottle exterior — see `bottle_body_profile()` for the actual profile.
const BODY_RADIUS: f32 = 0.030; // 3 cm widest point on body
const NECK_RADIUS: f32 = 0.011; // 1.1 cm at the neck top
const NECK_TOP_Y: f32 = 0.076;
const CAP_BOTTOM_Y: f32 = 0.076;
const CAP_TOP_Y: f32 = 0.096;
const CAP_RADIUS: f32 = 0.015; // slightly wider than neck
const CAP_INNER_RADIUS: f32 = 0.013; // bottom/top after chamfer

const BOTTOM_DISK_RADIUS: f32 = 0.026; // matches first profile point

/// Container kind hints from `ContainerSpec.kind`.
#[derive(Debug, Clone, Copy)]
pub enum BottleKind {
    Glass,
    Plastic,
}

impl BottleKind {
    pub fn from_str(kind: Option<&str>) -> Self {
        match kind.unwrap_or("") {
            k if k.contains("plastic") => BottleKind::Plastic,
            _ => BottleKind::Glass,
        }
    }

    fn material_name(self) -> &'static str {
        match self {
            BottleKind::Glass => "bottle_glass",
            BottleKind::Plastic => "bottle_plastic",
        }
    }

    fn default_color(self) -> [f32; 3] {
        match self {
            BottleKind::Glass => BOTTLE_GLASS_COLOR,
            BottleKind::Plastic => BOTTLE_PLASTIC_COLOR,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Profiles
// ─────────────────────────────────────────────────────────────────────────────

/// Bottle exterior — bottom inset → bottom bevel → straight body →
/// shoulder transition → neck. Eight points total; the curve compresses
/// the shoulder over ~16 mm and the neck stays at constant 1.1 cm.
fn bottle_body_profile() -> Profile {
    Profile::new(vec![
        ProfilePoint::new(0.026, -0.060), // bottom inset (foot)
        ProfilePoint::new(0.030, -0.056), // bottom bevel — wall starts here
        ProfilePoint::new(0.030, 0.020),  // straight body
        ProfilePoint::new(0.029, 0.028),  // shoulder begins (subtle taper)
        ProfilePoint::new(0.026, 0.036),  // shoulder mid
        ProfilePoint::new(0.016, 0.050),  // shoulder ends, narrow neck base
        ProfilePoint::new(0.011, 0.056),  // neck base
        ProfilePoint::new(0.011, 0.076),  // neck top (NECK_TOP_Y)
    ])
    .expect("hard-coded bottle body profile is valid")
}

/// Inner liquid surface — slightly smaller than the bottle wall to avoid
/// z-fight, with the meniscus sitting just below the shoulder.
fn liquid_profile() -> Profile {
    Profile::new(vec![
        ProfilePoint::new(0.027, -0.055), // inner bottom
        ProfilePoint::new(0.027, 0.018),  // inner straight wall
        ProfilePoint::new(0.023, 0.032),  // shoulder narrowing
        ProfilePoint::new(0.010, 0.050),  // tucked under the bottle shoulder
    ])
    .expect("hard-coded liquid profile is valid")
}

/// Cap exterior — bottom chamfer → straight wall → top chamfer.
/// Both chamfers are 2 mm tall, giving the cap two extra highlight bands.
fn cap_profile() -> Profile {
    Profile::new(vec![
        ProfilePoint::new(CAP_INNER_RADIUS, CAP_BOTTOM_Y),
        ProfilePoint::new(CAP_RADIUS, CAP_BOTTOM_Y + 0.002), // bottom chamfer
        ProfilePoint::new(CAP_RADIUS, CAP_TOP_Y - 0.002),    // straight wall
        ProfilePoint::new(CAP_INNER_RADIUS, CAP_TOP_Y),      // top chamfer
    ])
    .expect("hard-coded cap profile is valid")
}

// ─────────────────────────────────────────────────────────────────────────────
// Public entrypoint
// ─────────────────────────────────────────────────────────────────────────────

/// Generate a bottled-sauce mesh.
///
/// - `liquid_color_hex` — hex colour of the sauce inside (`product.color_hex`).
/// - `bottle_kind` — `glass` or `plastic` (from `container.kind`).
/// - `cap_color_hex` — optional override for the cap colour.
pub fn generate(
    liquid_color_hex: &str,
    bottle_kind: BottleKind,
    cap_color_hex: Option<&str>,
) -> Mesh {
    generate_with_label(liquid_color_hex, bottle_kind, cap_color_hex, None)
}

/// Same as [`generate`] but additionally wraps a cylindrical label band
/// around the body of the bottle. The `label_url` is stored as
/// `materials[i].extras.texture_url` in the GLB; the frontend resolves it
/// via `THREE.TextureLoader` and binds it as `material.map`.
pub fn generate_with_label(
    liquid_color_hex: &str,
    bottle_kind: BottleKind,
    cap_color_hex: Option<&str>,
    label_url: Option<&str>,
) -> Mesh {
    generate_with_label_and_quality(
        liquid_color_hex,
        bottle_kind,
        cap_color_hex,
        label_url,
        GeometryQuality::default(),
    )
}

/// Full entrypoint with explicit [`GeometryQuality`].
pub fn generate_with_label_and_quality(
    liquid_color_hex: &str,
    bottle_kind: BottleKind,
    cap_color_hex: Option<&str>,
    label_url: Option<&str>,
    quality: GeometryQuality,
) -> Mesh {
    let liquid_color = hex_to_rgb(liquid_color_hex);
    let bottle_color = bottle_kind.default_color();
    let cap_color = cap_color_hex.map(hex_to_rgb).unwrap_or(CAP_DEFAULT_COLOR);

    let segments = quality.radial_segments();

    let mut b = MeshBuilder::new();

    // Group 1: bottle exterior wall (continuous lathe — keeps shoulder smooth).
    let bottle_wall_g = b.add_group(
        Material::solid(bottle_kind.material_name(), bottle_color)
            .with_gloss(0.55, 96.0),
    );
    // Group 2: bottle bottom disk (same material, separate group so the
    // normal seam doesn't pull the wall normals downward).
    let bottle_bottom_g = b.add_group(
        Material::solid(bottle_kind.material_name(), bottle_color)
            .with_gloss(0.55, 96.0),
    );
    // Group 3: cap.
    let cap_g = b.add_group(Material::solid("cap_metal", cap_color).with_gloss(0.60, 64.0));
    // Group 4: liquid interior + meniscus.
    let liquid_g = b.add_group(
        Material::solid("liquid_material", liquid_color).with_gloss(0.55, 96.0),
    );

    // ── Bottle exterior wall ────────────────────────────────────────────────
    let body = lathe_profile(&bottle_body_profile(), segments)
        .expect("lathe bottle body");
    b.add_part(bottle_wall_g, &body);

    // ── Bottle bottom disk ──────────────────────────────────────────────────
    let bottom_cap = disk_fan_down(BOTTOM_DISK_RADIUS, -0.060, segments)
        .expect("bottle bottom disk");
    b.add_part(bottle_bottom_g, &bottom_cap);

    // ── Cap ─────────────────────────────────────────────────────────────────
    let cap_wall = lathe_profile(&cap_profile(), segments).expect("lathe cap");
    b.add_part(cap_g, &cap_wall);
    let cap_top = disk_fan_up(CAP_INNER_RADIUS, CAP_TOP_Y, segments)
        .expect("cap top disk");
    b.add_part(cap_g, &cap_top);
    // Underside ring of the cap so it isn't open from below.
    let cap_under = disk_fan_down(CAP_INNER_RADIUS, CAP_BOTTOM_Y, segments)
        .expect("cap underside disk");
    b.add_part(cap_g, &cap_under);

    // ── Liquid (inner wall + meniscus) ──────────────────────────────────────
    let liquid_wall =
        lathe_profile(&liquid_profile(), segments).expect("lathe liquid");
    b.add_part(liquid_g, &liquid_wall);
    // Meniscus sits at the topmost liquid profile point.
    let menisc_radius = liquid_profile().points.last().unwrap().radius;
    let menisc_y = liquid_profile().points.last().unwrap().y;
    let meniscus =
        disk_fan_up(menisc_radius, menisc_y, segments).expect("liquid meniscus");
    b.add_part(liquid_g, &meniscus);

    // ── Label band (optional, PR #15) ───────────────────────────────────────
    if let Some(url) = label_url {
        let label_g = b.add_group(
            Material::solid("bottle_label", LABEL_PAPER_COLOR)
                .with_gloss(0.20, 16.0)
                .with_texture_url(url),
        );
        let band = cylindrical_band(
            BODY_RADIUS + LABEL_RADIUS_OFFSET,
            LABEL_Y_MIN,
            LABEL_Y_MAX,
            segments,
        )
        .expect("label band");
        b.add_part(label_g, &band);
    }

    b.build()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::geometry::kernel::validate_mesh;

    #[test]
    fn bottled_sauce_mesh_is_non_empty() {
        let mesh = generate("#B8321F", BottleKind::Glass, None);
        assert!(!mesh.vertices.is_empty());
        assert_eq!(mesh.vertices.len(), mesh.normals.len());
        assert_eq!(mesh.vertices.len(), mesh.uvs.len());
        for g in &mesh.groups {
            assert!(!g.faces.is_empty(), "group {} has no faces", g.material.name);
        }
    }

    #[test]
    fn bottled_sauce_has_four_groups() {
        let mesh = generate("#B8321F", BottleKind::Glass, None);
        assert_eq!(
            mesh.groups.len(),
            4,
            "bottle_wall + bottle_bottom + cap + liquid"
        );
    }

    #[test]
    fn bottled_sauce_glass_kind_uses_glass_material_name() {
        let mesh = generate("#FF0000", BottleKind::Glass, None);
        assert!(mesh.groups.iter().any(|g| g.material.name == "bottle_glass"));
    }

    #[test]
    fn bottled_sauce_plastic_kind_uses_plastic_material_name() {
        let mesh = generate("#FF0000", BottleKind::Plastic, None);
        assert!(mesh
            .groups
            .iter()
            .any(|g| g.material.name == "bottle_plastic"));
    }

    #[test]
    fn bottled_sauce_uses_liquid_color() {
        let mesh = generate("#FF0000", BottleKind::Glass, None);
        let liquid = mesh
            .groups
            .iter()
            .find(|g| g.material.name == "liquid_material")
            .expect("liquid_material group missing");
        let [r, g, b] = liquid.material.diffuse_color;
        assert!((r - 1.0).abs() < 1e-4);
        assert!(g.abs() < 1e-4);
        assert!(b.abs() < 1e-4);
    }

    #[test]
    fn bottle_kind_from_str_detects_plastic() {
        assert!(matches!(
            BottleKind::from_str(Some("plastic_bottle")),
            BottleKind::Plastic
        ));
        assert!(matches!(
            BottleKind::from_str(Some("glass_bottle")),
            BottleKind::Glass
        ));
        assert!(matches!(BottleKind::from_str(None), BottleKind::Glass));
    }

    #[test]
    fn bottled_sauce_cap_above_neck() {
        let mesh = generate("#B8321F", BottleKind::Glass, None);
        let max_y = mesh
            .vertices
            .iter()
            .map(|v| v[1])
            .fold(f32::NEG_INFINITY, f32::max);
        assert!(
            max_y > NECK_TOP_Y - 1e-4,
            "cap top should sit at or above neck top (got {max_y})"
        );
        assert!((max_y - CAP_TOP_Y).abs() < 1e-4);
    }

    #[test]
    fn bottled_sauce_neck_radius_smaller_than_body() {
        let p = bottle_body_profile();
        assert!(
            p.max_radius() > NECK_RADIUS + 0.005,
            "body should be at least 5 mm wider than neck"
        );
    }

    #[test]
    fn bottled_sauce_passes_kernel_validation() {
        let mesh = generate("#B8321F", BottleKind::Glass, Some("#CCCCCC"));
        validate_mesh(&mesh).expect("kernel validation should pass on bottle mesh");
    }

    #[test]
    fn bottled_sauce_widest_radius_is_body_or_cap() {
        // The widest point in a sauce bottle is either the body wall
        // (BODY_RADIUS) or the cap. Should never live on the neck.
        let mesh = generate("#B8321F", BottleKind::Glass, None);
        let mut widest_xz: f32 = 0.0;
        let mut widest_y: f32 = 0.0;
        for v in &mesh.vertices {
            let r = (v[0] * v[0] + v[2] * v[2]).sqrt();
            if r > widest_xz {
                widest_xz = r;
                widest_y = v[1];
            }
        }
        // Should match BODY_RADIUS within tolerance — body wins over cap.
        assert!(
            (widest_xz - BODY_RADIUS).abs() < 1e-4,
            "widest radius should equal body radius {BODY_RADIUS}, got {widest_xz}"
        );
        // …and that widest ring lives on the body (between bottom bevel
        // and shoulder), not on the neck or above.
        assert!(
            widest_y >= -0.056 - 1e-4 && widest_y <= 0.028 + 1e-4,
            "widest ring should sit on the straight body section (got y={widest_y})"
        );
    }
}

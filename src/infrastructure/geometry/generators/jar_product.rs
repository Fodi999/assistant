//! Jar-product generator (PR #11 — kernel rewrite).
//!
//! Short, wide cylindrical jar — typical for jam, mustard, pickles, honey.
//! Produces a single mesh with **three material groups**:
//!
//!   1. **`jar_glass`** — outer wall + bottom disk. Built as a lathed
//!      profile with a small foot bevel and a subtle shoulder near the lip
//!      so highlights read on the rim.
//!   2. **`product_material`** — inner content cylinder (slightly inset)
//!      with a meniscus disk near the top.
//!   3. **`lid_metal`** — short metal cylinder with overhang + top disk +
//!      underside ring.
//!
//! All surfaces are now generated through the kernel (profile → lathe →
//! mesh_builder), so material upgrades, normals and UVs share the exact
//! same conventions as future generators (bottle, bowl, plate).
//!
//! Y-up, centred at origin, all units in metres.

use crate::infrastructure::geometry::kernel::{
    disk_fan_down, disk_fan_up, flat_patch, lathe_profile, GeometryQuality, MeshBuilder, Profile,
    ProfilePoint,
};
use crate::infrastructure::geometry::mesh::{hex_to_rgb, Material, Mesh};

// Label patch placement on the front face of the jar (PR #15).
const LABEL_WIDTH: f32 = 0.050; // 5 cm
const LABEL_HEIGHT: f32 = 0.040; // 4 cm
const LABEL_CENTER_Y: f32 = 0.000; // mid-jar
const LABEL_DEPTH_OFFSET: f32 = 0.0006; // 0.6 mm in front of the wall
const LABEL_PAPER_COLOR: [f32; 3] = [0.97, 0.96, 0.93];

// ── Default dimensions (metres) ─────────────────────────────────────────────
const JAR_RADIUS: f32 = 0.040; // 4 cm wall radius
const JAR_HEIGHT: f32 = 0.080; // 8 cm body height
const JAR_FOOT_BEVEL: f32 = 0.0035; // 3.5 mm foot bevel (rounded base)
const JAR_SHOULDER_INSET: f32 = 0.0015; // 1.5 mm subtle inward kick at the rim
const JAR_SHOULDER_HEIGHT: f32 = 0.004; // top 4 mm narrow toward the lid

const LID_HEIGHT: f32 = 0.012; // 1.2 cm
const LID_OVERHANG: f32 = 0.0025; // 2.5 mm radial overhang past the jar wall
const LID_RIM_BEVEL: f32 = 0.0010; // 1 mm chamfered rim

/// How full the jar is (0..1).
const FILL_RATIO: f32 = 0.88;

/// Inset of the product cylinder relative to the jar inner wall (avoids z-fight).
const PRODUCT_INSET: f32 = 0.0016; // 1.6 mm

// ── Default colours ─────────────────────────────────────────────────────────
const JAR_GLASS_COLOR: [f32; 3] = [0.92, 0.95, 0.94];
const LID_DEFAULT_COLOR: [f32; 3] = [0.62, 0.50, 0.18];

/// Generate a jar-product mesh.
///
/// - `product_color_hex` — hex colour of the product inside.
/// - `lid_color_hex` — optional override for the lid colour.
pub fn generate(product_color_hex: &str, lid_color_hex: Option<&str>) -> Mesh {
    generate_with_label(product_color_hex, lid_color_hex, None)
}

/// Same as [`generate`] but additionally adds a flat rectangular label
/// patch on the front face of the jar. The `label_url` is stored as
/// `materials[i].extras.texture_url` in the GLB.
pub fn generate_with_label(
    product_color_hex: &str,
    lid_color_hex: Option<&str>,
    label_url: Option<&str>,
) -> Mesh {
    generate_with_label_and_quality(
        product_color_hex,
        lid_color_hex,
        label_url,
        GeometryQuality::default(),
    )
}

/// Full entrypoint with explicit [`GeometryQuality`].
pub fn generate_with_label_and_quality(
    product_color_hex: &str,
    lid_color_hex: Option<&str>,
    label_url: Option<&str>,
    quality: GeometryQuality,
) -> Mesh {
    let product_color = hex_to_rgb(product_color_hex);
    let lid_color = lid_color_hex.map(hex_to_rgb).unwrap_or(LID_DEFAULT_COLOR);

    let segments = quality.radial_segments();

    let mut b = MeshBuilder::new();

    // ── Materials & groups ──────────────────────────────────────────────────
    // Frontend matches by name: `*glass*` → transmissive glass,
    // `*metal*|*lid*|*cap*` → metallic, `*product*|*sauce*|*liquid*` → glossy.
    let glass_g = b.add_group(
        Material::solid("jar_glass", JAR_GLASS_COLOR).with_gloss(0.50, 96.0),
    );
    let product_g = b.add_group(
        Material::solid("product_material", product_color).with_gloss(0.45, 64.0),
    );
    let lid_g = b.add_group(
        Material::solid("lid_metal", lid_color).with_gloss(0.65, 80.0),
    );

    // ── Glass jar: lathed wall with foot bevel + shoulder ───────────────────
    let jar_bottom = -JAR_HEIGHT / 2.0;
    let jar_top = JAR_HEIGHT / 2.0;

    // Profile (radius, y) from the bottom of the foot up to the rim:
    //   * (R - bevel,        bottom)              – inner edge of foot
    //   * (R,                bottom + bevel)      – wall starts here
    //   * (R,                jar_top - shoulder)  – plain wall
    //   * (R - shoulder_in,  jar_top)             – subtle inward shoulder
    let r = JAR_RADIUS;
    let jar_profile = Profile::new(vec![
        ProfilePoint::new(r - JAR_FOOT_BEVEL, jar_bottom),
        ProfilePoint::new(r, jar_bottom + JAR_FOOT_BEVEL),
        ProfilePoint::new(r, jar_top - JAR_SHOULDER_HEIGHT),
        ProfilePoint::new(r - JAR_SHOULDER_INSET, jar_top),
    ])
    .expect("hard-coded jar profile is valid");
    let jar_wall = lathe_profile(&jar_profile, segments).expect("lathe jar wall");
    b.add_part(glass_g, &jar_wall);

    // Bottom disk caps the foot opening.
    let jar_bottom_cap = disk_fan_down(r - JAR_FOOT_BEVEL, jar_bottom, segments)
        .expect("jar bottom disk");
    b.add_part(glass_g, &jar_bottom_cap);

    // ── Product: inset cylinder + meniscus ──────────────────────────────────
    let product_radius = JAR_RADIUS - PRODUCT_INSET;
    let product_top_y = jar_bottom + JAR_HEIGHT * FILL_RATIO;
    let product_profile = Profile::new(vec![
        ProfilePoint::new(product_radius, jar_bottom + 0.0005),
        ProfilePoint::new(product_radius, product_top_y),
    ])
    .expect("hard-coded product profile is valid");
    let product_wall =
        lathe_profile(&product_profile, segments).expect("lathe product wall");
    b.add_part(product_g, &product_wall);

    let meniscus =
        disk_fan_up(product_radius, product_top_y, segments).expect("meniscus disk");
    b.add_part(product_g, &meniscus);

    // ── Lid: metal cylinder with overhang + top + underside ─────────────────
    let lid_bottom = jar_top;
    let lid_top = jar_top + LID_HEIGHT;
    let lid_radius = JAR_RADIUS + LID_OVERHANG;

    // Profile: tiny chamfer at the rim so the metal catches a highlight.
    let lid_profile = Profile::new(vec![
        ProfilePoint::new(lid_radius - LID_RIM_BEVEL, lid_bottom),
        ProfilePoint::new(lid_radius, lid_bottom + LID_RIM_BEVEL),
        ProfilePoint::new(lid_radius, lid_top - LID_RIM_BEVEL),
        ProfilePoint::new(lid_radius - LID_RIM_BEVEL, lid_top),
    ])
    .expect("hard-coded lid profile is valid");
    let lid_wall = lathe_profile(&lid_profile, segments).expect("lathe lid wall");
    b.add_part(lid_g, &lid_wall);

    // Top of lid (sealed disk, faces up).
    let lid_top_cap = disk_fan_up(lid_radius - LID_RIM_BEVEL, lid_top, segments)
        .expect("lid top disk");
    b.add_part(lid_g, &lid_top_cap);

    // Underside of lid (faces down, hidden by jar but keeps the mesh closed).
    let lid_bottom_cap =
        disk_fan_down(lid_radius - LID_RIM_BEVEL, lid_bottom, segments)
            .expect("lid underside disk");
    b.add_part(lid_g, &lid_bottom_cap);

    // ── Label patch (optional, PR #15) ──────────────────────────────────────
    if let Some(url) = label_url {
        let label_g = b.add_group(
            Material::solid("jar_label", LABEL_PAPER_COLOR)
                .with_gloss(0.20, 16.0)
                .with_texture_url(url),
        );
        let patch = flat_patch(
            LABEL_WIDTH,
            LABEL_HEIGHT,
            LABEL_CENTER_Y,
            JAR_RADIUS + LABEL_DEPTH_OFFSET,
        )
        .expect("jar label patch");
        b.add_part(label_g, &patch);
    }

    b.build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jar_product_mesh_is_non_empty() {
        let mesh = generate("#A85B12", None);
        assert!(!mesh.vertices.is_empty());
        assert_eq!(mesh.vertices.len(), mesh.normals.len());
        assert_eq!(mesh.vertices.len(), mesh.uvs.len());
        assert_eq!(mesh.groups.len(), 3, "glass + product + lid");
        for g in &mesh.groups {
            assert!(!g.faces.is_empty(), "group {} has no faces", g.material.name);
        }
    }

    #[test]
    fn jar_product_uses_product_color() {
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
    fn jar_product_has_glass_and_lid_groups() {
        let mesh = generate("#A85B12", None);
        assert!(mesh.groups.iter().any(|g| g.material.name == "jar_glass"));
        assert!(mesh.groups.iter().any(|g| g.material.name == "lid_metal"));
    }

    #[test]
    fn jar_product_lid_color_override_applies() {
        let mesh = generate("#A85B12", Some("#00FF00"));
        let lid = mesh
            .groups
            .iter()
            .find(|g| g.material.name == "lid_metal")
            .unwrap();
        let [r, g, b] = lid.material.diffuse_color;
        assert!(r.abs() < 1e-4);
        assert!((g - 1.0).abs() < 1e-4);
        assert!(b.abs() < 1e-4);
    }

    #[test]
    fn jar_product_lid_extends_above_jar() {
        let mesh = generate("#A85B12", None);
        let max_y = mesh
            .vertices
            .iter()
            .map(|v| v[1])
            .fold(f32::NEG_INFINITY, f32::max);
        assert!(max_y > JAR_HEIGHT / 2.0 + 1e-4);
    }

    #[test]
    fn jar_product_passes_kernel_validation() {
        use crate::infrastructure::geometry::kernel::validate_mesh;
        let mesh = generate("#A85B12", Some("#CCCCCC"));
        validate_mesh(&mesh).expect("kernel validation should pass on jar mesh");
    }

    #[test]
    fn jar_product_has_widest_radius_at_lid() {
        // Lid overhangs the jar wall by `LID_OVERHANG` — the largest radius
        // across the whole mesh must therefore live above jar_top.
        let mesh = generate("#A85B12", None);
        let mut widest_xz: f32 = 0.0;
        let mut widest_y: f32 = 0.0;
        for v in &mesh.vertices {
            let r = (v[0] * v[0] + v[2] * v[2]).sqrt();
            if r > widest_xz {
                widest_xz = r;
                widest_y = v[1];
            }
        }
        assert!(widest_xz > JAR_RADIUS + LID_OVERHANG * 0.5);
        assert!(
            widest_y > JAR_HEIGHT / 2.0 - 1e-4,
            "widest ring must sit at or above jar_top (got y={widest_y})"
        );
    }
}

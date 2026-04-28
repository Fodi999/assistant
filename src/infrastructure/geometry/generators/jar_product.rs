//! Jar-product generator (PR #7).
//!
//! A short, wide cylindrical jar — typical for jam, mustard, pickles, honey.
//! Produces a single mesh with **three material groups**:
//!
//!   1. **Jar glass** — outer wall + bottom disk (frontend upgrades any
//!      `*glass*` material to `MeshPhysicalMaterial` with transmission).
//!   2. **Product** — inner content cylinder filled with `product.color_hex`,
//!      with a meniscus disk near the top so the colour is visible through
//!      the lid gap.
//!   3. **Lid** — short metal cylinder + top disk + slight rim overhang.
//!
//! Y-up, centred at origin. All units in metres.

use std::f32::consts::PI;

use crate::infrastructure::geometry::mesh::{hex_to_rgb, Material, MaterialGroup, Mesh};

const SEGMENTS: usize = 48;

// ── Default dimensions (metres) ─────────────────────────────────────────────
const JAR_RADIUS: f32 = 0.040; // 4 cm
const JAR_HEIGHT: f32 = 0.080; // 8 cm
const LID_HEIGHT: f32 = 0.012; // 1.2 cm
const LID_OVERHANG: f32 = 0.0025; // 2.5 mm radial overhang

/// How full the jar is (0..1).
const FILL_RATIO: f32 = 0.88;

// ── Default colours ─────────────────────────────────────────────────────────
const JAR_GLASS_COLOR: [f32; 3] = [0.92, 0.95, 0.94]; // light cool tint
const LID_DEFAULT_COLOR: [f32; 3] = [0.62, 0.50, 0.18]; // warm gold (jam/honey vibe)

/// Generate a jar-product mesh.
///
/// - `product_color_hex` — hex colour of the product inside (`product.color_hex`).
/// - `lid_color_hex` — optional override for the lid colour.
pub fn generate(product_color_hex: &str, lid_color_hex: Option<&str>) -> Mesh {
    let product_color = hex_to_rgb(product_color_hex);
    let lid_color = lid_color_hex.map(hex_to_rgb).unwrap_or(LID_DEFAULT_COLOR);

    let mut verts: Vec<[f32; 3]> = Vec::new();
    let mut norms: Vec<[f32; 3]> = Vec::new();
    let mut uvs: Vec<[f32; 2]> = Vec::new();

    let jar_bottom = -JAR_HEIGHT / 2.0;
    let jar_top = JAR_HEIGHT / 2.0;
    let lid_bottom = jar_top;
    let lid_top = jar_top + LID_HEIGHT;
    let product_top_y = jar_bottom + JAR_HEIGHT * FILL_RATIO;

    // ── Glass (wall + bottom) ───────────────────────────────────────────────
    let mut glass_faces: Vec<[usize; 3]> = Vec::new();
    append_cylinder_wall(
        &mut verts,
        &mut norms,
        &mut uvs,
        &mut glass_faces,
        JAR_RADIUS,
        JAR_RADIUS,
        jar_bottom,
        jar_top,
    );
    append_disk(
        &mut verts,
        &mut norms,
        &mut uvs,
        &mut glass_faces,
        JAR_RADIUS,
        jar_bottom,
        false,
    );

    // ── Product (slightly inset cylinder + meniscus) ────────────────────────
    let product_radius = JAR_RADIUS * 0.96;
    let mut product_faces: Vec<[usize; 3]> = Vec::new();
    append_cylinder_wall(
        &mut verts,
        &mut norms,
        &mut uvs,
        &mut product_faces,
        product_radius,
        product_radius,
        jar_bottom + 0.0005,
        product_top_y,
    );
    append_disk(
        &mut verts,
        &mut norms,
        &mut uvs,
        &mut product_faces,
        product_radius,
        product_top_y,
        true,
    );

    // ── Lid (cylinder with overhang + top disk) ─────────────────────────────
    let lid_radius = JAR_RADIUS + LID_OVERHANG;
    let mut lid_faces: Vec<[usize; 3]> = Vec::new();
    append_cylinder_wall(
        &mut verts,
        &mut norms,
        &mut uvs,
        &mut lid_faces,
        lid_radius,
        lid_radius,
        lid_bottom,
        lid_top,
    );
    append_disk(
        &mut verts,
        &mut norms,
        &mut uvs,
        &mut lid_faces,
        lid_radius,
        lid_top,
        true,
    );
    // Underside ring of the overhang so the lid isn't open from below.
    append_disk(
        &mut verts,
        &mut norms,
        &mut uvs,
        &mut lid_faces,
        lid_radius,
        lid_bottom,
        false,
    );

    // ── Materials ───────────────────────────────────────────────────────────
    // Frontend upgrades by name: `*glass*` → transmissive, `*metal*`/`*lid*`
    // → metallic, `*product*`/`*sauce*` → glossy diffuse.
    let glass_mat =
        Material::solid("jar_glass", JAR_GLASS_COLOR).with_gloss(0.50, 96.0);
    let product_mat =
        Material::solid("product_material", product_color).with_gloss(0.45, 64.0);
    let lid_mat = Material::solid("lid_metal", lid_color).with_gloss(0.65, 80.0);

    Mesh::new_multi(
        verts,
        norms,
        uvs,
        vec![
            MaterialGroup { material: glass_mat, faces: glass_faces },
            MaterialGroup { material: product_mat, faces: product_faces },
            MaterialGroup { material: lid_mat, faces: lid_faces },
        ],
    )
}

// ─────────────────────────────────────────────────────────────────────────────
// Geometry helpers (cylinder wall + disk) — same shape as in `bottled_sauce`.
// Kept duplicated for now; will be lifted into a shared `primitives` module
// when a third caller appears (e.g. `plate_food` in PR #10).
// ─────────────────────────────────────────────────────────────────────────────

fn append_cylinder_wall(
    verts: &mut Vec<[f32; 3]>,
    norms: &mut Vec<[f32; 3]>,
    uvs: &mut Vec<[f32; 2]>,
    faces: &mut Vec<[usize; 3]>,
    r_bot: f32,
    r_top: f32,
    y_bot: f32,
    y_top: f32,
) {
    let base = verts.len();
    let height = (y_top - y_bot).max(1e-6);
    let slope = (r_top - r_bot) / height;

    for i in 0..=SEGMENTS {
        let t = i as f32 / SEGMENTS as f32;
        let theta = t * 2.0 * PI;
        let cos_t = theta.cos();
        let sin_t = theta.sin();

        let nx = cos_t;
        let ny = -slope;
        let nz = sin_t;
        let len = (nx * nx + ny * ny + nz * nz).sqrt();
        let n = [nx / len, ny / len, nz / len];

        verts.push([cos_t * r_bot, y_bot, sin_t * r_bot]);
        norms.push(n);
        uvs.push([t, 0.0]);

        verts.push([cos_t * r_top, y_top, sin_t * r_top]);
        norms.push(n);
        uvs.push([t, 1.0]);
    }

    for i in 0..SEGMENTS {
        let b0 = base + i * 2;
        let t0 = base + i * 2 + 1;
        let b1 = base + (i + 1) * 2;
        let t1 = base + (i + 1) * 2 + 1;
        faces.push([b0, b1, t1]);
        faces.push([b0, t1, t0]);
    }
}

fn append_disk(
    verts: &mut Vec<[f32; 3]>,
    norms: &mut Vec<[f32; 3]>,
    uvs: &mut Vec<[f32; 2]>,
    faces: &mut Vec<[usize; 3]>,
    r: f32,
    y: f32,
    face_up: bool,
) {
    let n = if face_up { [0.0, 1.0, 0.0] } else { [0.0, -1.0, 0.0] };
    let centre = verts.len();
    verts.push([0.0, y, 0.0]);
    norms.push(n);
    uvs.push([0.5, 0.5]);

    for i in 0..=SEGMENTS {
        let t = i as f32 / SEGMENTS as f32;
        let theta = t * 2.0 * PI;
        let x = theta.cos() * r;
        let z = theta.sin() * r;
        verts.push([x, y, z]);
        norms.push(n);
        uvs.push([0.5 + theta.cos() * 0.5, 0.5 + theta.sin() * 0.5]);
    }

    for i in 0..SEGMENTS {
        if face_up {
            faces.push([centre, centre + 1 + i, centre + 1 + i + 1]);
        } else {
            faces.push([centre, centre + 1 + i + 1, centre + 1 + i]);
        }
    }
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
            assert!(!g.faces.is_empty());
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
}

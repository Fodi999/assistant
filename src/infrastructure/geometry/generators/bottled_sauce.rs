//! Bottled-sauce generator (PR #7).
//!
//! Produces a single mesh with **four material groups**:
//!
//!   1. **Body** — main vertical cylinder, the bottle wall (glass or plastic).
//!   2. **Neck** — short frustum tapering from body radius to cap radius.
//!   3. **Cap** — short cylinder + top disk on top of the neck.
//!   4. **Liquid** — inner cylinder filled with the product colour, with a
//!      visible meniscus disk near the top of the body (so the colour shows
//!      through the open neck once the frontend applies glass transmission).
//!
//! Naming convention for materials is important: the frontend (`ObjViewer`)
//! upgrades materials by name —
//!   * `*glass*` → `MeshPhysicalMaterial` with transmission/opacity
//!   * `*liquid*` / `*sauce*` → glossy `MeshStandardMaterial`
//!   * `*metal*` / `*cap*` → metallic `MeshStandardMaterial`
//!
//! All geometry is in metres, Y-up, centred at origin (Y = 0 is mid-height of
//! the body). Dimensions can be driven by `ContainerSpec` `diameter_mm` /
//! `height_mm` in a later PR.

use std::f32::consts::PI;

use crate::infrastructure::geometry::mesh::{hex_to_rgb, Material, MaterialGroup, Mesh};

/// Tessellation around the rotational axis.
const SEGMENTS: usize = 48;

// ── Default dimensions (metres) ─────────────────────────────────────────────
const BODY_RADIUS: f32 = 0.030; // 3 cm — slim sauce bottle
const BODY_HEIGHT: f32 = 0.120; // 12 cm
const NECK_HEIGHT: f32 = 0.025; // 2.5 cm shoulder + neck
const NECK_TOP_RADIUS: f32 = 0.011; // 1.1 cm
const CAP_HEIGHT: f32 = 0.018; // 1.8 cm
const CAP_RADIUS: f32 = 0.013; // slightly wider than neck

/// How full the bottle is (0..1) — used to position the liquid meniscus.
const FILL_RATIO: f32 = 0.85;

// ── Default colours ─────────────────────────────────────────────────────────
const BOTTLE_GLASS_COLOR: [f32; 3] = [0.90, 0.93, 0.92]; // very light cool tint
const BOTTLE_PLASTIC_COLOR: [f32; 3] = [0.96, 0.96, 0.96];
const CAP_DEFAULT_COLOR: [f32; 3] = [0.55, 0.55, 0.58]; // brushed metal grey

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
    let liquid_color = hex_to_rgb(liquid_color_hex);
    let bottle_color = bottle_kind.default_color();
    let cap_color = cap_color_hex.map(hex_to_rgb).unwrap_or(CAP_DEFAULT_COLOR);

    let mut verts: Vec<[f32; 3]> = Vec::new();
    let mut norms: Vec<[f32; 3]> = Vec::new();
    let mut uvs: Vec<[f32; 2]> = Vec::new();

    // Reference Y coordinates (centre of body at Y = 0).
    let body_bottom = -BODY_HEIGHT / 2.0;
    let body_top = BODY_HEIGHT / 2.0;
    let neck_top = body_top + NECK_HEIGHT;
    let cap_bottom = neck_top;
    let cap_top = neck_top + CAP_HEIGHT;
    let liquid_top_y = body_bottom + BODY_HEIGHT * FILL_RATIO;

    // ── Body (cylinder + bottom disk) ───────────────────────────────────────
    let mut body_faces: Vec<[usize; 3]> = Vec::new();
    append_cylinder_wall(
        &mut verts,
        &mut norms,
        &mut uvs,
        &mut body_faces,
        BODY_RADIUS,
        BODY_RADIUS,
        body_bottom,
        body_top,
    );
    append_disk(
        &mut verts,
        &mut norms,
        &mut uvs,
        &mut body_faces,
        BODY_RADIUS,
        body_bottom,
        false, // facing -Y
    );

    // ── Neck (frustum) ──────────────────────────────────────────────────────
    let mut neck_faces: Vec<[usize; 3]> = Vec::new();
    append_cylinder_wall(
        &mut verts,
        &mut norms,
        &mut uvs,
        &mut neck_faces,
        BODY_RADIUS,      // bottom radius (matches body)
        NECK_TOP_RADIUS,  // top radius (narrower)
        body_top,
        neck_top,
    );

    // ── Cap (cylinder + top disk) ───────────────────────────────────────────
    let mut cap_faces: Vec<[usize; 3]> = Vec::new();
    append_cylinder_wall(
        &mut verts,
        &mut norms,
        &mut uvs,
        &mut cap_faces,
        CAP_RADIUS,
        CAP_RADIUS,
        cap_bottom,
        cap_top,
    );
    append_disk(
        &mut verts,
        &mut norms,
        &mut uvs,
        &mut cap_faces,
        CAP_RADIUS,
        cap_top,
        true, // facing +Y
    );

    // ── Liquid (slightly inset cylinder + meniscus disk) ────────────────────
    // Inset so a glass shader doesn't z-fight with the bottle wall.
    let liquid_radius = BODY_RADIUS * 0.96;
    let mut liquid_faces: Vec<[usize; 3]> = Vec::new();
    append_cylinder_wall(
        &mut verts,
        &mut norms,
        &mut uvs,
        &mut liquid_faces,
        liquid_radius,
        liquid_radius,
        body_bottom + 0.0005, // tiny lift off the inner bottom
        liquid_top_y,
    );
    append_disk(
        &mut verts,
        &mut norms,
        &mut uvs,
        &mut liquid_faces,
        liquid_radius,
        liquid_top_y,
        true, // meniscus faces +Y
    );

    // ── Materials ───────────────────────────────────────────────────────────
    let bottle_mat = Material::solid(bottle_kind.material_name(), bottle_color)
        // Glassy / glossy highlight — frontend will upgrade by name.
        .with_gloss(0.45, 80.0);
    let neck_mat = Material::solid(bottle_kind.material_name(), bottle_color)
        .with_gloss(0.45, 80.0);
    let cap_mat = Material::solid("cap_metal", cap_color).with_gloss(0.60, 64.0);
    let liquid_mat =
        Material::solid("liquid_material", liquid_color).with_gloss(0.55, 96.0);

    Mesh::new_multi(
        verts,
        norms,
        uvs,
        vec![
            MaterialGroup { material: bottle_mat, faces: body_faces },
            // Neck shares the same kind of material but lives in its own group
            // to keep the OBJ structure clean and to allow per-group tweaks
            // later (e.g. label band on the body only).
            MaterialGroup { material: neck_mat, faces: neck_faces },
            MaterialGroup { material: cap_mat, faces: cap_faces },
            MaterialGroup { material: liquid_mat, faces: liquid_faces },
        ],
    )
}

// ─────────────────────────────────────────────────────────────────────────────
// Geometry helpers (generic cylinder wall + disk).
// Local copies — kept here to avoid prematurely abstracting; will be lifted
// into a shared `primitives` module once a 3rd generator needs them.
// ─────────────────────────────────────────────────────────────────────────────

/// Append a (possibly tapered) cylinder wall between `y_bot` and `y_top`,
/// pushing all required attributes and triangle indices.
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

        let xb = cos_t * r_bot;
        let zb = sin_t * r_bot;
        let xt = cos_t * r_top;
        let zt = sin_t * r_top;

        // Outward-pointing normal, slightly tilted for tapered walls.
        let nx = cos_t;
        let ny = -slope;
        let nz = sin_t;
        let len = (nx * nx + ny * ny + nz * nz).sqrt();
        let n = [nx / len, ny / len, nz / len];

        verts.push([xb, y_bot, zb]);
        norms.push(n);
        uvs.push([t, 0.0]);

        verts.push([xt, y_top, zt]);
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

/// Append a flat disk at height `y` with radius `r`, facing either +Y or -Y.
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
    fn bottled_sauce_mesh_is_non_empty() {
        let mesh = generate("#B8321F", BottleKind::Glass, None);
        assert!(!mesh.vertices.is_empty());
        assert_eq!(mesh.vertices.len(), mesh.normals.len());
        assert_eq!(mesh.vertices.len(), mesh.uvs.len());
        assert_eq!(mesh.groups.len(), 4, "body + neck + cap + liquid");
        for g in &mesh.groups {
            assert!(!g.faces.is_empty(), "every group must have faces");
        }
    }

    #[test]
    fn bottled_sauce_glass_kind_uses_glass_material_name() {
        let mesh = generate("#FF0000", BottleKind::Glass, None);
        assert!(mesh
            .groups
            .iter()
            .any(|g| g.material.name == "bottle_glass"));
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
    fn bottled_sauce_liquid_uses_product_color() {
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
    fn bottled_sauce_extends_above_body() {
        // Cap must sit above the body top — verifies neck + cap stacking.
        let mesh = generate("#B8321F", BottleKind::Glass, None);
        let max_y = mesh
            .vertices
            .iter()
            .map(|v| v[1])
            .fold(f32::NEG_INFINITY, f32::max);
        assert!(
            max_y > BODY_HEIGHT / 2.0 + NECK_HEIGHT - 1e-4,
            "cap should sit above body+neck"
        );
    }
}

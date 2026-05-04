//! Geometry dispatcher — routes `object_type` string → generator.
//!
//! Implemented:
//!   * `sauce_in_bowl`   — bowl frustum + swirl sauce surface
//!   * `bottled_sauce`   — body + neck + cap + liquid
//!   * `jar_product`     — wide glass jar + product + metal lid
//!   * `plate_food`      — ceramic plate + radial food mound
//!   * `product_card`    — rounded-rectangle card
//!   * `card_dock`       — hard-surface B-Rep-lite slot/cradle
//!   * `sci_fi_card`     — Plasticity-style precision hard-surface card
//!   * `organic_sphere`  — ZBrush-style UV sphere (3 material groups)
//!   * `flat_card`       — fallback rectangular card

use serde_json::Value;

use crate::application::laboratory_v2::Product3DSpec;
use crate::infrastructure::geometry::generators::food::{
    bottled_sauce, flat_card, jar_product, plate_food, sauce_in_bowl,
};
use crate::infrastructure::geometry::generators::hard_surface::card;
use crate::infrastructure::geometry::generators::hard_surface::sci_fi_card;
use crate::infrastructure::geometry::generators::hard_surface::organic_sphere;
use crate::infrastructure::geometry::kernel::GeometryQuality;
use crate::infrastructure::geometry::mesh::Mesh;
use crate::shared::AppError;

/// Generate a [`Mesh`] from `object_type` (the string stored in
/// `laboratory_3d_assets.object_type`) and the raw `object_spec_json`.
///
/// Uses [`GeometryQuality::default`] (= `High`).
///
/// The caller can pass `None` for `spec` if the spec is unavailable — all
/// generators have sensible defaults.
pub fn dispatch(object_type: &str, spec: Option<&Value>) -> Result<Mesh, AppError> {
    dispatch_with_quality(object_type, spec, GeometryQuality::default())
}

/// Same as [`dispatch`] but with an explicit [`GeometryQuality`] preset.
///
/// Studio default is `High`. Final-render exports use `Ultra`. The frontend
/// `Render Quality` switch is independent of this — render quality changes
/// instantly, geometry quality requires regenerating the GLB.
pub fn dispatch_with_quality(
    object_type: &str,
    spec: Option<&Value>,
    quality: GeometryQuality,
) -> Result<Mesh, AppError> {
    // Attempt to deserialise full Product3DSpec once — used by generators that
    // need rich Vision data (e.g. sauce surface params). Failure is non-fatal;
    // generators fall back to defaults.
    let full_spec: Option<Product3DSpec> = spec
        .and_then(|v| serde_json::from_value(v.clone()).ok());

    match object_type {
        "sauce_in_bowl" => {
            let sauce_color = extract_str(spec, "/product/color_hex").unwrap_or("#B8321F");
            let container_color = extract_str(spec, "/container/color_hex");
            let surface = full_spec.as_ref().and_then(|s| s.product.surface.as_ref());
            let container = full_spec.as_ref().and_then(|s| s.container.as_ref());
            Ok(sauce_in_bowl::generate_with_surface_and_quality(
                sauce_color,
                container_color,
                container,
                surface,
                quality,
            ))
        }
        "bottled_sauce" => {
            let liquid_color = extract_str(spec, "/product/color_hex").unwrap_or("#B8321F");
            let kind = bottled_sauce::BottleKind::from_str(
                extract_str(spec, "/container/kind"),
            );
            let label_url = extract_str(spec, "/labels/main_url");
            // Cap colour is not in the current spec — leave as default for now.
            Ok(bottled_sauce::generate_with_label_and_quality(
                liquid_color,
                kind,
                None,
                label_url,
                quality,
            ))
        }
        "jar_product" => {
            let product_color = extract_str(spec, "/product/color_hex").unwrap_or("#A85B12");
            let lid_color = extract_str(spec, "/container/color_hex");
            let label_url = extract_str(spec, "/labels/main_url");
            Ok(jar_product::generate_with_label_and_quality(
                product_color,
                lid_color,
                label_url,
                quality,
            ))
        }
        "plate_food" => {
            let product_color = extract_str(spec, "/product/color_hex").unwrap_or("#A85B12");
            let plate_color = extract_str(spec, "/container/color_hex");
            let surface = spec.and_then(|v| v.pointer("/product/surface"))
                .and_then(|v| serde_json::from_value::<crate::application::laboratory_v2::ProductSurfaceSpec>(v.clone()).ok());
            Ok(plate_food::generate_with_surface_and_quality(
                product_color,
                plate_color,
                surface.as_ref(),
                quality,
            ))
        }
        // Procedural rounded-rectangle card (PR extrude-kernel).
        // Spec fields:
        //   /product/color_hex  — front face colour (default #CCCCCC)
        //   /card/width         — width  in metres  (default 0.10)
        //   /card/height        — height in metres  (default 0.14)
        //   /card/thickness     — depth  in metres  (default 0.008)
        //   /card/corner_radius — arc radius        (default 0.012)
        //   /card/bevel         — chamfer width     (default 0.001)
        "product_card" => {
            use card::{generate_card, CardSpec};
            let color_hex = extract_str(spec, "/product/color_hex").unwrap_or("#CCCCCC");
            let width   = extract_f32(spec, "/card/width")         .unwrap_or(0.10);
            let height  = extract_f32(spec, "/card/height")        .unwrap_or(0.14);
            let thick   = extract_f32(spec, "/card/thickness")     .unwrap_or(0.008);
            let radius  = extract_f32(spec, "/card/corner_radius") .unwrap_or(0.012);
            let bevel   = extract_f32(spec, "/card/bevel")         .unwrap_or(0.001);
            let card_spec = CardSpec {
                width, height, thickness: thick,
                corner_radius: radius, bevel,
                color_hex, quality,
            };
            Ok(generate_card(&card_spec))
        }
        // CardDock — hard-surface B-Rep-lite slot/cradle.
        // Spec fields (all optional, safe defaults apply):
        //   /dock/width              — outer width  m (default 0.20)
        //   /dock/depth              — outer depth  m (default 0.58)
        //   /dock/height             — base height  m (default 0.10)
        //   /dock/slot_width         — slot opening (default 0.11)
        //   /dock/slot_depth         — slot depth   (default 0.15)
        //   /dock/accent_hex         — glow colour  (default #00C8FF)
        "card_dock" => {
            use crate::infrastructure::geometry::generators::hard_surface::dock::{
                generate_dock, CardDockSpec,
            };
            let dock_spec = CardDockSpec {
                width:      extract_f32(spec, "/dock/width")      .unwrap_or(0.20),
                depth:      extract_f32(spec, "/dock/depth")      .unwrap_or(0.58),
                height:     extract_f32(spec, "/dock/height")     .unwrap_or(0.10),
                slot_width: extract_f32(spec, "/dock/slot_width") .unwrap_or(0.11),
                slot_depth: extract_f32(spec, "/dock/slot_depth") .unwrap_or(0.15),
                accent_hex: "#00C8FF", // TODO: extract from spec when &'static str is relaxed
                quality,
                ..CardDockSpec::default()
            };
            Ok(generate_dock(&dock_spec))
        }
        // Sci-Fi Product Card — Plasticity-style precision hard-surface card.
        // Spec fields (all optional, safe defaults apply):
        //   /card/width              — width  m (default 0.12)
        //   /card/height             — height m (default 0.18)
        //   /card/thickness          — depth  m (default 0.012)
        //   /card/corner_radius      — arc radius (default 0.012)
        //   /card/bevel              — chamfer    (default 0.0015)
        //   /card/accent_hex         — glow colour (default #00C8FF)
        "sci_fi_card" => {
            use sci_fi_card::{generate_sci_fi_card, SciFiCardSpec};
            let spec_obj = SciFiCardSpec {
                width:     extract_f32(spec, "/card/width")         .unwrap_or(0.12),
                height:    extract_f32(spec, "/card/height")        .unwrap_or(0.18),
                thickness: extract_f32(spec, "/card/thickness")     .unwrap_or(0.012),
                corner_radius: extract_f32(spec, "/card/corner_radius").unwrap_or(0.012),
                bevel:     extract_f32(spec, "/card/bevel")         .unwrap_or(0.0015),
                accent_hex: extract_str(spec, "/card/accent_hex")
                    .unwrap_or("#00C8FF")
                    .to_string(),
                quality,
                ..SciFiCardSpec::default()
            };
            Ok(generate_sci_fi_card(&spec_obj))
        }
        // Organic Sphere — ZBrush-style UV sphere, 3 material groups.
        // Spec fields (all optional):
        //   /sphere/radius     — radius m (default 0.12)
        //   /sphere/color_hex  — base colour (default #B8B8C8)
        "organic_sphere" => {
            use organic_sphere::{generate_organic_sphere, OrganicSphereSpec};
            let spec_obj = OrganicSphereSpec {
                radius:    extract_f32(spec, "/sphere/radius")     .unwrap_or(0.12),
                color_hex: extract_str(spec, "/sphere/color_hex")
                    .unwrap_or("#B8B8C8")
                    .to_string(),
                ..OrganicSphereSpec::with_quality(quality)
            };
            Ok(generate_organic_sphere(&spec_obj))
        }
        // ── Copilot primitive shapes ─────────────────────────────────────────
        // Simple one-material primitives spawnable from the chat.
        // Optional: /shape/color_hex (default varies per shape)
        "shape_square" => {
            use crate::infrastructure::geometry::generators::primitives as prim;
            let c = extract_str(spec, "/shape/color_hex").unwrap_or("#38BDF8");
            Ok(prim::generate_square(c))
        }
        "shape_rectangle" => {
            use crate::infrastructure::geometry::generators::primitives as prim;
            let c = extract_str(spec, "/shape/color_hex").unwrap_or("#A78BFA");
            Ok(prim::generate_rectangle(c))
        }
        "shape_triangle" => {
            use crate::infrastructure::geometry::generators::primitives as prim;
            let c = extract_str(spec, "/shape/color_hex").unwrap_or("#FB923C");
            Ok(prim::generate_triangle(c))
        }
        "shape_circle" => {
            use crate::infrastructure::geometry::generators::primitives as prim;
            let c = extract_str(spec, "/shape/color_hex").unwrap_or("#34D399");
            Ok(prim::generate_circle(c, quality))
        }
        "shape_cube" => {
            use crate::infrastructure::geometry::generators::primitives as prim;
            let c   = extract_str(spec, "/shape/color_hex").unwrap_or("#F472B6");
            let sub = extract_f32(spec, "/shape/subdivisions").map(|v| v as u32).unwrap_or(match quality {
                GeometryQuality::Draft    => 1,
                GeometryQuality::Standard => 2,
                GeometryQuality::High     => 3,
                GeometryQuality::Ultra    => 5,
            });
            let bevel = extract_f32(spec, "/shape/bevel").unwrap_or(0.0);
            Ok(prim::generate_cube_grid(c, sub, bevel))
        }
        "shape_sphere" => {
            use crate::infrastructure::geometry::generators::primitives as prim;
            let c = extract_str(spec, "/shape/color_hex").unwrap_or("#FACC15");
            Ok(prim::generate_sphere(c, quality))
        }
        "shape_line" => {
            use crate::infrastructure::geometry::generators::primitives as prim;
            let c = extract_str(spec, "/shape/color_hex").unwrap_or("#94A3B8");
            Ok(prim::generate_line(c))
        }
        // effect on the simple textured rectangle.
        _ => {
            let color = extract_str(spec, "/product/color_hex").unwrap_or("#CCCCCC");
            Ok(flat_card::generate(color, None))
        }
    }
}

/// Extract a string at a JSON Pointer path from an optional `Value`.
fn extract_str<'a>(spec: Option<&'a Value>, pointer: &str) -> Option<&'a str> {
    spec?.pointer(pointer)?.as_str()
}

/// Extract an f32 at a JSON Pointer path from an optional `Value`.
fn extract_f32(spec: Option<&Value>, pointer: &str) -> Option<f32> {
    spec?.pointer(pointer)?.as_f64().map(|v| v as f32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn dispatch_sauce_in_bowl_returns_mesh() {
        let spec = json!({
            "object_type": "sauce_in_bowl",
            "confidence": 0.9,
            "product": { "color_hex": "#AA2200" },
            "container": { "kind": "ceramic_bowl" }
        });
        let mesh = dispatch("sauce_in_bowl", Some(&spec)).unwrap();
        assert!(!mesh.vertices.is_empty());
    }

    #[test]
    fn dispatch_unknown_falls_back_to_flat_card() {
        let mesh = dispatch("unknown", None).unwrap();
        // flat_card always has exactly 24 verts
        assert_eq!(mesh.vertices.len(), 24);
    }

    #[test]
    fn dispatch_flat_card_explicit() {
        let mesh = dispatch("flat_card", None).unwrap();
        assert_eq!(mesh.faces.len(), 12);
    }

    #[test]
    fn dispatch_bottled_sauce_returns_multi_material_mesh() {
        let spec = json!({
            "object_type": "bottled_sauce",
            "product": { "color_hex": "#B8321F" },
            "container": { "kind": "glass_bottle" }
        });
        let mesh = dispatch("bottled_sauce", Some(&spec)).unwrap();
        assert!(!mesh.vertices.is_empty());
        assert_eq!(mesh.groups.len(), 4, "body + neck + cap + liquid");
        assert!(mesh
            .groups
            .iter()
            .any(|g| g.material.name == "bottle_glass"));
    }

    #[test]
    fn dispatch_bottled_sauce_plastic_kind() {
        let spec = json!({
            "product": { "color_hex": "#FFCC00" },
            "container": { "kind": "plastic_bottle" }
        });
        let mesh = dispatch("bottled_sauce", Some(&spec)).unwrap();
        assert!(mesh
            .groups
            .iter()
            .any(|g| g.material.name == "bottle_plastic"));
    }

    #[test]
    fn dispatch_jar_product_returns_three_groups() {
        let spec = json!({
            "product": { "color_hex": "#A85B12" },
            "container": { "kind": "glass_jar" }
        });
        let mesh = dispatch("jar_product", Some(&spec)).unwrap();
        assert_eq!(mesh.groups.len(), 3, "glass + product + lid");
        assert!(mesh.groups.iter().any(|g| g.material.name == "jar_glass"));
        assert!(mesh.groups.iter().any(|g| g.material.name == "lid_metal"));
    }

    #[test]
    fn dispatch_plate_food_still_falls_back_to_flat_card() {
        let mesh = dispatch("plate_food", None).unwrap();
        assert_eq!(mesh.groups.len(), 2, "plate + product groups");
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
    fn dispatch_bottled_sauce_with_label_adds_label_group() {
        let spec = json!({
            "product": { "color_hex": "#B8321F" },
            "container": { "kind": "glass_bottle" },
            "labels": { "main_url": "https://cdn.example.com/labels/sauce.png" }
        });
        let mesh = dispatch("bottled_sauce", Some(&spec)).unwrap();
        assert_eq!(mesh.groups.len(), 5, "body+bottom+cap+liquid+label");
        let label = mesh
            .groups
            .iter()
            .find(|g| g.material.name == "bottle_label")
            .expect("bottle_label group missing");
        assert_eq!(
            label.material.texture_url.as_deref(),
            Some("https://cdn.example.com/labels/sauce.png")
        );
    }

    #[test]
    fn dispatch_bottled_sauce_without_label_has_four_groups() {
        let spec = json!({
            "product": { "color_hex": "#B8321F" },
            "container": { "kind": "glass_bottle" }
        });
        let mesh = dispatch("bottled_sauce", Some(&spec)).unwrap();
        assert_eq!(mesh.groups.len(), 4);
        assert!(mesh.groups.iter().all(|g| g.material.texture_url.is_none()));
    }

    #[test]
    fn dispatch_jar_product_with_label_adds_label_group() {
        let spec = json!({
            "product": { "color_hex": "#A85B12" },
            "container": { "kind": "glass_jar" },
            "labels": { "main_url": "https://cdn.example.com/labels/jar.png" }
        });
        let mesh = dispatch("jar_product", Some(&spec)).unwrap();
        assert_eq!(mesh.groups.len(), 4, "glass+product+lid+label");
        let label = mesh
            .groups
            .iter()
            .find(|g| g.material.name == "jar_label")
            .expect("jar_label group missing");
        assert_eq!(
            label.material.texture_url.as_deref(),
            Some("https://cdn.example.com/labels/jar.png")
        );
    }
}

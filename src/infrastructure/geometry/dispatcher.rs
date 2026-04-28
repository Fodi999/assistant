//! Geometry dispatcher — routes `object_type` string → generator.
//!
//! Implemented (PR #4 → PR #14):
//!   * `sauce_in_bowl`  — bowl frustum + swirl sauce surface
//!   * `bottled_sauce`  — body + neck + cap + liquid (glass / plastic)
//!   * `jar_product`    — wide glass jar + product + metal lid
//!   * `plate_food`     — ceramic plate + radial food mound (PR #14)
//!   * `flat_card`      — fallback rectangular card with product photo
//!
//! Anything else (`unknown`, …) falls back to `flat_card`.

use serde_json::Value;

use crate::infrastructure::geometry::generators::{
    bottled_sauce, flat_card, jar_product, plate_food, sauce_in_bowl,
};
use crate::infrastructure::geometry::mesh::Mesh;
use crate::shared::AppError;

/// Generate a [`Mesh`] from `object_type` (the string stored in
/// `laboratory_3d_assets.object_type`) and the raw `object_spec_json`.
///
/// The caller can pass `None` for `spec` if the spec is unavailable — all
/// generators have sensible defaults.
pub fn dispatch(object_type: &str, spec: Option<&Value>) -> Result<Mesh, AppError> {
    match object_type {
        "sauce_in_bowl" => {
            let sauce_color = extract_str(spec, "/product/color_hex").unwrap_or("#B8321F");
            let container_color = extract_str(spec, "/container/color_hex");
            Ok(sauce_in_bowl::generate(sauce_color, container_color))
        }
        "bottled_sauce" => {
            let liquid_color = extract_str(spec, "/product/color_hex").unwrap_or("#B8321F");
            let kind = bottled_sauce::BottleKind::from_str(
                extract_str(spec, "/container/kind"),
            );
            let label_url = extract_str(spec, "/labels/main_url");
            // Cap colour is not in the current spec — leave as default for now.
            Ok(bottled_sauce::generate_with_label(
                liquid_color,
                kind,
                None,
                label_url,
            ))
        }
        "jar_product" => {
            let product_color = extract_str(spec, "/product/color_hex").unwrap_or("#A85B12");
            let lid_color = extract_str(spec, "/container/color_hex");
            let label_url = extract_str(spec, "/labels/main_url");
            Ok(jar_product::generate_with_label(product_color, lid_color, label_url))
        }
        "plate_food" => {
            let product_color = extract_str(spec, "/product/color_hex").unwrap_or("#A85B12");
            let plate_color = extract_str(spec, "/container/color_hex");
            Ok(plate_food::generate(product_color, plate_color))
        }
        // Remaining types (flat_card, unknown) → flat_card.
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
            .any(|g| g.material.name == "product_material"));
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

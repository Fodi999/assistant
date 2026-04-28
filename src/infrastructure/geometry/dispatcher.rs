//! Geometry dispatcher — routes `object_type` string → generator.
//!
//! PR #4 implements `flat_card` and `sauce_in_bowl`.
//! All other types fall back to `flat_card` (they will get real generators in
//! later PRs once they're tested end-to-end).

use serde_json::Value;

use crate::infrastructure::geometry::generators::{flat_card, sauce_in_bowl};
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
        // All remaining types (bottled_sauce, jar_product, plate_food, flat_card,
        // unknown) fall back to flat_card for PR #4.
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
    fn dispatch_bottled_sauce_falls_back() {
        let mesh = dispatch("bottled_sauce", None).unwrap();
        assert_eq!(mesh.vertices.len(), 24, "should fall back to flat_card");
    }
}

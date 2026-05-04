//! Procedural card generator — rounded-rectangle extrude with bevel.
//!
//! Builds entirely on the geometry kernel:
//!   `kernel/rounded.rs`  → `rounded_rect_points()` — 2D contour
//!   `kernel/extrude.rs`  → `extrude_polygon()`     — 2D → 3D shell
//!   `kernel/mesh_builder.rs` → `MeshBuilder`        — multi-material assembly
//!
//! This follows the same pattern as `bottled_sauce`:
//!   kernel operation (extrude) + concrete generator (card).
//!
//! Output: three material groups
//!   * `card_front` — front face (+Z), card colour
//!   * `card_back`  — back face  (-Z), dark neutral
//!   * `card_edge`  — side walls + bevel, metallic dark

use crate::infrastructure::geometry::kernel::extrude::{extrude_polygon, ExtrudeOptions};
use crate::infrastructure::geometry::kernel::rounded::rounded_rect_points;
use crate::infrastructure::geometry::kernel::{GeometryQuality, MeshBuilder};
use crate::infrastructure::geometry::mesh::{hex_to_rgb, Material, Mesh};

// ─────────────────────────────────────────────────────────────────────────────
// Spec
// ─────────────────────────────────────────────────────────────────────────────

/// Parameters for [`generate_card`].
#[derive(Debug, Clone)]
pub struct CardSpec<'a> {
    /// Outer width in metres  (default 0.10 = 10 cm).
    pub width: f32,
    /// Outer height in metres (default 0.14 = 14 cm).
    pub height: f32,
    /// Card thickness in metres (default 0.008 = 8 mm).
    pub thickness: f32,
    /// Corner arc radius in metres.
    pub corner_radius: f32,
    /// Chamfer width for the edge (0 = sharp corner). Typical: 0.001.
    pub bevel: f32,
    /// Front-face colour as `"#RRGGBB"`.
    pub color_hex: &'a str,
    /// Geometry quality preset.
    pub quality: GeometryQuality,
}

impl Default for CardSpec<'_> {
    fn default() -> Self {
        Self {
            width:         0.10,
            height:        0.14,
            thickness:     0.008,
            corner_radius: 0.012,
            bevel:         0.001,
            color_hex:     "#CCCCCC",
            quality:       GeometryQuality::High,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Generator
// ─────────────────────────────────────────────────────────────────────────────

/// Generate a rounded-rectangle card mesh from `spec`.
///
/// # Panics
/// Panics only if `spec` values are degenerate (width/height/thickness ≤ 0).
pub fn generate_card(spec: &CardSpec<'_>) -> Mesh {
    let corner_segs: usize = match spec.quality {
        GeometryQuality::Draft    =>  4,
        GeometryQuality::Standard =>  8,
        GeometryQuality::High     => 16,
        GeometryQuality::Ultra    => 24,
    };

    let points = rounded_rect_points(
        spec.width,
        spec.height,
        spec.corner_radius,
        corner_segs,
    );

    let opts = ExtrudeOptions {
        depth: spec.thickness,
        bevel: spec.bevel,
    };

    let [front, back, sides] = extrude_polygon(&points, &opts)
        .expect("generate_card: invalid CardSpec — check width/height/thickness");

    let face_color = hex_to_rgb(spec.color_hex);

    let mut b = MeshBuilder::new();

    let g_front = b.add_group(
        Material::solid("card_front", face_color)
            .with_pbr(0.45, 0.05)
            .with_class("opaque"),
    );
    let g_back = b.add_group(
        Material::solid("card_back", [0.08, 0.09, 0.12])
            .with_pbr(0.60, 0.10)
            .with_class("opaque"),
    );
    let g_edge = b.add_group(
        Material::solid("card_edge", [0.12, 0.13, 0.18])
            .with_pbr(0.30, 0.65)
            .with_class("metal"),
    );

    b.add_part(g_front, &front);
    b.add_part(g_back,  &back);
    b.add_part(g_edge,  &sides);

    b.build()
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::geometry::kernel::validate::validate_mesh;

    #[test]
    fn generate_card_default_passes_validate() {
        let mesh = generate_card(&CardSpec::default());
        validate_mesh(&mesh).expect("default card failed validation");
    }

    #[test]
    fn generate_card_draft_quality() {
        let spec = CardSpec { quality: GeometryQuality::Draft, ..CardSpec::default() };
        let mesh = generate_card(&spec);
        validate_mesh(&mesh).expect("draft card failed validation");
    }

    #[test]
    fn generate_card_ultra_quality() {
        let spec = CardSpec {
            quality: GeometryQuality::Ultra,
            bevel: 0.0015,
            ..CardSpec::default()
        };
        let mesh = generate_card(&spec);
        validate_mesh(&mesh).expect("ultra card failed validation");
    }

    #[test]
    fn generate_card_has_three_groups() {
        let mesh = generate_card(&CardSpec::default());
        assert_eq!(mesh.groups.len(), 3, "expected card_front / card_back / card_edge");
        assert_eq!(mesh.groups[0].material.name, "card_front");
        assert_eq!(mesh.groups[1].material.name, "card_back");
        assert_eq!(mesh.groups[2].material.name, "card_edge");
    }

    #[test]
    fn generate_card_color_is_applied() {
        let spec = CardSpec { color_hex: "#FF0000", ..CardSpec::default() };
        let mesh = generate_card(&spec);
        let [r, g, b] = mesh.groups[0].material.diffuse_color;
        assert!((r - 1.0).abs() < 1e-3, "expected red r≈1.0, got {r}");
        assert!(g < 1e-3);
        assert!(b < 1e-3);
    }
}

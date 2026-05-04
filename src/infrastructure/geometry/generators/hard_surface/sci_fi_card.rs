//! Sci-Fi ProductCard generator — Plasticity-style precision card.
//!
//! This proves that Rust can build complex hard-surface geometry entirely
//! from parametric spec, no Blender needed. The resulting GLB looks like
//! an object modeled in Plasticity / Fusion 360.
//!
//! ## Visual structure
//!
//! ```text
//! ╭────────────────────────────╮  ← outer body (base)
//! │  ╭──────────────────────╮  │  ← raised front panel
//! │  │  ┌────────────────┐  │  │  ← dark inset (product area)
//! │  │  │                │  │  │
//! │  │  │   PRODUCT      │  │  │
//! │  │  └────────────────┘  │  │
//! │  ━━━━━━━━━━━━━━━━━━━━━━  │  ← glow strip
//! │  ╰──────────────────────╯  │
//! │ ██                      ██ │  ← side rails
//! ╰────────────────────────────╯
//!  ◉                          ◉  ← corner bolts (top)
//!  ◉                          ◉  ← corner bolts (bottom)
//! ```
//!
//! ## Material groups (7 total)
//! * `card_base`    — outer body shell,     dark metal (#0D0F14), metallic 0.80 / rough 0.35
//! * `card_panel`   — raised front panel,   mid metal  (#1C1F26), metallic 0.75 / rough 0.28
//! * `card_inset`   — product area recess,  very dark  (#090A0C), metallic 0.50 / rough 0.60
//! * `card_glow`    — accent glow strip,    accent     (#00C8FF), emissive 1.0
//! * `card_rails`   — side technical rails, mid-dark   (#161920), metallic 0.85 / rough 0.20
//! * `card_back`    — back face,            very dark  (#07080A), metallic 0.40 / rough 0.70
//! * `card_bolts`   — corner bolt dots,     bright     (#2A2E38), metallic 0.95 / rough 0.15
//!
//! ## Pipeline
//! ```text
//! rounded_rect_points → extrude_polygon → MeshBuilder (layer by layer)
//! circle_points       → extrude_polygon → 4× corner bolt placement
//! ```
//!
//! ## Orientation
//! The card stands face-to-camera:
//!   * X = width (horizontal)
//!   * Y = height (vertical)
//!   * Z = thickness (depth, into screen)
//!
//! This is the natural orientation for `extrude_polygon` (XY contour, Z extrude).

use std::f32::consts::PI;

use crate::infrastructure::geometry::kernel::extrude::{extrude_polygon, ExtrudeOptions, Point2};
use crate::infrastructure::geometry::kernel::rounded::rounded_rect_points;
use crate::infrastructure::geometry::kernel::{GeometryQuality, MeshBuilder, MeshPart};
use crate::infrastructure::geometry::mesh::{hex_to_rgb, Material, Mesh};

// ─────────────────────────────────────────────────────────────────────────────
// Spec
// ─────────────────────────────────────────────────────────────────────────────

/// Parameters for [`generate_sci_fi_card`].
///
/// All dimensions are in metres (SI). Default values produce a card that
/// looks like a 12 × 18 mm sci-fi NFC/product tag.
#[derive(Debug, Clone)]
pub struct SciFiCardSpec {
    // ── Outer body ──────────────────────────────────────────────
    /// Overall card width  (X), default 0.12 m = 12 cm.
    pub width: f32,
    /// Overall card height (Y), default 0.18 m = 18 cm.
    pub height: f32,
    /// Card thickness      (Z), default 0.012 m = 12 mm.
    pub thickness: f32,

    /// Corner arc radius for the outer shell, default 0.012.
    pub corner_radius: f32,
    /// Edge chamfer width, default 0.0015.
    pub bevel: f32,

    // ── Front panel (raised ring on face) ───────────────────────
    /// Inset from card edge to panel edge, default 0.005.
    pub panel_inset: f32,
    /// Panel extrude height above card face, default 0.003.
    pub panel_height: f32,

    // ── Product area (dark recess inside panel) ──────────────────
    /// Width  of the product area, default 0.095.
    pub inset_width: f32,
    /// Height of the product area, default 0.115.
    pub inset_height: f32,
    /// How deep the inset appears to go, default 0.003.
    pub inset_depth: f32,

    // ── Glow strip ───────────────────────────────────────────────
    /// Strip height (Y), default 0.002.
    pub glow_strip_height: f32,
    /// Strip vertical position: fraction from bottom of panel, default 0.22.
    pub glow_strip_y_frac: f32,

    // ── Side rails ────────────────────────────────────────────────
    /// Rail width (X), default 0.004.
    pub rail_width: f32,
    /// Rail height (Y) as fraction of card height, default 0.55.
    pub rail_height_frac: f32,
    /// Rail extrude depth above card face, default 0.001.
    pub rail_depth: f32,

    // ── Corner bolts ─────────────────────────────────────────────
    /// Bolt circle radius, default 0.004.
    pub bolt_radius: f32,
    /// Bolt extrude height, default 0.002.
    pub bolt_height: f32,
    /// Inset from each corner to bolt centre, default 0.010.
    pub bolt_inset: f32,

    // ── Appearance ───────────────────────────────────────────────
    /// Accent / emissive colour for the glow strip (default "#00C8FF").
    pub accent_hex: String,

    /// Geometry quality preset.
    pub quality: GeometryQuality,
}

impl Default for SciFiCardSpec {
    fn default() -> Self {
        Self {
            width:     0.12,
            height:    0.18,
            thickness: 0.012,

            corner_radius: 0.012,
            bevel:         0.0015,

            panel_inset:  0.005,
            panel_height: 0.003,

            inset_width:  0.095,
            inset_height: 0.115,
            inset_depth:  0.003,

            glow_strip_height: 0.002,
            glow_strip_y_frac: 0.22,

            rail_width:       0.004,
            rail_height_frac: 0.55,
            rail_depth:       0.001,

            bolt_radius: 0.004,
            bolt_height: 0.002,
            bolt_inset:  0.010,

            accent_hex: "#00C8FF".to_string(),
            quality:    GeometryQuality::High,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Internal helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Generate a closed CCW circle polygon (XY plane, centred at origin).
fn circle_points(radius: f32, segments: usize) -> Vec<Point2> {
    let n = segments.max(3);
    (0..n)
        .map(|i| {
            let angle = 2.0 * PI * (i as f32) / (n as f32);
            Point2::new(radius * angle.cos(), radius * angle.sin())
        })
        .collect()
}

/// Translate all vertices in a `MeshPart` by `(dx, dy, dz)`.
fn translate(mut part: MeshPart, dx: f32, dy: f32, dz: f32) -> MeshPart {
    for v in part.vertices.iter_mut() {
        v[0] += dx;
        v[1] += dy;
        v[2] += dz;
    }
    part
}

// ─────────────────────────────────────────────────────────────────────────────
// Generator
// ─────────────────────────────────────────────────────────────────────────────

/// Generate a Plasticity-style sci-fi product card mesh from `spec`.
///
/// Returns a `Mesh` with seven material groups:
/// `card_base`, `card_panel`, `card_inset`, `card_glow`,
/// `card_rails`, `card_back`, `card_bolts`.
pub fn generate_sci_fi_card(spec: &SciFiCardSpec) -> Mesh {
    let corner_segs: usize = match spec.quality {
        GeometryQuality::Draft    =>  4,
        GeometryQuality::Standard =>  8,
        GeometryQuality::High     => 16,
        GeometryQuality::Ultra    => 24,
    };
    let bolt_segs: usize = match spec.quality {
        GeometryQuality::Draft    =>  6,
        GeometryQuality::Standard => 10,
        GeometryQuality::High     => 16,
        GeometryQuality::Ultra    => 24,
    };

    let mut b = MeshBuilder::new();

    // ── Register material groups ───────────────────────────────────────────
    let g_base = b.add_group(
        Material::solid("card_base", hex_to_rgb("#0D0F14"))
            .with_pbr(0.35, 0.80)
            .with_class("metal"),
    );
    let g_panel = b.add_group(
        Material::solid("card_panel", hex_to_rgb("#1C1F26"))
            .with_pbr(0.28, 0.75)
            .with_class("metal"),
    );
    let g_inset = b.add_group(
        Material::solid("card_inset", hex_to_rgb("#090A0C"))
            .with_pbr(0.60, 0.50)
            .with_class("metal"),
    );
    let g_glow = b.add_group(
        Material::solid("card_glow", hex_to_rgb(&spec.accent_hex))
            .with_pbr(0.10, 0.0)
            .with_class("emissive"),
    );
    let g_rails = b.add_group(
        Material::solid("card_rails", hex_to_rgb("#161920"))
            .with_pbr(0.20, 0.85)
            .with_class("metal"),
    );
    let g_back = b.add_group(
        Material::solid("card_back", hex_to_rgb("#07080A"))
            .with_pbr(0.70, 0.40)
            .with_class("metal"),
    );
    let g_bolts = b.add_group(
        Material::solid("card_bolts", hex_to_rgb("#2A2E38"))
            .with_pbr(0.15, 0.95)
            .with_class("metal"),
    );

    // ── 1. Base body (full card thickness) ────────────────────────────────
    // The card face is at Z = spec.thickness, back at Z = 0.
    // front face  → card_base (same colour)
    // back face   → card_back (darker)
    // side walls  → card_base
    {
        let pts  = rounded_rect_points(spec.width, spec.height, spec.corner_radius, corner_segs);
        let opts = ExtrudeOptions { depth: spec.thickness, bevel: spec.bevel };
        if let Ok([front, back, sides]) = extrude_polygon(&pts, &opts) {
            b.add_part(g_base, &front);
            b.add_part(g_back, &back);
            b.add_part(g_base, &sides);
        }
    }

    // ── 2. Raised front panel ─────────────────────────────────────────────
    // Sits on top of the card face (Z = thickness).
    {
        let panel_w = spec.width  - spec.panel_inset * 2.0;
        let panel_h = spec.height - spec.panel_inset * 2.0;
        let panel_r = (spec.corner_radius - spec.panel_inset).max(0.002);

        let pts  = rounded_rect_points(panel_w, panel_h, panel_r, corner_segs);
        let opts = ExtrudeOptions { depth: spec.panel_height, bevel: spec.bevel * 0.5 };

        if let Ok([front, back, sides]) = extrude_polygon(&pts, &opts) {
            let z0 = spec.thickness;
            b.add_part(g_panel, &translate(front,  0.0, 0.0, z0));
            b.add_part(g_panel, &translate(back,   0.0, 0.0, z0));
            b.add_part(g_panel, &translate(sides,  0.0, 0.0, z0));
        }
    }

    // ── 3. Dark product inset ─────────────────────────────────────────────
    // Appears as a depressed rectangle in the panel.
    // We extrude it "inward" (–Z) from the panel surface, then place it at
    // z = thickness + panel_height so it's flush with the panel top.
    {
        let inset_r = (spec.corner_radius - spec.panel_inset - 0.003).max(0.001);
        let pts  = rounded_rect_points(spec.inset_width, spec.inset_height, inset_r, corner_segs);
        let opts = ExtrudeOptions { depth: spec.inset_depth, bevel: spec.bevel * 0.3 };

        if let Ok([front, back, sides]) = extrude_polygon(&pts, &opts) {
            // Push it up to the panel top surface, then rotate so it recesses inward.
            // We achieve "recess" by inverting Z: v.z = z_top - v.z
            let z_top = spec.thickness + spec.panel_height;
            let offset_y = (spec.height - spec.inset_height) * 0.5 - spec.panel_inset - 0.002;

            let recess = |mut part: MeshPart| {
                for v in part.vertices.iter_mut() {
                    let old_z = v[2];
                    v[2] = z_top - old_z;
                    v[1] += offset_y;
                }
                part
            };
            b.add_part(g_inset, &recess(front));
            b.add_part(g_inset, &recess(back));
            b.add_part(g_inset, &recess(sides));
        }
    }

    // ── 4. Glow strip ─────────────────────────────────────────────────────
    // Thin horizontal bar, sits on top of panel surface.
    {
        let strip_w = spec.inset_width * 0.90;
        let strip_h = spec.glow_strip_height;
        let strip_z = spec.thickness + spec.panel_height;

        // Position: bottom portion of inset area.
        let inset_bottom_y = -(spec.height * 0.5) + spec.panel_inset + 0.002;
        let strip_y = inset_bottom_y + spec.height * spec.glow_strip_y_frac;

        let pts  = rounded_rect_points(strip_w, strip_h, 0.0005, 2);
        let opts = ExtrudeOptions { depth: 0.0015, bevel: 0.0 };

        if let Ok([front, back, sides]) = extrude_polygon(&pts, &opts) {
            b.add_part(g_glow, &translate(front, 0.0, strip_y, strip_z));
            b.add_part(g_glow, &translate(back,  0.0, strip_y, strip_z));
            b.add_part(g_glow, &translate(sides, 0.0, strip_y, strip_z));
        }
    }

    // ── 5. Side rails (left + right) ──────────────────────────────────────
    // Two thin extruded strips along the left/right vertical edges.
    {
        let rail_h  = spec.height * spec.rail_height_frac;
        let rail_z  = spec.thickness;
        let rail_x  = spec.width * 0.5 - spec.panel_inset - spec.rail_width * 0.5;

        let pts  = rounded_rect_points(spec.rail_width, rail_h, 0.001, corner_segs.min(4));
        let opts = ExtrudeOptions { depth: spec.rail_depth, bevel: 0.0005 };

        for &x_sign in &[-1.0_f32, 1.0_f32] {
            if let Ok([front, back, sides]) = extrude_polygon(&pts, &opts) {
                let dx = x_sign * rail_x;
                b.add_part(g_rails, &translate(front, dx, 0.0, rail_z));
                b.add_part(g_rails, &translate(back,  dx, 0.0, rail_z));
                b.add_part(g_rails, &translate(sides, dx, 0.0, rail_z));
            }
        }
    }

    // ── 6. Corner bolts (4×) ──────────────────────────────────────────────
    // Small extruded circles at the four card corners.
    {
        let bi   = spec.bolt_inset;
        let hw   = spec.width  * 0.5 - bi;
        let hh   = spec.height * 0.5 - bi;
        let bolt_z = spec.thickness;

        let corners: [(f32, f32); 4] = [
            ( hw,  hh),   // top-right
            (-hw,  hh),   // top-left
            (-hw, -hh),   // bottom-left
            ( hw, -hh),   // bottom-right
        ];

        let pts  = circle_points(spec.bolt_radius, bolt_segs);
        let opts = ExtrudeOptions { depth: spec.bolt_height, bevel: spec.bolt_height * 0.4 };

        for (cx, cy) in corners {
            if let Ok([front, back, sides]) = extrude_polygon(&pts, &opts) {
                b.add_part(g_bolts, &translate(front, cx, cy, bolt_z));
                b.add_part(g_bolts, &translate(back,  cx, cy, bolt_z));
                b.add_part(g_bolts, &translate(sides, cx, cy, bolt_z));
            }
        }
    }

    b.build()
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sci_fi_card_default_generates_mesh() {
        let mesh = generate_sci_fi_card(&SciFiCardSpec::default());
        assert!(!mesh.groups.is_empty(), "sci_fi_card should have mesh groups");
    }

    #[test]
    fn sci_fi_card_has_seven_material_groups() {
        let mesh = generate_sci_fi_card(&SciFiCardSpec::default());
        let names: Vec<&str> = mesh.groups.iter().map(|g| g.material.name.as_str()).collect();
        for expected in ["card_base", "card_panel", "card_inset", "card_glow",
                         "card_rails", "card_back", "card_bolts"] {
            assert!(names.iter().any(|n| *n == expected), "missing group: {expected}");
        }
    }

    #[test]
    fn sci_fi_card_draft_quality_generates() {
        let spec = SciFiCardSpec { quality: GeometryQuality::Draft, ..Default::default() };
        let mesh = generate_sci_fi_card(&spec);
        assert!(!mesh.groups.is_empty());
    }

    #[test]
    fn sci_fi_card_ultra_quality_generates() {
        let spec = SciFiCardSpec { quality: GeometryQuality::Ultra, ..Default::default() };
        let mesh = generate_sci_fi_card(&spec);
        assert!(!mesh.groups.is_empty());
    }

    #[test]
    fn sci_fi_card_custom_accent() {
        let spec = SciFiCardSpec {
            accent_hex: "#FF6B00".to_string(),
            ..Default::default()
        };
        let mesh = generate_sci_fi_card(&spec);
        let glow = mesh.groups.iter().find(|g| g.material.name == "card_glow");
        assert!(glow.is_some(), "glow group must exist");
        assert_eq!(glow.unwrap().material.material_class, "emissive");
    }

    #[test]
    fn sci_fi_card_bolts_have_geometry() {
        let mesh = generate_sci_fi_card(&SciFiCardSpec::default());
        let bolt_group = mesh.groups.iter().find(|g| g.material.name == "card_bolts");
        assert!(bolt_group.is_some(), "bolt group must exist");
        assert!(!bolt_group.unwrap().faces.is_empty(), "bolt group must have parts");
    }

    #[test]
    fn sci_fi_card_glow_is_emissive_class() {
        let mesh = generate_sci_fi_card(&SciFiCardSpec::default());
        let glow = mesh.groups.iter().find(|g| g.material.name == "card_glow").unwrap();
        assert_eq!(glow.material.material_class, "emissive");
    }

    #[test]
    fn sci_fi_card_custom_dimensions() {
        let spec = SciFiCardSpec {
            width: 0.08,
            height: 0.12,
            thickness: 0.008,
            ..Default::default()
        };
        let mesh = generate_sci_fi_card(&spec);
        assert!(!mesh.groups.is_empty());
    }
}

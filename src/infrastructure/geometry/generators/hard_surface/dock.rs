//! CardDock generator — first hard-surface B-Rep-lite object.
//!
//! A CardDock is the physical slot/cradle that holds a ProductCard in the
//! 3D inventory scene. It is NOT a food object — it is a precision piece
//! of sci-fi furniture/hardware.
//!
//! ## Parts (assembled from extrude_polygon primitives)
//!
//! ```text
//!  ┌──────────────────────────────┐  ← top frame  (raised_frame)
//!  │  ┌────────────────────────┐  │  ← inner recess (dark inset slot)
//!  │  │                        │  │
//!  │  │      [ card slot ]     │  │
//!  │  └────────────────────────┘  │
//!  └──────────────────────────────┘  ← base plate
//!       ████ emissive strips ████
//! ```
//!
//! ## Material groups
//! * `dock_base`     — base plate,     dark metal  (#0D0F12), metallic 0.85 / rough 0.30
//! * `dock_frame`    — raised frame,   mid metal   (#1A1D22), metallic 0.80 / rough 0.25
//! * `dock_slot`     — inner recess,   very dark   (#07080A), metallic 0.60 / rough 0.55
//! * `dock_emissive` — edge strips,    accent glow (#00C8FF), emissive 1.0
//!
//! ## Pipeline
//!   `rounded_rect_points` → `extrude_polygon` → `MeshBuilder` → `Mesh`
//!
//! No GeometricShell needed here — the shape is fully prismatic so the
//! extrude kernel is sufficient. GeometricShell is reserved for shapes
//! that need topological B-Rep validation (booleans, complex joins).

use crate::infrastructure::geometry::kernel::extrude::{extrude_polygon, ExtrudeOptions};
use crate::infrastructure::geometry::kernel::rounded::rounded_rect_points;
use crate::infrastructure::geometry::kernel::{GeometryQuality, MeshBuilder, MeshPart};
use crate::infrastructure::geometry::mesh::{hex_to_rgb, Material, Mesh};

// ─────────────────────────────────────────────────────────────────────────────
// Spec
// ─────────────────────────────────────────────────────────────────────────────

/// All dimensions are in metres.
#[derive(Debug, Clone)]
pub struct CardDockSpec {
    /// Outer width  of the dock platform (default 0.20 = 20 cm).
    pub width: f32,
    /// Outer depth  of the dock platform (default 0.58 = 58 cm).
    pub depth: f32,
    /// Height (thickness) of the base plate (default 0.10 = 10 cm).
    pub height: f32,

    /// Width  of the card slot opening  (default 0.11 = card + clearance).
    pub slot_width: f32,
    /// Depth  of the card slot opening  (default 0.15 = card + clearance).
    pub slot_depth: f32,
    /// Slot recess depth into the base plate (default 0.06 = 6 cm).
    pub slot_recess: f32,

    /// Corner radius for the outer platform (default 0.015).
    pub corner_radius: f32,
    /// Corner radius for the inner slot opening (default 0.008).
    pub slot_corner_radius: f32,
    /// Chamfer width on all extrude edges (default 0.002).
    pub bevel: f32,

    /// Accent / emissive colour for the edge strips (default #00C8FF).
    pub accent_hex: &'static str,

    /// Geometry quality preset.
    pub quality: GeometryQuality,
}

impl Default for CardDockSpec {
    fn default() -> Self {
        Self {
            width:              0.20,
            depth:              0.58,
            height:             0.10,
            slot_width:         0.11,
            slot_depth:         0.15,
            slot_recess:        0.06,
            corner_radius:      0.015,
            slot_corner_radius: 0.008,
            bevel:              0.002,
            accent_hex:         "#00C8FF",
            quality:            GeometryQuality::High,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Generator
// ─────────────────────────────────────────────────────────────────────────────

/// Generate a CardDock mesh from `spec`.
///
/// Returns a `Mesh` with four material groups:
/// `dock_base`, `dock_frame`, `dock_slot`, `dock_emissive`.
pub fn generate_dock(spec: &CardDockSpec) -> Mesh {
    let corner_segs: usize = match spec.quality {
        GeometryQuality::Draft    =>  4,
        GeometryQuality::Standard =>  8,
        GeometryQuality::High     => 16,
        GeometryQuality::Ultra    => 24,
    };

    let mut b = MeshBuilder::new();

    // ── 1. Base plate ──────────────────────────────────────────────────────
    let base_points = rounded_rect_points(spec.width, spec.depth, spec.corner_radius, corner_segs);
    let base_opts   = ExtrudeOptions { depth: spec.height, bevel: spec.bevel };

    let g_base = b.add_group(
        Material::solid("dock_base", hex_to_rgb("#0D0F12"))
            .with_pbr(0.30, 0.85)
            .with_class("metal"),
    );

    if let Ok([front, back, sides]) = extrude_polygon(&base_points, &base_opts) {
        b.add_part(g_base, &front);
        b.add_part(g_base, &back);
        b.add_part(g_base, &sides);
    }

    // ── 2. Raised frame ring ───────────────────────────────────────────────
    let frame_inset = spec.bevel * 4.0;
    let frame_w     = spec.width - frame_inset * 2.0;
    let frame_d     = spec.depth - frame_inset * 2.0;
    let frame_h     = 0.018_f32;
    let frame_r     = (spec.corner_radius - frame_inset).max(0.002);

    let frame_points = rounded_rect_points(frame_w, frame_d, frame_r, corner_segs);
    let frame_opts   = ExtrudeOptions { depth: frame_h, bevel: spec.bevel };

    let g_frame = b.add_group(
        Material::solid("dock_frame", hex_to_rgb("#1A1D22"))
            .with_pbr(0.25, 0.80)
            .with_class("metal"),
    );

    if let Ok([front, back, sides]) = extrude_polygon(&frame_points, &frame_opts) {
        // Offset upward so the frame sits on top of base plate.
        let offset_y = spec.height;
        let shift = |mut part: MeshPart| {
            for v in part.vertices.iter_mut() { v[1] += offset_y; }
            part
        };
        b.add_part(g_frame, &shift(front));
        b.add_part(g_frame, &shift(back));
        b.add_part(g_frame, &shift(sides));
    }

    // ── 3. Inner slot recess ───────────────────────────────────────────────
    let slot_points = rounded_rect_points(
        spec.slot_width,
        spec.slot_depth,
        spec.slot_corner_radius,
        corner_segs,
    );
    let slot_opts = ExtrudeOptions { depth: spec.slot_recess, bevel: spec.bevel * 0.5 };

    let g_slot = b.add_group(
        Material::solid("dock_slot", hex_to_rgb("#07080A"))
            .with_pbr(0.55, 0.60)
            .with_class("metal"),
    );

    if let Ok([front, back, sides]) = extrude_polygon(&slot_points, &slot_opts) {
        // Recess sits on top of base + frame, extrudes downward.
        // We flip the Z axis to make it go "into" the surface.
        let top_y = spec.height + frame_h;
        let recess = |mut part: MeshPart| {
            for v in part.vertices.iter_mut() {
                let old_z = v[2];
                v[2] = 0.0;
                v[1] = top_y - old_z;
            }
            part
        };
        b.add_part(g_slot, &recess(front));
        b.add_part(g_slot, &recess(back));
        b.add_part(g_slot, &recess(sides));
    }

    // ── 4. Emissive accent strips ──────────────────────────────────────────
    // Two thin strips along front/rear edges of the frame, acting as glow lines.
    let strip_w = spec.width * 0.85;
    let strip_d = 0.004_f32;
    let strip_h = 0.003_f32;
    let strip_y = spec.height + frame_h * 0.5;

    let accent_color = hex_to_rgb(spec.accent_hex);
    let g_emit = b.add_group(
        Material::solid("dock_emissive", accent_color)
            .with_pbr(0.10, 0.0)
            .with_class("emissive"),
    );

    let strip_points = rounded_rect_points(strip_w, strip_d, 0.001, 2);
    let strip_opts   = ExtrudeOptions { depth: strip_h, bevel: 0.0 };

    let offsets: [f32; 2] = [
         spec.depth * 0.5 - strip_d * 0.5 - spec.bevel * 2.0,
        -spec.depth * 0.5 + strip_d * 0.5 + spec.bevel * 2.0,
    ];

    for z_off in offsets {
        if let Ok([front, back, sides]) = extrude_polygon(&strip_points, &strip_opts) {
            let place = |mut part: MeshPart| {
                for v in part.vertices.iter_mut() {
                    v[1] += strip_y;
                    v[2] += z_off;
                }
                part
            };
            b.add_part(g_emit, &place(front));
            b.add_part(g_emit, &place(back));
            b.add_part(g_emit, &place(sides));
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
    fn dock_default_generates_mesh() {
        let mesh = generate_dock(&CardDockSpec::default());
        assert!(!mesh.groups.is_empty(), "dock should have mesh parts");
    }

    #[test]
    fn dock_has_four_material_groups() {
        let mesh = generate_dock(&CardDockSpec::default());
        let names: Vec<&str> = mesh.groups.iter().map(|g| g.material.name.as_str()).collect();
        assert!(names.iter().any(|n| *n == "dock_base"),     "missing dock_base");
        assert!(names.iter().any(|n| *n == "dock_frame"),    "missing dock_frame");
        assert!(names.iter().any(|n| *n == "dock_slot"),     "missing dock_slot");
        assert!(names.iter().any(|n| *n == "dock_emissive"), "missing dock_emissive");
    }

    #[test]
    fn dock_draft_quality_generates() {
        let spec = CardDockSpec { quality: GeometryQuality::Draft, ..Default::default() };
        let mesh = generate_dock(&spec);
        assert!(!mesh.groups.is_empty());
    }

    #[test]
    fn dock_ultra_quality_generates() {
        let spec = CardDockSpec { quality: GeometryQuality::Ultra, ..Default::default() };
        let mesh = generate_dock(&spec);
        assert!(!mesh.groups.is_empty());
    }

    #[test]
    fn dock_custom_accent_color() {
        let spec = CardDockSpec { accent_hex: "#FF6B00", ..Default::default() };
        let mesh = generate_dock(&spec);
        let emissive_group = mesh.groups.iter().find(|g| g.material.name == "dock_emissive");
        assert!(emissive_group.is_some());
        // Emissive material class is set.
        let mat = &emissive_group.unwrap().material;
        assert_eq!(mat.material_class, "emissive");
    }
}

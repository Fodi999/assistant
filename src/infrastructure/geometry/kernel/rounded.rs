//! Rounded-rectangle contour generator for [`super::extrude::extrude_polygon`].
//!
//! Produces a closed CCW polygon (viewed from +Z) that traces the outline of
//! a rectangle with arc-smoothed corners. Feed the result straight into
//! `extrude_polygon` to get a card / panel / dock body.
//!
//! ```text
//!     ╭───────╮
//!     │       │
//!     ╰───────╯
//! ```
//!
//! Corner order (CCW from top-right):
//!   top-right → top-left → bottom-left → bottom-right
//!
//! UV note: UVs are assigned by the extrude operation, not here.

use std::f32::consts::PI;

use super::extrude::Point2;

/// Generate the outline of a rounded rectangle as a closed CCW polygon.
///
/// * `width` / `height` — outer bounding-box dimensions.
/// * `radius` — corner arc radius, clamped to `min(width, height) / 2`.
/// * `corner_segments` — number of line segments per 90° arc (clamped to ≥ 1).
///
/// Returns a `Vec` of at least `corner_segments * 4` points.
pub fn rounded_rect_points(
    width: f32,
    height: f32,
    radius: f32,
    corner_segments: usize,
) -> Vec<Point2> {
    let hw = width * 0.5;
    let hh = height * 0.5;
    let r = radius.clamp(0.0, hw.min(hh));
    let segs = corner_segments.max(1);

    // Corner centres + start angles (CCW, starting top-right).
    let corners: [(f32, f32, f32); 4] = [
        (hw - r, hh - r, 0.0),       // top-right  — 0° → 90°
        (-hw + r, hh - r, PI * 0.5), // top-left   — 90° → 180°
        (-hw + r, -hh + r, PI),      // bottom-left — 180° → 270°
        (hw - r, -hh + r, PI * 1.5), // bottom-right — 270° → 360°
    ];

    let mut points = Vec::with_capacity(segs * 4);

    for (cx, cy, start) in corners {
        for i in 0..segs {
            let t = i as f32 / segs as f32;
            let angle = start + t * (PI * 0.5);
            points.push(Point2::new(cx + angle.cos() * r, cy + angle.sin() * r));
        }
    }

    points
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rounded_rect_count() {
        // 4 corners × segs = total points.
        let pts = rounded_rect_points(1.0, 0.6, 0.1, 8);
        assert_eq!(pts.len(), 32);
    }

    #[test]
    fn zero_radius_gives_four_corners() {
        // With r=0 each corner arc degenerates to 1 point = segs points each.
        let pts = rounded_rect_points(1.0, 0.6, 0.0, 4);
        assert_eq!(pts.len(), 16);
    }

    #[test]
    fn all_points_within_bounding_box() {
        let (w, h) = (0.10, 0.14);
        let pts = rounded_rect_points(w, h, 0.012, 16);
        for p in &pts {
            assert!(p.x.abs() <= w * 0.5 + 1e-5, "x={} out of bounds", p.x);
            assert!(p.y.abs() <= h * 0.5 + 1e-5, "y={} out of bounds", p.y);
        }
    }

    #[test]
    fn max_radius_produces_circle_like() {
        // radius clamped to min(w,h)/2 → full semicircles at every corner.
        let pts = rounded_rect_points(0.10, 0.10, 9999.0, 16);
        // All points should be roughly on a circle of radius 0.05.
        for p in &pts {
            let r = (p.x * p.x + p.y * p.y).sqrt();
            assert!((r - 0.05).abs() < 1e-4, "r={r} far from 0.05");
        }
    }
}

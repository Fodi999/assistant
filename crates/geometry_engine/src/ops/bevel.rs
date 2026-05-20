//! Bevel — rounded rectangle contour generator.
//!
//! Производит замкнутый CCW полигон трассирующий прямоугольник со скруглёнными углами.
//! Подаётся прямо в `ops::extrude::extrude_polygon`.

use std::f64::consts::PI;
use crate::ops::extrude::Point2;

/// Генерирует outline скруглённого прямоугольника как замкнутый CCW полигон.
///
/// * `width` / `height` — внешние размеры
/// * `radius` — радиус угловой дуги, clamp до `min(w,h)/2`
/// * `corner_segments` — число сегментов на 90° дугу (минимум 1)
pub fn rounded_rect_points(
    width: f64,
    height: f64,
    radius: f64,
    corner_segments: usize,
) -> Vec<Point2> {
    let hw = width * 0.5;
    let hh = height * 0.5;
    let r    = radius.clamp(0.0, hw.min(hh));
    let segs = corner_segments.max(1);

    // Углы CCW, начиная с top-right (0°)
    let corners: [(f64, f64, f64); 4] = [
        ( hw-r,  hh-r, 0.0      ), // top-right   0° → 90°
        (-hw+r,  hh-r, PI*0.5   ), // top-left    90° → 180°
        (-hw+r, -hh+r, PI       ), // bottom-left 180° → 270°
        ( hw-r, -hh+r, PI*1.5   ), // bottom-right 270° → 360°
    ];

    let mut points = Vec::with_capacity(segs * 4);
    for (cx, cy, start) in corners {
        for i in 0..segs {
            let t = i as f64 / segs as f64;
            let angle = start + t * (PI * 0.5);
            points.push(Point2::new(cx + angle.cos() * r, cy + angle.sin() * r));
        }
    }
    points
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn count_is_4_times_segs() {
        assert_eq!(rounded_rect_points(1.0, 0.6, 0.1, 8).len(), 32);
    }

    #[test]
    fn all_inside_bbox() {
        let (w, h) = (0.10, 0.14);
        for p in rounded_rect_points(w, h, 0.012, 16) {
            assert!(p.x.abs() <= w*0.5 + 1e-5);
            assert!(p.y.abs() <= h*0.5 + 1e-5);
        }
    }
}

//! Ориентация 2D полигона: подписанная площадь, CCW.

/// Подписанная площадь замкнутого полигона (shoelace formula).
/// Положительная → CCW (вид со стороны +Z).
pub fn signed_area_2d(pts: &[(f32, f32)]) -> f32 {
    let n = pts.len();
    let mut s = 0.0_f32;
    for i in 0..n {
        let (ax, ay) = pts[i];
        let (bx, by) = pts[(i + 1) % n];
        s += ax * by - bx * ay;
    }
    s * 0.5
}

/// Возвращает `true` если полигон обходится против часовой стрелки (CCW).
#[inline]
pub fn is_ccw(pts: &[(f32, f32)]) -> bool {
    signed_area_2d(pts) > 0.0
}

/// Гарантирует CCW, разворачивая при необходимости.
pub fn ensure_ccw(pts: &mut Vec<(f32, f32)>) {
    if !is_ccw(pts) {
        pts.reverse();
    }
}

//! Ориентация 2D полигона: подписанная площадь, CCW. Uses Real (f64).

use crate::math::Real;

/// Подписанная площадь замкнутого полигона (shoelace formula).
/// Положительная → CCW (вид со стороны +Z).
pub fn signed_area_2d(pts: &[(Real, Real)]) -> Real {
    let n = pts.len();
    let mut s = 0.0_f64;
    for i in 0..n {
        let (ax, ay) = pts[i];
        let (bx, by) = pts[(i + 1) % n];
        s += ax * by - bx * ay;
    }
    s * 0.5
}

/// Возвращает `true` если полигон обходится против часовой стрелки (CCW).
#[inline]
pub fn is_ccw(pts: &[(Real, Real)]) -> bool {
    signed_area_2d(pts) > 0.0
}

/// Гарантирует CCW, разворачивая при необходимости.
pub fn ensure_ccw(pts: &mut Vec<(Real, Real)>) {
    if !is_ccw(pts) {
        pts.reverse();
    }
}

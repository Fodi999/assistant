//! Проверка самопересечений в 2D полигоне.

/// Возвращает `true` если полигон содержит самопересечения.
/// O(n²) — только для небольших профилей (≤ 1000 точек).
pub fn has_self_intersection(pts: &[(f32, f32)]) -> bool {
    let n = pts.len();
    if n < 4 { return false; }
    for i in 0..n {
        let a = pts[i];
        let b = pts[(i + 1) % n];
        for j in (i + 2)..n {
            if j + 1 == n && i == 0 { continue; } // смежные с началом
            let c = pts[j];
            let d = pts[(j + 1) % n];
            if segments_intersect(a, b, c, d) {
                return true;
            }
        }
    }
    false
}

fn cross2(ax: f32, ay: f32, bx: f32, by: f32) -> f32 {
    ax * by - ay * bx
}

fn segments_intersect(a: (f32, f32), b: (f32, f32), c: (f32, f32), d: (f32, f32)) -> bool {
    let (ax, ay) = a;
    let (bx, by) = b;
    let (cx, cy) = c;
    let (dx, dy) = d;

    let dx1 = bx - ax; let dy1 = by - ay;
    let dx2 = dx - cx; let dy2 = dy - cy;

    let denom = cross2(dx1, dy1, dx2, dy2);
    if denom.abs() < 1e-10 { return false; } // параллельны

    let t = cross2(cx - ax, cy - ay, dx2, dy2) / denom;
    let u = cross2(cx - ax, cy - ay, dx1, dy1) / denom;

    t > 1e-8 && t < 1.0 - 1e-8 && u > 1e-8 && u < 1.0 - 1e-8
}

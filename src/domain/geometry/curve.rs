//! Geometric curves — parametric curve trait + primitive implementations.
//!
//! ## Parasolid correspondence
//! Parasolid models every edge as lying on a `CURVE` entity. We define a
//! minimal trait that future implementations can satisfy:
//!
//! | This crate       | Parasolid          | Status      |
//! |------------------|--------------------|-------------|
//! | `Line`           | straight_curve     | ✅ done     |
//! | `Circle`         | circle             | ✅ done     |
//! | `BezierCurve`    | bs3_curve (deg 3)  | ✅ done     |
//! | `NurbsCurve`     | bs3_curve          | 🔜 planned  |
//!
//! ## Design
//! All curves implement `ParametricCurve` — evaluate at parameter `t`,
//! get derivative, compute arc-length. This is the minimum needed for:
//!   - Tessellation (sample at chord-error-controlled intervals)
//!   - Edge geometry in B-Rep
//!   - Sweep paths for future `sweep_along` operations

use crate::domain::geometry::vertex::Vertex;

// ─────────────────────────────────────────────────────────────────────────────
// Trait
// ─────────────────────────────────────────────────────────────────────────────

/// Minimal parametric curve contract.
///
/// `t` is normalised to `[0.0, 1.0]` for all implementations in this crate
/// (unlike raw Parasolid which uses arc-length parameters). This simplifies
/// tessellation and avoids magnitude scaling issues.
pub trait ParametricCurve: Send + Sync {
    /// Point on the curve at parameter `t ∈ [0, 1]`.
    fn point_at(&self, t: f64) -> Vertex;

    /// Tangent vector at `t` (not necessarily unit length).
    fn tangent_at(&self, t: f64) -> [f64; 3];

    /// Approximate arc length (adaptive Simpson's rule, `n` subdivisions).
    fn arc_length(&self, n: usize) -> f64 {
        let n = n.max(4);
        let dt = 1.0 / n as f64;
        let mut len = 0.0;
        let mut prev = self.point_at(0.0);
        for i in 1..=n {
            let cur = self.point_at(i as f64 * dt);
            len += prev.distance_to(cur);
            prev = cur;
        }
        len
    }

    /// Tessellate into points such that chord error ≤ `chord_tol`.
    /// Always includes t=0 and t=1.
    fn tessellate(&self, chord_tol: f64) -> Vec<Vertex> {
        let mut pts = vec![self.point_at(0.0)];
        self.tessellate_range(0.0, 1.0, chord_tol, &mut pts);
        pts
    }

    /// Recursive adaptive subdivision helper.
    fn tessellate_range(&self, t0: f64, t1: f64, chord_tol: f64, out: &mut Vec<Vertex>) {
        let tm = (t0 + t1) * 0.5;
        let p0 = self.point_at(t0);
        let pm = self.point_at(tm);
        let p1 = self.point_at(t1);
        // Midpoint deviation from chord p0→p1
        let mx = (p0.x + p1.x) * 0.5 - pm.x;
        let my = (p0.y + p1.y) * 0.5 - pm.y;
        let mz = (p0.z + p1.z) * 0.5 - pm.z;
        let dev = (mx * mx + my * my + mz * mz).sqrt();
        if dev > chord_tol && (t1 - t0) > 1e-10 {
            self.tessellate_range(t0, tm, chord_tol, out);
            self.tessellate_range(tm, t1, chord_tol, out);
        }
        out.push(p1);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Line
// ─────────────────────────────────────────────────────────────────────────────

/// Straight line segment from `start` to `end`.
pub struct Line {
    pub start: Vertex,
    pub end:   Vertex,
}

impl Line {
    pub fn new(start: Vertex, end: Vertex) -> Self {
        Self { start, end }
    }
}

impl ParametricCurve for Line {
    fn point_at(&self, t: f64) -> Vertex {
        self.start.lerp(self.end, t)
    }

    fn tangent_at(&self, _t: f64) -> [f64; 3] {
        [
            self.end.x - self.start.x,
            self.end.y - self.start.y,
            self.end.z - self.start.z,
        ]
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Circle
// ─────────────────────────────────────────────────────────────────────────────

/// Full or partial circle in the XZ plane (Y = centre.y), swept CCW.
///
/// `angle_start` and `angle_end` are in radians. For a full circle use
/// `0.0` and `std::f64::consts::TAU`.
pub struct Circle {
    pub centre:      Vertex,
    pub radius:      f64,
    pub angle_start: f64,
    pub angle_end:   f64,
}

impl Circle {
    /// Full circle in the XZ plane centred at `centre`.
    pub fn full(centre: Vertex, radius: f64) -> Self {
        Self { centre, radius, angle_start: 0.0, angle_end: std::f64::consts::TAU }
    }

    /// Arc from `start_rad` to `end_rad` (CCW).
    pub fn arc(centre: Vertex, radius: f64, start_rad: f64, end_rad: f64) -> Self {
        Self { centre, radius, angle_start: start_rad, angle_end: end_rad }
    }
}

impl ParametricCurve for Circle {
    fn point_at(&self, t: f64) -> Vertex {
        let a = self.angle_start + t * (self.angle_end - self.angle_start);
        Vertex::new(
            self.centre.x + self.radius * a.cos(),
            self.centre.y,
            self.centre.z + self.radius * a.sin(),
        )
    }

    fn tangent_at(&self, t: f64) -> [f64; 3] {
        let a = self.angle_start + t * (self.angle_end - self.angle_start);
        let da = self.angle_end - self.angle_start;
        [
            -self.radius * a.sin() * da,
            0.0,
            self.radius * a.cos() * da,
        ]
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Bézier curve (cubic)
// ─────────────────────────────────────────────────────────────────────────────

/// Cubic Bézier curve defined by four control points.
///
/// This is the building block for future NURBS (a NURBS curve is a
/// piecewise rational generalisation of Bézier).
pub struct BezierCurve {
    pub p0: Vertex,
    pub p1: Vertex,
    pub p2: Vertex,
    pub p3: Vertex,
}

impl BezierCurve {
    pub fn new(p0: Vertex, p1: Vertex, p2: Vertex, p3: Vertex) -> Self {
        Self { p0, p1, p2, p3 }
    }
}

impl ParametricCurve for BezierCurve {
    fn point_at(&self, t: f64) -> Vertex {
        let u  = 1.0 - t;
        let u2 = u * u;
        let u3 = u2 * u;
        let t2 = t * t;
        let t3 = t2 * t;
        Vertex::new(
            u3 * self.p0.x + 3.0 * u2 * t * self.p1.x + 3.0 * u * t2 * self.p2.x + t3 * self.p3.x,
            u3 * self.p0.y + 3.0 * u2 * t * self.p1.y + 3.0 * u * t2 * self.p2.y + t3 * self.p3.y,
            u3 * self.p0.z + 3.0 * u2 * t * self.p1.z + 3.0 * u * t2 * self.p2.z + t3 * self.p3.z,
        )
    }

    fn tangent_at(&self, t: f64) -> [f64; 3] {
        // Derivative of cubic Bézier: 3*(B(t) with degree-2 control points)
        let u = 1.0 - t;
        let q0x = self.p1.x - self.p0.x;
        let q0y = self.p1.y - self.p0.y;
        let q0z = self.p1.z - self.p0.z;
        let q1x = self.p2.x - self.p1.x;
        let q1y = self.p2.y - self.p1.y;
        let q1z = self.p2.z - self.p1.z;
        let q2x = self.p3.x - self.p2.x;
        let q2y = self.p3.y - self.p2.y;
        let q2z = self.p3.z - self.p2.z;
        [
            3.0 * (u * u * q0x + 2.0 * u * t * q1x + t * t * q2x),
            3.0 * (u * u * q0y + 2.0 * u * t * q1y + t * t * q2y),
            3.0 * (u * u * q0z + 2.0 * u * t * q1z + t * t * q2z),
        ]
    }
}

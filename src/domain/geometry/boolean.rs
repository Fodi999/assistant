//! Boolean operations on `GeometricShell` — stub implementations.
//!
//! ## Roadmap
//!
//! | Phase | Operation        | Status        |
//! |-------|-----------------|---------------|
//! | 1     | AABB overlap test| ✅ available  |
//! | 2     | Union stub       | 🔜 planned    |
//! | 3     | Subtract stub    | 🔜 planned    |
//! | 4     | Intersect stub   | 🔜 planned    |
//! | 5     | Full BSP/BVH     | 🔜 future     |
//!
//! ## Design
//! The public API is defined now so callers can be written against it
//! before the heavy geometry math is implemented. When `NotImplemented` is
//! returned the caller can fall back to a union-of-meshes approach and
//! re-try when full Boolean support lands.
//!
//! A real implementation will require:
//!   - Triangle–triangle intersection test (Möller–Trumbore)
//!   - Edge classification (inside / outside / on boundary)
//!   - Mesh stitching along the intersection curve
//!   - Consistent normal orientation pass

use crate::domain::geometry::shell::GeometricShell;

// ─────────────────────────────────────────────────────────────────────────────
// Operation selector
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BooleanOp {
    /// A ∪ B — volume enclosed by either shell.
    Union,
    /// A − B — volume inside A but not B.
    Subtract,
    /// A ∩ B — volume inside both shells.
    Intersect,
}

// ─────────────────────────────────────────────────────────────────────────────
// Error type
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum BooleanError {
    /// Full boolean math not yet implemented — returned by all ops currently.
    NotImplemented,
    /// The shells do not overlap (AABB check failed) — no intersection possible.
    NoOverlap,
    /// At least one shell is degenerate (no faces / not watertight).
    DegenerateInput { which: &'static str },
}

impl std::fmt::Display for BooleanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BooleanError::NotImplemented =>
                write!(f, "boolean operation not yet implemented"),
            BooleanError::NoOverlap =>
                write!(f, "shells do not overlap — no boolean result"),
            BooleanError::DegenerateInput { which } =>
                write!(f, "degenerate input shell: {}", which),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Public API
// ─────────────────────────────────────────────────────────────────────────────

/// Perform a boolean operation on two watertight shells.
///
/// Currently returns `Err(BooleanError::NotImplemented)` after a fast AABB
/// rejection test. Structure is final — swap the stub body for real logic
/// when the intersection kernel is ready.
pub fn boolean(
    a: &GeometricShell,
    b: &GeometricShell,
    op: BooleanOp,
) -> Result<GeometricShell, BooleanError> {
    // ── Guard: degenerate inputs ─────────────────────────────────────────────
    if !a.is_watertight() {
        return Err(BooleanError::DegenerateInput { which: "a" });
    }
    if !b.is_watertight() {
        return Err(BooleanError::DegenerateInput { which: "b" });
    }

    // ── Fast reject: AABB overlap ────────────────────────────────────────────
    let (a_min, a_max) = a.aabb();
    let (b_min, b_max) = b.aabb();

    let overlaps = a_min[0] <= b_max[0] && a_max[0] >= b_min[0]
                && a_min[1] <= b_max[1] && a_max[1] >= b_min[1]
                && a_min[2] <= b_max[2] && a_max[2] >= b_min[2];

    // For Intersect, no overlap → result is empty (not just stub)
    if !overlaps && op == BooleanOp::Intersect {
        return Err(BooleanError::NoOverlap);
    }

    // ── Stub: full implementation pending ───────────────────────────────────
    // TODO: implement triangle–triangle intersection → edge classification →
    //       mesh stitching. Until then, inform the caller gracefully.
    let _ = op; // suppress unused warning
    Err(BooleanError::NotImplemented)
}

// ─────────────────────────────────────────────────────────────────────────────
// Convenience wrappers
// ─────────────────────────────────────────────────────────────────────────────

/// `A ∪ B`
pub fn union(a: &GeometricShell, b: &GeometricShell) -> Result<GeometricShell, BooleanError> {
    boolean(a, b, BooleanOp::Union)
}

/// `A − B`
pub fn subtract(a: &GeometricShell, b: &GeometricShell) -> Result<GeometricShell, BooleanError> {
    boolean(a, b, BooleanOp::Subtract)
}

/// `A ∩ B`
pub fn intersect(a: &GeometricShell, b: &GeometricShell) -> Result<GeometricShell, BooleanError> {
    boolean(a, b, BooleanOp::Intersect)
}

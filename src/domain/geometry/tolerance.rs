//! Geometric tolerance — Parasolid-style precision tiers.
//!
//! Parasolid defines three independent tolerances that guard different
//! classes of geometric operation. We mirror that design exactly:
//!
//! | Tier       | Parasolid name     | Default (m)  | Meaning                                   |
//! |------------|--------------------|--------------|-------------------------------------------|
//! | Modeling   | `SPAresabs`        | 1 × 10⁻⁷     | Vertex coincidence / snap / merge         |
//! | Fitting    | `SPAresnor`        | 1 × 10⁻⁵     | Max chord error in curve tessellation     |
//! | Angular    | `SPAresang`        | 1 × 10⁻⁹ rad | Normal consistency / degenerate-edge test |
//!
//! All values are stored in **f64**. The domain never rounds to f32 — that
//! happens only at the infrastructure boundary when writing the GLB buffer.
//!
//! ## DDD role
//! `Tolerance` is a **Value Object**: equality is structural, it is
//! immutable after construction, and it carries no identity.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tolerance {
    /// Vertex-merge / snap distance (metres). Two vertices closer than this
    /// are considered coincident.
    pub modeling: f64,
    /// Maximum chord-height deviation allowed when tessellating a smooth
    /// curve into line segments (metres).
    pub fitting: f64,
    /// Maximum angle (radians) between normals that are considered parallel.
    /// Also used to detect degenerate (zero-length) edges.
    pub angular: f64,
}

impl Tolerance {
    /// Parasolid default tolerances. Use this for most geometry operations.
    pub const DEFAULT: Self = Self {
        modeling: 1e-7,
        fitting:  1e-5,
        angular:  1e-9,
    };

    /// Loose tolerances — useful for draft-quality previews where speed
    /// matters more than precision.
    pub const DRAFT: Self = Self {
        modeling: 1e-5,
        fitting:  1e-3,
        angular:  1e-6,
    };

    /// Ultra tolerances — final-render exports. Stricter than Parasolid
    /// default; catch micro-gaps that would be invisible at draft.
    pub const ULTRA: Self = Self {
        modeling: 1e-8,
        fitting:  1e-6,
        angular:  1e-11,
    };

    /// Create a custom tolerance set. Panics in debug builds if any value is
    /// non-positive or non-finite (a zero tolerance would accept every gap).
    pub fn new(modeling: f64, fitting: f64, angular: f64) -> Self {
        debug_assert!(modeling.is_finite() && modeling > 0.0,
            "modeling tolerance must be > 0");
        debug_assert!(fitting.is_finite()  && fitting  > 0.0,
            "fitting tolerance must be > 0");
        debug_assert!(angular.is_finite()  && angular  > 0.0,
            "angular tolerance must be > 0");
        Self { modeling, fitting, angular }
    }

    // ── Tolerance predicates ─────────────────────────────────────────────

    /// Returns `true` if `distance` is within the **modeling** tolerance
    /// (two vertices should be merged / considered coincident).
    #[inline]
    pub fn vertices_coincident(self, distance: f64) -> bool {
        distance.abs() <= self.modeling
    }

    /// Returns `true` if `gap` is within the **fitting** tolerance
    /// (curve approximation error is acceptable).
    #[inline]
    pub fn approximation_ok(self, gap: f64) -> bool {
        gap.abs() <= self.fitting
    }

    /// Returns `true` if the angle between two normals is within the
    /// **angular** tolerance (treat them as parallel / coincident direction).
    #[inline]
    pub fn normals_parallel(self, angle_rad: f64) -> bool {
        angle_rad.abs() <= self.angular
    }

    /// Returns `true` if a length is so small it should be treated as zero
    /// (degenerate edge / collapsed triangle). Uses modeling tolerance.
    #[inline]
    pub fn is_degenerate_length(self, length: f64) -> bool {
        length < self.modeling
    }
}

impl Default for Tolerance {
    fn default() -> Self {
        Self::DEFAULT
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_parasolid_standard() {
        assert_eq!(Tolerance::DEFAULT.modeling, 1e-7);
        assert_eq!(Tolerance::DEFAULT.fitting,  1e-5);
        assert_eq!(Tolerance::DEFAULT.angular,  1e-9);
    }

    #[test]
    fn vertices_coincident_within_tolerance() {
        let t = Tolerance::DEFAULT;
        assert!( t.vertices_coincident(5e-8));
        assert!(!t.vertices_coincident(2e-7));
    }

    #[test]
    fn approximation_ok_within_tolerance() {
        let t = Tolerance::DEFAULT;
        assert!( t.approximation_ok(9e-6));
        assert!(!t.approximation_ok(2e-5));
    }

    #[test]
    fn draft_is_looser_than_default() {
        assert!(Tolerance::DRAFT.modeling > Tolerance::DEFAULT.modeling);
        assert!(Tolerance::DRAFT.fitting  > Tolerance::DEFAULT.fitting);
    }

    #[test]
    fn ultra_is_stricter_than_default() {
        assert!(Tolerance::ULTRA.modeling < Tolerance::DEFAULT.modeling);
        assert!(Tolerance::ULTRA.fitting  < Tolerance::DEFAULT.fitting);
    }
}

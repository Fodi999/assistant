//! `Body` — top-level B-Rep aggregate.
//!
//! A `Body` represents a single solid. It holds one or more `GeometricShell`s:
//!   - `shells[0]` is the **outer** boundary (volume-enclosing)
//!   - `shells[1..]` are **void** shells (holes / pockets cut out of the solid)
//!
//! This mirrors the Parasolid body → shell → face hierarchy:
//!
//! ```text
//! Body
//! └── Shell (outer)
//!     └── Faces
//! └── Shell (void 0)  ← pocket
//! └── Shell (void 1)  ← through-hole
//! ```
//!
//! ## Volume calculation
//! We use the divergence theorem (signed volume of each shell, summed).
//! Outer shells contribute positive volume, void shells negative.

use crate::domain::geometry::shell::GeometricShell;

// ─────────────────────────────────────────────────────────────────────────────
// Error type
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum BodyError {
    /// No shells at all — a body must have at least one shell.
    NoShells,
    /// The outer shell is not watertight (open mesh).
    OuterShellNotClosed,
    /// A void shell is not watertight.
    VoidShellNotClosed { index: usize },
}

impl std::fmt::Display for BodyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BodyError::NoShells =>
                write!(f, "body has no shells"),
            BodyError::OuterShellNotClosed =>
                write!(f, "outer shell is not watertight"),
            BodyError::VoidShellNotClosed { index } =>
                write!(f, "void shell {} is not watertight", index),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Body
// ─────────────────────────────────────────────────────────────────────────────

/// A closed solid defined by one outer shell and zero or more void (inner) shells.
#[derive(Debug, Clone)]
pub struct Body {
    /// Unique id within a scene / document.
    pub id: u32,
    /// `shells[0]` = outer; `shells[1..]` = voids.
    shells: Vec<GeometricShell>,
}

impl Body {
    // ── Constructors ────────────────────────────────────────────────────────

    /// Create a body from a single outer shell.
    pub fn from_shell(id: u32, outer: GeometricShell) -> Self {
        Self { id, shells: vec![outer] }
    }

    // ── Mutation ─────────────────────────────────────────────────────────────

    /// Add a void (pocket/hole) shell. The void shell must be interior to the
    /// outer shell and oriented with inward-pointing normals — the caller is
    /// responsible for that invariant (not checked here, only watertightness).
    pub fn add_void(&mut self, void_shell: GeometricShell) {
        self.shells.push(void_shell);
    }

    // ── Accessors ────────────────────────────────────────────────────────────

    /// Borrow the outer shell.
    pub fn outer_shell(&self) -> &GeometricShell {
        &self.shells[0]
    }

    /// Mutable borrow of the outer shell.
    pub fn outer_shell_mut(&mut self) -> &mut GeometricShell {
        &mut self.shells[0]
    }

    /// All void shells (may be empty).
    pub fn void_shells(&self) -> &[GeometricShell] {
        if self.shells.len() > 1 { &self.shells[1..] } else { &[] }
    }

    /// Total shell count (outer + voids).
    pub fn shell_count(&self) -> usize {
        self.shells.len()
    }

    // ── Validation ───────────────────────────────────────────────────────────

    /// Validate that all shells are watertight.
    ///
    /// Returns the first error found. Use `validate_all` to collect every error.
    pub fn validate(&self) -> Result<(), BodyError> {
        if self.shells.is_empty() {
            return Err(BodyError::NoShells);
        }
        if !self.shells[0].is_watertight() {
            return Err(BodyError::OuterShellNotClosed);
        }
        for (i, shell) in self.shells[1..].iter().enumerate() {
            if !shell.is_watertight() {
                return Err(BodyError::VoidShellNotClosed { index: i });
            }
        }
        Ok(())
    }

    /// Collect all validation errors (not just the first).
    pub fn validate_all(&self) -> Vec<BodyError> {
        let mut errors = Vec::new();
        if self.shells.is_empty() {
            errors.push(BodyError::NoShells);
            return errors;
        }
        if !self.shells[0].is_watertight() {
            errors.push(BodyError::OuterShellNotClosed);
        }
        for (i, shell) in self.shells[1..].iter().enumerate() {
            if !shell.is_watertight() {
                errors.push(BodyError::VoidShellNotClosed { index: i });
            }
        }
        errors
    }

    // ── Geometry ─────────────────────────────────────────────────────────────

    /// Signed volume using the divergence theorem (Σ face · area · normal).
    ///
    /// Outer shell contributes +volume, void shells contribute −volume.
    /// Result in the same length unit³ as the vertex coordinates.
    pub fn volume(&self) -> f64 {
        if self.shells.is_empty() {
            return 0.0;
        }
        let outer = self.shells[0].signed_volume();
        let voids: f64 = self.shells[1..].iter().map(|s| s.signed_volume()).sum();
        outer.abs() - voids.abs()
    }

    /// Axis-aligned bounding box `([min_x,min_y,min_z], [max_x,max_y,max_z])`.
    /// Based on the outer shell only.
    pub fn aabb(&self) -> ([f64; 3], [f64; 3]) {
        self.shells[0].aabb()
    }
}

//! Per-particle shape classification & SDF for grid-packed clouds.
//!
//! Goal of this module is to make **one** particle render correctly in any
//! position of a packed grid — interior, on a face, on an edge, or at a
//! corner — so that:
//!
//! * unexposed faces are perfectly flush with neighbours (no seams),
//! * exposed faces / edges / corners are smoothly rounded (visible surface),
//! * the same logic runs on CPU (this module + tests) and on GPU (WGSL port
//!   inside `web/home/scripts/webgpu.rs`).
//!
//! ## Concepts
//!
//! * [`Axis`] — one of the six unit normals of a cell (±X, ±Y, ±Z).
//! * [`ExposedMask`] — 6-bit bitfield marking which faces of a cell are
//!   *not* covered by a neighbour. The number of set bits decides
//!   the [`SlotKind`].
//! * [`SlotKind`] — `Interior` (0 exposed), `Face` (1), `Edge` (2),
//!   `Corner` (≥ 3).
//! * [`CubeGrid`] / [`WallGrid`] — classifiers that turn a 3D / 2D grid
//!   coordinate into an `ExposedMask`.
//! * [`sdf_cell`] — signed distance function for *one* particle in cell-
//!   local space `[-1, 1]³`, given its mask and a corner radius.
//!
//! Cell-local convention:
//!
//! ```text
//!   p ∈ [-1, 1]³,  origin at cell centre
//!   +X right, +Y up, +Z toward viewer
//! ```
//!
//! Neighbour cells are at integer offsets ±1 along each axis; their cells
//! occupy `[1, 3]`, `[-3, -1]`, …, so unexposed faces of the current cell
//! must coincide with the neighbour's face at exactly `|component| = 1`
//! (flat, sharp, no inset) — that is what `sdf_cell` guarantees.

use crate::infrastructure::geometry::kernel::math::Vec3;

// ─────────────────────────────────────────────────────────────────────────
//  Axis
// ─────────────────────────────────────────────────────────────────────────

/// One of the six oriented unit normals of an axis-aligned cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Axis {
    PosX,
    NegX,
    PosY,
    NegY,
    PosZ,
    NegZ,
}

impl Axis {
    /// Bit position inside [`ExposedMask`].
    #[inline]
    pub const fn bit(self) -> u8 {
        match self {
            Axis::PosX => 0,
            Axis::NegX => 1,
            Axis::PosY => 2,
            Axis::NegY => 3,
            Axis::PosZ => 4,
            Axis::NegZ => 5,
        }
    }

    /// Outward unit normal in cell-local space.
    #[inline]
    pub fn normal(self) -> Vec3 {
        match self {
            Axis::PosX => Vec3::new(1.0, 0.0, 0.0),
            Axis::NegX => Vec3::new(-1.0, 0.0, 0.0),
            Axis::PosY => Vec3::new(0.0, 1.0, 0.0),
            Axis::NegY => Vec3::new(0.0, -1.0, 0.0),
            Axis::PosZ => Vec3::new(0.0, 0.0, 1.0),
            Axis::NegZ => Vec3::new(0.0, 0.0, -1.0),
        }
    }

    /// All six axes in canonical order.
    pub const ALL: [Axis; 6] = [
        Axis::PosX,
        Axis::NegX,
        Axis::PosY,
        Axis::NegY,
        Axis::PosZ,
        Axis::NegZ,
    ];
}

// ─────────────────────────────────────────────────────────────────────────
//  ExposedMask
// ─────────────────────────────────────────────────────────────────────────

/// Bitfield: which of the six cell faces have **no neighbour**.
///
/// Layout (bit 0 → bit 5): `+X, -X, +Y, -Y, +Z, -Z`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ExposedMask(pub u8);

impl ExposedMask {
    pub const NONE: ExposedMask = ExposedMask(0);
    pub const ALL: ExposedMask = ExposedMask(0b0011_1111);

    /// Build from an explicit list of axes.
    pub fn from_axes(axes: &[Axis]) -> Self {
        let mut m: u8 = 0;
        for a in axes {
            m |= 1 << a.bit();
        }
        ExposedMask(m)
    }

    /// True if the given face is exposed.
    #[inline]
    pub fn exposed(self, a: Axis) -> bool {
        (self.0 >> a.bit()) & 1 == 1
    }

    /// Number of exposed faces (0..=6).
    #[inline]
    pub fn count(self) -> u32 {
        self.0.count_ones()
    }

    /// Categorise this mask.
    #[inline]
    pub fn slot_kind(self) -> SlotKind {
        match self.count() {
            0 => SlotKind::Interior,
            1 => SlotKind::Face,
            2 => SlotKind::Edge,
            _ => SlotKind::Corner,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────
//  SlotKind
// ─────────────────────────────────────────────────────────────────────────

/// Topological role of a cell inside its packed shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SlotKind {
    /// 0 exposed faces — fully surrounded; can be rendered as cheapest path
    /// (or culled outright since it never contributes pixels).
    Interior,
    /// 1 exposed face — flat outer slab.
    Face,
    /// 2 exposed faces — outer dihedral edge, rounded along that edge.
    Edge,
    /// ≥ 3 exposed faces — outer trihedral corner, rounded sphere-cap.
    Corner,
}

// ─────────────────────────────────────────────────────────────────────────
//  Grid classifiers
// ─────────────────────────────────────────────────────────────────────────

/// A solid `side³` axis-aligned cube grid of cells.
///
/// `(0, 0, 0)` is the cell at the most-negative corner; `(side-1, side-1,
/// side-1)` is at the most-positive corner.
#[derive(Debug, Clone, Copy)]
pub struct CubeGrid {
    pub side: u32,
}

impl CubeGrid {
    pub const fn new(side: u32) -> Self {
        Self { side }
    }

    /// Classify a cell of the cube grid by its 3D coordinate.
    ///
    /// Faces touching the outer hull of the cube are exposed; everything
    /// inside is `Interior`.
    pub fn classify(&self, ix: u32, iy: u32, iz: u32) -> ExposedMask {
        debug_assert!(
            ix < self.side && iy < self.side && iz < self.side,
            "CubeGrid::classify out of bounds"
        );
        let last = self.side.saturating_sub(1);
        let mut m: u8 = 0;
        if ix == last {
            m |= 1 << Axis::PosX.bit();
        }
        if ix == 0 {
            m |= 1 << Axis::NegX.bit();
        }
        if iy == last {
            m |= 1 << Axis::PosY.bit();
        }
        if iy == 0 {
            m |= 1 << Axis::NegY.bit();
        }
        if iz == last {
            m |= 1 << Axis::PosZ.bit();
        }
        if iz == 0 {
            m |= 1 << Axis::NegZ.bit();
        }
        ExposedMask(m)
    }

    /// Walk every surface cell (all cells with `count() > 0`).
    pub fn surface_cells(&self) -> impl Iterator<Item = (u32, u32, u32, ExposedMask)> + '_ {
        let s = self.side;
        (0..s)
            .flat_map(move |iz| {
                (0..s).flat_map(move |iy| {
                    (0..s).map(move |ix| (ix, iy, iz, self.classify(ix, iy, iz)))
                })
            })
            .filter(|(_, _, _, m)| m.count() > 0)
    }
}

/// A `cols × rows × 1` slab grid lying in the plane `z = 0`.
///
/// All cells are exposed on `+Z` and `-Z` (the two faces of the slab);
/// border cells additionally expose `±X` / `±Y`.
#[derive(Debug, Clone, Copy)]
pub struct WallGrid {
    pub cols: u32,
    pub rows: u32,
}

impl WallGrid {
    pub const fn new(cols: u32, rows: u32) -> Self {
        Self { cols, rows }
    }

    pub fn classify(&self, ix: u32, iy: u32) -> ExposedMask {
        debug_assert!(
            ix < self.cols && iy < self.rows,
            "WallGrid::classify out of bounds"
        );
        let mut m: u8 = (1 << Axis::PosZ.bit()) | (1 << Axis::NegZ.bit());
        if ix == self.cols - 1 {
            m |= 1 << Axis::PosX.bit();
        }
        if ix == 0 {
            m |= 1 << Axis::NegX.bit();
        }
        if iy == self.rows - 1 {
            m |= 1 << Axis::PosY.bit();
        }
        if iy == 0 {
            m |= 1 << Axis::NegY.bit();
        }
        ExposedMask(m)
    }
}

// ─────────────────────────────────────────────────────────────────────────
//  Cell SDF
// ─────────────────────────────────────────────────────────────────────────

/// Signed distance to the surface of *one* cell in cell-local coords.
///
/// * `p`     — query point. Cell occupies the cube `[-1, 1]³`.
/// * `mask`  — which faces are exposed.
/// * `radius`— corner radius in cell units, clamped to `[0, 0.5]`.
///   `0.0` = sharp box (cell fills its allotted footprint exactly);
///   `0.5` = maximum rounding (each exposed face is inset by half a cell,
///   producing a perfect quarter-cylinder along edges and a sphere octant
///   at corners).
///
/// Geometric construction:
///
/// * Each *unexposed* face stays at `|component| = 1` (sharp; flush with
///   the neighbour cell that owns the opposite half-space).
/// * Each *exposed* face shifts inward by `r` to `|component| = 1 - r`.
/// * The slabs intersect with sharp corners; the natural `length(outside)`
///   term then turns the meeting of two/three exposed slabs into a
///   smooth rounded edge / sphere-octant corner of radius `r`.
///
/// Properties (proved by the unit tests):
///
/// 1. **Flush seams.** On any *unexposed* face the cell's surface coincides
///    exactly with `|component| = 1` (no inset, no rounding) — neighbour
///    cells share that plane and the union has no visible seam.
/// 2. **Inward inset.** On an exposed face the SDF is `0` at
///    `|component| = 1 - r`, so the visible outer hull of the megashape
///    sits exactly `r` inside the cell-grid bounding box.
/// 3. **Smooth corners.** On an `Edge` cell the two exposed slabs meet
///    in a quarter-cylinder of radius `r`; on a `Corner` cell three
///    exposed slabs meet in a sphere-octant of radius `r`.
/// 4. **Interior cull.** For an interior cell the SDF reduces to the
///    standard sharp-box SDF — fine for backface culling tests.
pub fn sdf_cell(p: Vec3, mask: ExposedMask, radius: f32) -> f32 {
    let r = radius.clamp(0.0, 0.5);

    // For each face: shift the slab inward by `r` *only* if exposed.
    // Unexposed slabs stay at ±1 so neighbours are perfectly flush.
    let sxp = if mask.exposed(Axis::PosX) { r } else { 0.0 };
    let sxn = if mask.exposed(Axis::NegX) { r } else { 0.0 };
    let syp = if mask.exposed(Axis::PosY) { r } else { 0.0 };
    let syn = if mask.exposed(Axis::NegY) { r } else { 0.0 };
    let szp = if mask.exposed(Axis::PosZ) { r } else { 0.0 };
    let szn = if mask.exposed(Axis::NegZ) { r } else { 0.0 };

    // Per-axis slab distance: positive when the point is outside the slab,
    // negative inside (deeper into the cell).
    let qx = (p.x - (1.0 - sxp)).max(-p.x - (1.0 - sxn));
    let qy = (p.y - (1.0 - syp)).max(-p.y - (1.0 - syn));
    let qz = (p.z - (1.0 - szp)).max(-p.z - (1.0 - szn));

    // Standard sharp-or-anisotropic box SDF — `length(outside)` term gives
    // the smooth corner whenever two/three exposed slabs meet.
    let outside = Vec3::new(qx.max(0.0), qy.max(0.0), qz.max(0.0));
    let inside = qx.max(qy).max(qz).min(0.0);
    outside.length() + inside
}

/// Approximate surface normal at `p`, computed by central differences of
/// [`sdf_cell`]. Returns a unit vector (length ≈ 1) except at degenerate
/// points (always-zero gradient) where it falls back to `+Y`.
pub fn cell_normal(p: Vec3, mask: ExposedMask, radius: f32) -> Vec3 {
    const EPS: f32 = 1.0e-3;
    let dx = sdf_cell(Vec3::new(p.x + EPS, p.y, p.z), mask, radius)
        - sdf_cell(Vec3::new(p.x - EPS, p.y, p.z), mask, radius);
    let dy = sdf_cell(Vec3::new(p.x, p.y + EPS, p.z), mask, radius)
        - sdf_cell(Vec3::new(p.x, p.y - EPS, p.z), mask, radius);
    let dz = sdf_cell(Vec3::new(p.x, p.y, p.z + EPS), mask, radius)
        - sdf_cell(Vec3::new(p.x, p.y, p.z - EPS), mask, radius);
    let len2 = dx * dx + dy * dy + dz * dz;
    if len2 < 1.0e-12 {
        Vec3::UP
    } else {
        let inv = 1.0 / len2.sqrt();
        Vec3::new(dx * inv, dy * inv, dz * inv)
    }
}

// ─────────────────────────────────────────────────────────────────────────
//  Tests — single particle correctness
// ─────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: nearly-equal float comparison.
    fn close(a: f32, b: f32, eps: f32) -> bool {
        (a - b).abs() <= eps
    }

    // ── ExposedMask ──────────────────────────────────────────────────────
    #[test]
    fn mask_count_and_slot_kind() {
        assert_eq!(ExposedMask::NONE.count(), 0);
        assert_eq!(ExposedMask::NONE.slot_kind(), SlotKind::Interior);

        let m1 = ExposedMask::from_axes(&[Axis::PosY]);
        assert_eq!(m1.count(), 1);
        assert_eq!(m1.slot_kind(), SlotKind::Face);

        let m2 = ExposedMask::from_axes(&[Axis::PosX, Axis::PosY]);
        assert_eq!(m2.count(), 2);
        assert_eq!(m2.slot_kind(), SlotKind::Edge);

        let m3 = ExposedMask::from_axes(&[Axis::PosX, Axis::PosY, Axis::PosZ]);
        assert_eq!(m3.count(), 3);
        assert_eq!(m3.slot_kind(), SlotKind::Corner);

        assert_eq!(ExposedMask::ALL.count(), 6);
        assert_eq!(ExposedMask::ALL.slot_kind(), SlotKind::Corner);
    }

    #[test]
    fn mask_exposed_lookup() {
        let m = ExposedMask::from_axes(&[Axis::PosX, Axis::NegZ]);
        assert!(m.exposed(Axis::PosX));
        assert!(!m.exposed(Axis::NegX));
        assert!(m.exposed(Axis::NegZ));
        assert!(!m.exposed(Axis::PosZ));
    }

    // ── CubeGrid ─────────────────────────────────────────────────────────
    #[test]
    fn cube_grid_classifies_corner_face_edge_interior() {
        let g = CubeGrid::new(4); // 4×4×4 cells

        // most-positive corner (3, 3, 3): +X, +Y, +Z exposed → Corner
        let c = g.classify(3, 3, 3);
        assert_eq!(c.slot_kind(), SlotKind::Corner);
        assert!(c.exposed(Axis::PosX));
        assert!(c.exposed(Axis::PosY));
        assert!(c.exposed(Axis::PosZ));

        // edge along +X+Y (3, 3, 1): +X, +Y exposed → Edge
        let e = g.classify(3, 3, 1);
        assert_eq!(e.slot_kind(), SlotKind::Edge);

        // face on +Y (1, 3, 1): only +Y exposed → Face
        let f = g.classify(1, 3, 1);
        assert_eq!(f.slot_kind(), SlotKind::Face);
        assert!(f.exposed(Axis::PosY));
        assert!(!f.exposed(Axis::PosX));

        // interior (1, 1, 1): nothing exposed
        let i = g.classify(1, 1, 1);
        assert_eq!(i.slot_kind(), SlotKind::Interior);
        assert_eq!(i.count(), 0);
    }

    #[test]
    fn cube_grid_surface_cells_count_matches_formula() {
        // Hollow cube surface: 6·s² − 12·s + 8 (inclusion-exclusion on faces).
        for s in 2u32..=8 {
            let g = CubeGrid::new(s);
            let n = g.surface_cells().count() as u32;
            let expected = 6 * s * s - 12 * s + 8;
            assert_eq!(n, expected, "side = {s}");
        }
    }

    #[test]
    fn cube_grid_corner_count_is_eight() {
        let g = CubeGrid::new(5);
        let corners = g
            .surface_cells()
            .filter(|(_, _, _, m)| m.slot_kind() == SlotKind::Corner)
            .count();
        assert_eq!(corners, 8);
    }

    // ── WallGrid ─────────────────────────────────────────────────────────
    #[test]
    fn wall_grid_basic_classification() {
        let w = WallGrid::new(5, 3);

        // every cell exposes ±Z (it's a single-layer slab)
        for iy in 0..3 {
            for ix in 0..5 {
                let m = w.classify(ix, iy);
                assert!(m.exposed(Axis::PosZ));
                assert!(m.exposed(Axis::NegZ));
            }
        }

        // four corners → 4 exposed faces (±Z + two of ±X / ±Y)
        for &(ix, iy) in &[(0, 0), (4, 0), (0, 2), (4, 2)] {
            assert_eq!(w.classify(ix, iy).count(), 4);
            assert_eq!(w.classify(ix, iy).slot_kind(), SlotKind::Corner);
        }

        // a non-border cell → 2 exposed faces (just ±Z)
        let m = w.classify(2, 1);
        assert_eq!(m.count(), 2);
        assert_eq!(m.slot_kind(), SlotKind::Edge);
    }

    // ── sdf_cell ─────────────────────────────────────────────────────────
    #[test]
    fn interior_cell_is_a_sharp_unit_box() {
        let m = ExposedMask::NONE;

        // Centre is fully inside.
        assert!(close(sdf_cell(Vec3::ZERO, m, 0.10), -1.0, 1e-6));
        // Right on a face → SDF = 0.
        assert!(close(
            sdf_cell(Vec3::new(1.0, 0.0, 0.0), m, 0.10),
            0.0,
            1e-6
        ));
        assert!(close(
            sdf_cell(Vec3::new(0.0, 1.0, 0.0), m, 0.10),
            0.0,
            1e-6
        ));
        assert!(close(
            sdf_cell(Vec3::new(0.0, 0.0, 1.0), m, 0.10),
            0.0,
            1e-6
        ));
        // Outside on a face → SDF = positive distance to face.
        assert!(close(
            sdf_cell(Vec3::new(1.5, 0.0, 0.0), m, 0.10),
            0.5,
            1e-6
        ));
    }

    #[test]
    fn unexposed_face_stays_flush_at_unit_plane() {
        // Cell exposes +X only (a Face cell). On the −X face (unexposed)
        // the SDF must be 0 at x = -1 regardless of the rounding radius.
        let m = ExposedMask::from_axes(&[Axis::PosX]);
        for r in [0.0, 0.1, 0.25, 0.5] {
            for &p in &[
                Vec3::new(-1.0, 0.0, 0.0),
                Vec3::new(-1.0, 0.5, -0.5),
                Vec3::new(-1.0, -0.5, 0.5),
            ] {
                let d = sdf_cell(p, m, r);
                assert!(
                    d.abs() < 0.02,
                    "unexposed −X face must be flush with x=-1 (r={r}, d={d})"
                );
            }
        }
    }

    #[test]
    fn exposed_face_is_inset_by_radius_at_centre() {
        // Cell exposes +Y only. r=0.2 → +Y face should be at y ≈ 1 - r·(1/6)
        // *near the centre of the face* (small inset because only one face
        // is exposed). What matters is that the surface is still convex and
        // the point (0, 1, 0) lies *outside* (positive SDF).
        let m = ExposedMask::from_axes(&[Axis::PosY]);
        let d_at_top = sdf_cell(Vec3::new(0.0, 1.0, 0.0), m, 0.20);
        assert!(
            d_at_top > 0.0,
            "Face cell: exposed +Y top must shrink inward (got d={d_at_top})"
        );

        // and the unexposed −Y face must still touch y = -1
        let d_bot = sdf_cell(Vec3::new(0.0, -1.0, 0.0), m, 0.20);
        assert!(
            d_bot.abs() < 0.02,
            "Face cell: unexposed −Y face must stay flush (d={d_bot})"
        );
    }

    #[test]
    fn corner_cell_with_full_radius_becomes_sphere_at_open_corner() {
        // A corner cell with all 3 of +X +Y +Z exposed and r = 0.5 should
        // pull the outer trihedral corner inward to ≈ (1−r) on each axis.
        // The diagonal point (1, 1, 1) must be clearly outside.
        let m = ExposedMask::from_axes(&[Axis::PosX, Axis::PosY, Axis::PosZ]);
        let d = sdf_cell(Vec3::new(1.0, 1.0, 1.0), m, 0.5);
        assert!(
            d > 0.5,
            "corner is rounded: outer apex must be far outside (d={d})"
        );

        // And the unexposed faces stay flush.
        for &p in &[
            Vec3::new(-1.0, 0.0, 0.0),
            Vec3::new(0.0, -1.0, 0.0),
            Vec3::new(0.0, 0.0, -1.0),
        ] {
            let d = sdf_cell(p, m, 0.5);
            assert!(d.abs() < 0.02, "unexposed face flush at {:?} (d={d})", p);
        }
    }

    #[test]
    fn fully_exposed_cell_with_max_radius_is_a_sphere() {
        // mask = all six, r = 0.5 → every slab shrinks to ±0.5; the cell
        // becomes a small box [-0.5, 0.5]³ with rounded edges/corners
        // (the rounding emerges naturally from `length(outside)`).
        // Outside that box the SDF must equal the distance to the box [-0.5, 0.5]³.
        let m = ExposedMask::ALL;
        let cases: &[(Vec3, f32)] = &[
            // p,                 expected = √(Σ max(|p_i|-0.5, 0)²)
            (Vec3::new(2.0, 0.0, 0.0), 1.5),
            (Vec3::new(0.0, 3.0, 0.0), 2.5),
            (Vec3::new(1.0, 1.0, 1.0), (3.0_f32 * 0.25).sqrt()),
            (Vec3::new(1.5, 0.5, 0.7), {
                let qx = 1.0_f32;
                let qy = 0.0_f32;
                let qz = 0.2_f32;
                (qx * qx + qy * qy + qz * qz).sqrt()
            }),
        ];
        for &(p, expected) in cases {
            let d = sdf_cell(p, m, 0.5);
            assert!(
                close(d, expected, 0.02),
                "fully-open r=0.5 cell at {:?}: sdf={d}, expected≈{expected}",
                p
            );
        }
    }

    #[test]
    fn fully_exposed_cell_max_radius_corner_radius_equals_r() {
        // Inscribed-sphere check: at p in the +X+Y+Z octant on the diagonal,
        // the SDF measured radially from the inner box corner (0.5, 0.5, 0.5)
        // should equal `|p - (0.5, 0.5, 0.5)|` — that *is* the sphere of
        // radius matching how far we are from the inner box corner.
        let m = ExposedMask::ALL;
        let r = 0.5_f32;
        let inner = Vec3::new(1.0 - r, 1.0 - r, 1.0 - r);
        let p = Vec3::new(0.7, 0.7, 0.7); // outside inner
        let dx = p.x - inner.x;
        let dy = p.y - inner.y;
        let dz = p.z - inner.z;
        let radial = (dx * dx + dy * dy + dz * dz).sqrt();
        let sdf = sdf_cell(p, m, r);
        assert!(
            close(sdf, radial, 1e-5),
            "rounded corner is a sphere: sdf={sdf}, expected={radial}"
        );
    }

    // ── cell_normal ──────────────────────────────────────────────────────
    #[test]
    fn normal_on_exposed_face_points_outward() {
        // Face cell exposing +Y. At (0, ~1, 0) the normal must be ≈ +Y.
        let m = ExposedMask::from_axes(&[Axis::PosY]);
        let n = cell_normal(Vec3::new(0.0, 0.99, 0.0), m, 0.10);
        assert!(n.y > 0.95, "normal should point ~+Y (got {:?})", n);
    }

    #[test]
    fn normal_at_open_corner_diagonal_points_outward_diagonal() {
        // Corner cell with +X +Y +Z exposed. On the diagonal direction the
        // normal should point roughly along (1,1,1)/√3.
        let m = ExposedMask::from_axes(&[Axis::PosX, Axis::PosY, Axis::PosZ]);
        let p = Vec3::new(0.6, 0.6, 0.6);
        let n = cell_normal(p, m, 0.30);
        assert!(
            n.x > 0.3 && n.y > 0.3 && n.z > 0.3,
            "diagonal corner normal should be roughly (+,+,+) (got {:?})",
            n
        );
    }

    // ── single particle integration ──────────────────────────────────────
    #[test]
    fn one_particle_full_pipeline() {
        // Take ONE particle on the (3,3,3) corner of a 4-cube grid, classify
        // it, then sample its SDF and normal. This is the canonical
        // single-particle path that the WGSL shader will mirror verbatim.
        let g = CubeGrid::new(4);
        let mask = g.classify(3, 3, 3);
        assert_eq!(mask.slot_kind(), SlotKind::Corner);

        // The cell occupies cell-local [-1,1]³.
        // Origin (0,0,0) of the cell is inside.
        assert!(sdf_cell(Vec3::ZERO, mask, 0.25) < 0.0);
        // The diagonal apex of the OPEN corner (+X+Y+Z) is outside.
        assert!(sdf_cell(Vec3::new(1.0, 1.0, 1.0), mask, 0.25) > 0.0);
        // The opposite, unexposed corner (-X-Y-Z) of the cell sits exactly
        // at three flush walls → SDF ≈ 0.
        let d = sdf_cell(Vec3::new(-1.0, -1.0, -1.0), mask, 0.25);
        assert!(d.abs() < 0.02, "shared corner must be flush (d={d})");

        // Normal at the open apex points outward.
        let n = cell_normal(Vec3::new(0.7, 0.7, 0.7), mask, 0.25);
        assert!(n.x > 0.3 && n.y > 0.3 && n.z > 0.3);
    }
}

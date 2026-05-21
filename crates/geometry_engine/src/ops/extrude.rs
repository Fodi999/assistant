//! Extrude operation — sweep a closed 2D polygon along the Z axis.
//!
//! Unlike `lathe_profile` (which revolves around Y), `extrude_polygon` sweeps
//! a planar 2D contour forward/backward along Z to produce:
//!   * a front cap (facing +Z)
//!   * a back cap  (facing -Z)
//!   * side walls  (one quad per edge, normals computed analytically per-edge)
//!
//! Optionally a `bevel` parameter produces a chamfered edge — 45° faces
//! connecting the inset cap contour to the main side wall. This is the
//! Plasticity-style look for cards, panels and dock parts.
//!
//! Return value: three separate [`MeshPart`]s so the caller can assign them
//! to different material groups (front face / back face / edge):
//!   `[0]` — front cap  (+Z)
//!   `[1]` — back cap   (-Z)
//!   `[2]` — side walls (+ bevel strips if `bevel > 0`)
//!
//! Conventions match the rest of the kernel:
//!   * Y-up, Z = depth axis (card faces the camera along +Z)
//!   * CCW winding seen from outside each face
//!   * All units in metres

use crate::mesh::MeshPart;
use crate::mesh::GeometryError;
use crate::brep::{BrepBuilder, BrepModel};
use crate::topology::BodyId;
use crate::tessellation::{tessellate_body, MeshWithMetadata, TessOptions};

// ─────────────────────────────────────────────────────────────────────────────
// Public types
// ─────────────────────────────────────────────────────────────────────────────

/// A 2D point in the XY plane. Uses Real (f64) for CAD precision.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point2 {
    pub x: crate::math::Real,
    pub y: crate::math::Real,
}

impl Point2 {
    #[inline]
    pub const fn new(x: crate::math::Real, y: crate::math::Real) -> Self {
        Self { x, y }
    }
}

/// Options for [`extrude_polygon`].
#[derive(Debug, Clone)]
pub struct ExtrudeOptions {
    /// Total depth (Z extent). The polygon is centred at z = 0, so the front
    /// cap sits at `+depth/2` and the back cap at `-depth/2`.
    pub depth: crate::math::Real,
    /// Optional chamfer width (metres). Clamped to `depth * 0.49`.
    pub bevel: crate::math::Real,
}

impl Default for ExtrudeOptions {
    fn default() -> Self {
        Self {
            depth: 0.08,
            bevel: 0.0,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Main entry point
// ─────────────────────────────────────────────────────────────────────────────

/// Extrude a closed 2D polygon into a 3D shell.
///
/// `points` must be in **counter-clockwise** order when viewed from +Z and
/// must contain at least 3 vertices.
///
/// Returns `[front_cap, back_cap, side_walls]` as separate [`MeshPart`]s.
pub fn extrude_polygon(
    points: &[Point2],
    options: &ExtrudeOptions,
) -> Result<[MeshPart; 3], GeometryError> {
    let n = points.len();
    if n < 3 {
        return Err(GeometryError::InvalidArgument(
            "extrude_polygon needs at least 3 points".into(),
        ));
    }
    if !options.depth.is_finite() || options.depth <= 0.0 {
        return Err(GeometryError::InvalidArgument(format!(
            "extrude depth must be > 0 (got {})",
            options.depth
        )));
    }

    let half = options.depth * 0.5;
    let bevel = options.bevel.clamp(0.0, half * 0.49);
    let has_bevel = bevel > 1e-5;

    // With bevel the cap sits slightly inward, so the chamfer face
    // connects from the main contour at the outer z to the inset contour.
    let cap_z_front = if has_bevel { half - bevel } else { half };
    let cap_z_back = if has_bevel { -(half - bevel) } else { -half };

    let front = build_cap(points, cap_z_front, [0.0, 0.0,  1.0], true);   // flip=true → geo-normal +Z(kernel)=+Y(world XZ)
    let back  = build_cap(points, cap_z_back,  [0.0, 0.0, -1.0], false);  // flip=false → geo-normal -Z(kernel)=-Y(world XZ)

    let sides = if has_bevel {
        build_sides_beveled(points, half, bevel)
    } else {
        build_sides_flat(points, half)
    };

    Ok([front, back, sides])
}

// ─────────────────────────────────────────────────────────────────────────────
// B-Rep extrude — single source of truth
// ─────────────────────────────────────────────────────────────────────────────

/// Result of the canonical B-Rep extrude path:
/// model owns the topology, body_id identifies the new body, and `mesh`
/// is the tessellated triangle mesh with per-triangle [`FaceId`] metadata.
#[derive(Debug)]
pub struct ExtrudeBrepResult {
    pub model:   BrepModel,
    pub body_id: BodyId,
    pub mesh:    MeshWithMetadata,
}

/// Canonical extrude: `polygon → BrepBuilder → tessellate → MeshWithMetadata`.
///
/// The polygon is interpreted as lying in the XY plane (z = 0) and is swept
/// along `+Z` by `options.depth`. The resulting solid sits with its bottom
/// at `z = 0` and its top at `z = depth`.
///
/// Bevel is currently ignored on the B-Rep path (planned: built via fillet
/// op on the rim edges once `fillet/` is implemented).
pub fn extrude_polygon_brep(
    points: &[Point2],
    options: &ExtrudeOptions,
) -> Result<ExtrudeBrepResult, GeometryError> {
    let n = points.len();
    if n < 3 {
        return Err(GeometryError::InvalidArgument(
            "extrude_polygon_brep needs at least 3 points".into(),
        ));
    }
    if !options.depth.is_finite() || options.depth <= 0.0 {
        return Err(GeometryError::InvalidArgument(format!(
            "extrude depth must be > 0 (got {})",
            options.depth
        )));
    }

    let polygon: Vec<crate::math::Point3> = points
        .iter()
        .map(|p| crate::math::Point3::new(p.x, p.y, 0.0))
        .collect();

    let mut builder = BrepBuilder::new();
    builder.begin_body("extrude");
    builder.prism_from_polygon(&polygon, [0.0, 0.0, options.depth]);
    let body_id = builder
        .context()
        .body
        .expect("begin_body should have set context");
    let model = builder.build();

    let opts = TessOptions::default();
    let mesh = tessellate_body(&model, body_id, &opts);

    Ok(ExtrudeBrepResult { model, body_id, mesh })
}

// ─────────────────────────────────────────────────────────────────────────────
// Cap (fan triangulation)
// ─────────────────────────────────────────────────────────────────────────────

fn build_cap(points: &[Point2], z: f64, normal: [f64; 3], flip: bool) -> MeshPart {
    let n = points.len();

    // Normalised UV bounding box.
    let (min_x, max_x, min_y, max_y) = points.iter().fold(
        (
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
        ),
        |(lx, hx, ly, hy), p| (lx.min(p.x), hx.max(p.x), ly.min(p.y), hy.max(p.y)),
    );
    let range_x = (max_x - min_x).max(1e-6);
    let range_y = (max_y - min_y).max(1e-6);

    let mut vertices: Vec<[f64; 3]> = Vec::with_capacity(n);
    let mut normals: Vec<[f64; 3]> = Vec::with_capacity(n);
    let mut uvs: Vec<[f64; 2]> = Vec::with_capacity(n);

    for p in points {
        vertices.push([p.x, p.y, z]);
        normals.push(normal);
        uvs.push([(p.x - min_x) / range_x, (p.y - min_y) / range_y]);
    }

    // Fan from vertex 0.
    let mut faces: Vec<[usize; 3]> = Vec::with_capacity(n - 2);
    for i in 1..(n - 1) {
        if flip {
            faces.push([0, i + 1, i]);
        } else {
            faces.push([0, i, i + 1]);
        }
    }

    MeshPart {
        vertices,
        normals,
        uvs,
        faces,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Side walls — no bevel
// ─────────────────────────────────────────────────────────────────────────────

fn build_sides_flat(points: &[Point2], half: f64) -> MeshPart {
    let n = points.len();

    // Cumulative perimeter → U coordinate.
    let (edge_u, total_perim) = edge_u_coords(points);

    let mut vertices: Vec<[f64; 3]> = Vec::with_capacity(n * 4);
    let mut normals: Vec<[f64; 3]> = Vec::with_capacity(n * 4);
    let mut uvs: Vec<[f64; 2]> = Vec::with_capacity(n * 4);
    let mut faces: Vec<[usize; 3]> = Vec::with_capacity(n * 2);

    for i in 0..n {
        let j = (i + 1) % n;
        let a = points[i];
        let b = points[j];

        // Outward edge normal in XY (perpendicular to edge direction).
        let (nx, ny) = outward_normal_2d(a, b);
        let norm: [f64; 3] = [nx, ny, 0.0];

        let u0 = edge_u[i] / total_perim;
        let u1 = edge_u[i + 1] / total_perim;

        let base = vertices.len();

        // 4 verts: front-left, front-right, back-right, back-left.
        // OLD (correct) ordering: front verts first so CCW-from-outside gives outward normal.
        vertices.push([a.x, a.y,  half]); // 0  front-left
        vertices.push([b.x, b.y,  half]); // 1  front-right
        vertices.push([b.x, b.y, -half]); // 2  back-right
        vertices.push([a.x, a.y, -half]); // 3  back-left

        for _ in 0..4 {
            normals.push(norm);
        }
        uvs.push([u0, 1.0]);
        uvs.push([u1, 1.0]);
        uvs.push([u1, 0.0]);
        uvs.push([u0, 0.0]);

        // CCW seen from outside.
        faces.push([base, base + 1, base + 2]);
        faces.push([base, base + 2, base + 3]);
    }

    MeshPart {
        vertices,
        normals,
        uvs,
        faces,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Side walls — beveled (Plasticity-style chamfered edge)
// ─────────────────────────────────────────────────────────────────────────────
//
// For each edge we emit 4 vertex rings × 2 XY positions = 8 verts:
//
//   ring 0:  z = +half,        XY inset by `bevel` (cap edge)
//   ring 1:  z = +half-bevel,  XY full               (bevel-top → wall join)
//   ring 2:  z = -(half-bevel) XY full               (wall → bevel-bottom join)
//   ring 3:  z = -half,        XY inset by `bevel`   (back cap edge)
//
// 3 quad rows → 6 triangles per edge.
// Normals: bevel rows at 45° blend, wall rows pure outward.

fn build_sides_beveled(points: &[Point2], half: f64, bevel: f64) -> MeshPart {
    let n = points.len();

    let (edge_u, total_perim) = edge_u_coords(points);

    let mut vertices: Vec<[f64; 3]> = Vec::with_capacity(n * 8);
    let mut normals: Vec<[f64; 3]> = Vec::with_capacity(n * 8);
    let mut uvs: Vec<[f64; 2]> = Vec::with_capacity(n * 8);
    let mut faces: Vec<[usize; 3]> = Vec::with_capacity(n * 6);

    let inv_sqrt2 = 1.0_f64 / 2.0_f64.sqrt();

    for i in 0..n {
        let j = (i + 1) % n;
        let a = points[i];
        let b = points[j];

        let (nx, ny) = outward_normal_2d(a, b);
        let side_n = [nx, ny, 0.0];
        let bvl_f_n = [nx * inv_sqrt2, ny * inv_sqrt2, inv_sqrt2];
        let bvl_b_n = [nx * inv_sqrt2, ny * inv_sqrt2, -inv_sqrt2];

        let u0 = edge_u[i] / total_perim;
        let u1 = edge_u[i + 1] / total_perim;

        // Inset endpoints.
        let ax_in = a.x - nx * bevel;
        let ay_in = a.y - ny * bevel;
        let bx_in = b.x - nx * bevel;
        let by_in = b.y - ny * bevel;

        let base = vertices.len();

        // Ring 0 — inset at +half.
        vertices.push([ax_in, ay_in, half]);
        vertices.push([bx_in, by_in, half]);
        // Ring 1 — full at +(half - bevel).
        vertices.push([a.x, a.y, half - bevel]);
        vertices.push([b.x, b.y, half - bevel]);
        // Ring 2 — full at -(half - bevel).
        vertices.push([a.x, a.y, -(half - bevel)]);
        vertices.push([b.x, b.y, -(half - bevel)]);
        // Ring 3 — inset at -half.
        vertices.push([ax_in, ay_in, -half]);
        vertices.push([bx_in, by_in, -half]);

        normals.push(bvl_f_n);
        normals.push(bvl_f_n);
        normals.push(side_n);
        normals.push(side_n);
        normals.push(side_n);
        normals.push(side_n);
        normals.push(bvl_b_n);
        normals.push(bvl_b_n);

        // UV: v ∈ [0, 1] front→back, u along perimeter.
        uvs.push([u0, 1.00]);
        uvs.push([u1, 1.00]);
        uvs.push([u0, 0.85]);
        uvs.push([u1, 0.85]);
        uvs.push([u0, 0.15]);
        uvs.push([u1, 0.15]);
        uvs.push([u0, 0.00]);
        uvs.push([u1, 0.00]);

        // Row 0: bevel front  (ring0 → ring1)
        faces.push([base, base + 1, base + 3]);
        faces.push([base, base + 3, base + 2]);
        // Row 1: straight wall (ring1 → ring2)
        faces.push([base + 2, base + 3, base + 5]);
        faces.push([base + 2, base + 5, base + 4]);
        // Row 2: bevel back  (ring2 → ring3)
        faces.push([base + 4, base + 5, base + 7]);
        faces.push([base + 4, base + 7, base + 6]);
    }

    MeshPart {
        vertices,
        normals,
        uvs,
        faces,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Shared helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Compute outward 2D normal for the edge `a → b`.
/// The outward direction is `(dy, -dx)` normalised.
#[inline]
fn outward_normal_2d(a: Point2, b: Point2) -> (f64, f64) {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let len = (dx * dx + dy * dy).sqrt().max(1e-8);
    (dy / len, -dx / len)
}

/// Cumulative edge lengths → U texture coordinate per vertex.
/// Returns `(edge_u, total_perimeter)` where `edge_u[i]` is the arc length
/// up to vertex `i` and `edge_u[n]` == total perimeter.
fn edge_u_coords(points: &[Point2]) -> (Vec<f64>, f64) {
    let n = points.len();
    let mut edge_u = Vec::with_capacity(n + 1);
    edge_u.push(0.0_f64);
    for i in 0..n {
        let j = (i + 1) % n;
        let dx = points[j].x - points[i].x;
        let dy = points[j].y - points[i].y;
        edge_u.push(edge_u[i] + (dx * dx + dy * dy).sqrt());
    }
    let total = (*edge_u.last().unwrap()).max(1e-6);
    (edge_u, total)
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn square(s: f64) -> Vec<Point2> {
        vec![
            Point2::new(s, s),
            Point2::new(-s, s),
            Point2::new(-s, -s),
            Point2::new(s, -s),
        ]
    }

    #[test]
    fn extrude_square_produces_valid_parts() {
        let pts = square(0.05);
        let opts = ExtrudeOptions {
            depth: 0.02,
            bevel: 0.0,
        };
        let [front, back, sides] = extrude_polygon(&pts, &opts).unwrap();

        // 4 verts → 2 triangles per cap.
        assert_eq!(front.faces.len(), 2);
        assert_eq!(back.faces.len(), 2);
        // 4 edges × 2 triangles.
        assert_eq!(sides.faces.len(), 8);
    }

    #[test]
    fn extrude_square_beveled() {
        let pts = square(0.05);
        let opts = ExtrudeOptions {
            depth: 0.02,
            bevel: 0.002,
        };
        let [front, back, sides] = extrude_polygon(&pts, &opts).unwrap();

        assert_eq!(front.faces.len(), 2);
        assert_eq!(back.faces.len(), 2);
        // 4 edges × 6 triangles (3 rows each).
        assert_eq!(sides.faces.len(), 24);
    }

    #[test]
    fn extrude_rejects_fewer_than_3_points() {
        let opts = ExtrudeOptions::default();
        assert!(extrude_polygon(&[Point2::new(0.0, 0.0), Point2::new(1.0, 0.0)], &opts).is_err());
    }

    #[test]
    fn extrude_rejects_non_positive_depth() {
        let pts = square(0.05);
        let opts = ExtrudeOptions {
            depth: -0.01,
            bevel: 0.0,
        };
        assert!(extrude_polygon(&pts, &opts).is_err());
    }

    #[test]
    fn cap_normals_are_unit_length() {
        let pts = square(0.05);
        let opts = ExtrudeOptions {
            depth: 0.02,
            bevel: 0.0,
        };
        let [front, back, _] = extrude_polygon(&pts, &opts).unwrap();
        for n in front.normals.iter().chain(back.normals.iter()) {
            let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
            assert!((len - 1.0).abs() < 1e-5, "non-unit normal: {len}");
        }
    }

    // ─────────────────────────────────────────────────────────────────────
    // B-Rep path tests — the new canonical pipeline.
    // ─────────────────────────────────────────────────────────────────────

    /// Helper: 100 × 50 mm rectangle in metres, CCW from +Z.
    fn rect_100x50() -> Vec<Point2> {
        // 100 mm along X, 50 mm along Y → in metres: 0.10 × 0.05
        vec![
            Point2::new(0.00, 0.00),
            Point2::new(0.10, 0.00),
            Point2::new(0.10, 0.05),
            Point2::new(0.00, 0.05),
        ]
    }

    fn bbox_extents_old(parts: &[MeshPart; 3]) -> [f64; 3] {
        let mut mn = [f64::INFINITY; 3];
        let mut mx = [f64::NEG_INFINITY; 3];
        for part in parts {
            for v in &part.vertices {
                for k in 0..3 {
                    if v[k] < mn[k] { mn[k] = v[k]; }
                    if v[k] > mx[k] { mx[k] = v[k]; }
                }
            }
        }
        [mx[0] - mn[0], mx[1] - mn[1], mx[2] - mn[2]]
    }

    fn bbox_extents_new(m: &crate::mesh::Mesh) -> [f64; 3] {
        let mut mn = [f64::INFINITY; 3];
        let mut mx = [f64::NEG_INFINITY; 3];
        for v in &m.vertices {
            for k in 0..3 {
                if v[k] < mn[k] { mn[k] = v[k]; }
                if v[k] > mx[k] { mx[k] = v[k]; }
            }
        }
        [mx[0] - mn[0], mx[1] - mn[1], mx[2] - mn[2]]
    }

    /// TEST 1 — old `[MeshPart;3]` and new `MeshWithMetadata` produce the
    /// same bounding-box extents for a 100 × 50 mm rectangle extruded by 18 mm.
    #[test]
    fn test1_old_and_new_bbox_match() {
        let pts = rect_100x50();
        let opts = ExtrudeOptions { depth: 0.018, bevel: 0.0 };

        let old = extrude_polygon(&pts, &opts).unwrap();
        let new = extrude_polygon_brep(&pts, &opts).unwrap();

        let e_old = bbox_extents_old(&old);
        let e_new = bbox_extents_new(&new.mesh.mesh);

        let eps = 1e-9;
        for k in 0..3 {
            assert!(
                (e_old[k] - e_new[k]).abs() < eps,
                "axis {} extent mismatch: old={} new={}",
                k, e_old[k], e_new[k],
            );
        }
        // And it really is 100 × 50 × 18 mm:
        assert!((e_new[0] - 0.100).abs() < eps, "X extent: {}", e_new[0]);
        assert!((e_new[1] - 0.050).abs() < eps, "Y extent: {}", e_new[1]);
        assert!((e_new[2] - 0.018).abs() < eps, "Z (depth) extent: {}", e_new[2]);
    }

    /// TEST 2 — box topology counts (already in builder.rs, replicated here
    /// against the public extrude API to guarantee the end-to-end pipeline
    /// preserves them).
    #[test]
    fn test2_box_topology_counts() {
        let pts = rect_100x50();
        let opts = ExtrudeOptions { depth: 0.018, bevel: 0.0 };
        let r = extrude_polygon_brep(&pts, &opts).unwrap();
        let c = r.model.store.entity_counts();

        assert_eq!(c.vertices, 8,  "V");
        assert_eq!(c.edges,    12, "E");
        assert_eq!(c.coedges,  24, "CE");
        assert_eq!(c.loops,    6,  "L");
        assert_eq!(c.faces,    6,  "F");
        assert_eq!(c.shells,   1,  "Shell");
        assert_eq!(c.solids,   1,  "Solid");
        assert_eq!(c.bodies,   1,  "Body");
    }

    /// TEST 3 — every edge of a closed solid must be referenced by exactly
    /// two co-edges (= 2-manifold). 1 ⇒ open shell, 3+ ⇒ non-manifold.
    #[test]
    fn test3_every_edge_has_two_coedges() {
        let pts = rect_100x50();
        let opts = ExtrudeOptions { depth: 0.018, bevel: 0.0 };
        let r = extrude_polygon_brep(&pts, &opts).unwrap();
        let store = &r.model.store;

        let mut usage: std::collections::HashMap<_, usize> = Default::default();
        for ce in store.coedges.values() {
            *usage.entry(ce.edge).or_insert(0) += 1;
        }

        assert_eq!(usage.len(), 12, "expected 12 distinct edges, got {}", usage.len());
        for (eid, n) in &usage {
            assert_eq!(*n, 2, "edge {:?} referenced by {} co-edges (expected 2)", eid, n);
        }
    }

    /// TEST 4 — triangle metadata count. Box ⇒ 12 triangles; one TriangleMeta
    /// per triangle; every face_id resolves to a real Face in the store.
    #[test]
    fn test4_triangle_metadata() {
        let pts = rect_100x50();
        let opts = ExtrudeOptions { depth: 0.018, bevel: 0.0 };
        let r = extrude_polygon_brep(&pts, &opts).unwrap();

        let tri_count = r.mesh.triangle_count();
        let meta_count = r.mesh.triangles.len();

        assert_eq!(tri_count, 12, "expected 12 triangles for box, got {}", tri_count);
        assert_eq!(meta_count, tri_count, "metadata length mismatch");

        let store = &r.model.store;
        for (i, meta) in r.mesh.triangles.iter().enumerate() {
            assert!(
                store.get_face(meta.face_id).is_some(),
                "triangle #{} references missing face {:?}",
                i, meta.face_id,
            );
        }

        // And every B-Rep face contributed ≥1 triangle.
        let by_face = r.mesh.triangles_by_face();
        assert_eq!(by_face.len(), 6, "expected 6 faces in metadata, got {}", by_face.len());
        for (fid, tris) in &by_face {
            assert!(!tris.is_empty(), "face {:?} produced 0 triangles", fid);
        }
    }
}

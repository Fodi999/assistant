//! High-level B-Rep tessellator.
//!
//! Walks the B-Rep model and emits a triangle [`Mesh`] annotated with the
//! [`FaceId`] that produced each triangle.
//!
//! Current implementation is a **planar fan** triangulator: every face is
//! assumed to be planar, and its outer loop is walked once to gather the
//! ordered polygon vertices. This is sufficient for prismatic / boxed
//! geometry produced by [`crate::brep::builder::BrepBuilder::prism_from_polygon`].
//!
//! Curved-surface support (cylinder, sphere, NURBS) will be added later by
//! delegating to `face_tessellator::tessellate_face` per surface type.
#![allow(dead_code, unused_variables, unused_imports)]

use crate::brep::BrepModel;
use crate::math::{Point3, Real};
use crate::mesh::{Material, Mesh};
use crate::topology::{BodyId, CoEdgeId, FaceId};
use super::mesh_with_metadata::{MeshWithMetadata, TriangleMeta};
use super::options::TessOptions;

/// Tessellate every face in every solid of `body_id` into a single mesh
/// with per-triangle [`FaceId`] metadata.
pub fn tessellate_body(
    model: &BrepModel,
    body_id: BodyId,
    _opts: &TessOptions,
) -> MeshWithMetadata {
    let store = &model.store;
    let body = match store.get_body(body_id) {
        Some(b) => b,
        None => return empty_mesh_with_metadata(),
    };

    let mut vertices: Vec<[Real; 3]> = Vec::new();
    let mut normals:  Vec<[Real; 3]> = Vec::new();
    let mut uvs:      Vec<[Real; 2]> = Vec::new();
    let mut faces:    Vec<[usize; 3]> = Vec::new();
    let mut triangles: Vec<TriangleMeta> = Vec::new();

    for &solid_id in &body.solids {
        let Some(solid) = store.get_solid(solid_id) else { continue };
        let shells = std::iter::once(solid.outer).chain(solid.cavities.iter().copied());
        for shell_id in shells {
            let Some(shell) = store.get_shell(shell_id) else { continue };
            for &face_id in &shell.faces {
                tessellate_face_planar(
                    model,
                    face_id,
                    &mut vertices,
                    &mut normals,
                    &mut uvs,
                    &mut faces,
                    &mut triangles,
                );
            }
        }
    }

    let mesh = Mesh::new(vertices, normals, uvs, faces, Material::default());
    MeshWithMetadata { mesh, triangles }
}

/// Compatibility shim — older callers used `tessellate(model, opts)` to mean
/// "tessellate the first body". We keep that, but real callers should use
/// [`tessellate_body`].
pub fn tessellate(model: &BrepModel, opts: &TessOptions) -> Mesh {
    if let Some(&body_id) = model.store.bodies.keys().next() {
        tessellate_body(model, body_id, opts).mesh
    } else {
        Mesh::new(Vec::new(), Vec::new(), Vec::new(), Vec::new(), Material::default())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Planar fan triangulation
// ─────────────────────────────────────────────────────────────────────────────

fn tessellate_face_planar(
    model: &BrepModel,
    face_id: FaceId,
    vertices:  &mut Vec<[Real; 3]>,
    normals:   &mut Vec<[Real; 3]>,
    uvs:       &mut Vec<[Real; 2]>,
    faces:     &mut Vec<[usize; 3]>,
    triangles: &mut Vec<TriangleMeta>,
) {
    let store = &model.store;
    let Some(face) = store.get_face(face_id) else { return };
    let Some(outer_loop) = store.get_loop(face.outer_loop) else { return };

    // 1. Walk the outer loop's co-edges in order; collect the *start vertex*
    //    of each co-edge → ordered polygon contour.
    let mut polygon: Vec<Point3> = Vec::with_capacity(outer_loop.coedges.len());
    for &ce_id in &outer_loop.coedges {
        let Some(ce) = store.get_coedge(ce_id) else { continue };
        let Some(edge) = store.get_edge(ce.edge) else { continue };
        let v_id = if ce.reversed { edge.end } else { edge.start };
        let Some(v) = store.get_vertex(v_id) else { continue };
        polygon.push(v.point);
    }
    if polygon.len() < 3 { return; }

    // 2. Compute polygon normal via Newell's method (robust for non-strictly
    //    convex polygons and arbitrary planes).
    let normal = newell_normal(&polygon);
    if normal.is_none() { return; } // degenerate

    let mut n = normal.unwrap();
    if !face.orientation {
        n = [-n[0], -n[1], -n[2]];
    }

    // 3. Project to 2D for UVs (simple bbox normalisation in the polygon plane).
    let (min_p, max_p) = polygon.iter().fold(
        ([f64::INFINITY; 3], [f64::NEG_INFINITY; 3]),
        |(mn, mx), p| (
            [mn[0].min(p.x), mn[1].min(p.y), mn[2].min(p.z)],
            [mx[0].max(p.x), mx[1].max(p.y), mx[2].max(p.z)],
        ),
    );
    let range = [
        (max_p[0] - min_p[0]).max(1e-9),
        (max_p[1] - min_p[1]).max(1e-9),
        (max_p[2] - min_p[2]).max(1e-9),
    ];

    // 4. Append vertices/normals/uvs.
    let base = vertices.len();
    for p in &polygon {
        vertices.push([p.x, p.y, p.z]);
        normals.push(n);
        // Cheap planar UV: drop the dominant-normal axis.
        let an = [n[0].abs(), n[1].abs(), n[2].abs()];
        let (u_axis, v_axis) = if an[2] >= an[0] && an[2] >= an[1] {
            (0usize, 1usize) // normal ≈ ±Z → UV from XY
        } else if an[1] >= an[0] {
            (0, 2)           // normal ≈ ±Y → UV from XZ
        } else {
            (1, 2)           // normal ≈ ±X → UV from YZ
        };
        let pa = [p.x, p.y, p.z];
        let u = (pa[u_axis] - min_p[u_axis]) / range[u_axis];
        let v = (pa[v_axis] - min_p[v_axis]) / range[v_axis];
        uvs.push([u, v]);
    }

    // 5. Fan triangulation (face is assumed planar and convex-enough).
    // If the face is reversed we flip the fan winding so the outward normal
    // is preserved.
    let m = polygon.len();
    for i in 1..(m - 1) {
        let tri = if face.orientation {
            [base, base + i, base + i + 1]
        } else {
            [base, base + i + 1, base + i]
        };
        faces.push(tri);
        triangles.push(TriangleMeta { face_id });
    }
}

/// Newell's method — robust polygon normal that does not fail on
/// near-degenerate triangles.
fn newell_normal(poly: &[Point3]) -> Option<[Real; 3]> {
    let n = poly.len();
    let mut nx = 0.0; let mut ny = 0.0; let mut nz = 0.0;
    for i in 0..n {
        let a = poly[i];
        let b = poly[(i + 1) % n];
        nx += (a.y - b.y) * (a.z + b.z);
        ny += (a.z - b.z) * (a.x + b.x);
        nz += (a.x - b.x) * (a.y + b.y);
    }
    let len = (nx * nx + ny * ny + nz * nz).sqrt();
    if len < 1e-12 { return None; }
    Some([nx / len, ny / len, nz / len])
}

fn empty_mesh_with_metadata() -> MeshWithMetadata {
    MeshWithMetadata {
        mesh: Mesh::new(vec![], vec![], vec![], vec![], Material::default()),
        triangles: Vec::new(),
    }
}



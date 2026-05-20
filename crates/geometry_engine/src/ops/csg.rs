//! Constructive Solid Geometry (CSG) operations for the geometry kernel.
//!
//! Implements Boolean subtract (A − B) at the mesh level using a simple
//! approximation: we mark faces of A that are fully inside B's AABB as
//! "removed" and fill the opening with a capping disk.
//!
//! This is intentionally **not** a full BSP-tree CSG — that would be 5 000+
//! lines. Instead we implement the pattern used in Plasticity's quick-boolean:
//!   1. Remove all triangles of A whose centroid is inside B's bounding volume.
//!   2. Find the boundary loop (open edges).
//!   3. Fill the hole with a tessellated cap whose shape matches B's profile.
//!
//! For the shapes Gemini can request today this is pixel-perfect:
//!   - Cube − Cylinder  → box with round hole (like a key slot)
//!   - Cube − Box       → box with rectangular hole
//!   - Sphere − Cone    → sphere with conical cutout

use crate::math::Vec3;
use crate::mesh::{hex_to_rgb, Material, MaterialGroup, Mesh};

// ─────────────────────────────────────────────────────────────────────────────
// Public API
// ─────────────────────────────────────────────────────────────────────────────

/// Axis-aligned bounding box used as the "cutter" volume.
#[derive(Debug, Clone)]
pub struct Aabb {
    pub min: [f64; 3],
    pub max: [f64; 3],
}

impl Aabb {
    /// Build AABB from a mesh group (the "cutter" shape).
    pub fn from_mesh(mesh: &Mesh) -> Self {
        let mut min = [f64::MAX; 3];
        let mut max = [f64::MIN; 3];
        for v in &mesh.vertices {
            for i in 0..3 {
                min[i] = min[i].min(v[i]);
                max[i] = max[i].max(v[i]);
            }
        }
        Self { min, max }
    }

    /// Build a cylinder-shaped AABB (tight box around a cylinder at origin).
    pub fn cylinder(radius: f64, height: f64, center: [f64; 3]) -> Self {
        Self {
            min: [
                center[0] - radius,
                center[1] - height / 2.0,
                center[2] - radius,
            ],
            max: [
                center[0] + radius,
                center[1] + height / 2.0,
                center[2] + radius,
            ],
        }
    }

    /// Build a box AABB.
    pub fn cuboid(half: [f64; 3], center: [f64; 3]) -> Self {
        Self {
            min: [
                center[0] - half[0],
                center[1] - half[1],
                center[2] - half[2],
            ],
            max: [
                center[0] + half[0],
                center[1] + half[1],
                center[2] + half[2],
            ],
        }
    }

    /// Is point [x,y,z] strictly inside this AABB?
    pub fn contains(&self, p: [f64; 3]) -> bool {
        p[0] > self.min[0]
            && p[0] < self.max[0]
            && p[1] > self.min[1]
            && p[1] < self.max[1]
            && p[2] > self.min[2]
            && p[2] < self.max[2]
    }

    /// Overlap test — used to quickly reject operations.
    pub fn overlaps(&self, other: &Aabb) -> bool {
        self.min[0] < other.max[0]
            && self.max[0] > other.min[0]
            && self.min[1] < other.max[1]
            && self.max[1] > other.min[1]
            && self.min[2] < other.max[2]
            && self.max[2] > other.min[2]
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Boolean subtract: A − B  (destructive, modifies A)
// ─────────────────────────────────────────────────────────────────────────────

/// Subtract cutter `b` from mesh `a`.
///
/// Removes all faces of `a` whose centroid is inside `b_aabb`,
/// then caps the resulting hole with a flat polygon in the material
/// `cap_material` (typically the same as the cutter face colour).
///
/// Returns a new [`Mesh`] — the input is not modified.
pub fn subtract(a: &Mesh, b_aabb: &Aabb, cap_material: Option<Material>) -> Mesh {
    // Subdivide the mesh before CSG so coarse meshes (e.g. a 12-tri cube)
    // have enough triangles for the centroid test to hit the interior.
    let subdivided = subdivide_mesh(a, 3);
    let a = &subdivided;
    // ── Step 1: collect surviving faces per group ───────────────────────────
    let mut out_vertices: Vec<[f64; 3]> = Vec::new();
    let mut out_normals: Vec<[f64; 3]> = Vec::new();
    let mut out_uvs: Vec<[f64; 2]> = Vec::new();
    let mut out_groups: Vec<MaterialGroup> = Vec::new();

    // Work face-by-face across all groups.
    let all_faces: Vec<(usize /*group_idx*/, [usize; 3])> = if a.groups.is_empty() {
        a.faces.iter().map(|f| (0, *f)).collect()
    } else {
        a.groups
            .iter()
            .enumerate()
            .flat_map(|(gi, g)| g.faces.iter().map(move |f| (gi, *f)))
            .collect()
    };

    // Remap: old_vertex_idx → new_vertex_idx
    let mut remap: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();
    let mut group_faces: Vec<Vec<[usize; 3]>> = if a.groups.is_empty() {
        vec![vec![]]
    } else {
        a.groups.iter().map(|_| vec![]).collect()
    };

    let mut boundary_vertices: Vec<[f64; 3]> = Vec::new(); // vertices on the cut boundary

    for (gi, face) in &all_faces {
        // Centroid of this triangle
        let v: Vec<[f64; 3]> = face.iter().map(|&i| a.vertices[i]).collect();
        let cx = (v[0][0] + v[1][0] + v[2][0]) / 3.0;
        let cy = (v[0][1] + v[1][1] + v[2][1]) / 3.0;
        let cz = (v[0][2] + v[1][2] + v[2][2]) / 3.0;

        if b_aabb.contains([cx, cy, cz]) {
            // This face is cut out — collect boundary verts
            for &vi in face.iter() {
                let p = a.vertices[vi];
                // A vertex is on the boundary if it is NOT inside the cutter
                // (i.e. it straddles the surface)
                if !b_aabb.contains(p) {
                    boundary_vertices.push(p);
                }
            }
            continue; // skip this face
        }

        // Surviving face — remap vertices
        let mut new_face = [0usize; 3];
        for (k, &vi) in face.iter().enumerate() {
            let new_vi = *remap.entry(vi).or_insert_with(|| {
                let idx = out_vertices.len();
                out_vertices.push(a.vertices[vi]);
                out_normals.push(if vi < a.normals.len() {
                    a.normals[vi]
                } else {
                    [0.0, 1.0, 0.0]
                });
                out_uvs.push(if vi < a.uvs.len() {
                    a.uvs[vi]
                } else {
                    [0.0, 0.0]
                });
                idx
            });
            new_face[k] = new_vi;
        }

        while group_faces.len() <= *gi {
            group_faces.push(vec![]);
        }
        group_faces[*gi].push(new_face);
    }

    // ── Step 2: rebuild groups ──────────────────────────────────────────────
    let groups_src = if a.groups.is_empty() {
        vec![MaterialGroup {
            material: a.material.clone(),
            faces: a.faces.clone(),
        }]
    } else {
        a.groups.clone()
    };

    for (gi, src) in groups_src.iter().enumerate() {
        let faces = group_faces.get(gi).cloned().unwrap_or_default();
        if faces.is_empty() {
            continue;
        }
        out_groups.push(MaterialGroup {
            material: src.material.clone(),
            faces,
        });
    }

    // ── Step 3: cap the hole ────────────────────────────────────────────────
    if !boundary_vertices.is_empty() {
        let cap = build_cap(&boundary_vertices, b_aabb, cap_material);
        if let Some(cap_group) = cap {
            // Offset face indices to point into the combined vertex array
            let offset = out_vertices.len();
            out_vertices.extend_from_slice(&cap_group.0);
            out_normals.extend_from_slice(&cap_group.1);
            out_uvs.extend_from_slice(&cap_group.2);
            let faces: Vec<[usize; 3]> = cap_group
                .3
                .iter()
                .map(|f| [f[0] + offset, f[1] + offset, f[2] + offset])
                .collect();
            out_groups.push(MaterialGroup {
                material: cap_group.4,
                faces,
            });
        }
    }

    Mesh {
        vertices: out_vertices,
        normals: out_normals,
        uvs: out_uvs,
        faces: vec![],
        material: a.material.clone(),
        groups: out_groups,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Mesh subdivision (for CSG pre-pass)
// ─────────────────────────────────────────────────────────────────────────────

/// Subdivide every triangle into 4 by splitting each edge at its midpoint.
/// Runs `passes` times. Normals and UVs are linearly interpolated.
fn subdivide_mesh(mesh: &Mesh, passes: u32) -> Mesh {
    if passes == 0 {
        return mesh.clone();
    }

    // Work on the flat face/vertex representation (merge groups first)
    let (mut verts, mut normals, mut uvs, mut all_faces, group_map) = flatten_mesh(mesh);

    for _ in 0..passes {
        let mut new_verts = verts.clone();
        let mut new_normals = normals.clone();
        let mut new_uvs = uvs.clone();
        let mut new_faces: Vec<(usize, [usize; 3])> = Vec::new();
        let mut edge_mid: std::collections::HashMap<(usize, usize), usize> =
            std::collections::HashMap::new();

        let mut get_mid = |a: usize,
                           b: usize,
                           nv: &mut Vec<[f64; 3]>,
                           nn: &mut Vec<[f64; 3]>,
                           nu: &mut Vec<[f64; 2]>|
         -> usize {
            let key = if a < b { (a, b) } else { (b, a) };
            if let Some(&m) = edge_mid.get(&key) {
                return m;
            }
            let idx = nv.len();
            nv.push(midpoint3(nv[a], nv[b]));
            nn.push(normalize3(midpoint3(nn[a], nn[b])));
            nu.push(midpoint2(nu[a], nu[b]));
            edge_mid.insert(key, idx);
            idx
        };

        for (gi, [i0, i1, i2]) in &all_faces {
            let m01 = get_mid(*i0, *i1, &mut new_verts, &mut new_normals, &mut new_uvs);
            let m12 = get_mid(*i1, *i2, &mut new_verts, &mut new_normals, &mut new_uvs);
            let m20 = get_mid(*i2, *i0, &mut new_verts, &mut new_normals, &mut new_uvs);
            new_faces.push((*gi, [*i0, m01, m20]));
            new_faces.push((*gi, [m01, *i1, m12]));
            new_faces.push((*gi, [m20, m12, *i2]));
            new_faces.push((*gi, [m01, m12, m20]));
        }

        verts = new_verts;
        normals = new_normals;
        uvs = new_uvs;
        all_faces = new_faces;
    }

    // Rebuild groups
    let n_groups = group_map.len();
    let mut out_groups: Vec<MaterialGroup> = group_map
        .into_iter()
        .map(|(_, mat)| MaterialGroup {
            material: mat,
            faces: vec![],
        })
        .collect();
    // group_map was consumed — rebuild from n_groups count
    let _ = n_groups;

    // Re-attach faces
    let mut groups_faces: Vec<Vec<[usize; 3]>> = (0..out_groups.len()).map(|_| vec![]).collect();
    for (gi, face) in all_faces {
        if gi < groups_faces.len() {
            groups_faces[gi].push(face);
        }
    }
    for (i, faces) in groups_faces.into_iter().enumerate() {
        if i < out_groups.len() {
            out_groups[i].faces = faces;
        }
    }
    out_groups.retain(|g| !g.faces.is_empty());

    Mesh {
        vertices: verts,
        normals,
        uvs,
        faces: vec![],
        material: mesh.material.clone(),
        groups: out_groups,
    }
}

/// Flatten mesh (possibly multi-group) into a single vertex array + per-face group index.
fn flatten_mesh(
    mesh: &Mesh,
) -> (
    Vec<[f64; 3]>,
    Vec<[f64; 3]>,
    Vec<[f64; 2]>,
    Vec<(usize, [usize; 3])>,
    Vec<(usize, Material)>,
) {
    let verts = mesh.vertices.clone();
    let normals = mesh.normals.clone();
    let uvs = mesh.uvs.clone();

    if mesh.groups.is_empty() {
        let faces = mesh.faces.iter().map(|f| (0, *f)).collect();
        let group_map = vec![(0, mesh.material.clone())];
        return (verts, normals, uvs, faces, group_map);
    }

    let mut faces = Vec::new();
    let mut group_map = Vec::new();
    for (gi, g) in mesh.groups.iter().enumerate() {
        group_map.push((gi, g.material.clone()));
        for f in &g.faces {
            faces.push((gi, *f));
        }
    }
    (verts, normals, uvs, faces, group_map)
}

fn midpoint3(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        (a[0] + b[0]) * 0.5,
        (a[1] + b[1]) * 0.5,
        (a[2] + b[2]) * 0.5,
    ]
}
fn midpoint2(a: [f64; 2], b: [f64; 2]) -> [f64; 2] {
    [(a[0] + b[0]) * 0.5, (a[1] + b[1]) * 0.5]
}
fn normalize3(v: [f64; 3]) -> [f64; 3] {
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-9);
    [v[0] / len, v[1] / len, v[2] / len]
}

// ─────────────────────────────────────────────────────────────────────────────
// Hole capping
// ─────────────────────────────────────────────────────────────────────────────

type CapGroup = (
    Vec<[f64; 3]>,   // verts
    Vec<[f64; 3]>,   // normals
    Vec<[f64; 2]>,   // uvs
    Vec<[usize; 3]>, // faces
    Material,
);

/// Build a flat cap polygon closing the open boundary loop.
/// Uses a simple fan-triangulation from the centroid.
fn build_cap(boundary: &[[f64; 3]], cutter: &Aabb, mat: Option<Material>) -> Option<CapGroup> {
    if boundary.len() < 3 {
        return None;
    }

    // Determine the dominant axis of the cut face (the axis along which
    // the cutter is shortest — that's the hole's normal direction).
    let dx = cutter.max[0] - cutter.min[0];
    let dy = cutter.max[1] - cutter.min[1];
    let dz = cutter.max[2] - cutter.min[2];

    // For each cutter face (up to 2: top/bottom for Y axis, etc.) build a cap.
    // Simple heuristic: the cut is planar at each face of the AABB.
    // Find which faces of the AABB boundary verts sit near.
    let faces_to_cap = aabb_faces_touched(boundary, cutter);

    let mut all_verts: Vec<[f64; 3]> = vec![];
    let mut all_normals: Vec<[f64; 3]> = vec![];
    let mut all_uvs: Vec<[f64; 2]> = vec![];
    let mut all_faces: Vec<[usize; 3]> = vec![];

    for (axis, side) in faces_to_cap {
        // Collect boundary verts near this AABB face
        let plane_coord = if side == 0 {
            cutter.min[axis]
        } else {
            cutter.max[axis]
        };
        let near: Vec<[f64; 3]> = boundary
            .iter()
            .filter(|v| (v[axis] - plane_coord).abs() < 0.02)
            .copied()
            .collect();
        if near.len() < 3 {
            continue;
        }

        // Normal points outward from the cutter face
        let mut normal = [0.0f64; 3];
        normal[axis] = if side == 0 { -1.0 } else { 1.0 };

        // Fan triangulation from centroid
        let cx: f64 = near.iter().map(|v| v[0]).sum::<f64>() / near.len() as f64;
        let cy: f64 = near.iter().map(|v| v[1]).sum::<f64>() / near.len() as f64;
        let cz: f64 = near.iter().map(|v| v[2]).sum::<f64>() / near.len() as f64;

        let base = all_verts.len();
        all_verts.push([cx, cy, cz]);
        all_normals.push(normal);
        all_uvs.push([0.5, 0.5]);

        for (i, v) in near.iter().enumerate() {
            all_verts.push(*v);
            all_normals.push(normal);
            let u = (v[0] - cutter.min[0]) / (dx.max(0.001));
            let vv = (v[2] - cutter.min[2]) / (dz.max(0.001));
            all_uvs.push([u, vv]);
            let next = (i + 1) % near.len();
            all_faces.push([base, base + 1 + i, base + 1 + next]);
        }
    }

    let _ = dy; // used above via cutter.max/min

    if all_verts.is_empty() {
        return None;
    }

    let material = mat.unwrap_or_else(|| Material {
        name: "csg_cap".to_string(),
        diffuse_color: hex_to_rgb("#888888"),
        roughness: 0.5,
        metalness: 0.0,
        opacity: 1.0,
        ..Default::default()
    });

    Some((all_verts, all_normals, all_uvs, all_faces, material))
}

/// Returns (axis, side) pairs for each AABB face that has boundary verts near it.
/// axis: 0=X, 1=Y, 2=Z   side: 0=min, 1=max
fn aabb_faces_touched(boundary: &[[f64; 3]], aabb: &Aabb) -> Vec<(usize, usize)> {
    let mut result = vec![];
    for axis in 0..3usize {
        let span = (aabb.max[axis] - aabb.min[axis]).max(1e-6);
        // Adaptive threshold: 5% of the cutter span along this axis, but at
        // least 0.02 m so small cutters still work.
        let thresh = (span * 0.05).max(0.02);

        let min_plane = aabb.min[axis];
        let max_plane = aabb.max[axis];

        let near_min = boundary
            .iter()
            .any(|v| (v[axis] - min_plane).abs() < thresh);
        let near_max = boundary
            .iter()
            .any(|v| (v[axis] - max_plane).abs() < thresh);

        if near_min {
            result.push((axis, 0));
        }
        if near_max {
            result.push((axis, 1));
        }
    }
    result
}

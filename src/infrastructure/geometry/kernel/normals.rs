//! Smooth-normal recomputation.
//!
//! Replaces every per-vertex normal with the area-weighted average of the
//! face normals that touch that vertex. Use after stitching together
//! multiple `MeshPart`s when seams need to disappear (e.g. body + neck of a
//! bottle that share vertices on the join ring).
//!
//! This pass is **not** required for `lathe_profile`-only meshes — its
//! analytic normals are already smooth.

use crate::infrastructure::geometry::mesh::Mesh;

use super::math::Vec3;

/// Recompute per-vertex normals as the area-weighted average of incident
/// face normals. After this call, every normal in `mesh.normals` is unit
/// length (or `Vec3::UP` if a vertex is unreferenced).
pub fn recalculate_smooth_normals(mesh: &mut Mesh) {
    let v = &mesh.vertices;
    if v.is_empty() {
        return;
    }
    let mut accum: Vec<Vec3> = vec![Vec3::ZERO; v.len()];

    // Walk every face from every group (the legacy `mesh.faces` is
    // mirrored from groups[0] in `Mesh::new_multi`, so iterating `groups`
    // is the canonical path).
    let groups: Vec<&Vec<[usize; 3]>> = if mesh.groups.is_empty() {
        vec![&mesh.faces]
    } else {
        mesh.groups.iter().map(|g| &g.faces).collect()
    };

    for faces in groups {
        for [a, b, c] in faces {
            let pa = Vec3::from_array(v[*a]);
            let pb = Vec3::from_array(v[*b]);
            let pc = Vec3::from_array(v[*c]);
            // Cross product of edges → normal whose length is 2 × area.
            // Using the unnormalised cross weights big triangles more,
            // which is exactly what we want.
            let face_n = (pb - pa).cross(pc - pa);
            accum[*a] = accum[*a] + face_n;
            accum[*b] = accum[*b] + face_n;
            accum[*c] = accum[*c] + face_n;
        }
    }

    for (i, n) in accum.iter().enumerate() {
        mesh.normals[i] = n.normalized().to_array();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::geometry::kernel::lathe::lathe_profile;
    use crate::infrastructure::geometry::kernel::mesh_builder::MeshBuilder;
    use crate::infrastructure::geometry::kernel::profile::{Profile, ProfilePoint};
    use crate::infrastructure::geometry::mesh::Material;

    fn lathe_to_mesh(p: &Profile, segments: usize) -> crate::infrastructure::geometry::mesh::Mesh {
        let part = lathe_profile(p, segments).unwrap();
        let mut b = MeshBuilder::new();
        let g = b.add_group(Material::solid("test", [1.0, 1.0, 1.0]));
        let mut idx_map = Vec::with_capacity(part.vertices.len());
        for i in 0..part.vertices.len() {
            idx_map.push(b.add_vertex(part.vertices[i], part.normals[i], part.uvs[i]));
        }
        for [a, bb, c] in part.faces {
            b.add_triangle(g, idx_map[a], idx_map[bb], idx_map[c]);
        }
        b.build()
    }

    #[test]
    fn smooth_normals_are_unit_length() {
        let p = Profile::new(vec![
            ProfilePoint::new(0.02, 0.0),
            ProfilePoint::new(0.05, 0.05),
            ProfilePoint::new(0.05, 0.10),
        ])
        .unwrap();
        let mut mesh = lathe_to_mesh(&p, 12);
        recalculate_smooth_normals(&mut mesh);
        for n in &mesh.normals {
            let l = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
            assert!((l - 1.0).abs() < 1e-4 || (l - 1.0).abs() < 1e-3, "n_len={l}");
        }
    }

    #[test]
    fn smooth_normals_on_cylinder_point_outward() {
        let p = Profile::new(vec![
            ProfilePoint::new(0.05, 0.0),
            ProfilePoint::new(0.05, 0.10),
        ])
        .unwrap();
        let mut mesh = lathe_to_mesh(&p, 24);
        recalculate_smooth_normals(&mut mesh);

        for (i, v) in mesh.vertices.iter().enumerate() {
            let r = (v[0] * v[0] + v[2] * v[2]).sqrt().max(1e-6);
            let outward = [v[0] / r, 0.0_f32, v[2] / r];
            let n = mesh.normals[i];
            let dot = n[0] * outward[0] + n[1] * outward[1] + n[2] * outward[2];
            // Smooth pass on a closed cylinder ring should still point outward.
            assert!(dot > 0.95, "smoothed normal drifted: dot={dot}");
        }
    }

    #[test]
    fn smooth_normals_handle_empty_mesh_gracefully() {
        let mut mesh = crate::infrastructure::geometry::mesh::Mesh::new(
            vec![],
            vec![],
            vec![],
            vec![],
            Material::solid("x", [1.0, 1.0, 1.0]),
        );
        // Should not panic.
        recalculate_smooth_normals(&mut mesh);
    }
}

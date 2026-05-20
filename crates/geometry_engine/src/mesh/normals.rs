//! Пересчёт сглаженных нормалей — area-weighted average.

use crate::mesh::Mesh;
use crate::math::Vec3;

/// Заменяет каждую per-vertex normal на area-weighted среднее нормалей граней.
pub fn recalculate_smooth_normals(mesh: &mut Mesh) {
    let vc = mesh.vertices.len();
    if vc == 0 { return; }

    let mut acc = vec![[0.0_f32; 3]; vc];

    let faces: Vec<[usize; 3]> = if mesh.groups.is_empty() {
        mesh.faces.clone()
    } else {
        mesh.groups.iter().flat_map(|g| g.faces.iter().copied()).collect()
    };

    for [a, b, c] in &faces {
        let pa = Vec3::from_array(mesh.vertices[*a]);
        let pb = Vec3::from_array(mesh.vertices[*b]);
        let pc = Vec3::from_array(mesh.vertices[*c]);
        let n  = (pb - pa).cross(pc - pa); // area-weighted, no normalize yet
        for &idx in &[*a, *b, *c] {
            acc[idx][0] += n.x;
            acc[idx][1] += n.y;
            acc[idx][2] += n.z;
        }
    }

    for (i, n) in acc.iter().enumerate() {
        let v = Vec3::from_array(*n).normalized();
        mesh.normals[i] = v.to_array();
    }
}

//! Vertex welding — merge duplicate vertices within epsilon distance.
//! Uses Real (f64) for precise distance comparison.

use crate::math::{Real, tolerance::WELD_EPS};
use crate::mesh::Mesh;

/// Merge vertices closer than `eps` metres. Updates face indices.
/// Call after stitching MeshParts when seam verts are numerically almost-equal.
pub fn weld_vertices(mesh: &mut Mesh, eps: Option<Real>) {
    let eps = eps.unwrap_or(WELD_EPS);
    let vc = mesh.vertices.len();
    if vc == 0 { return; }

    let mut remap: Vec<usize> = (0..vc).collect();

    for i in 0..vc {
        for j in 0..i {
            if remap[j] != j { continue; }
            let pi = mesh.vertices[i];
            let pj = mesh.vertices[j];
            let dx = pi[0]-pj[0]; let dy = pi[1]-pj[1]; let dz = pi[2]-pj[2];
            if (dx*dx + dy*dy + dz*dz).sqrt() < eps {
                remap[i] = j;
                break;
            }
        }
    }

    // Rebuild unique vertex list
    let mut new_verts: Vec<[Real; 3]> = Vec::new();
    let mut new_norms: Vec<[Real; 3]> = Vec::new();
    let mut new_uvs:   Vec<[Real; 2]> = Vec::new();
    let mut compact:   Vec<Option<usize>> = vec![None; vc];

    for i in 0..vc {
        let r = remap[i];
        if compact[r].is_none() {
            compact[r] = Some(new_verts.len());
            new_verts.push(mesh.vertices[r]);
            new_norms.push(mesh.normals[r]);
            new_uvs.push(mesh.uvs[r]);
        }
    }

    // Remap face indices
    let remap_idx = |old: usize| compact[remap[old]].unwrap();

    if mesh.groups.is_empty() {
        for f in &mut mesh.faces {
            *f = [remap_idx(f[0]), remap_idx(f[1]), remap_idx(f[2])];
        }
    } else {
        for g in &mut mesh.groups {
            for f in &mut g.faces {
                *f = [remap_idx(f[0]), remap_idx(f[1]), remap_idx(f[2])];
            }
        }
    }

    mesh.vertices = new_verts;
    mesh.normals  = new_norms;
    mesh.uvs      = new_uvs;
}

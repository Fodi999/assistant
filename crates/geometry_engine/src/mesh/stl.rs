//! STL (binary) экспорт — минимальный, для совместимости.

use crate::mesh::Mesh;
use crate::math::Vec3;

/// Записывает бинарный STL (little-endian).
pub fn export_stl(mesh: &Mesh) -> Vec<u8> {
    let faces: Vec<[usize; 3]> = if mesh.groups.is_empty() {
        mesh.faces.clone()
    } else {
        mesh.groups.iter().flat_map(|g| g.faces.iter().copied()).collect()
    };

    let mut out = Vec::with_capacity(84 + faces.len() * 50);

    // 80-byte header
    let mut header = [0u8; 80];
    let title = b"geometry-engine STL";
    header[..title.len()].copy_from_slice(title);
    out.extend_from_slice(&header);

    // Triangle count (u32 LE)
    out.extend_from_slice(&(faces.len() as u32).to_le_bytes());

    for [a, b, c] in &faces {
        let pa = Vec3::from_array(mesh.vertices[*a]);
        let pb = Vec3::from_array(mesh.vertices[*b]);
        let pc = Vec3::from_array(mesh.vertices[*c]);
        let n  = (pb - pa).cross(pc - pa).normalized();

        // Normal
        for x in [n.x, n.y, n.z] { out.extend_from_slice(&x.to_le_bytes()); }
        // Vertices
        for v in [pa, pb, pc] {
            for x in [v.x, v.y, v.z] { out.extend_from_slice(&x.to_le_bytes()); }
        }
        // Attribute byte count
        out.extend_from_slice(&0u16.to_le_bytes());
    }

    out
}

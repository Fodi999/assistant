//! OBJ + MTL exporter.
//!
//! Returns a pair of byte strings: `(obj_bytes, mtl_bytes)`.
//! The OBJ file references the MTL via `mtllib model.mtl`.
//! Both files are meant to be stored alongside each other so that any
//! standard OBJ loader (Three.js `OBJLoader` + `MTLLoader`) works out of
//! the box.

use crate::infrastructure::geometry::mesh::Mesh;
use crate::shared::AppError;

/// Exported pair — ready to write to storage.
pub struct ObjExport {
    /// Contents of `model.obj`
    pub obj_bytes: Vec<u8>,
    /// Contents of `model.mtl`
    pub mtl_bytes: Vec<u8>,
}

/// Serialise `mesh` into OBJ + MTL.
///
/// The MTL file name embedded in the OBJ is always `"model.mtl"` — the
/// caller must store it under that name relative to the OBJ.
pub fn export_obj(mesh: &Mesh) -> Result<ObjExport, AppError> {
    let obj = build_obj(mesh);
    let mtl = build_mtl(mesh);
    Ok(ObjExport {
        obj_bytes: obj.into_bytes(),
        mtl_bytes: mtl.into_bytes(),
    })
}

// ─────────────────────────────────────────────────────────────────────────────

fn build_obj(mesh: &Mesh) -> String {
    let mut out = String::with_capacity(
        mesh.vertices.len() * 32 + mesh.faces.len() * 24 + 128,
    );

    out.push_str("# Laboratory v2 — procedural OBJ (PR #4)\n");
    out.push_str("mtllib model.mtl\n\n");

    // Vertices
    for [x, y, z] in &mesh.vertices {
        out.push_str(&format!("v {x:.6} {y:.6} {z:.6}\n"));
    }
    out.push('\n');

    // Texture coordinates
    for [u, v] in &mesh.uvs {
        out.push_str(&format!("vt {u:.6} {v:.6}\n"));
    }
    out.push('\n');

    // Normals
    for [nx, ny, nz] in &mesh.normals {
        out.push_str(&format!("vn {nx:.6} {ny:.6} {nz:.6}\n"));
    }
    out.push('\n');

    // Material + faces
    // OBJ indices are 1-based.
    out.push_str(&format!("usemtl {}\n", mesh.material.name));
    out.push_str("s 1\n");
    for [a, b, c] in &mesh.faces {
        let (a, b, c) = (a + 1, b + 1, c + 1);
        out.push_str(&format!("f {a}/{a}/{a} {b}/{b}/{b} {c}/{c}/{c}\n"));
    }

    out
}

fn build_mtl(mesh: &Mesh) -> String {
    let mat = &mesh.material;
    let [r, g, b] = mat.diffuse_color;
    let mut out = String::with_capacity(256);

    out.push_str("# Laboratory v2 — material\n");
    out.push_str(&format!("newmtl {}\n", mat.name));
    // Ambient = half diffuse for a pleasant look
    out.push_str(&format!("Ka {:.4} {:.4} {:.4}\n", r * 0.5, g * 0.5, b * 0.5));
    out.push_str(&format!("Kd {r:.4} {g:.4} {b:.4}\n"));
    // Specular — mild highlight
    out.push_str("Ks 0.1500 0.1500 0.1500\n");
    out.push_str("Ns 32.0\n");
    out.push_str("d 1.0\n");
    out.push_str("illum 2\n");

    if let Some(tex) = &mat.texture_file {
        out.push_str(&format!("map_Kd {tex}\n"));
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::geometry::mesh::{Material, Mesh};

    fn simple_triangle() -> Mesh {
        Mesh::new(
            vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            vec![[0.0, 0.0, 1.0]; 3],
            vec![[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]],
            vec![[0, 1, 2]],
            Material::solid("test_mat", [1.0, 0.0, 0.0]),
        )
    }

    #[test]
    fn obj_contains_vertices_and_face() {
        let mesh = simple_triangle();
        let export = export_obj(&mesh).unwrap();
        let obj = String::from_utf8(export.obj_bytes).unwrap();
        assert!(obj.contains("v 0.000000 0.000000 0.000000"));
        assert!(obj.contains("f 1/1/1 2/2/2 3/3/3"));
        assert!(obj.contains("mtllib model.mtl"));
        assert!(obj.contains("usemtl test_mat"));
    }

    #[test]
    fn mtl_contains_diffuse() {
        let mesh = simple_triangle();
        let export = export_obj(&mesh).unwrap();
        let mtl = String::from_utf8(export.mtl_bytes).unwrap();
        assert!(mtl.contains("newmtl test_mat"));
        assert!(mtl.contains("Kd 1.0000 0.0000 0.0000"));
    }
}

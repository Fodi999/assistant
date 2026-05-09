//! OBJ + MTL exporter.
//!
//! Returns a pair of byte strings: `(obj_bytes, mtl_bytes)`.
//! The OBJ file references the MTL via `mtllib model.mtl`.
//! Both files are meant to be stored alongside each other so that any
//! standard OBJ loader (Three.js `OBJLoader` + `MTLLoader`) works out of
//! the box.

use crate::infrastructure::geometry::mesh::{Material, Mesh};
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
///
/// If `mesh.groups` is non-empty, each group is written as its own
/// `usemtl` block in the OBJ and as a separate `newmtl` entry in the MTL
/// (PR #6 multi-material support, used by `sauce_in_bowl`).
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
    let face_count: usize = if mesh.groups.is_empty() {
        mesh.faces.len()
    } else {
        mesh.groups.iter().map(|g| g.faces.len()).sum()
    };
    let mut out = String::with_capacity(mesh.vertices.len() * 32 + face_count * 24 + 256);

    out.push_str("# Laboratory v2 — procedural OBJ\n");
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

    // Material + faces — multi-group path takes priority.
    out.push_str("s 1\n");
    if mesh.groups.is_empty() {
        out.push_str(&format!("usemtl {}\n", mesh.material.name));
        write_faces(&mut out, &mesh.faces);
    } else {
        for (idx, group) in mesh.groups.iter().enumerate() {
            // `g` is OBJ "group name" — purely cosmetic but helps debugging.
            out.push_str(&format!("g group_{idx}\n"));
            out.push_str(&format!("usemtl {}\n", group.material.name));
            write_faces(&mut out, &group.faces);
        }
    }

    out
}

fn write_faces(out: &mut String, faces: &[[usize; 3]]) {
    // OBJ indices are 1-based.
    for [a, b, c] in faces {
        let (a, b, c) = (a + 1, b + 1, c + 1);
        out.push_str(&format!("f {a}/{a}/{a} {b}/{b}/{b} {c}/{c}/{c}\n"));
    }
}

fn build_mtl(mesh: &Mesh) -> String {
    let mut out = String::with_capacity(512);
    out.push_str("# Laboratory v2 — material library\n");
    if mesh.groups.is_empty() {
        write_material(&mut out, &mesh.material);
    } else {
        for group in &mesh.groups {
            write_material(&mut out, &group.material);
        }
    }
    out
}

fn write_material(out: &mut String, mat: &Material) {
    let [r, g, b] = mat.diffuse_color;
    out.push_str(&format!("\nnewmtl {}\n", mat.name));
    // Ambient = half diffuse for a pleasant look
    out.push_str(&format!(
        "Ka {:.4} {:.4} {:.4}\n",
        r * 0.5,
        g * 0.5,
        b * 0.5
    ));
    out.push_str(&format!("Kd {r:.4} {g:.4} {b:.4}\n"));
    let s = mat.specular;
    out.push_str(&format!("Ks {s:.4} {s:.4} {s:.4}\n"));
    out.push_str(&format!("Ns {:.1}\n", mat.shininess));
    out.push_str("d 1.0\n");
    out.push_str("illum 2\n");

    if let Some(tex) = &mat.texture_file {
        out.push_str(&format!("map_Kd {tex}\n"));
    }
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

    #[test]
    fn multi_material_groups_emit_separate_usemtl_blocks() {
        use crate::infrastructure::geometry::mesh::MaterialGroup;
        let mesh = Mesh::new_multi(
            vec![
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [2.0, 0.0, 0.0],
                [3.0, 0.0, 0.0],
                [2.0, 1.0, 0.0],
            ],
            vec![[0.0, 0.0, 1.0]; 6],
            vec![[0.0, 0.0]; 6],
            vec![
                MaterialGroup {
                    material: Material::solid("bowl_mat", [0.96, 0.94, 0.90]),
                    faces: vec![[0, 1, 2]],
                },
                MaterialGroup {
                    material: Material::solid("sauce_mat", [0.72, 0.20, 0.12])
                        .with_gloss(0.6, 96.0),
                    faces: vec![[3, 4, 5]],
                },
            ],
        );
        let export = export_obj(&mesh).unwrap();
        let obj = String::from_utf8(export.obj_bytes).unwrap();
        let mtl = String::from_utf8(export.mtl_bytes).unwrap();

        assert!(obj.contains("usemtl bowl_mat"), "missing bowl usemtl");
        assert!(obj.contains("usemtl sauce_mat"), "missing sauce usemtl");
        assert!(obj.contains("g group_0"));
        assert!(obj.contains("g group_1"));

        assert!(mtl.contains("newmtl bowl_mat"));
        assert!(mtl.contains("newmtl sauce_mat"));
        // glossy sauce must carry through the higher Ns
        assert!(mtl.contains("Ns 96.0"));
    }
}

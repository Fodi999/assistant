//! OBJ + MTL экспорт — без зависимости от AppError.

use crate::mesh::Mesh;

pub struct ObjExport {
    pub obj_bytes: Vec<u8>,
    pub mtl_bytes: Vec<u8>,
}

pub fn export_obj(mesh: &Mesh) -> ObjExport {
    ObjExport {
        obj_bytes: build_obj(mesh).into_bytes(),
        mtl_bytes: build_mtl(mesh).into_bytes(),
    }
}

fn build_obj(mesh: &Mesh) -> String {
    let face_count: usize = if mesh.groups.is_empty() {
        mesh.faces.len()
    } else {
        mesh.groups.iter().map(|g| g.faces.len()).sum()
    };
    let mut out = String::with_capacity(mesh.vertices.len() * 32 + face_count * 24 + 256);
    out.push_str("# geometry-engine procedural OBJ\n");
    out.push_str("mtllib model.mtl\n\n");

    for [x, y, z] in &mesh.vertices {
        out.push_str(&format!("v {x:.6} {y:.6} {z:.6}\n"));
    }
    out.push('\n');
    for [u, v] in &mesh.uvs {
        out.push_str(&format!("vt {u:.6} {v:.6}\n"));
    }
    out.push('\n');
    for [nx, ny, nz] in &mesh.normals {
        out.push_str(&format!("vn {nx:.6} {ny:.6} {nz:.6}\n"));
    }
    out.push('\n');

    out.push_str("s 1\n");
    if mesh.groups.is_empty() {
        out.push_str(&format!("usemtl {}\n", mesh.material.name));
        write_faces(&mut out, &mesh.faces);
    } else {
        for (idx, group) in mesh.groups.iter().enumerate() {
            out.push_str(&format!("g group_{idx}\n"));
            out.push_str(&format!("usemtl {}\n", group.material.name));
            write_faces(&mut out, &group.faces);
        }
    }
    out
}

fn write_faces(out: &mut String, faces: &[[usize; 3]]) {
    for [a, b, c] in faces {
        let (a, b, c) = (a+1, b+1, c+1);
        out.push_str(&format!("f {a}/{a}/{a} {b}/{b}/{b} {c}/{c}/{c}\n"));
    }
}

fn build_mtl(mesh: &Mesh) -> String {
    let mut out = String::with_capacity(512);
    out.push_str("# geometry-engine material library\n");
    if mesh.groups.is_empty() {
        write_material(&mut out, &mesh.material);
    } else {
        for group in &mesh.groups {
            write_material(&mut out, &group.material);
        }
    }
    out
}

fn write_material(out: &mut String, mat: &crate::mesh::Material) {
    let [r, g, b] = mat.diffuse_color;
    out.push_str(&format!("\nnewmtl {}\n", mat.name));
    out.push_str(&format!("Ka {:.4} {:.4} {:.4}\n", r*0.5, g*0.5, b*0.5));
    out.push_str(&format!("Kd {r:.4} {g:.4} {b:.4}\n"));
    let s = mat.specular;
    out.push_str(&format!("Ks {s:.4} {s:.4} {s:.4}\n"));
    out.push_str(&format!("Ns {:.1}\n", mat.shininess));
    out.push_str("d 1.0\nillum 2\n");
    if let Some(tex) = &mat.texture_file {
        out.push_str(&format!("map_Kd {tex}\n"));
    }
}

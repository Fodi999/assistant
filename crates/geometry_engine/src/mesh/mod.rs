pub mod gltf;
pub mod gpu_mesh;
pub mod mesh;
pub mod mesh_part;
pub mod normals;
pub mod obj;
pub mod stl;
pub mod weld;

// GeometryError живёт здесь — используется всеми ops/profile модулями
mod error;

pub use error::GeometryError;
pub use mesh::{hex_to_rgb, GpuMesh, Material, MaterialGroup, Mesh, MeshPart};
pub use normals::recalculate_smooth_normals;
pub use obj::{export_obj, ObjExport};
pub use weld::weld_vertices;

/// Валидация меша перед экспортом.
pub fn validate_mesh(mesh: &Mesh) -> Result<(), GeometryError> {
    if mesh.vertices.is_empty() {
        return Err(GeometryError::InvalidMesh("vertices is empty".into()));
    }
    if mesh.normals.len() != mesh.vertices.len() {
        return Err(GeometryError::InvalidMesh(format!(
            "normals.len()={} != vertices.len()={}", mesh.normals.len(), mesh.vertices.len()
        )));
    }
    if mesh.uvs.len() != mesh.vertices.len() {
        return Err(GeometryError::InvalidMesh(format!(
            "uvs.len()={} != vertices.len()={}", mesh.uvs.len(), mesh.vertices.len()
        )));
    }
    if mesh.groups.is_empty() && mesh.faces.is_empty() {
        return Err(GeometryError::InvalidMesh("no groups and no faces".into()));
    }
    let vc = mesh.vertices.len();
    if mesh.groups.is_empty() {
        for (i, f) in mesh.faces.iter().enumerate() {
            check_tri(i, f, vc, "(legacy)")?;
        }
    } else {
        for (gi, g) in mesh.groups.iter().enumerate() {
            if g.material.name.trim().is_empty() {
                return Err(GeometryError::InvalidMesh(format!("group {gi} empty material name")));
            }
            for (fi, f) in g.faces.iter().enumerate() {
                check_tri(fi, f, vc, &g.material.name)?;
            }
        }
    }
    Ok(())
}

fn check_tri(idx: usize, face: &[usize; 3], vc: usize, where_: &str) -> Result<(), GeometryError> {
    let [a, b, c] = *face;
    if a >= vc || b >= vc || c >= vc {
        return Err(GeometryError::InvalidMesh(format!(
            "face {idx} in {where_} out-of-range ({a},{b},{c}); vc={vc}"
        )));
    }
    if a == b || b == c || a == c {
        return Err(GeometryError::InvalidMesh(format!(
            "face {idx} in {where_} degenerate ({a},{b},{c})"
        )));
    }
    Ok(())
}

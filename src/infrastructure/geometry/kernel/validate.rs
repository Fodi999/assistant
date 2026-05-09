//! Mesh validation + kernel-level error type.
//!
//! `validate_mesh` is a cheap O(V + F) sanity check meant to run *before*
//! handing a mesh to the GLB exporter. It catches the most common bugs
//! introduced by hand-rolled generators:
//!
//!   * empty vertex / group / face arrays
//!   * mismatched vertex / normal / uv counts
//!   * face indices that reference non-existent vertices
//!   * NaN / infinity in any position, normal, or uv
//!   * non-unit normals (length outside [0.5, 2.0])
//!   * groups whose material has an empty name
//!
//! The cost is one linear scan, so generators can call it unconditionally
//! in debug builds.

use crate::infrastructure::geometry::mesh::Mesh;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GeometryError {
    InvalidProfile(String),
    InvalidArgument(String),
    InvalidMesh(String),
}

impl std::fmt::Display for GeometryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GeometryError::InvalidProfile(m) => write!(f, "invalid profile: {m}"),
            GeometryError::InvalidArgument(m) => write!(f, "invalid argument: {m}"),
            GeometryError::InvalidMesh(m) => write!(f, "invalid mesh: {m}"),
        }
    }
}

impl std::error::Error for GeometryError {}

/// Bridge to the application-level error type. Allows kernel functions to
/// be called from existing services with a plain `?`.
impl From<GeometryError> for crate::shared::AppError {
    fn from(e: GeometryError) -> Self {
        crate::shared::AppError::internal(format!("geometry kernel: {e}"))
    }
}

pub fn validate_mesh(mesh: &Mesh) -> Result<(), GeometryError> {
    if mesh.vertices.is_empty() {
        return Err(GeometryError::InvalidMesh("vertices is empty".into()));
    }
    if mesh.normals.len() != mesh.vertices.len() {
        return Err(GeometryError::InvalidMesh(format!(
            "normals.len() = {} != vertices.len() = {}",
            mesh.normals.len(),
            mesh.vertices.len()
        )));
    }
    if mesh.uvs.len() != mesh.vertices.len() {
        return Err(GeometryError::InvalidMesh(format!(
            "uvs.len() = {} != vertices.len() = {}",
            mesh.uvs.len(),
            mesh.vertices.len()
        )));
    }
    if mesh.groups.is_empty() && mesh.faces.is_empty() {
        return Err(GeometryError::InvalidMesh(
            "mesh has no groups and no faces".into(),
        ));
    }

    // Check finiteness of every per-vertex attribute.
    for (i, v) in mesh.vertices.iter().enumerate() {
        for (k, c) in v.iter().enumerate() {
            if !c.is_finite() {
                return Err(GeometryError::InvalidMesh(format!(
                    "vertex {i}.{k} is not finite ({c})"
                )));
            }
        }
    }
    for (i, n) in mesh.normals.iter().enumerate() {
        for (k, c) in n.iter().enumerate() {
            if !c.is_finite() {
                return Err(GeometryError::InvalidMesh(format!(
                    "normal {i}.{k} is not finite ({c})"
                )));
            }
        }
        let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
        if !(0.5..=2.0).contains(&len) {
            return Err(GeometryError::InvalidMesh(format!(
                "normal {i} has unreasonable length {len}"
            )));
        }
    }
    for (i, uv) in mesh.uvs.iter().enumerate() {
        if !uv[0].is_finite() || !uv[1].is_finite() {
            return Err(GeometryError::InvalidMesh(format!("uv {i} is not finite")));
        }
    }

    let v_count = mesh.vertices.len();

    // Walk every face source. When `groups` is empty we fall back to the
    // legacy single-material `faces` array.
    let groups: Vec<&crate::infrastructure::geometry::mesh::MaterialGroup> =
        mesh.groups.iter().collect();

    if groups.is_empty() {
        for (i, f) in mesh.faces.iter().enumerate() {
            check_triangle(i, f, v_count, "(legacy)")?;
        }
    } else {
        let mut total_faces = 0usize;
        for (gi, g) in groups.iter().enumerate() {
            if g.material.name.trim().is_empty() {
                return Err(GeometryError::InvalidMesh(format!(
                    "group {gi} has empty material name"
                )));
            }
            if g.faces.is_empty() {
                return Err(GeometryError::InvalidMesh(format!(
                    "group {gi} ({}) has no faces",
                    g.material.name
                )));
            }
            for (fi, f) in g.faces.iter().enumerate() {
                check_triangle(fi, f, v_count, &g.material.name)?;
            }
            total_faces += g.faces.len();
        }
        if total_faces == 0 {
            return Err(GeometryError::InvalidMesh(
                "no faces across all groups".into(),
            ));
        }
    }

    Ok(())
}

fn check_triangle(
    index: usize,
    face: &[usize; 3],
    v_count: usize,
    where_: &str,
) -> Result<(), GeometryError> {
    let [a, b, c] = *face;
    if a >= v_count || b >= v_count || c >= v_count {
        return Err(GeometryError::InvalidMesh(format!(
            "face {index} in {where_} has out-of-range index ({a}, {b}, {c}); v_count = {v_count}"
        )));
    }
    if a == b || b == c || a == c {
        return Err(GeometryError::InvalidMesh(format!(
            "face {index} in {where_} is degenerate ({a}, {b}, {c})"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::geometry::generators::food::{
        bottled_sauce, jar_product, sauce_in_bowl,
    };

    #[test]
    fn validate_passes_for_real_generators() {
        validate_mesh(&sauce_in_bowl::generate("#FF0000", None)).unwrap();
        validate_mesh(&jar_product::generate("#A85B12", None)).unwrap();
        validate_mesh(&bottled_sauce::generate(
            "#B8321F",
            bottled_sauce::BottleKind::Glass,
            None,
        ))
        .unwrap();
    }

    #[test]
    fn validate_rejects_out_of_range_face_index() {
        let mut mesh = sauce_in_bowl::generate("#FF0000", None);
        // Corrupt one face on purpose.
        let bogus_idx = mesh.vertices.len(); // exactly out of range
        mesh.groups[0].faces[0] = [0, 1, bogus_idx];
        let err = validate_mesh(&mesh).unwrap_err();
        assert!(matches!(err, GeometryError::InvalidMesh(_)));
    }

    #[test]
    fn validate_rejects_nan_vertex() {
        let mut mesh = jar_product::generate("#A85B12", None);
        mesh.vertices[0][1] = f32::NAN;
        let err = validate_mesh(&mesh).unwrap_err();
        assert!(matches!(err, GeometryError::InvalidMesh(m) if m.contains("not finite")));
    }

    #[test]
    fn validate_rejects_mismatched_normals() {
        let mut mesh = sauce_in_bowl::generate("#FF0000", None);
        mesh.normals.pop();
        let err = validate_mesh(&mesh).unwrap_err();
        assert!(matches!(err, GeometryError::InvalidMesh(_)));
    }

    #[test]
    fn validate_rejects_degenerate_face() {
        let mut mesh = sauce_in_bowl::generate("#FF0000", None);
        mesh.groups[0].faces[0] = [0, 0, 1];
        let err = validate_mesh(&mesh).unwrap_err();
        assert!(matches!(err, GeometryError::InvalidMesh(m) if m.contains("degenerate")));
    }

    #[test]
    fn validate_rejects_empty_group_name() {
        let mut mesh = sauce_in_bowl::generate("#FF0000", None);
        mesh.groups[0].material.name.clear();
        let err = validate_mesh(&mesh).unwrap_err();
        assert!(matches!(err, GeometryError::InvalidMesh(m) if m.contains("empty material name")));
    }
}

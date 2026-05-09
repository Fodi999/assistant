//! Append-only mesh builder.
//!
//! Wraps a growing `(vertices, normals, uvs)` array plus a list of
//! [`MaterialGroup`]s, exposing index-stable helpers so generators don't
//! need to track offsets manually.
//!
//! Typical use:
//!
//! ```ignore
//! let mut b = MeshBuilder::new();
//! let glass = b.add_group(materials::glass("jar_glass", color));
//! let v0 = b.add_vertex([0.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0]);
//! // …
//! b.add_triangle(glass, v0, v1, v2);
//! let mesh = b.build();
//! ```
//!
//! The builder never frees memory and never re-orders vertices, so indices
//! returned by `add_vertex` remain valid for the entire lifetime of the
//! builder.

use crate::infrastructure::geometry::mesh::{Material, MaterialGroup, Mesh};

use super::lathe::MeshPart;

#[derive(Debug, Default)]
pub struct MeshBuilder {
    vertices: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    groups: Vec<MaterialGroup>,
}

impl MeshBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capacity(verts: usize, faces: usize) -> Self {
        Self {
            vertices: Vec::with_capacity(verts),
            normals: Vec::with_capacity(verts),
            uvs: Vec::with_capacity(verts),
            groups: Vec::with_capacity(faces / 64 + 1),
        }
    }

    /// Push a new vertex (position, normal, uv) and return its global index.
    pub fn add_vertex(&mut self, position: [f32; 3], normal: [f32; 3], uv: [f32; 2]) -> usize {
        let idx = self.vertices.len();
        self.vertices.push(position);
        self.normals.push(normal);
        self.uvs.push(uv);
        idx
    }

    /// Return the current number of vertices. Useful for recording the start
    /// index before adding a ring of vertices (PR #29 fill volume).
    pub fn vertices_len(&self) -> usize {
        self.vertices.len()
    }

    /// Register a new material group and return its handle (the index into
    /// the groups array). Pass this handle back to `add_triangle` /
    /// `add_quad` to attach faces to that material.
    pub fn add_group(&mut self, material: Material) -> usize {
        let idx = self.groups.len();
        self.groups.push(MaterialGroup {
            material,
            faces: Vec::new(),
        });
        idx
    }

    /// Append a triangle (CCW winding) to the given group.
    pub fn add_triangle(&mut self, group: usize, a: usize, b: usize, c: usize) {
        debug_assert!(group < self.groups.len(), "unknown group {group}");
        debug_assert!(
            a < self.vertices.len() && b < self.vertices.len() && c < self.vertices.len(),
            "triangle index out of range"
        );
        self.groups[group].faces.push([a, b, c]);
    }

    /// Append a quad as two triangles `(a,b,c) (a,c,d)` (CCW winding).
    pub fn add_quad(&mut self, group: usize, a: usize, b: usize, c: usize, d: usize) {
        self.add_triangle(group, a, b, c);
        self.add_triangle(group, a, c, d);
    }

    /// Splice a self-contained [`MeshPart`] (e.g. a lathed wall or a disk
    /// fan) into the given material group. Vertices/normals/uvs are
    /// appended to the global arrays, and face indices are offset by the
    /// current vertex count so the part remains valid.
    pub fn add_part(&mut self, group: usize, part: &MeshPart) {
        debug_assert!(group < self.groups.len(), "unknown group {group}");
        debug_assert_eq!(part.vertices.len(), part.normals.len());
        debug_assert_eq!(part.vertices.len(), part.uvs.len());

        let offset = self.vertices.len();
        self.vertices.extend_from_slice(&part.vertices);
        self.normals.extend_from_slice(&part.normals);
        self.uvs.extend_from_slice(&part.uvs);

        let faces = &mut self.groups[group].faces;
        faces.reserve(part.faces.len());
        for f in &part.faces {
            faces.push([f[0] + offset, f[1] + offset, f[2] + offset]);
        }
    }

    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }

    pub fn group_count(&self) -> usize {
        self.groups.len()
    }

    pub fn face_count(&self) -> usize {
        self.groups.iter().map(|g| g.faces.len()).sum()
    }

    /// Borrow the position array (used by `normals::recalculate_smooth_normals`).
    pub fn vertices(&self) -> &[[f32; 3]] {
        &self.vertices
    }

    /// Mutably borrow the normals array (used by smooth-normal pass).
    pub fn normals_mut(&mut self) -> &mut [[f32; 3]] {
        &mut self.normals
    }

    /// Borrow the groups (used by smooth-normal pass to walk faces).
    pub fn groups(&self) -> &[MaterialGroup] {
        &self.groups
    }

    /// Finalise the builder into a [`Mesh`]. Panics if no groups were added,
    /// because a mesh with no material is meaningless.
    pub fn build(self) -> Mesh {
        assert!(
            !self.groups.is_empty(),
            "MeshBuilder::build called with no groups"
        );
        Mesh::new_multi(self.vertices, self.normals, self.uvs, self.groups)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_assembles_two_triangles() {
        let mut b = MeshBuilder::new();
        let mat = Material::solid("test", [1.0, 0.0, 0.0]);
        let g = b.add_group(mat);
        let v0 = b.add_vertex([0.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0]);
        let v1 = b.add_vertex([1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [1.0, 0.0]);
        let v2 = b.add_vertex([1.0, 0.0, 1.0], [0.0, 1.0, 0.0], [1.0, 1.0]);
        let v3 = b.add_vertex([0.0, 0.0, 1.0], [0.0, 1.0, 0.0], [0.0, 1.0]);
        b.add_quad(g, v0, v1, v2, v3);

        assert_eq!(b.vertex_count(), 4);
        assert_eq!(b.face_count(), 2);

        let mesh = b.build();
        assert_eq!(mesh.vertices.len(), 4);
        assert_eq!(mesh.groups.len(), 1);
        assert_eq!(mesh.groups[0].faces.len(), 2);
    }

    #[test]
    fn builder_supports_multiple_groups() {
        let mut b = MeshBuilder::new();
        let g1 = b.add_group(Material::solid("a", [1.0, 0.0, 0.0]));
        let g2 = b.add_group(Material::solid("b", [0.0, 1.0, 0.0]));
        let v0 = b.add_vertex([0.0; 3], [0.0, 1.0, 0.0], [0.0; 2]);
        let v1 = b.add_vertex([1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [1.0, 0.0]);
        let v2 = b.add_vertex([0.0, 0.0, 1.0], [0.0, 1.0, 0.0], [0.0, 1.0]);
        b.add_triangle(g1, v0, v1, v2);
        b.add_triangle(g2, v0, v2, v1);
        let mesh = b.build();
        assert_eq!(mesh.groups.len(), 2);
        assert_eq!(mesh.groups[0].faces.len(), 1);
        assert_eq!(mesh.groups[1].faces.len(), 1);
    }

    #[test]
    #[should_panic(expected = "no groups")]
    fn build_panics_with_no_groups() {
        let b = MeshBuilder::new();
        let _ = b.build();
    }
}

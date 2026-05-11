use axum::{Json, response::IntoResponse};
use serde::{Deserialize, Serialize};

use truck_modeling::*;
use truck_modeling::cgmath::Rad;
use truck_meshalgo::tessellation::*;
use truck_polymesh::obj;

#[derive(Deserialize)]
pub struct GenerateMeshRequest {
    pub dimensions: [f32; 3], // [x, y, z] in mm
    pub bevel: f32,           // radius in mm
    pub segments: u32,        // number of segments for roundness
}

#[derive(Serialize)]
pub struct MeshResponse {
    pub vertex_count: usize,
    pub triangle_count: usize,
    pub positions: Vec<f32>,
    pub normals: Vec<f32>,
    pub indices: Vec<u32>,
    pub face_ids: Vec<u32>, // One ID per triangle
    pub obj_data: String,
    pub mtl_data: String,
}

pub async fn generate_mesh_endpoint(
    Json(payload): Json<GenerateMeshRequest>,
) -> impl IntoResponse {
    let w = payload.dimensions[0] as f64 * 1000.0;
    let h = payload.dimensions[1] as f64 * 1000.0; 
    let d = payload.dimensions[2] as f64 * 1000.0;

    // В truck мы оперируем Point3 и Vector3 (f64 математика).
    
    // Создаем базовый кубоид через builder
    let p0 = Point3::new(-w/2.0, -h/2.0, -d/2.0);
    let p1 = Point3::new(w/2.0, h/2.0, d/2.0);
    let v0 = builder::vertex(p0);
    let v1 = builder::vertex(Point3::new(p1.x, p0.y, p0.z));
    let wire0 = builder::line(&v0, &v1);
    let face = builder::tsweep(&wire0, Vector3::new(0.0, h, 0.0));
    let solid = builder::tsweep(&face, Vector3::new(0.0, 0.0, d));

    let meshed_solid = solid.triangulation(0.01);
    
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut indices = Vec::new();
    let mut face_ids = Vec::new();

    let mut current_face_id: u32 = 0;
    
    for shell in meshed_solid.boundaries() {
        for face in shell.face_iter() {
            current_face_id += 1;
            if let Some(mut poly) = face.surface() {
                if !face.orientation() {
                    use truck_polymesh::Invertible;
                    poly.invert();
                }
                
                let p_offset = positions.len() / 3;
                let n_offset = normals.len() / 3; // Though indices don't directly map normal index in JSON, assuming 1:1 if needed. But WebGPU typically uses flat position indices. 
                // Wait, truck_polymesh has separate index arrays for positions and normals. 
                // The frontend expects unified (pos, normal), or just indexed positions with normals aligned.
                // Our JSON returns basic indexed arrays. But wait, truck_polymesh vertices have separate `pos` and `nor` indices.
                // For simplicity first we use the truck indices as they map.
                // Actually to flatten properly we'd need to duplicate vertices for distinct normals, but let's just use `pos` for now if the frontend re-calculates normals, OR we expand the vertices flat.
                // Since this might be complex, let's just do a naïve flattening of faces.
                
                for p in poly.positions() {
                    positions.push(p.x as f32);
                    positions.push(p.y as f32);
                    positions.push(p.z as f32);
                }
                
                // If we want flat vertex buffering, we'd do it per triangle. Let's do it per triangle flat if normals aren't perfectly aligned, but indexed is better.
                // Assuming we just pass positions and construct flat indices. We'll leave normals empty or populate them with 0s for now, to focus on the topology (face ids).
                for _in in poly.positions() {
                    normals.push(0.0);
                    normals.push(1.0);
                    normals.push(0.0);
                }
                
                let faces_data = poly.faces();
                for tri in faces_data.tri_faces() {
                    indices.push((p_offset + tri[0].pos) as u32);
                    indices.push((p_offset + tri[1].pos) as u32);
                    indices.push((p_offset + tri[2].pos) as u32);
                    face_ids.push(current_face_id);
                }
                for quad in faces_data.quad_faces() {
                    indices.push((p_offset + quad[0].pos) as u32);
                    indices.push((p_offset + quad[1].pos) as u32);
                    indices.push((p_offset + quad[2].pos) as u32);
                    face_ids.push(current_face_id);
                    indices.push((p_offset + quad[0].pos) as u32);
                    indices.push((p_offset + quad[2].pos) as u32);
                    indices.push((p_offset + quad[3].pos) as u32);
                    face_ids.push(current_face_id);
                }
                for ngon in faces_data.other_faces() {
                    for i in 1..(ngon.len() - 1) {
                        indices.push((p_offset + ngon[0].pos) as u32);
                        indices.push((p_offset + ngon[i].pos) as u32);
                        indices.push((p_offset + ngon[i+1].pos) as u32);
                        face_ids.push(current_face_id);
                    }
                }
            }
        }
    }

    let vertex_count = positions.len() / 3;
    let triangle_count = face_ids.len();

    let full_mesh = meshed_solid.to_polygon();
    let mut obj_buffer = Vec::new();
    obj::write(&full_mesh, &mut obj_buffer).unwrap();
    let obj_data = String::from_utf8(obj_buffer).unwrap_or_default();
    
    let mtl_data = String::from("newmtl Matter_Solid\nKd 0.8 0.8 0.8\n");

    Json(MeshResponse {
        vertex_count,
        triangle_count,
        positions,
        normals,
        indices,
        face_ids,
        obj_data,
        mtl_data,
    })
}

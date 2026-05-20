//! /api/matter/mesh endpoint — box mesh via own geometry kernel.

use axum::{Json, response::IntoResponse};
use serde::{Deserialize, Serialize};

use crate::infrastructure::geometry::kernel::extrude::{
    extrude_polygon, ExtrudeOptions, Point2,
};

#[derive(Deserialize)]
pub struct GenerateMeshRequest {
    pub dimensions: [f32; 3], // [x, y, z] in metres
    pub bevel: f32,           // chamfer width in metres
    pub segments: u32,        // unused, kept for API compat
}

#[derive(Serialize)]
pub struct MeshResponse {
    pub vertex_count: usize,
    pub triangle_count: usize,
    pub positions: Vec<f32>,
    pub normals: Vec<f32>,
    pub indices: Vec<u32>,
    pub face_ids: Vec<u32>,
    pub obj_data: String,
    pub mtl_data: String,
}

pub async fn generate_mesh_endpoint(
    Json(payload): Json<GenerateMeshRequest>,
) -> impl IntoResponse {
    let w = payload.dimensions[0] * 0.5;
    let h = payload.dimensions[1] * 0.5;
    let depth = payload.dimensions[2];

    // Rectangle centred at origin in XY, extruded along Z.
    let profile = vec![
        Point2::new( w,  h),
        Point2::new(-w,  h),
        Point2::new(-w, -h),
        Point2::new( w, -h),
    ];

    let opts = ExtrudeOptions { depth, bevel: payload.bevel };

    let parts = extrude_polygon(&profile, &opts).unwrap_or_else(|_| {
        // Fallback: unit cube
        let fb = vec![
            Point2::new( 0.5,  0.5),
            Point2::new(-0.5,  0.5),
            Point2::new(-0.5, -0.5),
            Point2::new( 0.5, -0.5),
        ];
        extrude_polygon(&fb, &ExtrudeOptions { depth: 1.0, bevel: 0.0 }).unwrap()
    });

    let mut positions: Vec<f32> = Vec::new();
    let mut normals:   Vec<f32> = Vec::new();
    let mut indices:   Vec<u32> = Vec::new();
    let mut face_ids:  Vec<u32> = Vec::new();

    for (part_idx, part) in parts.iter().enumerate() {
        let face_id = (part_idx + 1) as u32;
        let v_offset = (positions.len() / 3) as u32;

        for v in &part.vertices {
            positions.push(v[0]);
            positions.push(v[1]);
            positions.push(v[2]);
        }
        for n in &part.normals {
            normals.push(n[0]);
            normals.push(n[1]);
            normals.push(n[2]);
        }
        for tri in &part.faces {
            indices.push(v_offset + tri[0] as u32);
            indices.push(v_offset + tri[1] as u32);
            indices.push(v_offset + tri[2] as u32);
            face_ids.push(face_id);
        }
    }

    let vc = positions.len() / 3;
    let tc = indices.len() / 3;

    let mut obj_data = String::new();
    obj_data.push_str("# geometry-kernel box\n");
    for i in 0..vc {
        let b = i * 3;
        obj_data.push_str(&format!("v {} {} {}\n", positions[b], positions[b+1], positions[b+2]));
    }
    for i in 0..vc {
        let b = i * 3;
        obj_data.push_str(&format!("vn {} {} {}\n", normals[b], normals[b+1], normals[b+2]));
    }
    for t in 0..tc {
        let b = t * 3;
        let (a, bb, c) = (indices[b]+1, indices[b+1]+1, indices[b+2]+1);
        obj_data.push_str(&format!("f {a}//{a} {bb}//{bb} {c}//{c}\n"));
    }

    let mtl_data = "newmtl Matter_Solid\nKd 0.8 0.8 0.8\n".to_string();

    Json(MeshResponse {
        vertex_count: vc,
        triangle_count: tc,
        positions,
        normals,
        indices,
        face_ids,
        obj_data,
        mtl_data,
    })
}

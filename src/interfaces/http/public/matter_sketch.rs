//! Sketch → Extrude endpoint.
//!
//! Accepts a sketch profile + plane + depth from the WebGPU frontend and
//! returns a triangulated B-Rep solid produced by `truck-modeling`.
//!
//! Frontend usage:
//!   POST /api/matter/sketch/extrude
//!   {
//!     "plane": "XZ",          // "XY" | "XZ" | "YZ"
//!     "depth": 1.0,           // metres
//!     "direction": [0,1,0],   // optional; defaults to plane normal
//!     "profile": [            // closed CCW polyline, >=3 points
//!       { "x": 0.0, "y": 0.0, "z": 0.0 },
//!       { "x": 1.0, "y": 0.0, "z": 0.0 },
//!       ...
//!     ]
//!   }
//!
//! Response: same shape as `MeshResponse` so the frontend pipeline can
//! upload it straight to GPU CAD buffers.

use axum::{Json, http::StatusCode, response::IntoResponse};
use serde::{Deserialize, Serialize};
use truck_meshalgo::tessellation::*;
use truck_modeling::*;
use truck_polymesh::obj;

#[derive(Deserialize)]
pub struct ProfilePoint {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

#[derive(Deserialize)]
pub struct ExtrudeSketchRequest {
    /// Sketch plane: "XY" | "XZ" | "YZ".
    pub plane: String,
    /// Extrude depth in metres (>0).
    pub depth: f64,
    /// Optional direction unit vector. Defaults to plane normal.
    #[serde(default)]
    pub direction: Option<[f64; 3]>,
    /// Closed CCW profile points (>=3).
    pub profile: Vec<ProfilePoint>,
    /// Triangulation tolerance (metres). Default 0.01.
    #[serde(default)]
    pub tolerance: Option<f64>,
}

#[derive(Serialize)]
pub struct SketchMeshResponse {
    pub vertex_count: usize,
    pub triangle_count: usize,
    pub positions: Vec<f32>,
    pub normals: Vec<f32>,
    pub indices: Vec<u32>,
    pub face_ids: Vec<u32>, // one per triangle
    pub face_count: usize,
    pub obj_data: String,
    pub kernel: &'static str, // "truck-modeling"
}

#[derive(Serialize)]
pub struct ApiError {
    pub error: String,
}

/// Map a profile point onto the 2D plane and return (u,v).
/// XY: (x,y), normal +Z
/// XZ: (x,z), normal +Y
/// YZ: (y,z), normal +X
fn project_to_2d(plane: &str, p: &ProfilePoint) -> (f64, f64, [f64; 3]) {
    match plane {
        "XY" => (p.x, p.y, [0.0, 0.0, 1.0]),
        "YZ" => (p.y, p.z, [1.0, 0.0, 0.0]),
        _ /* XZ */ => (p.x, p.z, [0.0, 1.0, 0.0]),
    }
}

fn signed_area_2d(pts: &[(f64, f64)]) -> f64 {
    let n = pts.len();
    let mut s = 0.0;
    for i in 0..n {
        let (ax, ay) = pts[i];
        let (bx, by) = pts[(i + 1) % n];
        s += ax * by - bx * ay;
    }
    s * 0.5
}

pub async fn extrude_sketch_endpoint(
    Json(req): Json<ExtrudeSketchRequest>,
) -> std::result::Result<Json<SketchMeshResponse>, (StatusCode, Json<ApiError>)> {
    // ── 1. Validate ────────────────────────────────────────────────
    if req.profile.len() < 3 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiError {
                error: format!("profile needs >=3 points, got {}", req.profile.len()),
            }),
        ));
    }
    if !(req.depth.is_finite() && req.depth > 0.0) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiError {
                error: format!("depth must be positive finite, got {}", req.depth),
            }),
        ));
    }
    let plane = req.plane.to_uppercase();
    let tol = req.tolerance.unwrap_or(0.01).max(1e-6);

    // ── 2. Project to 2D, enforce CCW ─────────────────────────────
    let mut pts2: Vec<(f64, f64)> = Vec::with_capacity(req.profile.len());
    let mut plane_normal = [0.0, 1.0, 0.0];
    let mut plane_offset = 0.0_f64;
    for (i, p) in req.profile.iter().enumerate() {
        let (u, v, n) = project_to_2d(&plane, p);
        pts2.push((u, v));
        if i == 0 {
            plane_normal = n;
            plane_offset = match plane.as_str() {
                "XY" => p.z,
                "YZ" => p.x,
                _ => p.y,
            };
        }
    }
    if signed_area_2d(&pts2) < 0.0 {
        pts2.reverse();
    }

    let direction = req.direction.unwrap_or(plane_normal);

    // ── 3. Build B-Rep wire via truck ─────────────────────────────
    // Lift 2D points back to 3D on the sketch plane, then build vertex chain.
    let lift = |u: f64, v: f64| -> Point3 {
        match plane.as_str() {
            "XY" => Point3::new(u, v, plane_offset),
            "YZ" => Point3::new(plane_offset, u, v),
            _ => Point3::new(u, plane_offset, v),
        }
    };

    // Skip duplicate last == first (closed sketches sometimes include it)
    let mut clean: Vec<(f64, f64)> = Vec::with_capacity(pts2.len());
    for (i, p) in pts2.iter().enumerate() {
        if i + 1 == pts2.len()
            && (p.0 - pts2[0].0).abs() < 1e-9
            && (p.1 - pts2[0].1).abs() < 1e-9
        {
            break;
        }
        clean.push(*p);
    }
    if clean.len() < 3 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiError {
                error: "profile collapsed to <3 unique points".into(),
            }),
        ));
    }

    let vertices: Vec<_> = clean
        .iter()
        .map(|(u, v)| builder::vertex(lift(*u, *v)))
        .collect();

    // Build closed wire: edges v0→v1, v1→v2, ..., vN-1→v0
    let mut wire = Wire::new();
    for i in 0..vertices.len() {
        let a = &vertices[i];
        let b = &vertices[(i + 1) % vertices.len()];
        let edge = builder::line(a, b);
        wire.push_back(edge);
    }

    // Attach a planar face to the wire
    let face = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        builder::try_attach_plane(&[wire.clone()])
    })) {
        Ok(Ok(f)) => f,
        Ok(Err(e)) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ApiError {
                    error: format!("truck try_attach_plane failed: {e:?}"),
                }),
            ));
        }
        Err(_) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError {
                    error: "truck try_attach_plane panicked (self-intersecting or non-planar wire?)"
                        .into(),
                }),
            ));
        }
    };

    // ── 4. tsweep along direction × depth ─────────────────────────
    let dir = Vector3::new(
        direction[0] * req.depth,
        direction[1] * req.depth,
        direction[2] * req.depth,
    );
    let solid = builder::tsweep(&face, dir);

    // ── 5. Triangulate ────────────────────────────────────────────
    let meshed = solid.triangulation(tol);

    // ── 6. Flatten to GPU arrays with per-face IDs ────────────────
    let mut positions: Vec<f32> = Vec::new();
    let mut normals: Vec<f32> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    let mut face_ids: Vec<u32> = Vec::new();
    let mut current_face_id: u32 = 0;

    for shell in meshed.boundaries() {
        for face in shell.face_iter() {
            current_face_id += 1;
            if let Some(mut poly) = face.surface() {
                if !face.orientation() {
                    use truck_polymesh::Invertible;
                    poly.invert();
                }
                let p_offset = positions.len() / 3;

                for p in poly.positions() {
                    positions.push(p.x as f32);
                    positions.push(p.y as f32);
                    positions.push(p.z as f32);
                }
                // Pre-allocate normal slots (one per position)
                let v_added = poly.positions().len();
                let n_start = normals.len() / 3;
                for _ in 0..v_added {
                    normals.push(0.0);
                    normals.push(0.0);
                    normals.push(0.0);
                }

                let faces_data = poly.faces();

                // Helper to splat the truck per-vertex normal into the right slot
                let face_normals = poly.normals();
                let accumulate = |normals: &mut Vec<f32>, vidx: usize, nidx: usize| {
                    if nidx < face_normals.len() {
                        let n = face_normals[nidx];
                        let base = (n_start + vidx) * 3;
                        normals[base] += n.x as f32;
                        normals[base + 1] += n.y as f32;
                        normals[base + 2] += n.z as f32;
                    }
                };

                for tri in faces_data.tri_faces() {
                    indices.push((p_offset + tri[0].pos) as u32);
                    indices.push((p_offset + tri[1].pos) as u32);
                    indices.push((p_offset + tri[2].pos) as u32);
                    face_ids.push(current_face_id);
                    if let (Some(n0), Some(n1), Some(n2)) =
                        (tri[0].nor, tri[1].nor, tri[2].nor)
                    {
                        accumulate(&mut normals, tri[0].pos, n0);
                        accumulate(&mut normals, tri[1].pos, n1);
                        accumulate(&mut normals, tri[2].pos, n2);
                    }
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
                    if let (Some(n0), Some(n1), Some(n2), Some(n3)) =
                        (quad[0].nor, quad[1].nor, quad[2].nor, quad[3].nor)
                    {
                        accumulate(&mut normals, quad[0].pos, n0);
                        accumulate(&mut normals, quad[1].pos, n1);
                        accumulate(&mut normals, quad[2].pos, n2);
                        accumulate(&mut normals, quad[3].pos, n3);
                    }
                }
                for ngon in faces_data.other_faces() {
                    for i in 1..(ngon.len() - 1) {
                        indices.push((p_offset + ngon[0].pos) as u32);
                        indices.push((p_offset + ngon[i].pos) as u32);
                        indices.push((p_offset + ngon[i + 1].pos) as u32);
                        face_ids.push(current_face_id);
                    }
                }
            }
        }
    }

    // Normalize accumulated normals; fall back to plane normal where zero.
    for chunk in normals.chunks_mut(3) {
        let len = (chunk[0] * chunk[0] + chunk[1] * chunk[1] + chunk[2] * chunk[2]).sqrt();
        if len > 1e-6 {
            chunk[0] /= len;
            chunk[1] /= len;
            chunk[2] /= len;
        } else {
            chunk[0] = direction[0] as f32;
            chunk[1] = direction[1] as f32;
            chunk[2] = direction[2] as f32;
        }
    }

    // OBJ export (for download / debug)
    let full_mesh = meshed.to_polygon();
    let mut obj_buffer = Vec::new();
    let _ = obj::write(&full_mesh, &mut obj_buffer);
    let obj_data = String::from_utf8(obj_buffer).unwrap_or_default();

    let vertex_count = positions.len() / 3;
    let triangle_count = face_ids.len();

    Ok(Json(SketchMeshResponse {
        vertex_count,
        triangle_count,
        positions,
        normals,
        indices,
        face_ids,
        face_count: current_face_id as usize,
        obj_data,
        kernel: "truck-modeling",
    }))
}

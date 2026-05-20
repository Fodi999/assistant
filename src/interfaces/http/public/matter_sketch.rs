//! Sketch → Extrude endpoint.
//!
//! Accepts a sketch profile + plane + depth from the WebGPU frontend and
//! returns a triangulated solid produced by our own geometry kernel
//! (`infrastructure/geometry/kernel/extrude`).
//!
//! POST /api/matter/sketch/extrude
//! {
//!   "plane": "XZ",        // "XY" | "XZ" | "YZ"
//!   "depth": 0.1,         // metres
//!   "bevel": 0.005,       // optional chamfer (metres), default 0
//!   "profile": [{"x":0,"y":0,"z":0}, ...]
//! }
//!
//! Response: flat GPU-ready arrays — positions, normals, indices, face_ids.

use axum::{Json, http::StatusCode};
use serde::{Deserialize, Serialize};

use crate::infrastructure::geometry::kernel::extrude::{
    extrude_polygon, ExtrudeOptions, Point2,
};

// ── Request / Response types ───────────────────────────────────────────────

#[derive(Deserialize)]
pub struct ProfilePoint {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

#[derive(Deserialize)]
pub struct ExtrudeSketchRequest {
    pub plane: String,
    pub depth: f64,
    #[serde(default)]
    pub direction: Option<[f64; 3]>,
    pub profile: Vec<ProfilePoint>,
    #[serde(default)]
    pub tolerance: Option<f64>, // kept for API compat, unused
    #[serde(default)]
    pub bevel: Option<f64>,
}

#[derive(Serialize)]
pub struct SketchMeshResponse {
    pub vertex_count: usize,
    pub triangle_count: usize,
    pub positions: Vec<f32>,
    pub normals: Vec<f32>,
    pub indices: Vec<u32>,
    pub face_ids: Vec<u32>,
    pub face_count: usize,
    pub obj_data: String,
    pub kernel: &'static str,
}

#[derive(Serialize)]
pub struct ApiError {
    pub error: String,
}

// ── Helpers ────────────────────────────────────────────────────────────────

fn signed_area_2d(pts: &[(f32, f32)]) -> f32 {
    let n = pts.len();
    let mut s = 0.0_f32;
    for i in 0..n {
        let (ax, ay) = pts[i];
        let (bx, by) = pts[(i + 1) % n];
        s += ax * by - bx * ay;
    }
    s * 0.5
}

// ── Handler ────────────────────────────────────────────────────────────────

pub async fn extrude_sketch_endpoint(
    Json(req): Json<ExtrudeSketchRequest>,
) -> std::result::Result<Json<SketchMeshResponse>, (StatusCode, Json<ApiError>)> {
    tracing::info!(
        "🔧 sketch/extrude [own kernel]: plane={}, depth={:.4}m, pts={}",
        req.plane, req.depth, req.profile.len()
    );

    // ── 1. Validate ────────────────────────────────────────────────────────
    if req.profile.len() < 3 {
        return Err((StatusCode::BAD_REQUEST, Json(ApiError {
            error: format!("profile needs >=3 points, got {}", req.profile.len()),
        })));
    }
    if !(req.depth.is_finite() && req.depth > 0.0) {
        return Err((StatusCode::BAD_REQUEST, Json(ApiError {
            error: format!("depth must be positive finite, got {}", req.depth),
        })));
    }

    let plane = req.plane.to_uppercase();

    // ── 2. Project 3D profile → 2D based on sketch plane ──────────────────
    //   XY: (x, y),  extrude along Z
    //   XZ: (x, z),  extrude along Y  ← most common in our CAD
    //   YZ: (y, z),  extrude along X
    let mut pts2: Vec<(f32, f32)> = req.profile.iter().map(|p| match plane.as_str() {
        "XY" => (p.x as f32, p.y as f32),
        "YZ" => (p.y as f32, p.z as f32),
        _    => (p.x as f32, p.z as f32), // XZ
    }).collect();

    // Enforce CCW (our kernel expects it).
    if signed_area_2d(&pts2) < 0.0 {
        pts2.reverse();
    }

    // Remove duplicate closing vertex if present.
    if pts2.len() > 3 {
        let first = pts2[0];
        let last = *pts2.last().unwrap();
        if (first.0 - last.0).abs() < 1e-7 && (first.1 - last.1).abs() < 1e-7 {
            pts2.pop();
        }
    }

    if pts2.len() < 3 {
        return Err((StatusCode::BAD_REQUEST, Json(ApiError {
            error: "profile collapsed to <3 unique points".into(),
        })));
    }

    // ── 3. Build Point2 slice ──────────────────────────────────────────────
    let kernel_pts: Vec<Point2> = pts2.iter().map(|(u, v)| Point2::new(*u, *v)).collect();

    let depth = req.depth as f32;
    let bevel = req.bevel.unwrap_or(0.0) as f32;

    // ── 4. Extrude via own kernel ─────────────────────────────────────────
    let parts = extrude_polygon(&kernel_pts, &ExtrudeOptions { depth, bevel })
        .map_err(|e| (StatusCode::UNPROCESSABLE_ENTITY, Json(ApiError {
            error: format!("extrude_polygon: {e:?}"),
        })))?;

    // ── 5. Flatten MeshParts into GPU arrays ──────────────────────────────
    // parts[0] = front cap (+depth/2)
    // parts[1] = back cap  (-depth/2)
    // parts[2] = side walls
    // face_id: 1=front, 2=back, 3=sides
    let mut positions: Vec<f32> = Vec::new();
    let mut normals:   Vec<f32> = Vec::new();
    let mut indices:   Vec<u32> = Vec::new();
    let mut face_ids:  Vec<u32> = Vec::new();

    // Plane offset: where the sketch sits in the perpendicular axis.
    let plane_offset: f32 = match plane.as_str() {
        "XY" => req.profile.first().map(|p| p.z as f32).unwrap_or(0.0),
        "YZ" => req.profile.first().map(|p| p.x as f32).unwrap_or(0.0),
        _    => req.profile.first().map(|p| p.y as f32).unwrap_or(0.0),
    };

    for (part_idx, part) in parts.iter().enumerate() {
        let face_id = (part_idx + 1) as u32;
        let v_offset = (positions.len() / 3) as u32;

        for v in &part.vertices {
            // kernel coords: v[0]=u, v[1]=v_coord, v[2]=depth_axis
            // Lift back to 3D world space.
            let (wx, wy, wz) = match plane.as_str() {
                "XY" => (v[0], v[1], plane_offset + v[2]),
                "YZ" => (plane_offset + v[2], v[0], v[1]),
                _    => (v[0], plane_offset + v[2], v[1]), // XZ
            };
            positions.push(wx);
            positions.push(wy);
            positions.push(wz);
        }

        for n in &part.normals {
            let (nx, ny, nz) = match plane.as_str() {
                "XY" => (n[0], n[1], n[2]),
                "YZ" => (n[2], n[0], n[1]),
                _    => (n[0], n[2], n[1]), // XZ
            };
            normals.push(nx);
            normals.push(ny);
            normals.push(nz);
        }

        for tri in &part.faces {
            indices.push(v_offset + tri[0] as u32);
            indices.push(v_offset + tri[1] as u32);
            indices.push(v_offset + tri[2] as u32);
            face_ids.push(face_id);
        }
    }

    // ── 6. Minimal OBJ for download / debug ───────────────────────────────
    let vc = positions.len() / 3;
    let mut obj_data = String::with_capacity(vc * 32);
    obj_data.push_str("# geometry-kernel extrude\n");
    for i in 0..vc {
        let b = i * 3;
        obj_data.push_str(&format!("v {} {} {}\n", positions[b], positions[b+1], positions[b+2]));
    }
    for i in 0..vc {
        let b = i * 3;
        obj_data.push_str(&format!("vn {} {} {}\n", normals[b], normals[b+1], normals[b+2]));
    }
    let tc = indices.len() / 3;
    for t in 0..tc {
        let b = t * 3;
        let (a, bb, c) = (indices[b]+1, indices[b+1]+1, indices[b+2]+1);
        obj_data.push_str(&format!("f {a}//{a} {bb}//{bb} {c}//{c}\n"));
    }

    Ok(Json(SketchMeshResponse {
        vertex_count: vc,
        triangle_count: tc,
        positions,
        normals,
        indices,
        face_ids,
        face_count: 3,
        obj_data,
        kernel: "geometry-kernel",
    }))
}

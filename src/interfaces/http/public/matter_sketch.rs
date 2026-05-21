//! Sketch → Extrude endpoint.
//!
//! Accepts a sketch profile + plane + depth from the WebGPU frontend and
//! returns a triangulated solid produced by `geometry_engine`
//! (the first-party CAD crate with f64-precision internals).
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

use geometry_engine::{extrude_polygon, ExtrudeOptions, Point2};

// ── Vertex welding ────────────────────────────────────────────────────────────
/// Merge vertices that are closer than `tol` (metres) and remap indices.
/// Returns (welded_positions, welded_normals, welded_face_ids, remapped_indices).
fn weld_vertices(
    positions: &[f32],
    normals:   &[f32],
    face_ids:  &[u32],
    indices:   &[u32],
    tol:       f32,
) -> (Vec<f32>, Vec<f32>, Vec<u32>, Vec<u32>) {
    let vc = positions.len() / 3;
    let tol2 = tol * tol;

    // old_vtx → new_vtx mapping
    let mut remap: Vec<u32> = Vec::with_capacity(vc);
    let mut new_pos:  Vec<f32> = Vec::new();
    let mut new_nrm:  Vec<f32> = Vec::new();
    let mut new_fids: Vec<u32> = Vec::new();

    'outer: for i in 0..vc {
        let px = positions[i * 3];
        let py = positions[i * 3 + 1];
        let pz = positions[i * 3 + 2];
        let fid_i = face_ids[i];
        let nx_i  = normals[i * 3];
        let ny_i  = normals[i * 3 + 1];
        let nz_i  = normals[i * 3 + 2];
        // Search existing new verts for a match — same position, same face_id AND
        // similar normal direction (dot > 0.95).  This prevents corner vertices of
        // different side walls (all face_id=3) from being welded together, which
        // would corrupt the normal and produce false diagonal averaged normals that
        // cause face_metadata to split a box into 8 instead of 6 logical faces.
        let nv = new_pos.len() / 3;
        for j in 0..nv {
            if new_fids[j] != fid_i { continue; }
            let dx = new_pos[j*3]   - px;
            let dy = new_pos[j*3+1] - py;
            let dz = new_pos[j*3+2] - pz;
            if dx*dx + dy*dy + dz*dz > tol2 { continue; }
            // normal similarity guard — do not weld across wall directions
            let dot = new_nrm[j*3]*nx_i + new_nrm[j*3+1]*ny_i + new_nrm[j*3+2]*nz_i;
            if dot < 0.95 { continue; }
            remap.push(j as u32);
            continue 'outer;
        }
        remap.push(nv as u32);
        new_pos.push(px); new_pos.push(py); new_pos.push(pz);
        new_nrm.push(normals[i*3]); new_nrm.push(normals[i*3+1]); new_nrm.push(normals[i*3+2]);
        new_fids.push(face_ids[i]);
    }

    let new_idx: Vec<u32> = indices.iter().map(|&i| remap[i as usize]).collect();

    // Remove degenerate triangles produced after welding (a==b or b==c or a==c)
    let mut clean_idx: Vec<u32> = Vec::with_capacity(new_idx.len());
    for tri in new_idx.chunks(3) {
        if tri[0] != tri[1] && tri[1] != tri[2] && tri[0] != tri[2] {
            clean_idx.extend_from_slice(tri);
        }
    }

    (new_pos, new_nrm, new_fids, clean_idx)
}

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

fn signed_area_2d(pts: &[(f64, f64)]) -> f64 {
    let n = pts.len();
    let mut s = 0.0_f64;
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
    let mut pts2: Vec<(f64, f64)> = req.profile.iter().map(|p| match plane.as_str() {
        "XY" => (p.x, p.y),
        "YZ" => (p.y, p.z),
        _    => (p.x, p.z), // XZ
    }).collect();

    // Enforce CCW (our kernel expects it).
    if signed_area_2d(&pts2) < 0.0 {
        pts2.reverse();
    }

    // Remove duplicate closing vertex if present.
    if pts2.len() > 3 {
        let first = pts2[0];
        let last = *pts2.last().unwrap();
        if (first.0 - last.0).abs() < 1e-9 && (first.1 - last.1).abs() < 1e-9 {
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

    let depth = req.depth;
    let bevel = req.bevel.unwrap_or(0.0);

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

    // Plane offset: where the sketch sits in the perpendicular axis (f64).
    let plane_offset: f64 = match plane.as_str() {
        "XY" => req.profile.first().map(|p| p.z).unwrap_or(0.0),
        "YZ" => req.profile.first().map(|p| p.x).unwrap_or(0.0),
        _    => req.profile.first().map(|p| p.y).unwrap_or(0.0),
    };

    for (part_idx, part) in parts.iter().enumerate() {
        let face_id = (part_idx + 1) as u32;
        let v_offset = (positions.len() / 3) as u32;

        for v in &part.vertices {
            let (wx, wy, wz): (f64, f64, f64) = match plane.as_str() {
                "XY" => (v[0], v[1], plane_offset + v[2]),
                "YZ" => (plane_offset + v[2], v[0], v[1]),
                _    => (v[0], plane_offset + v[2], v[1]), // XZ
            };
            positions.push(wx as f32);
            positions.push(wy as f32);
            positions.push(wz as f32);
            face_ids.push(face_id);
        }

        for n in &part.normals {
            let (nx, ny, nz): (f64, f64, f64) = match plane.as_str() {
                "XY" => (n[0], n[1], n[2]),
                "YZ" => (n[2], n[0], n[1]),
                _    => (n[0], n[2], n[1]), // XZ
            };
            normals.push(nx as f32);
            normals.push(ny as f32);
            normals.push(nz as f32);
        }

        for tri in &part.faces {
            indices.push(v_offset + tri[0] as u32);
            indices.push(v_offset + tri[1] as u32);
            indices.push(v_offset + tri[2] as u32);
        }
    }

    // ── 5b. Weld shared boundary vertices → closed manifold ──────────────
    // Tolerance: adaptive = bbox_diagonal * 1e-4  (handles jewellery µm scale)
    // Minimum 1e-9 m, maximum 1e-4 m.
    let weld_tol: f32 = {
        let xs = positions.iter().step_by(3);
        let ys = positions.iter().skip(1).step_by(3);
        let zs = positions.iter().skip(2).step_by(3);
        let inf = f32::INFINITY;
        let (mnx, mxx) = xs.fold((inf, -inf), |(a,b), &v| (a.min(v), b.max(v)));
        let (mny, mxy) = ys.fold((inf, -inf), |(a,b), &v| (a.min(v), b.max(v)));
        let (mnz, mxz) = zs.fold((inf, -inf), |(a,b), &v| (a.min(v), b.max(v)));
        let dx = mxx - mnx; let dy = mxy - mny; let dz = mxz - mnz;
        let diag = (dx*dx + dy*dy + dz*dz).sqrt();
        (diag * 1e-4_f32).clamp(1e-9_f32, 1e-4_f32)
    };
    let (positions, normals, face_ids, indices) =
        weld_vertices(&positions, &normals, &face_ids, &indices, weld_tol);    // ── 6. Minimal OBJ for download / debug ───────────────────────────────
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

// ── Rounded-rectangle extrude ─────────────────────────────────────────────────
//
// POST /api/matter/sketch/rounded-rect
// {
//   "plane": "XZ",
//   "depth": 0.02,
//   "width": 0.1,
//   "height": 0.05,
//   "corner_radius": 0.01,
//   "segments": 8          // arc segments per corner, default 8
// }

#[derive(Deserialize)]
pub struct RoundedRectRequest {
    pub plane:         String,
    pub depth:         f64,
    pub width:         f64,
    pub height:        f64,
    pub corner_radius: f64,
    #[serde(default = "default_segments")]
    pub segments:      usize,
}

fn default_segments() -> usize { 8 }

pub async fn rounded_rect_endpoint(
    Json(req): Json<RoundedRectRequest>,
) -> std::result::Result<Json<SketchMeshResponse>, (StatusCode, Json<ApiError>)> {
    // ── Validate ────────────────────────────────────────────────────────────
    let max_r = req.width.min(req.height) / 2.0;
    if req.corner_radius <= 0.0 || !req.corner_radius.is_finite() {
        return Err((StatusCode::BAD_REQUEST, Json(ApiError {
            error: format!("InvalidRadius: corner_radius must be positive finite, got {}", req.corner_radius),
        })));
    }
    if req.corner_radius > max_r {
        return Err((StatusCode::BAD_REQUEST, Json(ApiError {
            error: format!(
                "InvalidRadius: corner_radius {:.6} exceeds max allowed {:.6} (= min(width,height)/2)",
                req.corner_radius, max_r,
            ),
        })));
    }
    if !(req.depth > 0.0 && req.depth.is_finite()) {
        return Err((StatusCode::BAD_REQUEST, Json(ApiError {
            error: format!("depth must be positive finite, got {}", req.depth),
        })));
    }
    if req.segments < 1 {
        return Err((StatusCode::BAD_REQUEST, Json(ApiError {
            error: "segments must be >= 1".into(),
        })));
    }

    let r  = req.corner_radius;
    let w  = req.width;
    let h  = req.height;
    let seg = req.segments;

    // ── Build CCW polygon in (u, v) = (x, z) for XZ-plane ──────────────────
    // Origin at (0, 0), rectangle spans [0,w] × [0,h].
    // Four corners; each arc sweeps π/2.  Start angle for each:
    //   bottom-right (w-r, r):   -π/2 → 0
    //   top-right    (w-r, h-r):  0   → π/2
    //   top-left     (r,   h-r):  π/2 → π
    //   bottom-left  (r,   r):    π   → 3π/2
    use std::f64::consts::PI;

    let corners: [(f64, f64, f64, f64); 4] = [
        (w - r, r,   -PI / 2.0, 0.0),
        (w - r, h-r,  0.0,      PI / 2.0),
        (r,     h-r,  PI / 2.0, PI),
        (r,     r,    PI,       3.0 * PI / 2.0),
    ];

    let mut pts2: Vec<(f64, f64)> = Vec::with_capacity(seg * 4);
    for &(cx, cy, a0, a1) in &corners {
        for i in 0..=seg {
            let t     = i as f64 / seg as f64;
            let angle = a0 + t * (a1 - a0);
            pts2.push((cx + r * angle.cos(), cy + r * angle.sin()));
        }
    }
    // Remove duplicate closing vertex if first ≈ last
    if let (Some(&first), Some(&last)) = (pts2.first(), pts2.last()) {
        if (first.0 - last.0).hypot(first.1 - last.1) < 1e-12 {
            pts2.pop();
        }
    }
    // Enforce CCW
    if signed_area_2d(&pts2) < 0.0 { pts2.reverse(); }

    let kernel_pts: Vec<Point2> = pts2.iter().map(|(u, v)| Point2::new(*u, *v)).collect();

    let depth = req.depth;
    let parts = extrude_polygon(&kernel_pts, &ExtrudeOptions { depth, bevel: 0.0 })
        .map_err(|e| (StatusCode::UNPROCESSABLE_ENTITY, Json(ApiError {
            error: format!("extrude_polygon: {e:?}"),
        })))?;

    // ── Flatten MeshParts (same logic as extrude_sketch_endpoint) ───────────
    let plane = req.plane.to_uppercase();
    let plane_offset: f64 = 0.0;

    let mut positions: Vec<f32> = Vec::new();
    let mut normals:   Vec<f32> = Vec::new();
    let mut indices:   Vec<u32> = Vec::new();
    let mut face_ids:  Vec<u32> = Vec::new();

    for (part_idx, part) in parts.iter().enumerate() {
        let face_id  = (part_idx + 1) as u32;
        let v_offset = (positions.len() / 3) as u32;
        for v in &part.vertices {
            let (wx, wy, wz): (f64, f64, f64) = match plane.as_str() {
                "XY" => (v[0], v[1], plane_offset + v[2]),
                "YZ" => (plane_offset + v[2], v[0], v[1]),
                _    => (v[0], plane_offset + v[2], v[1]), // XZ
            };
            positions.push(wx as f32); positions.push(wy as f32); positions.push(wz as f32);
            face_ids.push(face_id);
        }
        for n in &part.normals {
            let (nx, ny, nz): (f64, f64, f64) = match plane.as_str() {
                "XY" => (n[0], n[1], n[2]),
                "YZ" => (n[2], n[0], n[1]),
                _    => (n[0], n[2], n[1]),
            };
            normals.push(nx as f32); normals.push(ny as f32); normals.push(nz as f32);
        }
        for tri in &part.faces {
            indices.push(v_offset + tri[0] as u32);
            indices.push(v_offset + tri[1] as u32);
            indices.push(v_offset + tri[2] as u32);
        }
    }

    // Adaptive weld tolerance: 1e-4 × bbox diagonal
    let (min_p, max_p) = positions.chunks(3).fold(
        ([f32::INFINITY; 3], [f32::NEG_INFINITY; 3]),
        |(mn, mx), c| {
            ([mn[0].min(c[0]), mn[1].min(c[1]), mn[2].min(c[2])],
             [mx[0].max(c[0]), mx[1].max(c[1]), mx[2].max(c[2])])
        },
    );
    let diag = ((max_p[0]-min_p[0]).powi(2) + (max_p[1]-min_p[1]).powi(2)
               + (max_p[2]-min_p[2]).powi(2)).sqrt();
    let weld_tol = (diag * 1e-4_f32).max(1e-12_f32);

    let (positions, normals, face_ids, indices) =
        weld_vertices(&positions, &normals, &face_ids, &indices, weld_tol);

    let vc = positions.len() / 3;
    let tc = indices.len() / 3;

    tracing::info!(
        "🔧 sketch/rounded-rect: plane={}, {:.4}×{:.4}m r={:.6}m depth={:.4}m → {vc}v {tc}t",
        plane, w, h, r, depth
    );

    Ok(Json(SketchMeshResponse {
        vertex_count:   vc,
        triangle_count: tc,
        positions,
        normals,
        indices,
        face_ids,
        face_count: 3,
        obj_data:   String::new(),
        kernel:     "geometry-kernel",
    }))
}

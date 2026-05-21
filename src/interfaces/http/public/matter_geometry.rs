//! Geometry-engine operations exposed as HTTP endpoints.
//!
//! # Endpoints
//!
//! ## POST /api/matter/geometry/boolean
//! Run a CSG boolean operation on two             let fid = tri_meta.map(|m| m.face_id.id.0 as u32).unwrap_or(1);xtruded solids.
//!
//! ```json
//! {
//!   "op": "subtract",            // "union" | "subtract" | "intersect"
//!   "a": {
//!     "plane": "XZ", "depth": 0.1,
//!     "profile": [{"x":0,"y":0,"z":0}, ...]
//!   },
//!   "b": {
//!     "plane": "XZ", "depth": 0.05,
//!     "profile": [{"x":0.02,"y":0,"z":0.02}, ...]
//!   }
//! }
//! ```
//! Response: same GPU-ready arrays as `/api/matter/sketch/extrude`.
//!
//! ## POST /api/matter/geometry/tessellate
//! Re-tessellate a B-Rep model that was previously produced by a boolean op
//! (pass back `brep_json` from a prior boolean response, or just re-extrude
//! with different options — this endpoint is intentionally a superset).

use axum::{Json, http::StatusCode};
use serde::{Deserialize, Serialize};

// ── Vertex welding (same as matter_sketch) ────────────────────────────────────
fn weld_vertices(
    positions: &[f32],
    normals:   &[f32],
    face_ids:  &[u32],
    indices:   &[u32],
    tol:       f32,
) -> (Vec<f32>, Vec<f32>, Vec<u32>, Vec<u32>) {
    let vc   = positions.len() / 3;
    let tol2 = tol * tol;
    let mut remap: Vec<u32>  = Vec::with_capacity(vc);
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
        new_pos.push(px);  new_pos.push(py);  new_pos.push(pz);
        new_nrm.push(normals[i*3]); new_nrm.push(normals[i*3+1]); new_nrm.push(normals[i*3+2]);
        new_fids.push(face_ids[i]);
    }
    let mapped: Vec<u32> = indices.iter().map(|&i| remap[i as usize]).collect();
    let mut clean: Vec<u32> = Vec::with_capacity(mapped.len());
    for tri in mapped.chunks(3) {
        if tri[0] != tri[1] && tri[1] != tri[2] && tri[0] != tri[2] {
            clean.extend_from_slice(tri);
        }
    }
    (new_pos, new_nrm, new_fids, clean)
}

/// Ensure all triangle normals point outward (away from mesh centroid).
/// For each triangle: compute face normal from geometry; if dot(normal, centroid→face) < 0 → flip.
fn fix_winding_outward(positions: &[f32], normals: &mut Vec<f32>, indices: &mut Vec<u32>) {
    let vc = positions.len() / 3;
    if vc == 0 { return; }

    // Mesh centroid
    let mut cx = 0.0_f32; let mut cy = 0.0_f32; let mut cz = 0.0_f32;
    for i in 0..vc {
        cx += positions[i*3]; cy += positions[i*3+1]; cz += positions[i*3+2];
    }
    let inv = 1.0 / vc as f32;
    cx *= inv; cy *= inv; cz *= inv;

    for tri in indices.chunks_mut(3) {
        let i0 = tri[0] as usize;
        let i1 = tri[1] as usize;
        let i2 = tri[2] as usize;

        let ax = positions[i0*3]; let ay = positions[i0*3+1]; let az = positions[i0*3+2];
        let bx = positions[i1*3]; let by = positions[i1*3+1]; let bz = positions[i1*3+2];
        let dx = positions[i2*3]; let dy = positions[i2*3+1]; let dz = positions[i2*3+2];

        // Face normal via cross product
        let ex = bx-ax; let ey = by-ay; let ez = bz-az;
        let fx = dx-ax; let fy = dy-ay; let fz = dz-az;
        let nx = ey*fz - ez*fy;
        let ny = ez*fx - ex*fz;
        let nz = ex*fy - ey*fx;

        // Vector from centroid to triangle centre
        let tx = (ax+bx+dx)/3.0 - cx;
        let ty = (ay+by+dy)/3.0 - cy;
        let tz = (az+bz+dz)/3.0 - cz;

        // If inward → flip triangle and average normals
        if nx*tx + ny*ty + nz*tz < 0.0 {
            tri.swap(1, 2);
            // Flip stored vertex normals for these 3 verts
            for &vi in &[i0, i1, i2] {
                normals[vi*3]   = -normals[vi*3];
                normals[vi*3+1] = -normals[vi*3+1];
                normals[vi*3+2] = -normals[vi*3+2];
            }
        }
    }
}

use geometry_engine::{
    extrude_polygon_brep, ExtrudeBrepResult, ExtrudeOptions, Point2,
    boolean::{boolean_subtract, boolean_intersect},
    tessellation::{tessellate_body, TessOptions},
};

// ── Shared types ──────────────────────────────────────────────────────────────

#[derive(Deserialize, Clone)]
pub struct ProfilePoint {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

/// Specification for one extruded solid (same schema as sketch/extrude).
#[derive(Deserialize, Clone)]
pub struct SolidSpec {
    pub plane: String,
    pub depth: f64,
    pub profile: Vec<ProfilePoint>,
    #[serde(default)]
    pub bevel: Option<f64>,
}

#[derive(Serialize)]
pub struct GeometryMeshResponse {
    pub ok: bool,
    pub op: String,
    pub vertex_count: usize,
    pub triangle_count: usize,
    pub face_count: usize,
    pub positions: Vec<f32>,
    pub normals: Vec<f32>,
    pub indices: Vec<u32>,
    pub face_ids: Vec<u32>,
    pub kernel: &'static str,
}

#[derive(Serialize)]
pub struct ApiError {
    pub error: String,
}

type AppResult<T> = std::result::Result<T, (StatusCode, Json<ApiError>)>;

// ── Helpers ───────────────────────────────────────────────────────────────────

fn bad(msg: impl Into<String>) -> (StatusCode, Json<ApiError>) {
    (StatusCode::BAD_REQUEST, Json(ApiError { error: msg.into() }))
}

fn unprocessable(msg: impl Into<String>) -> (StatusCode, Json<ApiError>) {
    (StatusCode::UNPROCESSABLE_ENTITY, Json(ApiError { error: msg.into() }))
}

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

/// Build a `BrepModel` from a `SolidSpec`.
fn spec_to_brep(spec: &SolidSpec, label: &str) -> AppResult<ExtrudeBrepResult> {
    if spec.profile.len() < 3 {
        return Err(bad(format!("{label}: profile needs >=3 points, got {}", spec.profile.len())));
    }
    if !(spec.depth.is_finite() && spec.depth > 0.0) {
        return Err(bad(format!("{label}: depth must be positive finite, got {}", spec.depth)));
    }

    let plane = spec.plane.to_uppercase();

    let mut pts2: Vec<(f64, f64)> = spec.profile.iter().map(|p| match plane.as_str() {
        "XY" => (p.x, p.y),
        "YZ" => (p.y, p.z),
        _    => (p.x, p.z), // XZ default
    }).collect();

    if signed_area_2d(&pts2) < 0.0 { pts2.reverse(); }

    // Remove duplicate closing vertex
    if pts2.len() > 3 {
        let first = pts2[0];
        let last  = *pts2.last().unwrap();
        if (first.0 - last.0).abs() < 1e-9 && (first.1 - last.1).abs() < 1e-9 {
            pts2.pop();
        }
    }

    if pts2.len() < 3 {
        return Err(bad(format!("{label}: profile collapsed to <3 unique points")));
    }

    let kernel_pts: Vec<Point2> = pts2.iter().map(|(u, v)| Point2::new(*u, *v)).collect();
    let bevel = spec.bevel.unwrap_or(0.0);

    extrude_polygon_brep(&kernel_pts, &ExtrudeOptions { depth: spec.depth, bevel })
        .map_err(|e| unprocessable(format!("{label}: extrude_polygon_brep: {e:?}")))
}

// ── Boolean endpoint ──────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct BooleanRequest {
    pub op: String,
    pub a: SolidSpec,
    pub b: SolidSpec,
}

pub async fn boolean_endpoint(
    Json(req): Json<BooleanRequest>,
) -> AppResult<Json<GeometryMeshResponse>> {
    let op = req.op.to_lowercase();
    tracing::info!(
        "🔧 geometry/boolean: op={op}, plane_a={}, depth_a={:.4}m, pts_a={}, plane_b={}, depth_b={:.4}m, pts_b={}",
        req.a.plane, req.a.depth, req.a.profile.len(),
        req.b.plane, req.b.depth, req.b.profile.len(),
    );

    // ── 1. Build both BrepModels ─────────────────────────────────────────
    let brep_a = spec_to_brep(&req.a, "solid_a")?;
    let brep_b = spec_to_brep(&req.b, "solid_b")?;

    // ── 2. Run boolean op ────────────────────────────────────────────────
    let result_model = match op.as_str() {
        "subtract"  => boolean_subtract(&brep_a.model, &brep_b.model),
        "intersect" => boolean_intersect(&brep_a.model, &brep_b.model),
        "union"     => {
            // union not yet fully implemented — fall back to solid A for now
            tracing::warn!("geometry/boolean: union not yet fully implemented, returning solid A");
            // Re-extrude solid A as stand-in result
            let fallback = spec_to_brep(&req.a, "union_fallback")?;
            fallback.model
        }
        other => return Err(bad(format!("unknown op '{other}'; expected union|subtract|intersect"))),
    };

    // ── 3. Tessellate result ─────────────────────────────────────────────
    let body_id = result_model
        .store
        .bodies
        .keys()
        .next()
        .copied()
        .ok_or_else(|| unprocessable("boolean produced an empty model (no bodies)"))?;

    let opts     = TessOptions::default();
    let mesh_meta = tessellate_body(&result_model, body_id, &opts);
    let mesh      = &mesh_meta.mesh;

    // ── 4. Flatten to GPU arrays ─────────────────────────────────────────
    let plane = req.a.plane.to_uppercase();
    let plane_s = plane.as_str();

    let mut positions: Vec<f32> = Vec::with_capacity(mesh.vertices.len() * 3);
    let mut normals:   Vec<f32> = Vec::with_capacity(mesh.normals.len() * 3);
    let mut indices:   Vec<u32> = Vec::with_capacity(mesh.faces.len() * 3);
    let mut face_ids:  Vec<u32> = Vec::with_capacity(mesh.vertices.len());

    for v in &mesh.vertices {
        let (wx, wy, wz): (f64, f64, f64) = match plane_s {
            "XY" => (v[0], v[1], v[2]),
            "YZ" => (v[2], v[0], v[1]),
            _    => (v[0], v[2], v[1]),
        };
        positions.push(wx as f32);
        positions.push(wy as f32);
        positions.push(wz as f32);
        face_ids.push(1); // per-vertex: simplified (all face 1 for now)
    }

    // Per-vertex face_id from triangle metadata (each triangle → 3 vertices)
    // Rebuild correctly: one face_id per vertex = face_id of the triangle it belongs to
    {
        let vc = positions.len() / 3;
        face_ids.clear();
        face_ids.resize(vc, 1u32);
        for (tri_idx, tri) in mesh.faces.iter().enumerate() {
            let fid = mesh_meta.triangles
                .get(tri_idx)
                .map(|m| m.face_id.id.0 as u32)
                .unwrap_or(1);
            face_ids[tri[0]] = fid;
            face_ids[tri[1]] = fid;
            face_ids[tri[2]] = fid;
        }
    }

    for n in &mesh.normals {
        let (nx, ny, nz): (f64, f64, f64) = match plane_s {
            "XY" => (n[0], n[1], n[2]),
            "YZ" => (n[2], n[0], n[1]),
            _    => (n[0], n[2], n[1]),
        };
        normals.push(nx as f32);
        normals.push(ny as f32);
        normals.push(nz as f32);
    }

    for tri in &mesh.faces {
        indices.push(tri[0] as u32);
        indices.push(tri[1] as u32);
        indices.push(tri[2] as u32);
    }

    let vc = positions.len() / 3;
    let tc = indices.len() / 3;

    // ── 5. Weld (adaptive tol = bbox_diag * 1e-4, clamp 1e-9..1e-4) ─────
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
    let (positions, normals, face_ids, mut indices) =
        weld_vertices(&positions, &normals, &face_ids, &indices, weld_tol);

    // ── 6. Ensure consistent outward winding ─────────────────────────────
    let mut normals = normals;
    fix_winding_outward(&positions, &mut normals, &mut indices);

    let vc = positions.len() / 3;
    let tc = indices.len() / 3;

    // Count unique face IDs
    let mut unique_faces: Vec<u32> = face_ids.clone();
    unique_faces.sort_unstable();
    unique_faces.dedup();
    let fc = unique_faces.len();

    tracing::info!(
        "✅ geometry/boolean[{op}]: {vc} verts, {tc} tris, {fc} faces"
    );

    Ok(Json(GeometryMeshResponse {
        ok: true,
        op,
        vertex_count:   vc,
        triangle_count: tc,
        face_count:     fc,
        positions,
        normals,
        indices,
        face_ids,
        kernel: "geometry-engine-boolean",
    }))
}

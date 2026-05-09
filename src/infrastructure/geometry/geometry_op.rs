//! geometry_op dispatcher — converts a GeminiGeometryOp JSON spec into GLB bytes.
//!
//! Called from `POST /api/laboratory/geometry-op`.
//!
//! Input JSON (GeminiGeometryOp):
//! ```json
//! {
//!   "operation": "subtract",
//!   "target":  { "type": "shape_cube",   "color": "#38BDF8", "size": 1.0 },
//!   "cutter":  { "type": "cylinder",     "radius": 0.25, "height": 1.5, "center": [0,0,0] },
//!   "quality": "high"
//! }
//! ```
//!
//! Returns raw GLB bytes (model/gltf-binary).

use crate::infrastructure::geometry::dispatcher::dispatch_with_quality;
use crate::infrastructure::geometry::kernel::csg::{subtract, Aabb};
use crate::infrastructure::geometry::kernel::quality::GeometryQuality;
use crate::infrastructure::geometry::mesh::hex_to_rgb;
use crate::infrastructure::geometry::mesh::Material;
use crate::shared::AppError;
use serde::{Deserialize, Serialize};

// ─────────────────────────────────────────────────────────────────────────────
// Input schema
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GeometryOpRequest {
    /// CSG operation. Supported: "subtract", "union" (no-op union = merge groups).
    pub operation: String,
    /// Primary shape (the thing being cut into).
    pub target: ShapeSpec,
    /// Cutter / tool shape.
    pub cutter: CutterSpec,
    #[serde(default)]
    pub quality: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ShapeSpec {
    /// Dispatcher slug: "shape_cube", "shape_sphere", "sci_fi_card", etc.
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub color: Option<String>,
    /// Uniform scale multiplier (default 1.0).
    #[serde(default)]
    pub size: Option<f32>,
    /// Grid subdivisions per face axis (1 = 2×2 quads/face, 4 = 5×5).
    /// Higher = more geometry, needed for smooth bevel.
    #[serde(default)]
    pub subdivisions: Option<u32>,
    /// Corner bevel strength 0.0 (sharp) … 1.0 (sphere-like).
    /// Requires subdivisions ≥ 2 to look smooth.
    #[serde(default)]
    pub bevel: Option<f32>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CutterSpec {
    /// "cylinder" | "box" | "sphere" | dispatcher slug
    #[serde(rename = "type")]
    pub kind: String,
    /// Cylinder: radius in metres
    #[serde(default)]
    pub radius: Option<f32>,
    /// Cylinder/box: height
    #[serde(default)]
    pub height: Option<f32>,
    /// Box: [w, h, d] half-extents
    #[serde(default)]
    pub half_extents: Option<[f32; 3]>,
    /// World-space centre of the cutter (default [0,0,0])
    #[serde(default)]
    pub center: Option<[f32; 3]>,
    /// Material colour for the cap face
    #[serde(default)]
    pub cap_color: Option<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Execution
// ─────────────────────────────────────────────────────────────────────────────

/// Execute a geometry operation and return GLB bytes.
pub fn execute_geometry_op(req: &GeometryOpRequest) -> Result<Vec<u8>, AppError> {
    use crate::infrastructure::geometry::gltf_exporter::export_glb;

    let quality = GeometryQuality::from_opt(req.quality.as_deref());

    // Build the spec JSON for the target shape
    let target_spec = build_target_spec(&req.target);
    let mut mesh = dispatch_with_quality(&req.target.kind, target_spec.as_ref(), quality)?;

    match req.operation.as_str() {
        "subtract" => {
            let mut cutter_aabb = build_cutter_aabb(&req.cutter);
            // Clamp cutter to mesh bounds so it never fully removes top/bottom
            // faces — otherwise the hole disappears (only side walls of AABB
            // will have boundary verts, and the cap builder finds nothing).
            clamp_aabb_to_mesh(&mut cutter_aabb, &mesh);
            let cap_material = req.cutter.cap_color.as_deref().map(|c| Material {
                name: "csg_cut_face".to_string(),
                diffuse_color: hex_to_rgb(c),
                roughness: 0.35,
                metalness: 0.1,
                opacity: 1.0,
                ..Default::default()
            });
            mesh = subtract(&mesh, &cutter_aabb, cap_material);
        }
        "union" => {
            // Union: just return the target as-is (cutter adds detail via
            // a second dispatch that we append as a new group).
            let cutter_spec = build_cutter_spec_for_dispatch(&req.cutter);
            if let Some((slug, spec)) = cutter_spec {
                if let Ok(cutter_mesh) = dispatch_with_quality(&slug, spec.as_ref(), quality) {
                    // Append cutter groups to target
                    if cutter_mesh.groups.is_empty() {
                        mesh.groups
                            .push(crate::infrastructure::geometry::mesh::MaterialGroup {
                                material: cutter_mesh.material.clone(),
                                faces: {
                                    let offset = mesh.vertices.len();
                                    mesh.vertices.extend_from_slice(&cutter_mesh.vertices);
                                    mesh.normals.extend_from_slice(&cutter_mesh.normals);
                                    mesh.uvs.extend_from_slice(&cutter_mesh.uvs);
                                    cutter_mesh
                                        .faces
                                        .iter()
                                        .map(|f| [f[0] + offset, f[1] + offset, f[2] + offset])
                                        .collect()
                                },
                            });
                    } else {
                        let offset = mesh.vertices.len();
                        mesh.vertices.extend_from_slice(&cutter_mesh.vertices);
                        mesh.normals.extend_from_slice(&cutter_mesh.normals);
                        mesh.uvs.extend_from_slice(&cutter_mesh.uvs);
                        for g in cutter_mesh.groups {
                            mesh.groups.push(
                                crate::infrastructure::geometry::mesh::MaterialGroup {
                                    material: g.material,
                                    faces: g
                                        .faces
                                        .iter()
                                        .map(|f| [f[0] + offset, f[1] + offset, f[2] + offset])
                                        .collect(),
                                },
                            );
                        }
                    }
                }
            }
        }
        op => {
            return Err(AppError::Validation(format!(
                "Unsupported geometry operation: {op}"
            )));
        }
    }

    let export = export_glb(&mesh)?;
    Ok(export.glb_bytes)
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

fn build_target_spec(shape: &ShapeSpec) -> Option<serde_json::Value> {
    let color = shape.color.as_deref().unwrap_or("#38BDF8");
    let mut obj = serde_json::json!({ "shape": { "color_hex": color } });
    if let Some(sub) = shape.subdivisions {
        obj["shape"]["subdivisions"] = serde_json::json!(sub);
    }
    if let Some(bevel) = shape.bevel {
        obj["shape"]["bevel"] = serde_json::json!(bevel);
    }
    Some(obj)
}

fn build_cutter_aabb(cutter: &CutterSpec) -> Aabb {
    let center = cutter.center.unwrap_or([0.0, 0.0, 0.0]);
    match cutter.kind.as_str() {
        "cylinder" => {
            let r = cutter.radius.unwrap_or(0.25);
            let h = cutter.height.unwrap_or(1.5);
            Aabb::cylinder(r, h, center)
        }
        "box" | "cuboid" => {
            let half = cutter.half_extents.unwrap_or([0.25, 0.25, 0.25]);
            Aabb::cuboid(half, center)
        }
        "sphere" => {
            // Approximate sphere as AABB
            let r = cutter.radius.unwrap_or(0.25);
            Aabb::cylinder(r, r * 2.0, center)
        }
        _ => {
            // Fallback: small box at center
            Aabb::cuboid([0.2, 0.2, 0.2], center)
        }
    }
}

fn build_cutter_spec_for_dispatch(
    cutter: &CutterSpec,
) -> Option<(String, Option<serde_json::Value>)> {
    match cutter.kind.as_str() {
        "cylinder" | "box" | "cuboid" | "sphere" => None, // primitives handled via AABB only
        slug => {
            let color = cutter.cap_color.as_deref().unwrap_or("#CCCCCC");
            Some((
                slug.to_string(),
                Some(serde_json::json!({ "shape": { "color_hex": color } })),
            ))
        }
    }
}

/// Clamp the cutter AABB so it never fully exceeds the mesh bounding box on
/// any axis.  We leave a 2 mm gap inside each mesh face so that the top/bottom
/// faces of the target are only *partially* removed — producing proper boundary
/// vertices for the cap builder instead of disappearing entirely.
fn clamp_aabb_to_mesh(aabb: &mut Aabb, mesh: &crate::infrastructure::geometry::mesh::Mesh) {
    const GAP: f32 = 0.002; // 2 mm inset from the mesh surface
    let mut mesh_min = [f32::MAX; 3];
    let mut mesh_max = [f32::MIN; 3];
    for v in &mesh.vertices {
        for i in 0..3 {
            mesh_min[i] = mesh_min[i].min(v[i]);
            mesh_max[i] = mesh_max[i].max(v[i]);
        }
    }
    for i in 0..3 {
        aabb.min[i] = aabb.min[i].max(mesh_min[i] + GAP);
        aabb.max[i] = aabb.max[i].min(mesh_max[i] - GAP);
    }
}

//! GLB (binary glTF 2.0) exporter — PR #8.
//!
//! Produces a single self-contained `.glb` file from a [`Mesh`]:
//!
//!   * one buffer with positions, normals, uvs and per-group indices
//!   * one mesh with N primitives (one per `MaterialGroup`)
//!   * N materials (`pbrMetallicRoughness`, `baseColorFactor` from
//!     `Material.diffuse_color`, `roughnessFactor` derived from
//!     `Material.shininess`)
//!
//! No external `gltf` crate dependency — the file is small enough to assemble
//! by hand. The hand-rolled writer is ~250 lines and well covered by tests.
//!
//! Per glTF 2.0 spec:
//!   * GLB header is 12 bytes (magic/version/length)
//!   * each chunk has 8-byte header (length / type) + 4-byte aligned payload
//!   * JSON chunk is padded with `0x20` (space)
//!   * BIN chunk is padded with `0x00`
//!   * `baseColorFactor` is in **linear** colour space (we do sRGB→linear)

use serde_json::{json, Value};

use crate::infrastructure::geometry::mesh::{Material, Mesh};
use crate::shared::AppError;

/// glTF magic — ASCII "glTF".
const GLB_MAGIC: u32 = 0x46546C67;
/// GLB container version.
const GLB_VERSION: u32 = 2;
/// Chunk type "JSON".
const CHUNK_JSON: u32 = 0x4E4F534A;
/// Chunk type "BIN\0".
const CHUNK_BIN: u32 = 0x004E4942;

/// Component type — UNSIGNED_INT (5125), used for indices.
const COMPONENT_U32: u32 = 5125;
/// Component type — FLOAT (5126).
const COMPONENT_F32: u32 = 5126;

/// Target — ARRAY_BUFFER (vertex attributes).
const TARGET_ARRAY_BUFFER: u32 = 34962;
/// Target — ELEMENT_ARRAY_BUFFER (indices).
const TARGET_ELEMENT_ARRAY_BUFFER: u32 = 34963;

/// Primitive mode — TRIANGLES.
const MODE_TRIANGLES: u32 = 4;

/// Exported GLB payload — ready to write to storage.
pub struct GltfExport {
    /// Contents of `model.glb`.
    pub glb_bytes: Vec<u8>,
}

/// Serialise `mesh` into a single binary glTF (`.glb`) byte array.
pub fn export_glb(mesh: &Mesh) -> Result<GltfExport, AppError> {
    if mesh.vertices.is_empty() {
        return Err(AppError::internal("export_glb: empty mesh"));
    }

    // Normalise: always work with `groups`. If the legacy single-material
    // path is used (groups empty), wrap it.
    let groups: Vec<(&Material, &Vec<[usize; 3]>)> = if mesh.groups.is_empty() {
        vec![(&mesh.material, &mesh.faces)]
    } else {
        mesh.groups
            .iter()
            .map(|g| (&g.material, &g.faces))
            .collect()
    };

    // ── 1. Build the binary buffer ──────────────────────────────────────────
    let mut bin: Vec<u8> = Vec::new();

    // Positions (vec3 float)
    let positions_offset = bin.len();
    let mut pos_min = [f32::INFINITY; 3];
    let mut pos_max = [f32::NEG_INFINITY; 3];
    for v in &mesh.vertices {
        for i in 0..3 {
            bin.extend_from_slice(&v[i].to_le_bytes());
            if v[i] < pos_min[i] {
                pos_min[i] = v[i];
            }
            if v[i] > pos_max[i] {
                pos_max[i] = v[i];
            }
        }
    }
    let positions_length = bin.len() - positions_offset;
    pad_to_4(&mut bin);

    // Normals (vec3 float)
    let normals_offset = bin.len();
    for n in &mesh.normals {
        for i in 0..3 {
            bin.extend_from_slice(&n[i].to_le_bytes());
        }
    }
    let normals_length = bin.len() - normals_offset;
    pad_to_4(&mut bin);

    // UVs (vec2 float)
    let uvs_offset = bin.len();
    for uv in &mesh.uvs {
        for i in 0..2 {
            bin.extend_from_slice(&uv[i].to_le_bytes());
        }
    }
    let uvs_length = bin.len() - uvs_offset;
    pad_to_4(&mut bin);

    // Indices — one buffer view per group (uint32).
    let mut index_views: Vec<(usize, usize, usize)> = Vec::new(); // (offset, length, count)
    for (_, faces) in &groups {
        let off = bin.len();
        let mut count = 0usize;
        for [a, b, c] in *faces {
            bin.extend_from_slice(&(*a as u32).to_le_bytes());
            bin.extend_from_slice(&(*b as u32).to_le_bytes());
            bin.extend_from_slice(&(*c as u32).to_le_bytes());
            count += 3;
        }
        let len = bin.len() - off;
        pad_to_4(&mut bin);
        index_views.push((off, len, count));
    }

    // ── 2. Build the JSON chunk ─────────────────────────────────────────────
    let mut buffer_views: Vec<Value> = Vec::new();
    // bufferView 0 — positions
    buffer_views.push(json!({
        "buffer": 0,
        "byteOffset": positions_offset,
        "byteLength": positions_length,
        "target": TARGET_ARRAY_BUFFER,
    }));
    // bufferView 1 — normals
    buffer_views.push(json!({
        "buffer": 0,
        "byteOffset": normals_offset,
        "byteLength": normals_length,
        "target": TARGET_ARRAY_BUFFER,
    }));
    // bufferView 2 — uvs
    buffer_views.push(json!({
        "buffer": 0,
        "byteOffset": uvs_offset,
        "byteLength": uvs_length,
        "target": TARGET_ARRAY_BUFFER,
    }));
    // bufferViews 3..3+N — per-group indices
    for (off, len, _) in &index_views {
        buffer_views.push(json!({
            "buffer": 0,
            "byteOffset": off,
            "byteLength": len,
            "target": TARGET_ELEMENT_ARRAY_BUFFER,
        }));
    }

    let mut accessors: Vec<Value> = Vec::new();
    // accessor 0 — POSITION (with required min/max)
    accessors.push(json!({
        "bufferView": 0,
        "componentType": COMPONENT_F32,
        "count": mesh.vertices.len(),
        "type": "VEC3",
        "min": pos_min,
        "max": pos_max,
    }));
    // accessor 1 — NORMAL
    accessors.push(json!({
        "bufferView": 1,
        "componentType": COMPONENT_F32,
        "count": mesh.vertices.len(),
        "type": "VEC3",
    }));
    // accessor 2 — TEXCOORD_0
    accessors.push(json!({
        "bufferView": 2,
        "componentType": COMPONENT_F32,
        "count": mesh.vertices.len(),
        "type": "VEC2",
    }));
    // accessors 3..3+N — indices
    for (i, (_, _, count)) in index_views.iter().enumerate() {
        accessors.push(json!({
            "bufferView": 3 + i,
            "componentType": COMPONENT_U32,
            "count": count,
            "type": "SCALAR",
        }));
    }

    // Materials — one per group.
    let materials: Vec<Value> = groups
        .iter()
        .map(|(mat, _)| material_to_gltf(mat))
        .collect();

    // Mesh primitives — one per group.
    let primitives: Vec<Value> = (0..groups.len())
        .map(|i| {
            json!({
                "attributes": {
                    "POSITION": 0,
                    "NORMAL": 1,
                    "TEXCOORD_0": 2,
                },
                "indices": 3 + i,
                "material": i,
                "mode": MODE_TRIANGLES,
            })
        })
        .collect();

    let gltf_json = json!({
        "asset": {
            "version": "2.0",
            "generator": "ChefOS Laboratory v2 (Rust hand-rolled GLB)",
        },
        "scene": 0,
        "scenes": [{ "nodes": [0] }],
        "nodes": [{ "mesh": 0 }],
        "meshes": [{ "primitives": primitives }],
        "materials": materials,
        "accessors": accessors,
        "bufferViews": buffer_views,
        "buffers": [{ "byteLength": bin.len() }],
    });

    let json_text = serde_json::to_string(&gltf_json).map_err(|e| {
        AppError::internal(format!("export_glb: serialize json: {e}"))
    })?;
    let mut json_bytes = json_text.into_bytes();
    // Pad JSON chunk with spaces to 4-byte alignment.
    while json_bytes.len() % 4 != 0 {
        json_bytes.push(0x20);
    }

    // BIN chunk is already padded to 4 bytes after each section.
    debug_assert!(bin.len() % 4 == 0, "bin chunk must be 4-byte aligned");

    // ── 3. Assemble the GLB container ───────────────────────────────────────
    let total_length = 12              // header
        + 8 + json_bytes.len()         // JSON chunk header + data
        + 8 + bin.len();               // BIN chunk header + data

    let mut glb: Vec<u8> = Vec::with_capacity(total_length);
    glb.extend_from_slice(&GLB_MAGIC.to_le_bytes());
    glb.extend_from_slice(&GLB_VERSION.to_le_bytes());
    glb.extend_from_slice(&(total_length as u32).to_le_bytes());

    // JSON chunk
    glb.extend_from_slice(&(json_bytes.len() as u32).to_le_bytes());
    glb.extend_from_slice(&CHUNK_JSON.to_le_bytes());
    glb.extend_from_slice(&json_bytes);

    // BIN chunk
    glb.extend_from_slice(&(bin.len() as u32).to_le_bytes());
    glb.extend_from_slice(&CHUNK_BIN.to_le_bytes());
    glb.extend_from_slice(&bin);

    debug_assert_eq!(glb.len(), total_length);

    Ok(GltfExport { glb_bytes: glb })
}

// ─────────────────────────────────────────────────────────────────────────────
// Material → glTF mapping
// ─────────────────────────────────────────────────────────────────────────────
//
// Minimal PBR mapping — PR #9 will refine glass / metal / liquid by name.
// For now:
//   * `baseColorFactor` = sRGB→linear of `diffuse_color`
//   * `metallicFactor`  = 0
//   * `roughnessFactor` = clamp(1 - shininess/200, 0.15, 0.9)
//   * `alphaMode`       = OPAQUE
//   * material `name`   = `Material.name` (frontend keys upgrades on this)

fn material_to_gltf(mat: &Material) -> Value {
    let [r, g, b] = mat.diffuse_color;
    let base_color_linear = [
        srgb_to_linear(r),
        srgb_to_linear(g),
        srgb_to_linear(b),
        1.0_f32,
    ];
    let roughness = (1.0 - mat.shininess / 200.0).clamp(0.15, 0.9);
    json!({
        "name": mat.name,
        "pbrMetallicRoughness": {
            "baseColorFactor": base_color_linear,
            "metallicFactor": 0.0_f32,
            "roughnessFactor": roughness,
        },
        "alphaMode": "OPAQUE",
        "doubleSided": false,
    })
}

/// sRGB (0..1) → linear (0..1). glTF requires linear-space colour factors.
fn srgb_to_linear(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// Pad `buf` with zeros so its length is a multiple of 4.
fn pad_to_4(buf: &mut Vec<u8>) {
    while buf.len() % 4 != 0 {
        buf.push(0x00);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::geometry::generators::{
        bottled_sauce, jar_product, sauce_in_bowl,
    };

    fn read_u32_le(b: &[u8], off: usize) -> u32 {
        u32::from_le_bytes([b[off], b[off + 1], b[off + 2], b[off + 3]])
    }

    #[test]
    fn glb_header_is_well_formed() {
        let mesh = sauce_in_bowl::generate("#B8321F", None);
        let export = export_glb(&mesh).unwrap();
        let b = &export.glb_bytes;

        assert!(b.len() > 12 + 8 + 8, "GLB must contain header + 2 chunks");
        assert_eq!(read_u32_le(b, 0), GLB_MAGIC, "magic must be glTF");
        assert_eq!(read_u32_le(b, 4), GLB_VERSION);
        assert_eq!(read_u32_le(b, 8) as usize, b.len(), "length field == total");
    }

    #[test]
    fn glb_has_two_chunks_json_then_bin() {
        let mesh = sauce_in_bowl::generate("#B8321F", None);
        let export = export_glb(&mesh).unwrap();
        let b = &export.glb_bytes;

        let json_len = read_u32_le(b, 12) as usize;
        let json_type = read_u32_le(b, 16);
        assert_eq!(json_type, CHUNK_JSON, "first chunk must be JSON");

        let bin_offset = 12 + 8 + json_len;
        let bin_len = read_u32_le(b, bin_offset) as usize;
        let bin_type = read_u32_le(b, bin_offset + 4);
        assert_eq!(bin_type, CHUNK_BIN, "second chunk must be BIN");
        assert_eq!(b.len(), bin_offset + 8 + bin_len);
    }

    #[test]
    fn glb_json_describes_one_material_per_group() {
        let mesh = bottled_sauce::generate(
            "#FF0000",
            bottled_sauce::BottleKind::Glass,
            None,
        );
        let export = export_glb(&mesh).unwrap();
        let b = &export.glb_bytes;

        let json_len = read_u32_le(b, 12) as usize;
        let json_text =
            std::str::from_utf8(&b[20..20 + json_len]).unwrap().trim_end();
        let v: Value = serde_json::from_str(json_text.trim_end_matches('\0').trim()).unwrap();

        let materials = v["materials"].as_array().unwrap();
        assert_eq!(materials.len(), 4, "bottled_sauce → 4 materials");
        let primitives = v["meshes"][0]["primitives"].as_array().unwrap();
        assert_eq!(primitives.len(), 4, "one primitive per group");

        // Material names must be preserved (frontend keys on them in PR #9).
        let names: Vec<&str> = materials
            .iter()
            .map(|m| m["name"].as_str().unwrap())
            .collect();
        assert!(names.iter().any(|n| n.contains("glass") || n.contains("plastic")));
        assert!(names.contains(&"liquid_material"));
        assert!(names.contains(&"cap_metal"));
    }

    #[test]
    fn glb_jar_product_has_three_materials() {
        let mesh = jar_product::generate("#A85B12", None);
        let export = export_glb(&mesh).unwrap();
        let b = &export.glb_bytes;
        let json_len = read_u32_le(b, 12) as usize;
        let json_text = std::str::from_utf8(&b[20..20 + json_len]).unwrap();
        let v: Value = serde_json::from_str(json_text.trim_end()).unwrap();
        assert_eq!(v["materials"].as_array().unwrap().len(), 3);
    }

    #[test]
    fn glb_position_accessor_has_min_max() {
        let mesh = sauce_in_bowl::generate("#FF0000", None);
        let export = export_glb(&mesh).unwrap();
        let b = &export.glb_bytes;
        let json_len = read_u32_le(b, 12) as usize;
        let json_text = std::str::from_utf8(&b[20..20 + json_len]).unwrap();
        let v: Value = serde_json::from_str(json_text.trim_end()).unwrap();

        let pos = &v["accessors"][0];
        assert_eq!(pos["type"], "VEC3");
        assert!(pos["min"].is_array());
        assert!(pos["max"].is_array());
    }

    #[test]
    fn glb_chunks_are_4_byte_aligned() {
        let mesh = sauce_in_bowl::generate("#FF0000", None);
        let export = export_glb(&mesh).unwrap();
        let b = &export.glb_bytes;
        let json_len = read_u32_le(b, 12) as usize;
        assert_eq!(json_len % 4, 0, "JSON chunk length must be 4-byte aligned");
        let bin_len_offset = 12 + 8 + json_len;
        let bin_len = read_u32_le(b, bin_len_offset) as usize;
        assert_eq!(bin_len % 4, 0, "BIN chunk length must be 4-byte aligned");
    }

    #[test]
    fn srgb_to_linear_known_values() {
        // Pure black / white / mid-grey sanity check.
        assert!((srgb_to_linear(0.0) - 0.0).abs() < 1e-6);
        assert!((srgb_to_linear(1.0) - 1.0).abs() < 1e-4);
        // mid-grey 0.5 sRGB ≈ 0.214 linear
        let m = srgb_to_linear(0.5);
        assert!((m - 0.214).abs() < 0.01, "got {m}");
    }
}

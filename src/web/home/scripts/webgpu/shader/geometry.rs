// ── WGSL: particle geometry types and cube mesh builder ─────────────────────────
// Domain: Particle geometry — interpolant struct (Pv) + cube mesh (CubeV, cubeVert).

pub const WGSL: &str = r##"
// ── Particle pass — instanced billboards as TRUE 3D imposters ─
struct Pv {
  @builtin(position) pos:    vec4f,
  @location(0)       quadUV: vec2f,   // -1..1 inside the quad
  @location(1)       color:  vec3f,
  @location(2)       depth:  f32,
  @location(3)       phase:  f32,
  @location(4)       wCenter: vec3f,  // world-space particle center
  @location(5)       size:   f32,     // particle radius (world units)
  @location(6) @interpolate(flat) cellMask: u32, // exposed-faces bitmask
  @location(7) @interpolate(flat) halfCell: f32, // cell half-extent (world)
  @location(8) @interpolate(flat) meshMode: u32, // 1 = cube-mesh path (flat tri), 0 = billboard ray-march
  @location(9) meshN:    vec3f, // world-space face normal (mesh mode); constant across triangle
}

// ─── Cube mesh: 36 verts = 6 faces × 2 tris × 3 verts ─────────
// Returns (localPos in [-1,1]³, outward face normal, faceBit).
// Outward CCW winding so back-face culling can be enabled later.
struct CubeV { pos: vec3f, nrm: vec3f, bit: u32 }
fn cubeVert(vi: u32) -> CubeV {
  let faceIdx = vi / 6u;            // 0..5
  let triVi   = vi % 6u;
  // Two-triangle quad indices: (0,1,2) and (0,2,3)
  var quadIdx = array<u32,6>(0u, 1u, 2u, 0u, 2u, 3u);
  let corner  = quadIdx[triVi];
  // Tangent-space corners CCW: (-,-) (+,-) (+,+) (-,+)
  var tx: f32 = -1.0;  var ty: f32 = -1.0;
  if (corner == 1u) { tx =  1.0; ty = -1.0; }
  if (corner == 2u) { tx =  1.0; ty =  1.0; }
  if (corner == 3u) { tx = -1.0; ty =  1.0; }

  let axis  = faceIdx / 2u;                                // 0=X, 1=Y, 2=Z
  let isPos = (faceIdx & 1u) == 0u;                        // even=+, odd=-
  let s     = select(-1.0, 1.0, isPos);

  var pos: vec3f;  var nrm: vec3f;  var bit: u32;
  if (axis == 0u) {
    // ±X face: outward = ±X. Flip Z on negative face for CCW winding.
    pos = vec3f(s, ty, tx * s);
    nrm = vec3f(s, 0.0, 0.0);
    bit = select(2u, 1u, isPos);
  } else if (axis == 1u) {
    pos = vec3f(tx * s, s, ty);
    nrm = vec3f(0.0, s, 0.0);
    bit = select(8u, 4u, isPos);
  } else {
    pos = vec3f(tx, ty * s, s);
    nrm = vec3f(0.0, 0.0, s);
    bit = select(32u, 16u, isPos);
  }
  return CubeV(pos, nrm, bit);
}
"##;

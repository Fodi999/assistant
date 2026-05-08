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
struct CubeV { pos: vec3f, nrm: vec3f, bit: u32, uv: vec2f }
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

  var nrm = vec3f(0.0);
  var tan = vec3f(0.0);
  var bitan = vec3f(0.0);
  var bitMask = 0u;

  // Explicit CCW winding definition for each face. 
  // From the outside of the cube looking at the face, 
  // tan goes right, bitan goes up.
  if faceIdx == 0u {        // +X
    nrm = vec3f(1.0, 0.0, 0.0); tan = vec3f(0.0, 0.0, -1.0); bitan = vec3f(0.0, 1.0, 0.0); bitMask = 1u;
  } else if faceIdx == 1u { // -X
    nrm = vec3f(-1.0, 0.0, 0.0); tan = vec3f(0.0, 0.0, 1.0); bitan = vec3f(0.0, 1.0, 0.0); bitMask = 2u;
  } else if faceIdx == 2u { // +Y
    nrm = vec3f(0.0, 1.0, 0.0); tan = vec3f(1.0, 0.0, 0.0); bitan = vec3f(0.0, 0.0, -1.0); bitMask = 4u;
  } else if faceIdx == 3u { // -Y
    nrm = vec3f(0.0, -1.0, 0.0); tan = vec3f(1.0, 0.0, 0.0); bitan = vec3f(0.0, 0.0, 1.0); bitMask = 8u;
  } else if faceIdx == 4u { // +Z
    nrm = vec3f(0.0, 0.0, 1.0); tan = vec3f(1.0, 0.0, 0.0); bitan = vec3f(0.0, 1.0, 0.0); bitMask = 16u;
  } else if faceIdx == 5u { // -Z
    nrm = vec3f(0.0, 0.0, -1.0); tan = vec3f(-1.0, 0.0, 0.0); bitan = vec3f(0.0, 1.0, 0.0); bitMask = 32u;
  }

  let pos = nrm + tan * tx + bitan * ty;
  return CubeV(pos, nrm, bitMask, vec2f(tx, ty));
}
"##;

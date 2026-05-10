pub const WGSL: &str = r##"
// ── CAD Solid pipeline vertex shader ──
// Reuses Pv interpolant; meshMode=1 tells fs_cad to shade it globally.
// Always renders 36-vert rasterized cubes, but uses particle formation logic for placement.

@vertex fn vs_cad(
  @builtin(vertex_index)   vi:   u32,
  @builtin(instance_index) inst: u32,
) -> Pv {
  let sp          = spheres.data[inst];
  let ro          = u.u1.xyz;
  let right       = u.u2.xyz;
  let upv         = u.u3.xyz;
  let fwd         = u.u4.xyz;
  let isOrtho     = u.u9.y > 0.5;
  let orthoHeight = distance(ro, u.u8.xyz) * 0.45;
  let asp         = u.u0.y / u.u0.z;

  let formMix   = clamp(u.u6.x, 0.0, 1.0);
  let formMode  = u.u6.y;
  let formA     = u.u6.z;
  let formScale = u.u6.w;

  var formed    = vec3f(0.0);
  var aliveForm = true;
  var halfCell  = sp.posR.w;
  var cellMask: u32 = 63u;

  // Compute structure offset if in formation
  if formMode > 0.5 {
    let side = u32(formA);
    let totalCells = side * side * side;
    if inst < totalCells {
      let ix = inst % side;
      let iy = (inst / side) % side;
      let iz = inst / (side * side);

      var m: u32 = 0u;
      if ix == side - 1u { m |=  1u; }
      if ix == 0u        { m |=  2u; }
      if iy == side - 1u { m |=  4u; }
      if iy == 0u        { m |=  8u; }
      if iz == side - 1u { m |= 16u; }
      if iz == 0u        { m |= 32u; }

      if m == 0u {
        aliveForm = false;
      } else {
        let cellSize = formScale / formA * 2.0;
        let halfCellWorld = cellSize * 0.5;
        let fx = f32(ix) - (formA - 1.0) * 0.5;
        let fy = f32(iy) - (formA - 1.0) * 0.5;
        let fz = f32(iz) - (formA - 1.0) * 0.5;
        formed   = vec3f(fx, fy, fz) * cellSize;
        halfCell = halfCellWorld;
        cellMask = m;
      }
    } else {
      aliveForm = false;
    }
  }

  // Cloud position (particle pos) vs Formed position
  var center = sp.posR.xyz;
  let objPos   = u.u8.xyz;
  let objScale = max(0.001, u.u8.w);
  let placeT   = smoothstep(0.0, 1.0, formMix);
  
  if formMode > 0.5 {
    let placedC = formed * objScale + objPos;
    center = mix(sp.posR.xyz, placedC, placeT);
    halfCell = mix(sp.posR.w, halfCell * objScale, placeT);
  }

  // Kill interior/excess instance
  if !aliveForm {
    var dead: Pv;
    dead.pos      = vec4f(0.0, 0.0, -2.0, 1.0);
    dead.quadUV   = vec2f(0.0);
    dead.color    = vec3f(0.0);
    dead.depth    = 0.0;
    dead.phase    = 0.0;
    dead.wCenter  = vec3f(0.0);
    dead.size     = 0.0;
    dead.cellMask = 0u;
    dead.halfCell = 0.0;
    dead.meshMode = 0u;
    dead.meshN    = vec3f(0.0);
    return dead;
  }

  let cv          = cubeVert(vi % 36u);
  // Kill hidden faces to optimize rasterization
  if (cellMask & cv.bit) == 0u {
    var dead: Pv;
    dead.pos      = vec4f(0.0, 0.0, -2.0, 1.0);
    dead.quadUV   = vec2f(0.0);
    dead.color    = vec3f(0.0);
    dead.depth    = 0.0;
    dead.phase    = 0.0;
    dead.wCenter  = vec3f(0.0);
    dead.size     = 0.0;
    dead.cellMask = 0u;
    dead.halfCell = 0.0;
    dead.meshMode = 0u;
    dead.meshN    = vec3f(0.0);
    return dead;
  }

  let wp          = center + cv.pos * halfCell;
  let relV        = wp - ro;
  let mvx         = dot(relV, right);
  let mvy         = dot(relV, upv);
  let mvz         = dot(relV, fwd);
  let smvz        = max(mvz, 0.001);
  let focal       = 2.414;
  let cx          = select(mvx * focal / smvz / asp, mvx / orthoHeight / asp, isOrtho);
  let cy          = select(mvy * focal / smvz,       mvy / orthoHeight,       isOrtho);
  let zNdc        = clamp(mvz / (mvz + 8.0), 0.0, 0.9999);

  var o: Pv;
  o.pos      = vec4f(cx, cy, zNdc, 1.0);
  o.quadUV   = cv.uv;
  o.color    = sp.colorP.xyz;
  o.depth    = mvz;
  o.phase    = 0.0;
  o.wCenter  = center;
  o.size     = halfCell;
  o.cellMask = cellMask;
  o.halfCell = halfCell;
  o.meshMode = 1u;
  o.meshN    = cv.nrm;
  return o;
}
"##;

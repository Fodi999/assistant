cat << 'INNER_EOF' > src/web/home/scripts/webgpu/shader/cad_vert.rs
pub const WGSL: &str = r##"
// ── CAD Solid pipeline vertex shader ──
// Renders a single clean solid primitive geometry representing the container 
// (e.g. perfect Blender default cube) without grid subdivisions.

// Helper mapping UI Euler Degrees to Rotation Matrix (XYZ)
fn euler_to_matrix(deg: vec3f) -> mat3x3f {
  let rad = deg * (3.14159265 / 180.0);
  
  let cx = cos(rad.x); let sx = sin(rad.x);
  let cy = cos(rad.y); let sy = sin(rad.y);
  let cz = cos(rad.z); let sz = sin(rad.z);

  let mX = mat3x3f(
    1.0, 0.0, 0.0,
    0.0, cx,  sx,
    0.0, -sx, cx
  );

  let mY = mat3x3f(
    cy,  0.0, -sy,
    0.0, 1.0, 0.0,
    sy,  0.0, cy
  );

  let mZ = mat3x3f(
    cz,  sz,  0.0,
    -sz, cz,  0.0,
    0.0, 0.0, 1.0
  );

  return mZ * mY * mX;
}

@vertex fn vs_cad(
  @builtin(vertex_index)   vi:   u32,
  @builtin(instance_index) inst: u32,
) -> Pv {
  // We only draw 1 solid mesh for the bounding object (inst 0)
  if inst != 0u {
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

  let sp          = spheres.data[0];
  let ro          = u.u1.xyz;
  let right       = u.u2.xyz;
  let upv         = u.u3.xyz;
  let fwd         = u.u4.xyz;
  let isOrtho     = u.u9.y > 0.5;
  let orthoHeight = distance(ro, u.u8.xyz) * 0.45;
  let asp         = u.u0.y / u.u0.z;

  let formScale   = u.u6.w;

  let objPos      = u.u8.xyz;
  let objScale    = max(0.001, u.u8.w);
  let objRot      = u.u10.xyz;
  
  let halfCell    = formScale * objScale;
  let center      = objPos;
  let cellMask    = 63u;

  let cv          = cubeVert(vi % 36u);

  let rotMat      = euler_to_matrix(objRot);
  let localPos    = rotMat * (cv.pos * halfCell);
  
  let wp          = center + localPos;
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
  o.color    = sp.colorP.xyz; // Object base color
  o.depth    = mvz;
  o.phase    = 0.0;
  o.wCenter  = center;
  o.size     = halfCell;
  o.cellMask = cellMask;
  o.halfCell = halfCell;
  o.meshMode = 1u;
  o.meshN    = rotMat * cv.nrm;
  return o;
}
"##;
INNER_EOF
